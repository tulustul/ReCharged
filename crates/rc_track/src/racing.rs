//! Racing line and speed profile (pure).
//!
//! - **Line:** lateral offsets that minimise the squared curvature (discrete second differences)
//!   of the path, kept [`LINE_MARGIN_U`] inside the edges; projected successive over-relaxation,
//!   coarse to fine (8 → 4 → 2 → 0.5 u) so long corners converge. Locked samples (jumps and
//!   their run-ups) stay centred.
//! - **Speed:** per point the corner limit `√(lateral / κ)` (lateral = grip × the model's
//!   lateral acceleration + gravity × sin(bank toward the inside)), capped at the surface's top
//!   speed; then a forward pass (acceleration) and a backward pass (braking) around the loop,
//!   starting from the slowest corner. No acceleration or braking in the air.

use bevy::math::Vec3;
use rc_core::GRAVITY;

/// Clearance between the racing line and the drivable edge (u): half a car plus room.
pub const LINE_MARGIN_U: f32 = 1.6;
/// Largest offset of the line where it is locked (jumps, u).
pub const LOCKED_OFFSET_U: f32 = 0.3;
/// Coarse-to-fine levels: (samples between line points, relaxation sweeps). The last level
/// smooths the interpolation kinks of the coarser ones.
const LEVELS: [(usize, usize); 4] = [(16, 1500), (8, 1500), (4, 1500), (1, 200)];
/// Over-relaxation factor (0 < ω < 2).
const OMEGA: f32 = 1.5;

/// Reference car of the speed profile (AI scales it per car and difficulty).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpeedModel {
    /// Top speed on a top-speed factor 1 surface (u/s).
    pub top_speed: f32,
    /// Cornering acceleration on grip 1 (u/s²).
    pub lateral_accel: f32,
    /// Longitudinal acceleration (u/s²).
    pub accel: f32,
    /// Braking deceleration (u/s²).
    pub brake: f32,
}

impl SpeedModel {
    /// A 3★ car (GDD §5.1: 30 u/s, 0 → top ≈ 3 s) on dry ground.
    pub const REFERENCE: Self = Self {
        top_speed: 30.0,
        lateral_accel: 30.0,
        accel: 12.0,
        brake: 25.0,
    };
}

/// One centreline sample as the line optimiser sees it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct LineInput {
    pub centre: Vec3,
    pub right: Vec3,
    pub half_width: f32,
    pub locked: bool,
}

impl LineInput {
    fn bound(&self) -> f32 {
        if self.locked {
            LOCKED_OFFSET_U.min(self.half_width)
        } else {
            (self.half_width - LINE_MARGIN_U).max(0.0)
        }
    }
}

/// Lateral offset of the racing line at every sample of a closed loop.
pub(crate) fn racing_line(input: &[LineInput]) -> Vec<f32> {
    let n = input.len();
    let mut offsets = vec![0.0f32; n];
    for (step, sweeps) in LEVELS {
        let m = (n / step).max(8).min(n);
        let idx: Vec<usize> = (0..m).map(|k| k * n / m).collect();
        let pts: Vec<LineInput> = idx.iter().map(|&i| input[i]).collect();
        let mut a: Vec<f32> = idx.iter().map(|&i| offsets[i]).collect();
        relax(&pts, &mut a, sweeps);
        // Interpolate the level's result back onto every sample.
        for k in 0..m {
            let (i0, i1) = (idx[k], if k + 1 < m { idx[k + 1] } else { n });
            for (j, o) in offsets[i0..i1].iter_mut().enumerate() {
                *o = a[k] + (a[(k + 1) % m] - a[k]) * j as f32 / (i1 - i0) as f32;
            }
        }
    }
    for (o, p) in offsets.iter_mut().zip(input) {
        *o = o.clamp(-p.bound(), p.bound());
    }
    offsets
}

fn relax(pts: &[LineInput], a: &mut [f32], sweeps: usize) {
    let m = pts.len();
    let at = |a: &[f32], k: usize| pts[k].centre + pts[k].right * a[k];
    for _ in 0..sweeps {
        for k in 0..m {
            let [km2, km1, kp1, kp2] = [m - 2, m - 1, 1, 2].map(|d| (k + d) % m);
            let target = (4.0 * (at(a, km1) + at(a, kp1)) - (at(a, km2) + at(a, kp2))) / 6.0;
            let best = (target - pts[k].centre).dot(pts[k].right);
            let bound = pts[k].bound();
            a[k] = (a[k] + OMEGA * (best - a[k])).clamp(-bound, bound);
        }
    }
}

