//! Pure pursuit and pedal control (pure).
//!
//! Steering: the car aims at a point `lookahead` ahead on its chosen line; the arc through that
//! point has curvature κ = 2·sin α / d (α = angle between the heading and the point). rc_vehicle
//! steers speed-sensitively — full lock asks the front tyres for a fixed share of their grip — so
//! the input for κ is κ / κ_max(v) with κ_max(v) = min(tan(lock)/L, a_steer / v²).

use bevy::math::Vec3;

// ponytail: one steering geometry for every car (rc_ai cannot read vehicle params); pure pursuit
// absorbs the ±10 % gain error. Per-car wheelbase/lock from the catalog if precision matters.
/// Typical wheelbase of the roster (u; 1.2–1.4).
const WHEELBASE_U: f32 = 1.3;
/// Typical low-speed wheel lock (rad; 28–34°).
const LOCK_RAD: f32 = 31.0 * std::f32::consts::PI / 180.0;
/// rc_vehicle's full lock asks for this share above the tyres' measured cornering grip.
const STEER_OVERDRIVE: f32 = 1.2;
/// Above this error (u/s) under the target speed the throttle is fully open.
const FULL_THROTTLE_ERROR_UPS: f32 = 2.0;
/// Coast (no pedal: rc_vehicle's 5 u/s² engine braking) until this far over the target (u/s) …
const COAST_BAND_UPS: f32 = 1.0;
/// … then brake, fully at this much over it (u/s).
const FULL_BRAKE_ERROR_UPS: f32 = 4.0;

/// Signed angle (rad, + = right) from `heading` to the direction `to_target`, both seen in the
/// plane perpendicular to `up`.
pub fn angle_to(heading: Vec3, to_target: Vec3, up: Vec3) -> f32 {
    let right = heading.cross(up);
    let fwd = up.cross(right);
    to_target.dot(right).atan2(to_target.dot(fwd))
}

/// Curvature (1/u, + = right) of the arc from the car through `target`.
pub fn pursuit_curvature(pos: Vec3, heading: Vec3, up: Vec3, target: Vec3) -> f32 {
    let d = target - pos;
    let planar = d - up * d.dot(up);
    let dist = planar.length().max(0.5);
    2.0 * angle_to(heading, planar, up).sin() / dist
}

/// Pure-pursuit lookahead distance (u) at `speed`.
pub fn lookahead(speed: f32, secs: f32, min: f32, max: f32) -> f32 {
    (speed.abs() * secs).clamp(min, max)
}

/// Steering input (−1…1) that makes a car with cornering acceleration `lateral` (u/s²) follow
/// curvature `kappa` at `speed`.
pub fn steer_for_curvature(kappa: f32, speed: f32, lateral: f32) -> f32 {
    let v2 = speed * speed;
    let low_speed = LOCK_RAD.tan() / WHEELBASE_U;
    let max = if v2 < 1e-3 {
        low_speed
    } else {
        low_speed.min(STEER_OVERDRIVE * lateral / v2)
    };
    (kappa / max).clamp(-1.0, 1.0)
}

/// Throttle and brake (0–1) from `speed` (u/s): throttle up to `target`, brake only above
/// `brake_to` (corners ahead); in between the car coasts (rc_vehicle's engine braking), so a
/// speed cap — difficulty scale, a boost running out — never makes the AI brake on a straight.
/// rc_vehicle has no rolling resistance under throttle, so holding a speed alternates throttle
/// and coasting.
pub fn pedals(speed: f32, target: f32, brake_to: f32) -> (f32, f32) {
    let error = target - speed;
    let over = speed - brake_to;
    if error >= 0.0 {
        ((0.25 + error / FULL_THROTTLE_ERROR_UPS).min(1.0), 0.0)
    } else if over <= COAST_BAND_UPS {
        (0.0, 0.0)
    } else {
        let brake = (over - COAST_BAND_UPS) / (FULL_BRAKE_ERROR_UPS - COAST_BAND_UPS);
        (0.0, brake.clamp(0.2, 1.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const UP: Vec3 = Vec3::Y;
    const FWD: Vec3 = Vec3::NEG_Z;

    #[test]
    fn angle_sign_is_right_positive() {
        assert!((angle_to(FWD, Vec3::new(1.0, 0.0, -1.0), UP) - 0.785).abs() < 1e-3);
        assert!(angle_to(FWD, Vec3::new(-1.0, 0.0, -1.0), UP) < 0.0);
        assert!((angle_to(FWD, Vec3::Z, UP).abs() - std::f32::consts::PI).abs() < 1e-3);
        // Height differences (ramps, helices) do not change the plan angle.
        assert!(angle_to(FWD, Vec3::new(0.0, 5.0, -1.0), UP).abs() < 1e-5);
    }

    #[test]
    fn pursuit_arc_passes_through_the_target() {
        // A target on a circle of radius 10 to the right: the arc through it has κ = 1/10.
        let r = 10.0;
        let a: f32 = 0.5;
        let target = Vec3::new(r - r * a.cos(), 0.0, -r * a.sin());
        let k = pursuit_curvature(Vec3::ZERO, FWD, UP, target);
        assert!((k - 1.0 / r).abs() < 1e-4, "{k}");
        let straight = pursuit_curvature(Vec3::ZERO, FWD, UP, Vec3::new(0.0, 0.0, -8.0));
        assert!(straight.abs() < 1e-6);
    }

    #[test]
    fn steering_is_speed_sensitive() {
        let k = 1.0 / 20.0;
        let slow = steer_for_curvature(k, 5.0, 30.0);
        let fast = steer_for_curvature(k, 25.0, 30.0);
        assert!(fast > slow, "the same arc needs more input at speed");
        assert!((fast - k * 625.0 / (STEER_OVERDRIVE * 30.0)).abs() < 1e-4);
        assert_eq!(steer_for_curvature(-5.0, 10.0, 30.0), -1.0, "clamped");
        assert!(
            steer_for_curvature(k, 0.0, 30.0) > 0.0,
            "standstill uses the lock"
        );
    }

    #[test]
    fn lookahead_grows_with_speed_within_bounds() {
        assert_eq!(lookahead(0.0, 0.45, 4.0, 16.0), 4.0);
        assert!((lookahead(20.0, 0.45, 4.0, 16.0) - 9.0).abs() < 1e-5);
        assert_eq!(lookahead(60.0, 0.45, 4.0, 16.0), 16.0);
    }

    #[test]
    fn pedals_track_the_target() {
        assert_eq!(pedals(0.0, 20.0, 20.0), (1.0, 0.0));
        let (t, b) = pedals(19.5, 20.0, 20.0);
        assert!(
            t > 0.0 && t < 1.0 && b == 0.0,
            "gentle throttle near the target"
        );
        assert_eq!(pedals(20.5, 20.0, 20.0), (0.0, 0.0), "coast just above");
        assert_eq!(pedals(30.0, 20.0, 20.0), (0.0, 1.0), "full brake far above");
        let (t, b) = pedals(22.0, 20.0, 20.0);
        assert!(t == 0.0 && b > 0.0 && b < 1.0);
    }

    #[test]
    fn speed_caps_are_coasted_down_not_braked() {
        // 8 u/s over a straight's cap with no corner ahead: lift off, no brake.
        assert_eq!(pedals(38.0, 30.0, 80.0), (0.0, 0.0));
    }
}
