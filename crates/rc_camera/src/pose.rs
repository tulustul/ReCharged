//! Pure camera pose maths (GDD §11): the critically damped spring, headings, the chase boom, the
//! hood / orbit / fly-by views and FOV widening.

use std::f32::consts::{PI, TAU};

use bevy::prelude::*;

/// Chase-style framing: boom behind the car, in car-heading space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Boom {
    /// Distance behind the car, u.
    pub distance: f32,
    /// Height above the car origin, u.
    pub height: f32,
    /// Height of the look-at point above the car origin, u.
    pub look_height: f32,
    /// Heading lag (spring smooth time), s.
    pub yaw_smooth_s: f32,
}

/// Chase camera (default).
pub(crate) const CHASE: Boom = Boom {
    distance: 4.6,
    height: 1.7,
    look_height: 0.7,
    yaw_smooth_s: 0.22,
};
/// Far chase camera.
pub(crate) const FAR: Boom = Boom {
    distance: 7.5,
    height: 2.8,
    look_height: 0.8,
    yaw_smooth_s: 0.3,
};
/// Hood camera position in car space (Y up, −Z forward).
// ponytail: one offset for every car; per-car from the model's metadata once rc_render has it.
pub(crate) const HOOD_OFFSET: Vec3 = Vec3::new(0.0, 0.6, -0.4);
/// Finish orbit: radius and height, u.
const ORBIT_RADIUS_U: f32 = 6.0;
const ORBIT_HEIGHT_U: f32 = 2.4;
/// Intro fly-by: start offset (radius, height in u) and start angle around the car (rad).
const FLYBY_RADIUS_U: f32 = 18.0;
const FLYBY_HEIGHT_U: f32 = 9.0;
const FLYBY_ANGLE: f32 = 1.2 * PI;
/// Look-ahead along the heading: seconds of speed, capped in u.
const LOOK_AHEAD_S: f32 = 0.12;
const LOOK_AHEAD_MAX_U: f32 = 4.0;
/// FOV widening (GDD §11: up to +12°): at [`FULL_SPEED_UPS`] and while boosting, in degrees.
const WIDEN_SPEED_DEG: f32 = 8.0;
const WIDEN_BOOST_DEG: f32 = 4.0;
const MAX_WIDEN_DEG: f32 = 12.0;
/// Speed of full widening, u/s (the 5★ top speed, GDD §5.1).
const FULL_SPEED_UPS: f32 = 36.0;

/// Critically damped spring toward `target` (Game Programming Gems 4, §1.10): settles in about
/// `smooth_s` seconds without overshoot and stays stable for any `dt`.
pub(crate) fn damp(current: f32, target: f32, velocity: &mut f32, smooth_s: f32, dt: f32) -> f32 {
    let omega = 2.0 / smooth_s.max(1e-4);
    let x = omega * dt;
    let decay = 1.0 / (1.0 + x + 0.48 * x * x + 0.235 * x * x * x);
    let change = current - target;
    let temp = (*velocity + omega * change) * dt;
    *velocity = (*velocity - omega * temp) * decay;
    let out = target + (change + temp) * decay;
    if (target - current) * (out - target) > 0.0 {
        *velocity = 0.0; // the polynomial decay overshot on a huge dt
        return target;
    }
    out
}

/// [`damp`] for angles: takes the short way around.
pub(crate) fn damp_angle(
    current: f32,
    target: f32,
    velocity: &mut f32,
    smooth_s: f32,
    dt: f32,
) -> f32 {
    let delta = (target - current + PI).rem_euclid(TAU) - PI;
    damp(current, current + delta, velocity, smooth_s, dt)
}

/// Heading (rotation about +Y) of a pose's forward axis, `None` when it points straight up/down.
pub(crate) fn heading(rotation: Quat) -> Option<f32> {
    let f = rotation * Vec3::NEG_Z;
    (f.x * f.x + f.z * f.z > 1e-4).then(|| (-f.x).atan2(-f.z))
}

/// Camera pose looking at `target` from `eye`, Y up (falls back to −Z when they coincide).
pub(crate) fn look(eye: Vec3, target: Vec3) -> Transform {
    let t = Transform::from_translation(eye);
    if eye.distance_squared(target) < 1e-6 {
        t
    } else {
        t.looking_at(target, Vec3::Y)
    }
}

/// Where the boom pivots (the look-at point) and where its end wants to be.
pub(crate) fn chase_points(focus: Vec3, yaw: f32, speed: f32, boom: Boom) -> (Vec3, Vec3, Vec3) {
    let rot = Quat::from_rotation_y(yaw);
    let pivot = focus + Vec3::Y * boom.look_height;
    let ahead = (speed.max(0.0) * LOOK_AHEAD_S).min(LOOK_AHEAD_MAX_U);
    let look_at = pivot + rot * Vec3::NEG_Z * ahead;
    let eye = focus + rot * Vec3::new(0.0, boom.height, boom.distance);
    (pivot, eye, look_at)
}

/// The hood view of a car pose (rigid; turned around while looking back).
pub(crate) fn hood_pose(car: Transform, look_back: bool) -> Transform {
    let rotation = if look_back {
        car.rotation * Quat::from_rotation_y(PI)
    } else {
        car.rotation
    };
    Transform::from_translation(car.translation + car.rotation * HOOD_OFFSET)
        .with_rotation(rotation)
}

