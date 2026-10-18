//! Arcade assists on top of the physical model (pure, no ECS): air attitude control, auto-right,
//! the handbrake drift (target slip angle + turning force) and the stability catch.

use bevy::prelude::*;
use rc_core::prelude::*;

use crate::model::{CarSpec, Command, Kinematics};

/// Air control (× `air_control`): attitude targets — throttle tips the nose down, brake lifts
/// it, handbrake + steering rolls; idle axes level out. Steering yaws at a rate instead.
const AIR_PITCH_ANGLE: f32 = 15.0 * std::f32::consts::PI / 180.0;
const AIR_ROLL_ANGLE: f32 = 30.0 * std::f32::consts::PI / 180.0;
const AIR_YAW_RATE: f32 = 3.0;
/// Attitude error (sin of the angle) → commanded rate, 1/s; the rate cap, rad/s.
const AIR_ATTITUDE_RATE: f32 = 4.0;
const AIR_MAX_RATE: f32 = 4.0;
/// How fast the body follows the commanded rate (1/s) and its largest acceleration (rad/s², so
/// tumbles from hits still spin for a moment).
const AIR_GAIN: f32 = 6.0;
const AIR_MAX_ACCEL: f32 = 25.0;
/// Slip-angle assist (arcade drift): the body slip angle approaches its target at this rate
/// (1/s); the yaw servo gain (1/s); its largest yaw acceleration (rad/s², × grip).
const SLIP_ASSIST_RATE: f32 = 5.0;
const SLIP_ASSIST_GAIN: f32 = 16.0;
const SLIP_ASSIST_MAX_ACCEL: f32 = 30.0;
/// Steering into the drift widens the target angle, counter-steering narrows it, by this share.
const DRIFT_ANGLE_STEER_RANGE: f32 = 0.45;
/// Arcade drift line: while a drift is engaged the car gets this share of its lateral grip as an
/// extra force toward the turn (steering into the drift adds up to half again, counter-steering
/// takes it away), so drifting follows a corner as tightly as gripping does.
const DRIFT_TURN_GRIP: f32 = 0.22;
/// Stability assist (no handbrake): only slides beyond this slip angle (rad) are caught.
const STABILITY_DEADZONE_RAD: f32 = 6.0 * std::f32::consts::PI / 180.0;
/// Auto-right: rotation rate per radian of tilt, its cap (rad/s) and the servo gain (1/s).
const AUTO_RIGHT_GAIN: f32 = 3.5;
const AUTO_RIGHT_MAX_RATE: f32 = 8.0;
const AUTO_RIGHT_SERVO: f32 = 12.0;

/// World torque that drives the car's angular velocity toward `target_local` (car space) on the
/// axes where `gain_local` > 0, each acceleration capped at `max_accel`.
fn servo_torque(
    spec: &CarSpec,
    kin: &Kinematics,
    target: Vec3,
    gain: Vec3,
    max_accel: Vec3,
) -> Vec3 {
    let local = kin.rot.inverse() * kin.ang;
    let accel = ((target - local) * gain).clamp(-max_accel, max_accel);
    kin.rot * (accel * spec.inertia)
}

