//! Corners of a built track (pure): where the centreline's heading turns, which way, how far and
//! how hard. The HUD's turn indicator and the turn signs along the track read them.
//!
//! The heading change per sample (seen from above; jumps fly straight) is averaged over
//! ±[`SMOOTH_U`]; a corner is a run of samples turning one way faster than [`MIN_TURN_RATE`]
//! (runs closer than [`MERGE_U`] join) whose heading changes at least [`MIN_ANGLE_DEG`].
//! Severity comes from the reference car's slowest target speed in the corner and from the angle.

use bevy::math::Vec3;
use rc_core::TrackTurn;

use crate::racing::SpeedModel;
use crate::track::Node;

/// Heading rate is averaged over this much track either side of a sample (u).
pub const SMOOTH_U: f32 = 3.0;
/// A corner turns faster than this (rad/u: a radius under 40 u).
pub const MIN_TURN_RATE: f32 = 1.0 / 40.0;
/// Runs turning the same way closer than this are one corner (u).
pub const MERGE_U: f32 = 4.0;
/// Smallest heading change that counts as a corner (degrees).
pub const MIN_ANGLE_DEG: f32 = 25.0;
/// Severity 2 from this heading change (degrees)…
const MEDIUM_ANGLE_DEG: f32 = 75.0;
/// …or a slowest target speed under this share of the reference top speed.
const MEDIUM_SPEED: f32 = 0.75;
/// Severity 3 (sharp) from this heading change (degrees: a hairpin)…
const SHARP_ANGLE_DEG: f32 = 150.0;
/// …or a slowest target speed under this share of the reference top speed.
const SHARP_SPEED: f32 = 0.5;

/// Signed heading change from `a` to `b` seen from above (radians, + = right).
fn turn(a: Vec3, b: Vec3) -> f32 {
    let (a, b) = (a.with_y(0.0), b.with_y(0.0));
    if a.length_squared() < 1e-9 || b.length_squared() < 1e-9 {
        return 0.0;
    }
    // Y up, −Z forward: a right turn has a negative cross product's Y.
    -a.cross(b).y.atan2(a.dot(b))
}

/// Corners of the loop sampled by `nodes` every `ds` u, sorted by `start_s`.
pub(crate) fn turns(nodes: &[Node], ds: f32) -> Vec<TrackTurn> {
    let n = nodes.len();
    if n < 3 {
        return Vec::new();
    }
    let step: Vec<f32> = (0..n)
        .map(|i| {
            let (a, b) = (&nodes[i].frame, &nodes[(i + 1) % n].frame);
            if a.in_gap || b.in_gap {
                0.0
            } else {
                turn(a.forward, b.forward)
            }
        })
        .collect();
    let w = ((SMOOTH_U / ds).round() as usize).max(1);
    let rate: Vec<f32> = (0..n)
        .map(|i| {
            let window = (0..=2 * w).map(|k| {
                step[(i as isize + k as isize - w as isize).rem_euclid(n as isize) as usize]
            });
            window.sum::<f32>() / ((2 * w + 1) as f32 * ds)
        })
        .collect();
    let side = |i: usize| -> i8 {
        let r = rate[i % n];
        if r > MIN_TURN_RATE {
            1
        } else if r < -MIN_TURN_RATE {
            -1
        } else {
            0
        }
    };
    // Start the scan on a straight so no corner is split by the wrap.
    let Some(origin) = (0..n).find(|&i| side(i) == 0) else {
        let angle: f32 = step.iter().sum();
        return vec![corner(nodes, ds, 0, n, angle)];
    };
    let mut runs: Vec<(usize, usize, i8)> = Vec::new(); // (first, end exclusive, side), scan order
    let mut i = 0;
    while i < n {
        let s = side(origin + i);
        if s == 0 {
            i += 1;
            continue;
        }
        let first = i;
        while i < n && side(origin + i) == s {
            i += 1;
        }
        let merge = (MERGE_U / ds).ceil() as usize;
        match runs.last_mut() {
            Some(last) if last.2 == s && first - last.1 <= merge => last.1 = i,
            _ => runs.push((first, i, s)),
        }
    }
    let mut out: Vec<TrackTurn> = runs
        .into_iter()
        .filter_map(|(first, end, _)| {
            let angle: f32 = (first..end).map(|k| step[(origin + k) % n]).sum();
            (angle.abs().to_degrees() >= MIN_ANGLE_DEG)
                .then(|| corner(nodes, ds, (origin + first) % n, end - first, angle))
        })
        .collect();
    out.sort_by(|a, b| a.start_s.total_cmp(&b.start_s));
    out
}

/// The corner over `count` samples from `first`, turning `angle` radians (+ = right).
fn corner(nodes: &[Node], ds: f32, first: usize, count: usize, angle: f32) -> TrackTurn {
    let n = nodes.len();
    let slowest = (0..count)
        .map(|k| nodes[(first + k) % n].speed)
        .fold(f32::INFINITY, f32::min);
    let share = slowest / SpeedModel::REFERENCE.top_speed;
    let degrees = angle.abs().to_degrees();
    let severity = if degrees >= SHARP_ANGLE_DEG || share < SHARP_SPEED {
        3
    } else if degrees >= MEDIUM_ANGLE_DEG || share < MEDIUM_SPEED {
        2
    } else {
        1
    };
    TrackTurn {
        start_s: nodes[first].frame.s,
        span: count as f32 * ds,
        right: angle > 0.0,
        angle: angle.abs(),
        severity,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turning_toward_plus_x_is_right() {
        let (fwd, right) = (Vec3::NEG_Z, Vec3::X);
        assert!((turn(fwd, right) - std::f32::consts::FRAC_PI_2).abs() < 1e-5);
        assert!((turn(fwd, -right) + std::f32::consts::FRAC_PI_2).abs() < 1e-5);
        assert_eq!(turn(fwd, fwd), 0.0);
        assert_eq!(turn(Vec3::Y, fwd), 0.0, "a vertical tangent has no heading");
    }
}
