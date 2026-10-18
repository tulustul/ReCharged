//! Shared helpers of the rc_ai integration tests: headless AI races with every sim plugin.
#![allow(dead_code)]

use bevy::ecs::message::MessageCursor;
use bevy::prelude::*;
use rc_core::prelude::*;
use rc_testkit::*;
use rc_track::Track;

/// A headless app with every simulation plugin and the catalog.
pub fn race_app() -> App {
    let mut app = headless_app(SimPlugins);
    let catalog = Catalog::load(&assets_dir()).expect("catalog loads");
    app.insert_resource(catalog);
    app
}

/// A field of `n` AI racers (the catalog's rivals in order, then neutral drivers) of
/// `difficulty` in cars of `class`.
pub fn ai_field(app: &App, n: usize, difficulty: Difficulty, class: CarClass) -> Vec<RacerSetup> {
    let catalog = app.world().resource::<Catalog>();
    (0..n)
        .map(|i| {
            let (rival, name, car) = match catalog.rivals.get(i) {
                Some(r) => (
                    r.id.clone(),
                    r.name.clone(),
                    r.cars.for_class(class).clone(),
                ),
                None => {
                    let r = &catalog.rivals[i % catalog.rivals.len()];
                    (
                        RivalId::new("neutral"),
                        format!("Neutral {i}"),
                        r.cars.for_class(class).clone(),
                    )
                }
            };
            RacerSetup {
                name,
                car,
                controller: Controller::Ai { rival, difficulty },
                ..default()
            }
        })
        .collect()
}

/// A race on `track` with `racers` (items on with `AI_ITEMS=1`).
pub fn config(track: &str, racers: Vec<RacerSetup>, laps: u8, seed: u64) -> RaceConfig {
    RaceConfig {
        mode: GameMode::QuickRace,
        variant: TrackVariant::forward(track),
        laps,
        racers,
        items: std::env::var("AI_ITEMS").is_ok_and(|v| v == "1"),
        ai_catch_up: false,
        seed,
        ..default()
    }
}

/// Reference lap time of the track: its speed profile at target speed (rc_track's plot tool).
pub fn reference_lap(track: &Track) -> f32 {
    (0..(track.length() / 0.5) as usize)
        .map(|k| 0.5 / track.target_speed_at(k as f32 * 0.5).max(1.0))
        .sum()
}

/// One racer's result.
#[derive(Debug, Clone)]
pub struct Outcome {
    pub name: String,
    pub car: String,
    pub finished: bool,
    pub laps: Vec<f32>,
    pub total: f32,
    pub respawns: u32,
}

/// The outcome of a whole race.
#[derive(Debug, Clone)]
pub struct RaceRun {
    /// Items used, by kind, and hits landed (blocked ones counted apart).
    pub items_used: Vec<(ItemKind, u32)>,
    pub hits: (u32, u32),
    pub reference_lap: f32,
    pub laps: u8,
    pub racers: Vec<Outcome>,
    /// Final positions (racer index), by rank.
    pub order: Vec<usize>,
}

impl RaceRun {
    /// Prints a table (tests run with `--nocapture`).
    pub fn print(&self, title: &str) {
        if !self.items_used.is_empty() {
            println!(
                "items used {:?}, hits {} (+{} blocked)",
                self.items_used, self.hits.0, self.hits.1
            );
        }
        println!(
            "== {title}: reference lap {:.2} s × {} laps = {:.1} s",
            self.reference_lap,
            self.laps,
            self.reference_lap * self.laps as f32
        );
        for r in &self.racers {
            let laps: Vec<String> = r.laps.iter().map(|l| format!("{l:.2}")).collect();
            println!(
                "  {:<12} {:<9} {} total {:6.2} s ({:.2}×) laps [{}] respawns {}",
                r.name,
                r.car,
                if r.finished { "fin" } else { "DNF" },
                r.total,
                r.total / (self.reference_lap * self.laps as f32),
                laps.join(", "),
                r.respawns
            );
        }
    }

    /// Mean lap time of the finishers (s).
    pub fn mean_lap(&self) -> f32 {
        let laps: Vec<f32> = self
            .racers
            .iter()
            .flat_map(|r| r.laps.iter().copied())
            .collect();
        laps.iter().sum::<f32>() / laps.len().max(1) as f32
    }
}

/// Runs `config` from the grid (intro and countdown included) until every racer finished or
/// `max_secs` of race time passed.
pub fn run_race(app: &mut App, config: RaceConfig, max_secs: f32) -> RaceRun {
    run_race_traced(app, config, max_secs, |_, _| {})
}

