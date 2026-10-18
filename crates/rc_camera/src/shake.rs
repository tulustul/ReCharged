//! Camera shake (GDD §11 "subtle shake on impacts and landings", setting `camera_shake`): impacts,
//! landings and nearby explosions add trauma; the shake angle grows with trauma² and decays.

use bevy::prelude::*;
use rc_core::prelude::*;

use crate::follow::Rig;

/// Trauma lost per second.
const DECAY_PER_S: f32 = 1.6;
/// Largest shake angles at full trauma (yaw, pitch, roll), radians (≈ 2°, 2°, 1.2°).
const MAX_ANGLES: Vec3 = Vec3::new(0.035, 0.035, 0.02);
/// Trauma per unit of impact / landing strength.
const IMPACT_GAIN: f32 = 0.6;
const LANDING_GAIN: f32 = 0.35;
/// Landings after shorter air time do not shake, s.
const LANDING_MIN_AIR_S: f32 = 0.3;
/// Trauma of an explosion at the camera; it fades to zero at `radius × EXPLOSION_REACH`.
const EXPLOSION_GAIN: f32 = 0.9;
const EXPLOSION_REACH: f32 = 3.0;

/// Adds trauma to the cameras that follow the racer involved (and to cameras near explosions).
pub(crate) fn add_trauma(
    mut impacts: MessageReader<CarImpact>,
    mut landings: MessageReader<CarLanded>,
    mut explosions: MessageReader<Explosion>,
    mut rigs: Query<(&mut Rig, &Transform), With<GameCamera>>,
) {
    let add =
        |car: Entity, amount: f32, rigs: &mut Query<(&mut Rig, &Transform), With<GameCamera>>| {
            for (mut rig, _) in rigs.iter_mut() {
                if rig.followed() == Some(car) {
                    rig.trauma = (rig.trauma + amount).min(1.0);
                }
            }
        };
    for impact in impacts.read() {
        add(impact.car, impact.strength * IMPACT_GAIN, &mut rigs);
    }
    for landing in landings.read() {
        if landing.air_time >= LANDING_MIN_AIR_S {
            add(landing.car, landing.strength * LANDING_GAIN, &mut rigs);
        }
    }
    for explosion in explosions.read() {
        for (mut rig, transform) in &mut rigs {
            let amount = explosion_trauma(transform.translation, explosion);
            rig.trauma = (rig.trauma + amount).min(1.0);
        }
    }
}

/// Trauma an explosion gives a camera at `eye`.
pub(crate) fn explosion_trauma(eye: Vec3, explosion: &Explosion) -> f32 {
    let reach = (explosion.radius * EXPLOSION_REACH).max(f32::EPSILON);
    EXPLOSION_GAIN * (1.0 - eye.distance(explosion.position) / reach).max(0.0)
}

/// Smooth noise in −1..1 (two incommensurate sines; presentation only, so no keyed RNG).
fn noise(t: f32, channel: f32) -> f32 {
    ((t * 17.3 + channel * 3.1).sin() + 0.5 * (t * 31.7 + channel * 7.9).sin()) / 1.5
}

/// Advances the shake clock and decay by `dt`; the rotation to apply on top of the view.
pub(crate) fn offset(rig: &mut Rig, dt: f32, enabled: bool) -> Quat {
    rig.trauma = (rig.trauma - DECAY_PER_S * dt).max(0.0);
    rig.clock += dt;
    if !enabled || rig.trauma <= 0.0 {
        return Quat::IDENTITY;
    }
    let a = MAX_ANGLES * rig.trauma * rig.trauma;
    let t = rig.clock;
    Quat::from_euler(
        EulerRot::YXZ,
        a.x * noise(t, 0.0),
        a.y * noise(t, 1.0),
        a.z * noise(t, 2.0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shake_scales_with_trauma_squared_and_decays_to_rest() {
        let mut rig = Rig::new(CameraMode::Chase);
        rig.trauma = 1.0;
        let strong = offset(&mut rig, 0.01, true).angle_between(Quat::IDENTITY);
        assert!(strong > 0.0 && strong < 0.06, "subtle: {strong}");
        for _ in 0..100 {
            offset(&mut rig, 0.01, true);
        }
        assert_eq!(rig.trauma, 0.0, "decays within a second");
        assert_eq!(offset(&mut rig, 0.01, true), Quat::IDENTITY);
    }

    #[test]
    fn the_setting_turns_shake_off() {
        let mut rig = Rig::new(CameraMode::Chase);
        rig.trauma = 1.0;
        assert_eq!(offset(&mut rig, 0.01, false), Quat::IDENTITY);
    }

    #[test]
    fn explosions_shake_only_nearby_cameras() {
        let boom = Explosion {
            position: Vec3::ZERO,
            radius: 4.0,
            kind: ItemKind::SparkRocket,
        };
        assert!((explosion_trauma(Vec3::ZERO, &boom) - EXPLOSION_GAIN).abs() < 1e-6);
        assert!(explosion_trauma(Vec3::X * 6.0, &boom) > 0.0);
        assert_eq!(explosion_trauma(Vec3::X * 13.0, &boom), 0.0);
    }
}