/// Air control (no wheel on the ground): an attitude servo on pitch and roll (targets from
/// throttle/brake and handbrake + steering, level when idle) and a yaw rate from steering.
/// Scaled by `air_control` (weight): heavy cars turn slower in the air.
pub fn air_torque(spec: &CarSpec, kin: &Kinematics, cmd: &Command) -> Vec3 {
    let air = spec.params.air_control;
    let local = kin.rot.inverse() * kin.ang;
    // Rotation (car space) bringing the up axis to world up; x ≈ −sin(pitch), z ≈ −sin(roll).
    let level = kin.rot.inverse() * (kin.rot * Vec3::Y).cross(Vec3::Y);
    let pitch_target = (cmd.brake - cmd.throttle).clamp(-1.0, 1.0) * AIR_PITCH_ANGLE;
    let roll_target = match cmd.handbrake {
        true => -cmd.steer * AIR_ROLL_ANGLE,
        false => 0.0,
    };
    let max_rate = AIR_MAX_RATE * air;
    let attitude = |target: f32, error: f32| {
        ((target.sin() + error) * AIR_ATTITUDE_RATE).clamp(-max_rate, max_rate)
    };
    let mut target = Vec3::new(
        attitude(pitch_target, level.x),
        local.y,
        attitude(roll_target, level.z),
    );
    let mut gain = Vec3::new(AIR_GAIN, 0.0, AIR_GAIN) * air;
    if !cmd.handbrake && cmd.steer.abs() > 0.05 {
        target.y = -cmd.steer * AIR_YAW_RATE * air;
        gain.y = AIR_GAIN * air;
    }
    servo_torque(spec, kin, target, gain, Vec3::splat(AIR_MAX_ACCEL * air))
}

/// Target body slip angle (rad) of a drift in `dir` (+1 = sliding right of the heading, i.e. a
/// left-hand drift; −1 = right-hand drift): the car's drift angle, widened by steering into the
/// turn and narrowed by counter-steering.
pub fn drift_target(spec: &CarSpec, dir: f32, steer: f32) -> f32 {
    let into = (-steer * dir).clamp(-1.0, 1.0);
    let angle = spec.params.steering.drift_angle_deg.to_radians();
    dir * angle * (1.0 + DRIFT_ANGLE_STEER_RANGE * into)
}

/// Extra force (world) of an engaged drift in `dir` (see [`drift_target`]): perpendicular to the
/// planar velocity, toward the turn centre, × `grip` (surface and effects).
pub fn drift_turn_force(spec: &CarSpec, kin: &Kinematics, dir: f32, steer: f32, grip: f32) -> Vec3 {
    let up = kin.rot * Vec3::Y;
    let planar = (kin.lin - up * kin.lin.dot(up)).normalize_or_zero();
    let into = (-steer * dir).clamp(-1.0, 1.0);
    let share = DRIFT_TURN_GRIP * (1.0 + 0.5 * into);
    let accel = spec.params.tyres.grip * GRAVITY * share * grip;
    planar.cross(up) * (-dir * spec.mass() * accel)
}

/// Body slip angle (rad, + = moving right of the heading) and yaw rate (rad/s, + = left).
pub fn slip_and_yaw(kin: &Kinematics) -> (f32, f32) {
    let fwd = kin.rot * Vec3::NEG_Z;
    let right = kin.rot * Vec3::X;
    let up = kin.rot * Vec3::Y;
    (kin.lin.dot(right).atan2(kin.lin.dot(fwd)), kin.ang.dot(up))
}

/// Yaw torque steering the body slip angle toward `target` (rad): the yaw rate is servoed to the
/// velocity's own turn rate `path_rate` (rad/s) plus a correction of the slip error. `None`
/// target = stability mode: only slip beyond the dead zone is pulled back. `strength` scales the
/// largest yaw acceleration (grip, stability).
pub fn slip_assist_torque(
    spec: &CarSpec,
    kin: &Kinematics,
    path_rate: f32,
    target: Option<f32>,
    strength: f32,
) -> Vec3 {
    let (slip, yaw) = slip_and_yaw(kin);
    let target = match target {
        Some(t) => t,
        None if slip.abs() > STABILITY_DEADZONE_RAD => {
            slip.clamp(-STABILITY_DEADZONE_RAD, STABILITY_DEADZONE_RAD)
        }
        None => return Vec3::ZERO,
    };
    // dβ/dt = yaw − path_rate (β = heading − velocity angle, both counter-clockwise).
    let want = path_rate + SLIP_ASSIST_RATE * (target - slip);
    let max = SLIP_ASSIST_MAX_ACCEL * strength.max(0.0);
    let accel = ((want - yaw) * SLIP_ASSIST_GAIN).clamp(-max, max);
    kin.rot * Vec3::Y * (accel * spec.inertia.y)
}

