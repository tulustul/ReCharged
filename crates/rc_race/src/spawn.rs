//! Racer spawning in `OnEnter(RaceAttempt)`.
//!
//! ponytail: placeholder grid (two staggered columns behind the origin, facing −Z) until T06
//! places racers on the track's grid slots from `Track`.

use bevy::prelude::*;
use rc_core::prelude::*;

/// Lateral distance between the two grid columns, world units.
const GRID_COLUMN_GAP_U: f32 = 3.0;
/// Distance between grid rows, world units.
const GRID_ROW_GAP_U: f32 = 4.0;
/// Extra stagger of the right column, world units.
const GRID_STAGGER_U: f32 = 1.5;
/// Spawn height above the ground, world units.
const GRID_HEIGHT_U: f32 = 0.5;

/// Grid pose of racer `index` (−Z is forward, so rows grow toward +Z = behind the start).
pub(crate) fn grid_transform(index: usize) -> Transform {
    let column = (index % 2) as f32;
    let row = (index / 2) as f32;
    Transform::from_xyz(
        (column - 0.5) * GRID_COLUMN_GAP_U,
        GRID_HEIGHT_U,
        row * GRID_ROW_GAP_U + column * GRID_STAGGER_U,
    )
}

/// One racer entity per `RaceConfig.racers` entry, scoped to the attempt.
pub(crate) fn spawn_racers(
    mut commands: Commands,
    config: Option<Res<RaceConfig>>,
    mut teleported: MessageWriter<Teleported>,
) {
    let Some(config) = config else {
        warn_once!("rc_race: no RaceConfig when the attempt started; no racers spawned");
        return;
    };
    for (index, setup) in config.racers.iter().enumerate() {
        let mut racer = commands.spawn((
            Racer {
                index,
                name: setup.name.clone(),
                car: setup.car.clone(),
                paint: setup.paint,
                color: setup.color,
            },
            grid_transform(index),
            DespawnOnExit(RaceAttempt),
        ));
        match &setup.controller {
            Controller::Human { slot, autopilot } => {
                racer.insert(LocalPlayer(*slot));
                if *autopilot {
                    // rc_ai treats an unknown rival id as a neutral personality.
                    racer.insert(AiDriver {
                        rival: RivalId::new("autopilot"),
                        difficulty: Difficulty::Elite,
                    });
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_slots_are_distinct_and_behind_the_start() {
        let slots: Vec<Vec3> = (0..MAX_RACERS)
            .map(|i| grid_transform(i).translation)
            .collect();
        for (i, a) in slots.iter().enumerate() {
            assert!(a.z >= 0.0, "slot {i} is ahead of the start line");
            for b in &slots[i + 1..] {
                assert!(a.distance(*b) > 2.5, "slots overlap: {a} vs {b}");
            }
        }
    }
}
