//! Shared headless world for rc_game's integration tests: a private data dir per test, a dummy
//! loader (session entities, `TrackSummary`), and placeholder racers standing in for rc_race —
//! spawned in `OnEnter(RaceAttempt)`, finishing after `FinishAfter` race ticks in `FinishOrder` —
//! plus a stand-in for rc_race's phase driver (Intro and Countdown on 3 s timers).

#![allow(dead_code)] // each test binary uses a subset

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use bevy::prelude::*;
use rc_core::prelude::*;
use rc_game::{GameCommand, GameFlowPlugin, GameNotice, RaceRequest};
use rc_testkit::*;

/// Session entities the dummy loader spawns.
pub const SESSION_THINGS: usize = 3;
/// Default track geometry hash.
pub const HASH: u64 = 42;

/// A session-scoped entity from the dummy loader.
#[derive(Component)]
pub struct SessionThing;

/// Loads and attempts so far.
#[derive(Resource, Default)]
pub struct Counters {
    pub loads: u32,
    pub attempts: u32,
    pub phases: Vec<RacePhase>,
    /// Fixed ticks of `SimSet::Race` (gated by pause).
    pub sim_ticks: u32,
}

/// Makes the dummy loader fail.
#[derive(Resource)]
pub struct FailLoading;

/// Geometry hash the dummy track reports.
#[derive(Resource)]
pub struct Geometry(pub u64);

/// Placeholder racers finish after this many race ticks (`None` = they never finish).
#[derive(Resource)]
pub struct FinishAfter(pub Option<u32>);

/// Grid indices in finishing order (empty = grid order).
#[derive(Resource, Default)]
pub struct FinishOrder(pub Vec<usize>);

/// Every `GameNotice` / `PlayerDropped` sent so far.
#[derive(Resource, Default)]
pub struct Seen {
    pub notices: Vec<GameNotice>,
    pub dropped: Vec<PlayerSlot>,
}

#[derive(Resource, Default)]
struct RaceClock(u32);

/// Phase stand-in state of the session (inserted on `OnEnter(Loading)`).
#[derive(Resource, Default)]
struct PhaseClock {
    /// Attempts started in this session (Intro only before the first).
    attempts: u32,
    /// Fixed ticks spent in the current phase.
    ticks: u32,
}

/// Intro and countdown length of the phase stand-in (ticks, 3 s each).
const PHASE_TICKS: u32 = 180;