/// Auto-right: torque rolling the car upright (about the horizontal axis between its up and
/// world up; about its forward axis when exactly on the roof).
pub fn righting_torque(spec: &CarSpec, kin: &Kinematics) -> Vec3 {
    let up = kin.rot * Vec3::Y;
    let mut axis = up.cross(Vec3::Y);
    if axis.length_squared() < 1e-4 {
        axis = kin.rot * Vec3::NEG_Z;
    }
    let tilt = up.dot(Vec3::Y).clamp(-1.0, 1.0).acos();
    let rate = (tilt * AUTO_RIGHT_GAIN).min(AUTO_RIGHT_MAX_RATE);
    let target = kin.rot.inverse() * (axis.normalize() * rate);
    servo_torque(
        spec,
        kin,
        target,
        Vec3::splat(AUTO_RIGHT_SERVO),
        Vec3::splat(f32::INFINITY),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::VehicleParams;

    fn spec() -> CarSpec {
        let p: VehicleParams =
            ron::from_str(include_str!("../../../assets/data/vehicles/hornet.ron"))
                .expect("parses");
        CarSpec::new(&p)
    }

    #[test]
    fn air_control_follows_input_and_levels() {
        let s = spec();
        let cmd = Command {
            throttle: 1.0,
            brake: 0.0,
            steer: 0.0,
            handbrake: false,
            held: false,
            drive_mul: 1.0,
            top_speed: 30.0,
            grip_mul: 1.0,
            rear_grip_mul: 1.0,
        };
        let level = Kinematics {
            rot: Quat::IDENTITY,
            ..default()
        };
        let nose_down = air_torque(&s, &level, &cmd);
        assert!(
            nose_down.x < 0.0,
            "throttle pitches the nose down: {nose_down}"
        );
        let rolled = Kinematics {
            rot: Quat::from_rotation_z(0.5),
            ..default()
        };
        let idle = Command {
            throttle: 0.0,
            ..cmd
        };
        assert!(air_torque(&s, &rolled, &idle).z < 0.0, "levels a roll");
        let right = air_torque(&s, &level, &Command { steer: 1.0, ..idle });
        assert!(right.y < 0.0, "steer right yaws right");
    }

    #[test]
    fn righting_rolls_toward_upright() {
        let s = spec();
        let on_side = Kinematics {
            rot: Quat::from_rotation_z(1.4),
            ..default()
        };
        assert!(righting_torque(&s, &on_side).z < 0.0);
        let on_roof = Kinematics {
            rot: Quat::from_rotation_z(std::f32::consts::PI),
            ..default()
        };
        assert!(righting_torque(&s, &on_roof).length() > 0.0);
    }

    #[test]
    fn drift_target_follows_steering() {
        let s = spec();
        let angle = s.params.steering.drift_angle_deg.to_radians();
        // Right-hand drift (dir −1): steering right widens it, steering left narrows it.
        assert!((drift_target(&s, -1.0, 0.0) + angle).abs() < 1e-6);
        assert!(drift_target(&s, -1.0, 1.0) < -angle);
        assert!(drift_target(&s, -1.0, -1.0) > -angle && drift_target(&s, -1.0, -1.0) < 0.0);
    }

    #[test]
    fn stability_assist_ignores_small_slip_and_catches_slides() {
        let s = spec();
        let small = Kinematics {
            lin: Quat::from_rotation_y(0.05) * Vec3::NEG_Z * 20.0,
            ..default()
        };
        assert_eq!(slip_assist_torque(&s, &small, 0.0, None, 1.0), Vec3::ZERO);
        // Moving 30° right of the heading: the assist yaws the nose right (negative about Y).
        let slide = Kinematics {
            lin: Quat::from_rotation_y(-0.52) * Vec3::NEG_Z * 20.0,
            ..default()
        };
        assert!(slip_assist_torque(&s, &slide, 0.0, None, 1.0).y < 0.0);
    }
}
