//! `SimSet::Vehicle`: executing other crates' requests — `RespawnCar` (rc_race decides, we
//! teleport) and `ApplyImpulse` (rc_items' tumbles, scaled by weight).

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use rc_core::prelude::*;

use crate::model::CarSpec;
use crate::state::CarState;

/// Teleports to the requested pose with zero velocity (the ghost effect comes from rc_race via
/// rc_items), then reports `Teleported` and `CarRespawned`.
pub(crate) fn execute_respawns(
    mut requests: MessageReader<RespawnCar>,
    mut cars: Query<(
        &CarSpec,
        &mut CarState,
        &mut Transform,
        &mut Velocity,
        &mut ExternalImpulse,
    )>,
    mut teleported: MessageWriter<Teleported>,
    mut respawned: MessageWriter<CarRespawned>,
) {
    for r in requests.read() {
        let Ok((spec, mut state, mut transform, mut velocity, mut impulse)) = cars.get_mut(r.car)
        else {
            continue;
        };
        transform.translation = r.transform.translation;
        transform.rotation = r.transform.rotation.normalize();
        *velocity = Velocity::zero();
        *impulse = ExternalImpulse::default();
        state.reset_motion(spec);
        teleported.write(Teleported { entity: r.car });
        respawned.write(CarRespawned {
            car: r.car,
            kind: r.kind,
        });
    }
}

/// Weight scaling of an `ApplyImpulse` (GDD §5.5: heavier cars resist knock-back).
pub(crate) fn impulse_scale(mass: f32) -> f32 {
    (REFERENCE_CAR_MASS / mass).sqrt()
}

/// Adds the scaled impulses to the bodies (rapier applies them in the coming step).
pub(crate) fn apply_impulses(
    mut requests: MessageReader<ApplyImpulse>,
    mut cars: Query<(&CarSpec, &mut ExternalImpulse)>,
) {
    for r in requests.read() {
        let Ok((spec, mut impulse)) = cars.get_mut(r.car) else {
            continue;
        };
        let k = impulse_scale(spec.mass());
        impulse.impulse += r.linear * k;
        impulse.torque_impulse += r.angular * k;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heavier_cars_get_smaller_impulses() {
        assert!((impulse_scale(REFERENCE_CAR_MASS) - 1.0).abs() < 1e-6);
        assert!(impulse_scale(2.0 * REFERENCE_CAR_MASS) < 1.0);
        assert!((impulse_scale(REFERENCE_CAR_MASS / 4.0) - 2.0).abs() < 1e-6);
    }
}
