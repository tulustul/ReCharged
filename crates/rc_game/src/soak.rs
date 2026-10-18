//! `--soak <minutes>` (ARCHITECTURE §8): chains random races through every exit path (restart,
//! rematch, next race, next track, pause, quit during Loading / Setup / the race, load errors)
//! and, at every settled return to the menu, compares a census of the world — the set of resource
//! types and the number of entities — with the one taken at the first return after a session that
//! reached the race (by then lazily built caches exist). A difference fails the run (exit code 1)
//! and names what was left behind (resources, and entities per component signature); otherwise it
//! exits with success after the given minutes of wall time.

use std::collections::{BTreeMap, BTreeSet};

use bevy::ecs::resource::IsResource;
use bevy::prelude::*;
use rc_core::prelude::*;

use crate::series::{CupRun, VersusSession};
use crate::{GameCommand, PlayerCar, RaceRequest, Step};

/// Seconds the menu settles (scenes, UI) before the census.
const SETTLE_SECS: f32 = 2.0;
/// Seconds spent in a phase before the next action (min, max).
const ACT_SECS: (f32, f32) = (0.5, 5.0);
/// Actions per session; the next one quits to the menu (keeps menu checks frequent).
const MAX_ACTIONS: u32 = 4;

/// What a settled menu contains. UI nodes, text spans, React elements and gamepads come and go
/// legitimately (the screen React shows, plugged pads) and are not counted; neither are Bevy's state bookkeeping
/// (`PreviousState<S>` stays after a sub-state's first transition) and the cup / Versus series,
/// which outlive sessions by design.
#[derive(Debug, Clone, Default, PartialEq)]
struct Census {
    /// Counted entities.
    total: usize,
    /// Entity count per sorted component-name signature (diagnostics: persistent entities may
    /// gain components, e.g. the window's picking state).
    entities: BTreeMap<String, usize>,
    /// Resource type names.
    resources: BTreeSet<String>,
}

type Counted = (
    Without<IsResource>,
    Without<Node>,
    Without<TextSpan>,
    Without<Gamepad>,
);

impl Census {
    fn take(world: &mut World) -> Self {
        let mut entities = BTreeMap::new();
        let mut total = 0;
        let mut query = world.query_filtered::<EntityRef, Counted>();
        let components = world.components();
        for entity in query.iter(world) {
            let mut names: Vec<String> = entity
                .archetype()
                .components()
                .iter()
                .filter_map(|&id| components.get_name(id))
                .map(|name| name.to_string())
                .collect();
            if names.iter().any(|n| n.starts_with("bevy_react")) {
                continue; // React's elements (e.g. SVG shapes): the screen shown decides them
            }
            names.sort();
            *entities.entry(names.join(" + ")).or_default() += 1;
            total += 1;
        }
        let series = [
            std::any::type_name::<CupRun>(),
            std::any::type_name::<VersusSession>(),
        ];
        let resources = world
            .iter_resources()
            .map(|(_, info, _)| info.name().to_string())
            .filter(|name| !series.contains(&name.as_str()) && !name.starts_with("bevy_state::"))
            .collect();
        Self {
            total,
            entities,
            resources,
        }
    }

    /// Nothing was left behind or lost.
    fn matches(&self, base: &Self) -> bool {
        self.total == base.total && self.resources == base.resources
    }

    /// What differs from `base`, one line per difference.
    fn diff(&self, base: &Self) -> Vec<String> {
        let mut lines = vec![format!("{} → {} entities", base.total, self.total)];
        lines.extend(
            self.resources
                .symmetric_difference(&base.resources)
                .map(|r| match self.resources.contains(r) {
                    true => format!("new resource {r}"),
                    false => format!("missing resource {r}"),
                }),
        );
        let signatures: BTreeSet<&String> =
            self.entities.keys().chain(base.entities.keys()).collect();
        for signature in signatures {
            let count = |c: &Self| c.entities.get(signature).copied().unwrap_or(0);
            let (then, now) = (count(base), count(self));
            if now != then {
                lines.push(format!("{then} → {now} entities [{signature}]"));
            }
        }
        lines
    }
}

#[derive(Resource, Debug)]
struct Soak {
    end_secs: f32,
    seed: u64,
    draws: u64,
    act_at: f32,
    /// State and phase seen last frame.
    seen: (Option<AppState>, Option<RacePhase>),
    baseline: Option<Census>,
    /// A session reached the race (the baseline is taken after one did).
    raced: bool,
    menus: u32,
    /// Menu returns compared with the baseline.
    checks: u32,
    races: u32,
    /// Actions in the current session.
    actions: u32,
}

