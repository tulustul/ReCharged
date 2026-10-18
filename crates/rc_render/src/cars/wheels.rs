//! Wheel animation (GDD §4.1 CAR-3): the `wheel_fl/fr/rl/rr` nodes of a racer's model spin, steer
//! and travel with the suspension from `VehicleTelemetry::wheels` (rc_vehicle's conventions:
//! `steer` > 0 turns right, `spin` grows while rolling forward, `offset` moves the wheel centre
//! along the car's up axis).

use bevy::prelude::*;
use rc_core::prelude::*;

use super::{CarParts, DrivenBy};

/// Pose of a wheel node from its rest pose (as authored in the glb) and the wheel's state.
pub(crate) fn wheel_pose(rest: Transform, wheel: &WheelState) -> Transform {
    Transform {
        translation: rest.translation + Vec3::Y * wheel.offset,
        rotation: Quat::from_rotation_y(-wheel.steer)
            * Quat::from_rotation_x(-wheel.spin)
            * rest.rotation,
        scale: rest.scale,
    }
}

/// `VisualSet`: wheel nodes of every racer model follow their racer's telemetry.
pub(super) fn drive_wheels(
    models: Query<(&DrivenBy, &CarParts)>,
    telemetry: Query<&VehicleTelemetry>,
    mut nodes: Query<&mut Transform>,
) {
    for (driven, parts) in &models {
        let Ok(tel) = telemetry.get(driven.0) else {
            continue;
        };
        for (slot, wheel) in parts.wheels.iter().zip(&tel.wheels) {
            let Some((node, rest)) = slot else {
                continue;
            };
            if let Ok(mut t) = nodes.get_mut(*node) {
                t.set_if_neq(wheel_pose(*rest, wheel));
            }
        }
    }
}

/// Distance a model without telemetry (a ghost) has rolled forward, for its wheel spin.
#[derive(Component, Debug, Clone, Copy, Default)]
pub(crate) struct Rolling {
    last: Option<Vec3>,
    distance: f32,
}

/// `VisualSet`: ghost cars have no telemetry; their wheels roll with the model's own motion
/// (a wheel's radius is its rest height: wheel centres sit at y = radius).
pub(super) fn roll_ghost_wheels(
    mut models: Query<(&GlobalTransform, &CarParts, &mut Rolling), Without<DrivenBy>>,
    mut nodes: Query<&mut Transform>,
) {
    for (global, parts, mut rolling) in &mut models {
        let pos = global.translation();
        if let Some(last) = rolling.last {
            rolling.distance += (pos - last).dot(*global.forward());
        }
        rolling.last = Some(pos);
        for (node, rest) in parts.wheels.iter().flatten() {
            let radius = rest.translation.y.max(0.05);
            let wheel = WheelState {
                spin: (rolling.distance / radius).rem_euclid(std::f32::consts::TAU),
                ..default()
            };
            if let Ok(mut t) = nodes.get_mut(*node) {
                t.set_if_neq(wheel_pose(*rest, &wheel));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rest() -> Transform {
        Transform::from_xyz(0.55, 0.3, -0.65)
    }

    #[test]
    fn a_wheel_at_rest_keeps_its_authored_pose() {
        assert_eq!(wheel_pose(rest(), &WheelState::default()), rest());
    }

    #[test]
    fn positive_steer_turns_the_wheel_to_the_right() {
        let w = WheelState {
            steer: 0.4,
            ..default()
        };
        let forward = wheel_pose(rest(), &w).rotation * Vec3::NEG_Z;
        assert!(
            forward.x > 0.3 && forward.z < 0.0,
            "front of the tyre points right: {forward}"
        );
    }

    #[test]
    fn spinning_forward_moves_the_tyre_top_forward() {
        let w = WheelState {
            spin: 0.3,
            ..default()
        };
        let top = wheel_pose(rest(), &w).rotation * Vec3::Y;
        assert!(top.z < -0.2, "top of the tyre rolls toward −Z: {top}");
    }

    #[test]
    fn suspension_offset_moves_the_wheel_along_car_up() {
        let w = WheelState {
            offset: 0.07,
            steer: 0.2,
            spin: 1.0,
            ..default()
        };
        let pose = wheel_pose(rest(), &w);
        assert!((pose.translation - (rest().translation + Vec3::Y * 0.07)).length() < 1e-6);
    }
}
