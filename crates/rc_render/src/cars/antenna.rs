//! Antenna sway (art bible §2.2): the whip antenna (node `antenna`, pivot at its base) leans
//! against the car's acceleration on an underdamped spring, so it whips on launches, braking,
//! corners and landings. Works on any car model that moves (racers, ghosts), from the model's
//! own world motion.

use bevy::prelude::*;

/// Spring stiffness (1/s²) and damping (1/s): ≈ 1.7 Hz, visibly wobbly (ζ ≈ 0.3).
const STIFFNESS: f32 = 120.0;
const DAMPING: f32 = 7.0;
/// Lean per unit of acceleration (rad per u/s²) and the largest lean (rad).
const LEAN_PER_ACCEL: f32 = 0.012;
const MAX_LEAN: f32 = 0.45;
/// Longest time step one update integrates (s): hitches never kick the spring.
const MAX_STEP_SECS: f32 = 1.0 / 30.0;

/// The antenna node of an instanced car model and its spring state.
#[derive(Component, Debug, Clone)]
pub(crate) struct Antenna {
    node: Entity,
    rest: Quat,
    /// Lean: x = about the car's X axis (tip backward), y = about its Z axis (tip left).
    lean: Vec2,
    rate: Vec2,
    /// The model's last world position and velocity.
    last: Option<(Vec3, Vec3)>,
}

impl Antenna {
    pub(crate) fn new(node: Entity, rest: Quat) -> Self {
        Self {
            node,
            rest,
            lean: Vec2::ZERO,
            rate: Vec2::ZERO,
            last: None,
        }
    }
}

/// One spring step toward the lean asked for by `accel` (car space: −Z forward, +X right).
pub(crate) fn sway_step(lean: Vec2, rate: Vec2, accel: Vec3, dt: f32) -> (Vec2, Vec2) {
    // Speeding up (accel −Z) tips the tip back; accelerating right tips it left.
    let target = (Vec2::new(-accel.z, accel.x) * LEAN_PER_ACCEL).clamp_length_max(MAX_LEAN);
    let rate = rate + (STIFFNESS * (target - lean) - DAMPING * rate) * dt;
    let lean = (lean + rate * dt).clamp_length_max(MAX_LEAN);
    (lean, rate)
}

/// `VisualSet`: integrate every antenna from its model's motion.
pub(super) fn sway(
    time: Res<Time>,
    mut antennas: Query<(&GlobalTransform, &mut Antenna)>,
    mut nodes: Query<&mut Transform>,
) {
    let dt = time.delta_secs().min(MAX_STEP_SECS);
    if dt <= 0.0 {
        return;
    }
    for (global, mut antenna) in &mut antennas {
        let pos = global.translation();
        let (vel, accel) = match antenna.last {
            Some((last_pos, last_vel)) => {
                let vel = (pos - last_pos) / dt;
                (vel, (vel - last_vel) / dt)
            }
            None => (Vec3::ZERO, Vec3::ZERO),
        };
        antenna.last = Some((pos, vel));
        let local = global.rotation().inverse() * accel;
        let (lean, rate) = sway_step(antenna.lean, antenna.rate, local, dt);
        antenna.lean = lean;
        antenna.rate = rate;
        if let Ok(mut t) = nodes.get_mut(antenna.node) {
            t.rotation =
                antenna.rest * Quat::from_rotation_x(lean.x) * Quat::from_rotation_z(lean.y);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settle(accel: Vec3) -> Vec2 {
        let (mut lean, mut rate) = (Vec2::ZERO, Vec2::ZERO);
        for _ in 0..600 {
            (lean, rate) = sway_step(lean, rate, accel, 1.0 / 60.0);
        }
        lean
    }

    #[test]
    fn launching_tips_the_antenna_back_and_turning_right_tips_it_left() {
        let launch = settle(Vec3::new(0.0, 0.0, -12.0));
        let tip = Quat::from_rotation_x(launch.x) * Vec3::Y;
        assert!(tip.z > 0.05, "tip leans backward: {tip}");
        let right = settle(Vec3::new(20.0, 0.0, 0.0));
        let tip = Quat::from_rotation_z(right.y) * Vec3::Y;
        assert!(tip.x < -0.05, "tip leans left: {tip}");
    }

    #[test]
    fn huge_kicks_stay_bounded_and_the_spring_comes_to_rest() {
        let (mut lean, mut rate) = sway_step(Vec2::ZERO, Vec2::ZERO, Vec3::splat(1e6), 1.0 / 60.0);
        assert!(lean.length() <= MAX_LEAN + 1e-6);
        for _ in 0..600 {
            (lean, rate) = sway_step(lean, rate, Vec3::ZERO, 1.0 / 60.0);
        }
        assert!(
            lean.length() < 1e-3 && rate.length() < 1e-2,
            "at rest: {lean} {rate}"
        );
    }
}