impl Soak {
    fn unit(&mut self) -> f32 {
        self.draws += 1;
        rng::unit(self.seed, self.draws, RngStream::RaceSetup, 0)
    }

    fn pick<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        let i = (self.unit() * items.len() as f32) as usize;
        items.get(i.min(items.len().saturating_sub(1)))
    }

    fn schedule(&mut self, now: f32, (min, max): (f32, f32)) {
        self.act_at = now + min + (max - min) * self.unit();
    }
}

pub(crate) fn build(app: &mut App) {
    app.add_systems(OnEnter(AppState::Boot), start).add_systems(
        Update,
        drive.run_if(resource_exists::<Soak>).in_set(Step::Signals),
    );
}

fn start(automation: Option<Res<AutomationConfig>>, mut commands: Commands) {
    let Some(minutes) = automation.as_ref().and_then(|a| a.soak_minutes) else {
        return;
    };
    let seed = automation
        .and_then(|a| a.soak_seed)
        .unwrap_or_else(|| crate::save::unix_time().as_nanos() as u64);
    info!("--soak {minutes} min (seed {seed}; replay with --soak-seed {seed})");
    commands.insert_resource(Soak {
        end_secs: minutes * 60.0,
        seed,
        draws: 0,
        act_at: 0.0,
        seen: (None, None),
        baseline: None,
        raced: false,
        menus: 0,
        checks: 0,
        races: 0,
        actions: 0,
    });
}

fn drive(world: &mut World) {
    world.resource_scope(|world, mut soak: Mut<Soak>| step(world, &mut soak));
}

fn step(world: &mut World, soak: &mut Soak) {
    let now = world.resource::<Time<Real>>().elapsed_secs();
    let state = *world.resource::<State<AppState>>().get();
    let phase = world.get_resource::<State<RacePhase>>().map(|p| *p.get());
    if soak.seen != (Some(state), phase) {
        soak.seen = (Some(state), phase);
        soak.raced |= state == AppState::InRace;
        if state == AppState::Menu {
            soak.menus += 1;
            soak.schedule(now, (SETTLE_SECS, SETTLE_SECS));
        } else {
            soak.schedule(now, ACT_SECS);
        }
        if phase == Some(RacePhase::Setup) && soak.unit() < 0.1 {
            info!("soak: quit during Setup");
            world.write_message(GameCommand::QuitToMenu);
        }
        return;
    }
    if now < soak.act_at {
        return;
    }
    match state {
        AppState::Menu => {
            if !check_menu(world, soak) {
                world.write_message(AppExit::error());
                return;
            }
            if now >= soak.end_secs && soak.checks > 0 {
                info!(
                    "soak passed: {} races, {} menu returns checked",
                    soak.races, soak.checks
                );
                world.write_message(AppExit::Success);
                return;
            }
            let catalog = world.get_resource::<Catalog>();
            let request = match (catalog, world.get_resource::<UserPaths>()) {
                (Some(catalog), Some(paths)) => Some(random_request(soak, catalog, paths)),
                _ => None,
            };
            if let Some(request) = request {
                soak.races += 1;
                soak.actions = 0;
                world.write_message(GameCommand::StartRace(request));
            }
            soak.schedule(now, (SETTLE_SECS, SETTLE_SECS));
        }
        AppState::Loading => {
            if soak.unit() < 0.1 {
                info!("soak: quit while loading");
                world.write_message(GameCommand::QuitToMenu);
            }
            soak.schedule(now, ACT_SECS);
        }
        AppState::InRace => {
            let paused = world
                .get_resource::<State<PauseState>>()
                .is_some_and(|p| *p.get() == PauseState::Paused);
            let command = match paused {
                true => Some(GameCommand::Resume),
                false => {
                    let config = world.get_resource::<RaceConfig>();
                    race_action(soak, config, now >= soak.end_secs)
                }
            };
            if let Some(command) = command {
                info!("soak: {command:?}");
                world.write_message(command);
            }
            soak.schedule(now, ACT_SECS);
        }
        AppState::Boot | AppState::Unloading => {}
    }
}