/// One point of the racing line as the speed profile sees it.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SpeedInput {
    /// Distance to the next point (u).
    pub step: f32,
    /// Path curvature (1/u).
    pub curvature: f32,
    /// sin(bank toward the inside of the turn); negative = off-camber.
    pub bank_help: f32,
    /// Surface grip multiplier.
    pub grip: f32,
    /// Surface top-speed factor.
    pub top: f32,
    /// In the air (gap): no corner limit, no acceleration or braking.
    pub airborne: bool,
}

/// Corner-limited speed of one point (u/s).
pub(crate) fn corner_limit(p: &SpeedInput, model: &SpeedModel) -> f32 {
    let top = model.top_speed * p.top;
    if p.airborne || p.curvature < 1e-6 {
        return top;
    }
    let lateral =
        (model.lateral_accel * p.grip + GRAVITY * p.bank_help).max(0.5 * model.lateral_accel);
    (lateral / p.curvature).sqrt().min(top)
}

/// Target speed at every point of a closed loop (u/s).
pub(crate) fn speed_profile(pts: &[SpeedInput], model: &SpeedModel) -> Vec<f32> {
    let n = pts.len();
    let mut v: Vec<f32> = pts.iter().map(|p| corner_limit(p, model)).collect();
    let start = (0..n).min_by(|&a, &b| v[a].total_cmp(&v[b])).unwrap_or(0);
    for step in 1..n {
        let i = (start + step) % n;
        let prev = (i + n - 1) % n;
        let reach = if pts[prev].airborne {
            v[prev]
        } else {
            (v[prev] * v[prev] + 2.0 * model.accel * pts[prev].step).sqrt()
        };
        v[i] = v[i].min(reach);
    }
    for step in 1..n {
        let i = (start + n - step) % n;
        if !pts[i].airborne {
            let next = v[(i + 1) % n];
            v[i] = v[i].min((next * next + 2.0 * model.brake * pts[i].step).sqrt());
        }
    }
    v
}