/// The finish orbit around `focus` at `angle`.
pub(crate) fn orbit_pose(focus: Vec3, angle: f32) -> Transform {
    let eye = focus + Quat::from_rotation_y(angle) * Vec3::new(0.0, ORBIT_HEIGHT_U, ORBIT_RADIUS_U);
    look(eye, focus + Vec3::Y * CHASE.look_height)
}

/// Intro fly-by at progress `t` (0–1): a descending sweep around the car that ends exactly on the
/// chase pose, so the race starts without a cut.
// ponytail: circles the car; flying along the `Track` spline needs T02's query API.
pub(crate) fn flyby_pose(car: Vec3, car_yaw: f32, t: f32) -> Transform {
    let s = t.clamp(0.0, 1.0);
    let e = s * s * (3.0 - 2.0 * s); // smoothstep
    let angle = car_yaw + FLYBY_ANGLE * (1.0 - e);
    let radius = FLYBY_RADIUS_U + (CHASE.distance - FLYBY_RADIUS_U) * e;
    let height = FLYBY_HEIGHT_U + (CHASE.height - FLYBY_HEIGHT_U) * e;
    let eye = car + Quat::from_rotation_y(angle) * Vec3::new(0.0, height, radius);
    look(eye, car + Vec3::Y * CHASE.look_height)
}

/// FOV widening target in degrees for a speed (u/s) and boost state.
pub(crate) fn widen_target(speed: f32, boosting: bool) -> f32 {
    let speed_part = WIDEN_SPEED_DEG * (speed / FULL_SPEED_UPS).clamp(0.0, 1.0);
    let boost_part = if boosting { WIDEN_BOOST_DEG } else { 0.0 };
    (speed_part + boost_part).min(MAX_WIDEN_DEG)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn damp_converges_without_overshoot_and_is_frame_rate_independent() {
        let (mut a, mut va) = (0.0, 0.0);
        let (mut b, mut vb) = (0.0, 0.0);
        for _ in 0..60 {
            a = damp(a, 10.0, &mut va, 0.2, 1.0 / 60.0);
            assert!(a <= 10.0 + 1e-4, "overshoot {a}");
        }
        for _ in 0..240 {
            b = damp(b, 10.0, &mut vb, 0.2, 1.0 / 240.0);
        }
        assert!((a - 10.0).abs() < 0.1, "settles in ~smooth time: {a}");
        assert!((a - b).abs() < 0.05, "60 Hz {a} vs 240 Hz {b}");
        let mut v = 0.0;
        assert!(
            (damp(0.0, 5.0, &mut v, 0.1, 100.0) - 5.0).abs() < 1e-3,
            "huge dt lands on target"
        );
        assert_eq!(damp(3.0, 5.0, &mut v, 0.1, 0.0), 3.0, "zero dt holds");
    }

    #[test]
    fn damp_angle_takes_the_short_way_round() {
        let mut v = 0.0;
        let a = damp_angle(PI - 0.1, -PI + 0.1, &mut v, 0.2, 1.0 / 60.0);
        assert!(a > PI - 0.1, "moved forward through ±π, got {a}");
    }

    #[test]
    fn heading_matches_the_minus_z_forward_convention() {
        assert!(heading(Quat::IDENTITY).is_some_and(|y| y.abs() < 1e-6));
        let left = heading(Quat::from_rotation_y(0.5)).expect("level");
        assert!((left - 0.5).abs() < 1e-5);
        assert_eq!(
            heading(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
            None
        );
    }

    #[test]
    fn chase_sits_behind_and_above_and_looks_ahead_with_speed() {
        let (pivot, eye, look_at) = chase_points(Vec3::ZERO, 0.0, 0.0, CHASE);
        assert!(eye.z > 4.0 && eye.y > 1.0, "behind (+Z) and above: {eye}");
        assert_eq!(look_at, pivot);
        let (_, _, fast) = chase_points(Vec3::ZERO, 0.0, 30.0, CHASE);
        assert!(fast.z < -3.0, "looks ahead along −Z at speed: {fast}");
        let (_, _, capped) = chase_points(Vec3::ZERO, 0.0, 500.0, CHASE);
        assert!((capped.z + LOOK_AHEAD_MAX_U).abs() < 1e-5);
    }

    #[test]
    fn flyby_ends_on_the_chase_pose() {
        let car = Vec3::new(3.0, 0.5, -2.0);
        let start = flyby_pose(car, 0.0, 0.0);
        assert!(
            start.translation.distance(car) > 15.0,
            "starts far and high"
        );
        let end = flyby_pose(car, 0.0, 1.0);
        let (_, eye, _) = chase_points(car, 0.0, 0.0, CHASE);
        assert!(
            end.translation.distance(eye) < 1e-3,
            "{} vs {eye}",
            end.translation
        );
    }

    #[test]
    fn orbit_circles_the_car_at_constant_distance() {
        let car = Vec3::new(1.0, 0.0, 1.0);
        let d0 = orbit_pose(car, 0.0).translation.distance(car);
        let d1 = orbit_pose(car, 2.0).translation.distance(car);
        assert!((d0 - d1).abs() < 1e-4);
    }

    #[test]
    fn widening_grows_with_speed_and_boost_up_to_12_degrees() {
        assert_eq!(widen_target(0.0, false), 0.0);
        assert!(widen_target(18.0, false) > 0.0);
        assert_eq!(widen_target(FULL_SPEED_UPS * 3.0, true), MAX_WIDEN_DEG);
        assert!(widen_target(0.0, true) > 0.0);
    }
}