/// A fresh, private user-data directory for one test.
pub fn data_dir(name: &str) -> PathBuf {
    static N: AtomicU32 = AtomicU32::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("rc_game-it-{}-{name}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// The game flow on `dir`, with `automation` (if any) and placeholder racers.
pub fn app_in(dir: &Path, automation: Option<AutomationConfig>, finish_after: Option<u32>) -> App {
    let dir = dir.to_owned();
    let setup = move |app: &mut App| {
        app.insert_resource(UserPaths {
            assets_dir: assets_dir(),
            data_dir: dir.clone(),
        })
        .insert_resource(FinishAfter(finish_after));
        if let Some(automation) = automation.clone() {
            app.insert_resource(automation);
        }
    };
    headless_app((setup, GameFlowPlugin, dummy_world))
}

fn dummy_world(app: &mut App) {
    LoadingTasks::register(app, "dummy");
    app.init_resource::<Counters>()
        .init_resource::<FinishOrder>()
        .init_resource::<Seen>()
        .init_resource::<RaceClock>()
        .insert_resource(Geometry(HASH))
        .add_systems(OnEnter(AppState::Loading), dummy_loader)
        .add_systems(OnEnter(AppState::Loading), |mut c: Commands| {
            c.insert_resource(PhaseClock::default());
        })
        .add_systems(OnEnter(RaceAttempt), spawn_grid)
        .add_systems(OnEnter(RaceAttempt), |clock: Option<ResMut<PhaseClock>>| {
            if let Some(mut clock) = clock {
                clock.attempts += 1;
            }
        })
        .add_systems(
            OnEnter(RacePhase::Setup),
            |mut c: Commands, mut clock: ResMut<RaceClock>| {
                clock.0 = 0;
                c.remove_resource::<RaceResults>();
            },
        )
        .add_systems(OnEnter(AppState::Unloading), |mut c: Commands| {
            c.remove_resource::<RaceResults>();
            c.remove_resource::<TrackSummary>();
            c.remove_resource::<PhaseClock>();
        })
        .add_systems(
            FixedUpdate,
            (
                |mut c: ResMut<Counters>| c.sim_ticks += 1,
                placeholder_phases,
                placeholder_race,
            )
                .chain()
                .in_set(SimSet::Race),
        )
        .add_systems(Update, record.after(FlowSet)); // sees the frame's notices
}

fn dummy_loader(
    mut commands: Commands,
    mut counters: ResMut<Counters>,
    (fail, geometry): (Option<Res<FailLoading>>, Res<Geometry>),
    mut done: MessageWriter<LoadingDone>,
    mut failed: MessageWriter<LoadingFailed>,
) {
    counters.loads += 1;
    for _ in 0..SESSION_THINGS {
        commands.spawn((SessionThing, DespawnOnExit(RaceSession)));
    }
    if fail.is_some() {
        failed.write(LoadingFailed {
            key: "dummy",
            error: "tracks/broken.ron: bad data".into(),
        });
    } else {
        commands.insert_resource(TrackSummary {
            geometry_hash: geometry.0,
            ..default()
        });
        done.write(LoadingDone("dummy"));
    }
}

fn spawn_grid(mut commands: Commands, config: Res<RaceConfig>, mut counters: ResMut<Counters>) {
    counters.attempts += 1;
    for (index, setup) in config.racers.iter().enumerate() {
        let mut racer = commands.spawn((
            Racer {
                index,
                name: setup.name.clone(),
                car: setup.car.clone(),
                paint: setup.paint,
                color: setup.color,
            },
            DespawnOnExit(RaceAttempt),
        ));
        if let Controller::Human { slot, .. } = setup.controller {
            racer.insert(LocalPlayer(slot));
        }
    }
}

/// rc_race stand-in for the phase driver: Setup → Intro (first attempt of a session) → Countdown
/// → Racing, counted in fixed ticks (so pausing freezes it).
fn placeholder_phases(
    phase: Option<Res<State<RacePhase>>>,
    clock: Option<ResMut<PhaseClock>>,
    mut next: ResMut<NextState<RacePhase>>,
) {
    let (Some(phase), Some(mut clock)) = (phase, clock) else {
        return;
    };
    clock.ticks = if phase.is_changed() {
        1
    } else {
        clock.ticks + 1
    };
    let done = clock.ticks >= PHASE_TICKS;
    let to = match phase.get() {
        RacePhase::Setup if clock.attempts == 0 => Some(RacePhase::Intro),
        RacePhase::Setup => Some(RacePhase::Countdown),
        RacePhase::Intro if done => Some(RacePhase::Countdown),
        RacePhase::Countdown if done => Some(RacePhase::Racing),
        _ => None,
    };
    if let Some(to) = to {
        next.set_if_different(to);
    }
}

/// rc_race stand-in: everyone finishes at once in `FinishOrder`; results and phase as rc_race
/// writes them.
fn placeholder_race(
    phase: Option<Res<State<RacePhase>>>,
    (mut clock, finish, order, geometry): (
        ResMut<RaceClock>,
        Res<FinishAfter>,
        Res<FinishOrder>,
        Res<Geometry>,
    ),
    config: Option<Res<RaceConfig>>,
    mut next: ResMut<NextState<RacePhase>>,
    mut commands: Commands,
) {
    if phase.is_none_or(|p| *p.get() != RacePhase::Racing) {
        return;
    }
    clock.0 += 1;
    let (Some(after), Some(config)) = (finish.0, config) else {
        return;
    };
    if clock.0 != after {
        return;
    }
    let order: Vec<usize> = match order.0.is_empty() {
        true => (0..config.racers.len()).collect(),
        false => order.0.clone(),
    };
    let entries = order
        .iter()
        .enumerate()
        .map(|(i, &index)| {
            let r = &config.racers[index];
            RaceResultEntry {
                index,
                name: r.name.clone(),
                car: r.car.clone(),
                slot: match r.controller {
                    Controller::Human { slot, .. } => Some(slot),
                    Controller::Ai { .. } => None,
                },
                rank: i as u8 + 1,
                finished: true,
                time_ticks: 3000 + 60 * i as u32,
                best_lap_ticks: Some(900 + 30 * i as u32),
            }
        })
        .collect();
    let ghost = (config.mode == GameMode::TimeTrial).then(|| GhostData {
        car: config.racers[0].car.clone(),
        paint: config.racers[0].paint,
        time_ticks: 3000,
        geometry_hash: geometry.0,
        samples: vec![GhostSample::default(); 4],
    });
    commands.insert_resource(RaceResults {
        entries,
        ghost,
        geometry_hash: geometry.0,
    });
    next.set(RacePhase::Finished);
}

fn record(
    mut phases: MessageReader<StateTransitionEvent<RacePhase>>,
    mut notices: MessageReader<GameNotice>,
    mut dropped: MessageReader<PlayerDropped>,
    mut counters: ResMut<Counters>,
    mut seen: ResMut<Seen>,
) {
    counters
        .phases
        .extend(phases.read().filter_map(|e| e.entered));
    seen.notices.extend(notices.read().cloned());
    seen.dropped.extend(dropped.read().map(|d| d.slot));
}

pub fn send(app: &mut App, command: GameCommand) {
    app.world_mut().write_message(command);
}

pub fn start(app: &mut App, request: RaceRequest) {
    send(app, GameCommand::StartRace(request));
}

pub fn counters(app: &App) -> &Counters {
    app.world().resource::<Counters>()
}

pub fn seen(app: &App) -> &Seen {
    app.world().resource::<Seen>()
}

pub fn wait_for_phase(app: &mut App, phase: RacePhase) {
    step_until(app, &format!("{phase:?}"), |w| {
        w.get_resource::<State<RacePhase>>()
            .is_some_and(|s| *s.get() == phase)
    });
}

pub fn wait_for_state(app: &mut App, state: AppState) {
    step_until(app, &format!("{state:?}"), |w| {
        *w.resource::<State<AppState>>().get() == state
    });
}

/// Waits until the race's results have been processed.
pub fn wait_for_outcome(app: &mut App) {
    step_until(app, "the race outcome", |w| {
        w.contains_resource::<rc_game::RaceOutcome>()
    });
}

/// Waits until the current outcome is gone (restart, next race) and the next race's is in.
pub fn wait_for_next_outcome(app: &mut App) {
    step_until(app, "the old outcome to clear", |w| {
        !w.contains_resource::<rc_game::RaceOutcome>()
    });
    wait_for_outcome(app);
}