/// Menger curvature through three points (1/u), and whether the path turns right (seen from
/// `up`).
pub(crate) fn curvature(a: Vec3, b: Vec3, c: Vec3, up: Vec3) -> (f32, bool) {
    let cross = (b - a).cross(c - b);
    let denom = a.distance(b) * b.distance(c) * a.distance(c);
    let k = if denom > 1e-9 {
        2.0 * (b - a).cross(c - a).length() / denom
    } else {
        0.0
    };
    (k, cross.dot(up) < 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    /// A stadium: two 60 u straights joined by two radius-20 half circles, centreline samples
    /// every 0.5 u, clockwise from above (right turns).
    fn stadium(width: f32) -> Vec<LineInput> {
        let mut pts = Vec::new();
        let straight = 60.0;
        let r = 20.0;
        let mut push = |c: Vec3, f: Vec3| {
            pts.push(LineInput {
                centre: c,
                right: f.cross(Vec3::Y),
                half_width: width / 2.0,
                locked: false,
            })
        };
        let steps = (straight / 0.5) as usize;
        for i in 0..steps {
            push(Vec3::new(0.0, 0.0, -(i as f32) * 0.5), Vec3::NEG_Z);
        }
        let arc = (PI * r / 0.5) as usize;
        for i in 0..arc {
            let a = i as f32 / arc as f32 * PI;
            push(
                Vec3::new(r - r * a.cos(), 0.0, -straight - r * a.sin()),
                Vec3::new(a.sin(), 0.0, -a.cos()),
            );
        }
        for i in 0..steps {
            push(Vec3::new(2.0 * r, 0.0, -straight + i as f32 * 0.5), Vec3::Z);
        }
        for i in 0..arc {
            let a = i as f32 / arc as f32 * PI;
            push(
                Vec3::new(r + r * a.cos(), 0.0, r * a.sin()),
                Vec3::new(-a.sin(), 0.0, a.cos()),
            );
        }
        pts
    }

    /// Reference-car lap time along a closed path of samples (curvature over ±2 u).
    fn lap_time(pts: &[Vec3]) -> f32 {
        let n = pts.len();
        let input: Vec<SpeedInput> = (0..n)
            .map(|i| {
                let [a, b, c] = [n - 4, 0, 4].map(|d| pts[(i + d) % n]);
                SpeedInput {
                    step: b.distance(pts[(i + 1) % n]),
                    curvature: curvature(a, b, c, Vec3::Y).0,
                    grip: 1.0,
                    top: 1.0,
                    ..Default::default()
                }
            })
            .collect();
        let v = speed_profile(&input, &SpeedModel::REFERENCE);
        input.iter().zip(&v).map(|(p, v)| p.step / v).sum()
    }

    #[test]
    fn racing_line_straightens_corners() {
        let input = stadium(12.0);
        let offsets = racing_line(&input);
        for (o, p) in offsets.iter().zip(&input) {
            assert!(
                o.abs() <= p.half_width - LINE_MARGIN_U + 1e-4,
                "offset {o} beyond the margin"
            );
        }
        let line: Vec<Vec3> = input
            .iter()
            .zip(&offsets)
            .map(|(p, o)| p.centre + p.right * *o)
            .collect();
        let centre: Vec<Vec3> = input.iter().map(|p| p.centre).collect();
        let (tl, tc) = (lap_time(&line), lap_time(&centre));
        assert!(
            tl < 0.95 * tc,
            "racing line lap {tl:.2} s vs centreline {tc:.2} s"
        );
        // Apex of the first turn (a right turn): the line hugs the inside (positive offset).
        let apex = 120 + (PI * 20.0) as usize;
        assert!(offsets[apex] > 2.0, "apex offset {}", offsets[apex]);
    }

    #[test]
    fn racing_line_stays_centred_where_locked() {
        let mut input = stadium(12.0);
        for p in input.iter_mut().skip(40).take(20) {
            p.locked = true;
        }
        let offsets = racing_line(&input);
        for (o, p) in offsets.iter().zip(&input) {
            let bound = if p.locked {
                LOCKED_OFFSET_U
            } else {
                p.half_width - LINE_MARGIN_U
            };
            assert!(o.abs() <= bound + 1e-4, "offset {o} beyond {bound}");
        }
    }

    #[test]
    fn speed_profile_respects_limits() {
        let model = SpeedModel::REFERENCE;
        let input = stadium(12.0);
        let n = input.len();
        let pts: Vec<SpeedInput> = (0..n)
            .map(|i| {
                let [a, b, c] = [n - 1, 0, 1].map(|d| input[(i + d) % n].centre);
                SpeedInput {
                    step: b.distance(c),
                    curvature: curvature(a, b, c, Vec3::Y).0,
                    grip: 1.0,
                    top: 1.0,
                    airborne: (300..310).contains(&i),
                    ..Default::default()
                }
            })
            .collect();
        let v = speed_profile(&pts, &model);
        let corner = (model.lateral_accel * 20.0).sqrt();
        for i in 0..n {
            let j = (i + 1) % n;
            assert!(v[i] > 0.0 && v[i] <= corner_limit(&pts[i], &model) + 1e-3);
            if !pts[i].airborne {
                assert!(
                    v[j] * v[j] - v[i] * v[i] <= 2.0 * model.accel * pts[i].step + 1e-2,
                    "accel at {i}"
                );
                assert!(
                    v[i] * v[i] - v[j] * v[j] <= 2.0 * model.brake * pts[i].step + 1e-2,
                    "brake at {i}"
                );
            } else {
                assert!(v[j] <= v[i] + 1e-4, "no acceleration in the air");
            }
        }
        let min = v.iter().copied().fold(f32::MAX, f32::min);
        let max = v.iter().copied().fold(0.0, f32::max);
        assert!((min - corner).abs() < 0.5, "corner speed {min} vs {corner}");
        assert!(max > corner + 3.0 && max <= model.top_speed);
    }

    #[test]
    fn curvature_sign_and_magnitude() {
        let r = 10.0;
        let p = |a: f32| Vec3::new(r - r * a.cos(), 0.0, -r * a.sin());
        let (k, right) = curvature(p(0.0), p(0.1), p(0.2), Vec3::Y);
        assert!((k - 0.1).abs() < 1e-3 && right);
        let (_, right) = curvature(p(0.2), p(0.1), p(0.0), Vec3::Y);
        assert!(!right);
    }
}
