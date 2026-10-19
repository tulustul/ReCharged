//! `--soak <minutes>` (ARCHITECTURE §8): chains random races through every exit path (restart,
//! rematch, next race, next track, pause, quit during Loading / Setup / the race, load errors)
//! and, at every settled return to the menu, compares a census of the world — the set of resource
//! types, the number of entities and the number of meshes, images, standard materials and audio
//! sources — with the one taken at the first return after a session that reached the race (by
//! then lazily built caches exist). A difference that is still there after [`CENSUS_RETRIES`]
//! more censuses fails the run (exit code 1) and names what was left behind (resources, asset
//! types, and entities per component signature); otherwise it exits with success after the given
//! minutes of wall time. Every census line also logs what the census cannot see (RSS, wgpu memory,
//! all entities, glyph cache: [`memory`]).
//!
//! Asset types this crate cannot name (rc_audio's kira `AudioSource`) are counted through Bevy
//! diagnostics: every diagnostic under [`ASSET_COUNT_PREFIX`] is part of the census. Font atlas
//! textures are not: Bevy's glyph cache keeps one set per font size ever drawn (bounded by the
//! UI's sizes), so it only grows as new screens and split-screen HUD scales appear.

use std::collections::{BTreeMap, BTreeSet};

use bevy::diagnostic::DiagnosticsStore;
use bevy::ecs::resource::IsResource;
use bevy::prelude::*;
use bevy::render::renderer::RenderDevice;
use bevy::text::FontAtlasSet;
use rc_core::prelude::*;

use crate::series::{CupRun, VersusSession};
use crate::{GameCommand, PlayerCar, RaceRequest, Step};

