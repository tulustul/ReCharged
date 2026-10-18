//! Attempt entities (spawned in `OnEnter(RaceAttempt)`, scoped to it): racers on the track's grid
//! slots from `RaceConfig` (read per attempt), the Golden Battery and the Time Trial ghost car;
//! the Golden Battery pickup; dropped Versus players handed to the AI.

use bevy::prelude::*;
use rc_core::prelude::*;
use rc_track::Track;

use crate::ghost::GhostRun;
use crate::phases::Launch;
use crate::progress::Tracker;
use crate::recovery::Recovery;

/// A local racer within this distance of the Golden Battery collects it (u).
const BATTERY_PICKUP_U: f32 = 1.6;

/// The AI driving an autopilot or dropped player's car (rc_ai treats the unknown rival id as a
/// neutral personality).
fn autopilot() -> AiDriver {
    AiDriver {
        rival: RivalId::new("autopilot"),
        difficulty: Difficulty::Elite,
    }
}

/// `OnEnter(RaceAttempt)`: racers, Golden Battery, ghost car.
pub(crate) fn spawn_attempt(
    mut commands: Commands,
    config: Option<Res<RaceConfig>>,
    track: Option<Res<Track>>,
    status: Option<ResMut<RaceStatus>>,
    mut teleported: MessageWriter<Teleported>,
) {
    let (Some(config), Some(track)) = (config, track) else {
        warn_once!("rc_race: no RaceConfig or Track when the attempt started; no racers spawned");
        return;
    };
    if let Some(mut status) = status {
        status.total_laps = config.laps.max(1);
    }
    let grid = track.grid();
    if config.racers.len() > grid.len() {
        warn!(
            "rc_race: {} racers but {} grid slots; the rest stay home",
            config.racers.len(),
            grid.len()
        );
    }
    let start = RaceProgress {
        last_valid_checkpoint: track.checkpoints().len() - 1,
        ..default()
    };
    for (index, (setup, pose)) in config.racers.iter().zip(grid).enumerate() {
        let mut racer = commands.spawn((
            Racer {
                index,
                name: setup.name.clone(),
                car: setup.car.clone(),
                paint: setup.paint,
                color: setup.color,
            },
            *pose,
            start.clone(),
            (Tracker::default(), Recovery::default(), Launch::default()),
            DespawnOnExit(RaceAttempt),
        ));
        match &setup.controller {
            Controller::Human {
                slot,
                autopilot: ai,
            } => {
                racer.insert(LocalPlayer(*slot));
                if *ai {
                    racer.insert(autopilot());
                }
            }
            Controller::Ai { rival, difficulty } => {
                racer.insert(AiDriver {
                    rival: rival.clone(),
                    difficulty: *difficulty,
                });
            }
        }
        teleported.write(Teleported { entity: racer.id() });
    }
    if let (true, Some(pos)) = (config.golden_battery, track.golden_battery()) {
        commands.spawn((
            GoldenBattery,
            Transform::from_translation(pos),
            DespawnOnExit(RaceAttempt),
        ));
    }
    let ghost = config
        .ghost
        .as_ref()
        .filter(|_| config.mode == GameMode::TimeTrial);
    if let Some(ghost) = ghost
        && let Some(first) = ghost.samples.first()
    {
        let car = commands.spawn((
            GhostCar {
                car: ghost.car.clone(),
                paint: ghost.paint,
            },
            Transform::from_translation(first.translation).with_rotation(first.rotation),
            GhostRun::new(&track, ghost.samples.clone()),
            DespawnOnExit(RaceAttempt),
        ));
        teleported.write(Teleported { entity: car.id() });
    }
    info!(
        "rc_race: {} racers on the grid, {} laps",
        config.racers.len().min(grid.len()),
        config.laps.max(1)
    );
}

/// `SimSet::Race` (Racing): a local racer touching the Golden Battery collects it.
pub(crate) fn collect_battery(
    battery: Option<Single<(Entity, &Transform), With<GoldenBattery>>>,
    racers: Query<(Entity, &Transform), With<LocalPlayer>>,
    mut events: MessageWriter<RaceEvent>,
    mut commands: Commands,
) {
    let Some((battery, at)) = battery.map(|b| b.into_inner()) else {
        return;
    };
    let near =
        |t: &Transform| t.translation.distance_squared(at.translation) <= BATTERY_PICKUP_U.powi(2);
    if let Some((car, _)) = racers.iter().find(|(_, t)| near(t)) {
        commands.entity(battery).despawn();
        events.write(RaceEvent {
            racer: Some(car),
            kind: RaceEventKind::GoldenBattery,
        });
    }
}

/// `Update`: the AI takes over a dropped Versus player's car (later attempts get autopilot from
/// `RaceConfig`). Runs while paused — the drop happens in the pause dialog.
pub(crate) fn drop_players(
    mut dropped: MessageReader<PlayerDropped>,
    racers: Query<(Entity, &LocalPlayer), Without<AiDriver>>,
    mut commands: Commands,
) {
    for PlayerDropped { slot } in dropped.read() {
        for (car, _) in racers.iter().filter(|(_, l)| l.0 == *slot) {
            info!("rc_race: the AI drives P{}'s car", slot.0 + 1);
            commands.entity(car).insert(autopilot());
        }
    }
}