/// [`run_race`] calling `trace(app, race_tick)` after every racing tick.
pub fn run_race_traced(
    app: &mut App,
    config: RaceConfig,
    max_secs: f32,
    mut trace: impl FnMut(&mut App, u32),
) -> RaceRun {
    let laps = config.laps;
    load_race(app, config).expect("race loads");
    let mut respawned = MessageCursor::<CarRespawned>::default();
    let mut item_events = MessageCursor::<ItemEvent>::default();
    let mut hit_events = MessageCursor::<HitEvent>::default();
    let mut items_used: Vec<(ItemKind, u32)> = Vec::new();
    let mut hits = (0, 0);
    let mut respawns: Vec<(Entity, u32)> = Vec::new();
    let mut count = |app: &App, respawns: &mut Vec<(Entity, u32)>| {
        let messages = app.world().resource::<Messages<CarRespawned>>();
        for m in respawned.read(messages) {
            if m.kind == RespawnKind::ToTrack {
                match respawns.iter_mut().find(|(e, _)| *e == m.car) {
                    Some((_, n)) => *n += 1,
                    None => respawns.push((m.car, 1)),
                }
            }
        }
    };
    for _ in 0..secs_to_ticks(10.0) {
        step(app, 1);
        if race_phase(app) == Some(RacePhase::Racing) {
            break;
        }
    }
    let ticks = secs_to_ticks(max_secs);
    for tick in 0..ticks {
        step(app, 1);
        count(app, &mut respawns);
        for e in item_events.read(app.world().resource::<Messages<ItemEvent>>()) {
            if let ItemEventKind::Used(kind) = e.kind {
                match items_used.iter_mut().find(|(k, _)| *k == kind) {
                    Some((_, n)) => *n += 1,
                    None => items_used.push((kind, 1)),
                }
            }
        }
        for h in hit_events.read(app.world().resource::<Messages<HitEvent>>()) {
            if h.blocked {
                hits.1 += 1;
            } else {
                hits.0 += 1;
            }
        }
        trace(app, tick);
        let all_done = {
            let world = app.world_mut();
            let mut q = world.query::<&RaceProgress>();
            q.iter(world).all(|p| p.finished)
        };
        if all_done || race_phase(app) == Some(RacePhase::Finished) {
            break;
        }
    }
    let reference = reference_lap(app.world().resource::<Track>());
    let world = app.world_mut();
    let mut q = world.query::<(Entity, &Racer, &RaceProgress)>();
    let mut rows: Vec<(usize, u8, Outcome)> = q
        .iter(world)
        .map(|(e, racer, p)| {
            let laps: Vec<f32> = p.lap_times.iter().map(|&t| ticks_to_secs(t)).collect();
            (
                racer.index,
                p.rank,
                Outcome {
                    name: racer.name.clone(),
                    car: racer.car.to_string(),
                    finished: p.finished,
                    total: laps.iter().sum(),
                    laps,
                    respawns: respawns
                        .iter()
                        .find(|(x, _)| *x == e)
                        .map_or(0, |(_, n)| *n),
                },
            )
        })
        .collect();
    rows.sort_by_key(|r| r.0);
    let mut order: Vec<(u8, usize)> = rows.iter().map(|r| (r.1, r.0)).collect();
    order.sort();
    RaceRun {
        items_used,
        hits,
        reference_lap: reference,
        laps,
        order: order.into_iter().map(|(_, i)| i).collect(),
        racers: rows.into_iter().map(|r| r.2).collect(),
    }
}

/// A difficulty by its name (`Rookie`, `Pro`, `Elite`, `ElitePlus`).
pub fn difficulty_named(name: &str) -> Difficulty {
    match name {
        "Rookie" => Difficulty::Rookie,
        "Pro" => Difficulty::Pro,
        "ElitePlus" => Difficulty::ElitePlus,
        _ => Difficulty::Elite,
    }
}

/// Prints racer `index`'s state every `AI_TRACE_EVERY` ticks (default 6).
pub fn trace_racer(app: &mut App, index: usize, tick: u32) {
    let every: u32 = std::env::var("AI_TRACE_EVERY")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(6);
    if !tick.is_multiple_of(every) {
        return;
    }
    let world = app.world_mut();
    let track = world.resource::<Track>().clone();
    if std::env::var("AI_TRACE_HAZARDS").is_ok() && tick.is_multiple_of(60) {
        let mut h = world.query::<(&Hazard, &Transform, Option<&ItemRadius>)>();
        for (hz, tf, r) in h.iter(world) {
            let tp = track.project(tf.translation, None);
            println!(
                "   hazard {:?} s {:.1} lat {:.2} r {:?}",
                hz.kind,
                tp.s,
                tp.lateral,
                r.map(|r| r.0)
            );
        }
    }
    let mut q = world.query::<(
        &Racer,
        &rc_ai::TrackView,
        &VehicleTelemetry,
        &CarControl,
        &rc_ai::AiBrain,
        &RaceProgress,
    )>();
    for (racer, view, tel, c, brain, progress) in q.iter(world) {
        if racer.index != index {
            continue;
        }
        println!(
            "r{} t {:6.2} lap {} s {:6.1} lat {:5.2} line {:5.2} plan {:5.1} h {:5.2} v {:5.1} slip {:5.1} up {:4.2} gw {} | th {:.2} br {:.2} st {:5.2} dr {} bo {} use {} rs {} | lane {:5.2} {:?}",
            racer.index,
            tick as f32 / 60.0,
            progress.lap,
            view.s,
            view.lateral,
            track.racing_line_offset_at(view.s),
            brain.profile.corner_at(view.s).min(99.0),
            view.height,
            tel.forward_speed,
            tel.slip_angle.to_degrees(),
            tel.up_dot,
            tel.grounded_wheels,
            c.throttle,
            c.brake,
            c.steer,
            c.drift as u8,
            c.boost as u8,
            c.use_item as u8,
            c.reset_held as u8,
            brain.lane,
            brain.recovery,
        );
    }
}