/// Seconds the menu settles (scenes, UI) before the census.
const SETTLE_SECS: f32 = 2.0;
/// Seconds spent in a phase before the next action (min, max).
const ACT_SECS: (f32, f32) = (0.5, 5.0);
/// Actions per session; the next one quits to the menu (keeps menu checks frequent).
const MAX_ACTIONS: u32 = 4;
/// Censuses taken again, [`SETTLE_SECS`] apart, before a difference fails the run: under load
/// the menu settles later (the navy backdrop, a window-sized SVG raster, stays until the
/// showroom is ready and has faded in).
const CENSUS_RETRIES: u32 = 3;
/// Diagnostics whose path starts with this hold an asset count for the census (e.g. rc_audio's
/// `assets/audio_sources`).
const ASSET_COUNT_PREFIX: &str = "assets/";

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
    /// Number of assets per asset type (`Mesh`, `Image`, `StandardMaterial`, diagnostics).
    assets: BTreeMap<String, usize>,
    /// Images per size and format (diagnostics only: names what an image leak keeps).
    images: BTreeMap<String, usize>,
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
            assets: asset_counts(world),
            images: image_kinds(world),
        }
    }

    /// Nothing was left behind or lost.
    fn matches(&self, base: &Self) -> bool {
        self.total == base.total && self.resources == base.resources && self.assets == base.assets
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
        let kinds: BTreeSet<&String> = self.assets.keys().chain(base.assets.keys()).collect();
        for kind in kinds {
            let count = |c: &Self| c.assets.get(kind).copied().unwrap_or(0);
            if count(self) != count(base) {
                lines.push(format!("{} → {} {kind}", count(base), count(self)));
            }
        }
        let images: BTreeSet<&String> = self.images.keys().chain(base.images.keys()).collect();
        for kind in images {
            let count = |c: &Self| c.images.get(kind).copied().unwrap_or(0);
            if count(self) != count(base) {
                lines.push(format!("  {} → {} images {kind}", count(base), count(self)));
            }
        }
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

/// Asset counts of the census: meshes, images and standard materials, plus every diagnostic
/// under [`ASSET_COUNT_PREFIX`].
fn asset_counts(world: &World) -> BTreeMap<String, usize> {
    fn count<A: Asset>(world: &World) -> Option<(String, usize)> {
        let name = std::any::type_name::<A>()
            .rsplit("::")
            .next()
            .unwrap_or("?");
        world
            .get_resource::<Assets<A>>()
            .map(|assets| (name.to_owned(), assets.len()))
    }
    let mut counts: BTreeMap<String, usize> = [
        count::<Mesh>(world),
        count::<Image>(world),
        count::<StandardMaterial>(world),
    ]
    .into_iter()
    .flatten()
    .collect();
    if let Some(images) = counts.get_mut("Image") {
        *images -= font_atlases(world).len().min(*images);
    }
    if let Some(store) = world.get_resource::<DiagnosticsStore>() {
        for diagnostic in store.iter() {
            let path = diagnostic.path().as_str();
            if let (Some(kind), Some(value)) =
                (path.strip_prefix(ASSET_COUNT_PREFIX), diagnostic.value())
            {
                counts.insert(kind.to_owned(), value as usize);
            }
        }
    }
    counts
}

/// Textures of Bevy's glyph cache (left out of the census, see the module doc).
fn font_atlases(world: &World) -> BTreeSet<AssetId<Image>> {
    let atlases = world.get_resource::<FontAtlasSet>().into_iter();
    atlases
        .flat_map(|set| set.values().flatten().map(|atlas| atlas.texture.id()))
        .collect()
}

/// Images by `<width>x<height>x<layers> <format>` (font atlases left out).
fn image_kinds(world: &World) -> BTreeMap<String, usize> {
    let fonts = font_atlases(world);
    let mut kinds = BTreeMap::new();
    let images = world.get_resource::<Assets<Image>>().into_iter();
    for (_, image) in images
        .flat_map(|a| a.iter())
        .filter(|(id, _)| !fonts.contains(id))
    {
        let size = image.texture_descriptor.size;
        let kind = format!(
            "{}x{}x{} {:?}",
            size.width, size.height, size.depth_or_array_layers, image.texture_descriptor.format
        );
        *kinds.entry(kind).or_default() += 1;
    }
    kinds
}

/// What the census cannot see, logged at every menu check (QA R3): the process RSS as the kernel
/// counts it (what `ps` shows), wgpu's GPU memory, all entities (UI included) against the
/// allocated indices, and the glyph cache.
fn memory(world: &World) -> String {
    let rss = std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| rss_mb(&status))
        .map_or("RSS n/a".to_owned(), |[rss, anon, file, shmem]| {
            format!("RSS {rss} MB (anon {anon}, file {file}, shmem {shmem})")
        });
    let gpu = world
        .get_resource::<RenderDevice>()
        .and_then(|device| device.wgpu_device().generate_allocator_report())
        .map_or(String::new(), |r| {
            let mb = |bytes: u64| bytes >> 20;
            format!(
                ", wgpu {}/{} MB in {} allocations",
                mb(r.total_allocated_bytes),
                mb(r.total_reserved_bytes),
                r.allocations.len()
            )
        });
    let atlas_bytes = match (
        world.get_resource::<FontAtlasSet>(),
        world.get_resource::<Assets<Image>>(),
    ) {
        (Some(set), Some(images)) => set.total_bytes(images),
        _ => 0,
    };
    // Indices grow when despawned entities are not freed for reuse (Bevy 0.20's `DespawnOnExit`
    // never frees them: upstream U6). The census leaves UI entities out; the spawned count has them.
    let entities = world.entities();
    format!(
        "{rss}{gpu}, entities {} spawned / {} indices, {} font atlases ({} MB)",
        entities.count_spawned(),
        entities.len(),
        font_atlases(world).len(),
        atlas_bytes >> 20
    )
}

/// `[VmRSS, RssAnon, RssFile, RssShmem]` in MB from the text of `/proc/<pid>/status`.
fn rss_mb(status: &str) -> Option<[u64; 4]> {
    let mb = |key: &str| {
        status.lines().find_map(|line| {
            let kb = line.strip_prefix(key)?.trim().strip_suffix("kB")?;
            Some(kb.trim().parse::<u64>().ok()? >> 10)
        })
    };
    Some([
        mb("VmRSS:")?,
        mb("RssAnon:")?,
        mb("RssFile:")?,
        mb("RssShmem:")?,
    ])
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
    /// Censuses that differed from the baseline since the last equal one.
    retries: u32,
}