/// Census of a settled menu against the baseline; `false` = something was left behind.
fn check_menu(world: &mut World, soak: &mut Soak) -> bool {
    if !soak.raced {
        return true; // no session reached the race yet: lazily built caches are missing
    }
    let census = Census::take(world);
    let Some(base) = &soak.baseline else {
        info!(
            "soak baseline: {} entities, {} resources",
            census.entities.values().sum::<usize>(),
            census.resources.len()
        );
        soak.baseline = Some(census);
        return true;
    };
    if census.matches(base) {
        soak.checks += 1;
        return true;
    }
    error!(
        "soak FAILED after {} races (seed {}): the menu differs from the baseline:\n  {}",
        soak.races,
        soak.seed,
        census.diff(base).join("\n  ")
    );
    false
}

/// A random action during a race (`None` = keep racing); quits once the session had its
/// actions or the time is up.
fn race_action(soak: &mut Soak, config: Option<&RaceConfig>, time_up: bool) -> Option<GameCommand> {
    soak.actions += 1;
    if time_up || soak.actions > MAX_ACTIONS {
        return Some(GameCommand::QuitToMenu);
    }
    let cup = config.is_some_and(|c| matches!(c.mode, GameMode::Championship { .. }));
    Some(match soak.unit() {
        r if r < 0.25 => GameCommand::Restart,
        r if r < 0.40 => GameCommand::Pause,
        r if r < 0.55 => GameCommand::NextRace,
        r if r < 0.65 && !cup => GameCommand::NextTrack(config?.variant.clone()),
        r if r < 0.72 => GameCommand::Rematch,
        r if r < 0.90 => GameCommand::QuitToMenu,
        _ => return None,
    })
}

fn random_request(soak: &mut Soak, catalog: &Catalog, paths: &UserPaths) -> RaceRequest {
    let built = |t: &&TrackInfo| {
        paths
            .assets_dir
            .join(format!("data/tracks/{}.ron", t.id))
            .is_file()
    };
    let mut tracks: Vec<&TrackInfo> = catalog.tracks.iter().filter(built).collect();
    if tracks.is_empty() {
        tracks = catalog.tracks.iter().collect(); // no track data yet: exercises load errors
    }
    let cars: Vec<&CarId> = catalog.cars.iter().map(|c| &c.id).collect();
    let car = |soak: &mut Soak| soak.pick(&cars).map(|c| (*c).clone());
    let track = soak.pick(&tracks).map(|t| t.id.clone()).unwrap_or_default();
    let mut request = RaceRequest {
        variant: TrackVariant {
            track,
            reverse: soak.unit() < 0.5,
            mirror: soak.unit() < 0.5,
        },
        car: car(soak),
        players: 1,
        ai: Some((soak.unit() * 8.0) as u8),
        seed: Some(u64::from(soak.unit().to_bits())),
        ..default()
    };
    let roll = soak.unit();
    request.mode = if roll < 0.2 {
        GameMode::TimeTrial
    } else if roll < 0.4 {
        let players = 2 + (soak.unit() * 3.0) as u8;
        request.humans = (0..players)
            .map(|slot| PlayerCar {
                slot: PlayerSlot(slot),
                car: car(soak).unwrap_or_default(),
                paint: PaintId(slot),
            })
            .collect();
        GameMode::Versus
    } else if roll < 0.6
        && let Some(cup) = soak.pick(&catalog.cups)
    {
        // Cups race on their own tracks, which may not be built yet: load errors are a path too.
        GameMode::Championship {
            cup: cup.id.clone(),
            race_index: 0,
        }
    } else {
        GameMode::QuickRace
    };
    request
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn census_differences_name_resources_and_entity_kinds() {
        let mut base = Census::default();
        base.resources.insert("A".into());
        base.entities.insert("Window".into(), 1);
        base.total = 1;
        let mut now = base.clone();
        now.entities = [("Window + Hovered".to_string(), 1)].into();
        assert!(
            now.matches(&base),
            "a persistent entity gaining a component is no leak"
        );
        now.resources.insert("B".into());
        now.resources.remove("A");
        now.entities.insert("Leak".into(), 1);
        now.total = 2;
        assert!(!now.matches(&base));
        assert_eq!(
            now.diff(&base),
            [
                "1 → 2 entities",
                "missing resource A",
                "new resource B",
                "0 → 1 entities [Leak]",
                "1 → 0 entities [Window]",
                "0 → 1 entities [Window + Hovered]",
            ]
        );
    }
}