impl Soak {
    fn new(minutes: f32, seed: u64) -> Self {
        Self {
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
            retries: 0,
        }
    }

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
    commands.insert_resource(Soak::new(minutes, seed));
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
            match check_menu(world, soak) {
                Check::Equal => {}
                Check::Again => {
                    soak.schedule(now, (SETTLE_SECS, SETTLE_SECS));
                    return;
                }
                Check::Differs => {
                    world.write_message(AppExit::error());
                    return;
                }
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

/// Outcome of a menu census.
enum Check {
    /// Equal to the baseline (or the baseline was just taken).
    Equal,
    /// Different: census again later (the menu may not have settled yet).
    Again,
    /// Still different after [`CENSUS_RETRIES`] retries: something was left behind.
    Differs,
}

/// Census of a settled menu against the baseline.
fn check_menu(world: &mut World, soak: &mut Soak) -> Check {
    if !soak.raced {
        return Check::Equal; // no session reached the race yet: lazily built caches are missing
    }
    let census = Census::take(world);
    let Some(base) = &soak.baseline else {
        info!(
            "soak baseline: {} entities, {} resources, assets {:?}; {}",
            census.entities.values().sum::<usize>(),
            census.resources.len(),
            census.assets,
            memory(world)
        );
        soak.baseline = Some(census);
        return Check::Equal;
    };
    if census.matches(base) {
        soak.checks += 1;
        soak.retries = 0;
        info!(
            "soak check {} ({} races): {}",
            soak.checks,
            soak.races,
            memory(world)
        );
        return Check::Equal;
    }
    if soak.retries < CENSUS_RETRIES {
        soak.retries += 1;
        info!(
            "soak: the menu is not settled yet ({}), census again in {SETTLE_SECS} s",
            census.diff(base).join("; ")
        );
        return Check::Again;
    }
    error!(
        "soak FAILED after {} races (seed {}): the menu differs from the baseline:\n  {}",
        soak.races,
        soak.seed,
        census.diff(base).join("\n  ")
    );
    Check::Differs
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

    #[test]
    fn a_menu_difference_must_outlast_the_retries_to_fail() {
        let mut world = World::new();
        let mut soak = Soak::new(1.0, 7);
        soak.raced = true;
        let check = |world: &mut World, soak: &mut Soak| match check_menu(world, soak) {
            Check::Equal => "equal",
            Check::Again => "again",
            Check::Differs => "differs",
        };
        assert_eq!(check(&mut world, &mut soak), "equal", "the baseline");
        let late = world.spawn(Transform::default()).id();
        assert_eq!(check(&mut world, &mut soak), "again");
        world.despawn(late);
        assert_eq!(check(&mut world, &mut soak), "equal", "settled on a retry");
        world.spawn(Transform::default());
        for _ in 0..CENSUS_RETRIES {
            assert_eq!(check(&mut world, &mut soak), "again");
        }
        assert_eq!(check(&mut world, &mut soak), "differs");
    }

    #[test]
    fn rss_is_read_from_proc_status_in_mb() {
        let status = "Name:\trecharged\nVmRSS:\t 3471836 kB\nRssAnon:\t 2007724 kB\n\
                      RssFile:\t  871964 kB\nRssShmem:\t  592148 kB\nThreads:\t48\n";
        assert_eq!(rss_mb(status), Some([3390, 1960, 851, 578]));
        assert_eq!(rss_mb("VmRSS:\t1024 kB\n"), None, "every field is needed");
    }

    #[test]
    fn the_census_counts_assets_including_diagnostic_counts() {
        use bevy::diagnostic::{Diagnostic, DiagnosticMeasurement, DiagnosticPath};
        let mut world = World::new();
        world.init_resource::<Assets<Mesh>>();
        let path = DiagnosticPath::const_new("assets/audio_sources");
        let mut store = DiagnosticsStore::default();
        store.add(Diagnostic::new(path.clone()));
        world.insert_resource(store);
        let set_sources = |world: &mut World, value: f64| {
            let mut store = world.resource_mut::<DiagnosticsStore>();
            let diagnostic = store.get_mut(&path).expect("registered");
            diagnostic.add_measurement(DiagnosticMeasurement {
                time: bevy::platform::time::Instant::now(),
                value,
            });
        };
        set_sources(&mut world, 63.0);
        let base = Census::take(&mut world);
        assert_eq!(base.assets.get("Mesh"), Some(&0));
        assert_eq!(base.assets.get("audio_sources"), Some(&63));
        let mesh = world
            .resource_mut::<Assets<Mesh>>()
            .add(Mesh::from(Cuboid::default()));
        set_sources(&mut world, 64.0);
        let now = Census::take(&mut world);
        assert!(
            !now.matches(&base),
            "a kept mesh and audio source are leaks"
        );
        assert_eq!(
            now.diff(&base)[1..],
            ["0 → 1 Mesh", "63 → 64 audio_sources"]
        );
        world.resource_mut::<Assets<Mesh>>().remove(&mesh);
        set_sources(&mut world, 63.0);
        assert!(Census::take(&mut world).matches(&base));
    }
}
