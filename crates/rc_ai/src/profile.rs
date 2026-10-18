//! Per-car speed plan (pure): the corner- and brake-limited speed along the racing line for one
//! car's capability (stars → top speed, launch, cornering grip) and one difficulty's cornering
//! margin. rc_track's `target_speed_at` is the reference car's; every AI builds its own plan when
//! it is created (ARCHITECTURE §5.6 "speed profile × difficulty × per-car profile").
//!
//! The plan has no top-speed cap: straights stay at [`UNCAPPED_UPS`] and the caller caps them
//! with the car's *current* top speed (surface, catch-up, boost), so the braking zones are right
//! whatever the car is doing.

use bevy::math::Vec3;
use rc_core::{CarStars, GRAVITY, SurfaceKind, top_speed_for_stars, zero_to_top_secs_for_stars};
use rc_track::Track;

/// Spacing of the plan's samples along the centreline (u).
const PROFILE_DS_U: f32 = 1.0;
/// Racing-line points either side of a sample used for its curvature (u).
const CURVATURE_SPAN_U: f32 = 3.0;
/// Speed of a sample with no corner limit (u/s): above every car's capped top speed.
pub const UNCAPPED_UPS: f32 = 80.0;
/// Off-camber banking never lowers the cornering acceleration below this share.
const MIN_BANK_SHARE: f32 = 0.5;

/// What a car can do, from its catalog stars (GDD §5.1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CarCaps {
    /// Top speed on a top-speed factor 1 surface (u/s).
    pub top_speed: f32,
    /// Seconds from standstill to top speed.
    pub zero_to_top: f32,
    /// Cornering acceleration on grip 1 (u/s²).
    pub lateral_accel: f32,
}

impl CarCaps {
    /// Full-throttle acceleration (u/s²) at `speed` toward `top`: rc_vehicle's drive taper
    /// a(v) = A·(1 − v/V)^½, with A reaching 99 % of the top speed in `zero_to_top`.
    pub fn accel_at(&self, speed: f32, top: f32) -> f32 {
        if top <= 0.0 || speed >= top {
            return 0.0;
        }
        let launch = self.top_speed * (1.0 - 0.01f32.sqrt()) / (0.5 * self.zero_to_top);
        launch * (1.0 - speed.max(0.0) / top).sqrt()
    }

    /// Capabilities of a car with `stars`; `lateral_by_handling` maps Handling stars 1–5 (the
    /// last value covers more stars than listed).
    pub fn from_stars(stars: CarStars, lateral_by_handling: &[f32]) -> Self {
        let i = usize::from(stars.handling.clamp(1, 5) - 1).min(lateral_by_handling.len() - 1);
        Self {
            top_speed: top_speed_for_stars(stars.speed),
            zero_to_top: zero_to_top_secs_for_stars(stars.accel),
            lateral_accel: lateral_by_handling[i],
        }
    }
}

/// Menger curvature (1/u) of three points seen from above, and whether they turn right.
pub fn curvature(a: Vec3, b: Vec3, c: Vec3) -> (f32, bool) {
    let flat = |p: Vec3| Vec3::new(p.x, 0.0, p.z);
    let (a, b, c) = (flat(a), flat(b), flat(c));
    let denom = a.distance(b) * b.distance(c) * a.distance(c);
    if denom < 1e-9 {
        return (0.0, false);
    }
    let cross = (b - a).cross(c - b);
    (2.0 * cross.length() / denom, cross.y < 0.0)
}

/// Corner-limited speed (u/s) at curvature `kappa` with cornering acceleration `lateral`
/// (grip included) and `bank_help` = sin(bank toward the inside). Banking toward the inside
/// does not help: rc_vehicle's speed-sensitive lock is sized for flat ground (full lock asks for
/// a fixed share of the grip), so the turnable curvature at a speed stays the flat one. An
/// off-camber bank costs grip.
pub fn corner_speed(kappa: f32, lateral: f32, bank_help: f32) -> f32 {
    if kappa < 1e-5 {
        return UNCAPPED_UPS;
    }
    let a = (lateral + GRAVITY * bank_help.min(0.0)).max(MIN_BANK_SHARE * lateral);
    (a / kappa).sqrt().min(UNCAPPED_UPS)
}

/// Lowers `v` so that every sample can brake into the next one at `decel` over `ds`, around a
/// closed loop; airborne samples cannot brake at all.
pub fn brake_pass(v: &mut [f32], airborne: &[bool], ds: f32, decel: f32) {
    let n = v.len();
    if n == 0 {
        return;
    }
    let start = (0..n).min_by(|&a, &b| v[a].total_cmp(&v[b])).unwrap_or(0);
    for step in 1..n {
        let i = (start + n - step) % n;
        let next = v[(i + 1) % n];
        let reach = match airborne[i] {
            true => next,
            false => (next * next + 2.0 * decel * ds).sqrt(),
        };
        v[i] = v[i].min(reach);
    }
}

/// One car's speed plan along the lap.
#[derive(Debug, Clone, PartialEq)]
pub struct SpeedProfile {
    ds: f32,
    length: f32,
    /// Corner/brake-limited speed per sample (u/s, uncapped on straights).
    corner: Vec<f32>,
    /// Surface under the centreline per sample (top-speed factor, weather grip).
    surface: Vec<SurfaceKind>,
    /// Curvature of the racing line per sample (1/u, signed: + = right turn).
    kappa: Vec<f32>,
}

impl SpeedProfile {
    /// Plans `caps` on `track` with `corner_grip` × the car's cornering grip and braking at
    /// `brake_decel`.
    pub fn build(track: &Track, caps: &CarCaps, corner_grip: f32, brake_decel: f32) -> Self {
        let length = track.length();
        let n = ((length / PROFILE_DS_U).ceil() as usize).max(8);
        let ds = length / n as f32;
        let mut corner = Vec::with_capacity(n);
        let mut surface = Vec::with_capacity(n);
        let mut kappa = Vec::with_capacity(n);
        let mut airborne = Vec::with_capacity(n);
        for i in 0..n {
            let s = i as f32 * ds;
            let frame = track.frame_at(s);
            let (k, right) = curvature(
                track.racing_line_at(s - CURVATURE_SPAN_U),
                track.racing_line_at(s),
                track.racing_line_at(s + CURVATURE_SPAN_U),
            );
            let grip = frame.surface.props().grip;
            let bank_help = frame.bank.sin() * if right { 1.0 } else { -1.0 };
            corner.push(match frame.in_gap {
                true => UNCAPPED_UPS,
                false => corner_speed(k, caps.lateral_accel * corner_grip * grip, bank_help),
            });
            surface.push(frame.surface);
            kappa.push(if right { k } else { -k });
            airborne.push(frame.in_gap);
        }
        brake_pass(&mut corner, &airborne, ds, brake_decel);
        Self {
            ds,
            length,
            corner,
            surface,
            kappa,
        }
    }

    fn lerp(&self, values: &[f32], s: f32) -> f32 {
        let x = s.rem_euclid(self.length) / self.ds;
        let i = (x.floor() as usize).min(values.len() - 1);
        let t = x - i as f32;
        values[i] + (values[(i + 1) % values.len()] - values[i]) * t
    }

    fn index(&self, s: f32) -> usize {
        ((s.rem_euclid(self.length) / self.ds) as usize).min(self.corner.len() - 1)
    }

    /// Corner/brake-limited speed at `s` on dry ground (u/s; [`UNCAPPED_UPS`] on straights).
    pub fn corner_at(&self, s: f32) -> f32 {
        self.lerp(&self.corner, s)
    }

    /// Surface under the centreline at `s`.
    pub fn surface_at(&self, s: f32) -> SurfaceKind {
        self.surface[self.index(s)]
    }

    /// Signed racing-line curvature at `s` (1/u, + = right turn).
    pub fn kappa_at(&self, s: f32) -> f32 {
        self.lerp(&self.kappa, s)
    }

    /// Sample spacing (u).
    pub fn spacing(&self) -> f32 {
        self.ds
    }

    /// Lap length (u).
    pub fn length(&self) -> f32 {
        self.length
    }
}

/// Highest speed at the current position from which the car can still brake (at `decel`) to
/// `target(d)` at every distance `d` ahead up to its braking distance.
pub fn brake_point_speed(v: f32, decel: f32, step: f32, target: impl Fn(f32) -> f32) -> f32 {
    let reach = (v * v / (2.0 * decel)).max(step);
    let mut best = target(0.0);
    let mut d = step;
    while d <= reach + step {
        best = best.min((target(d).powi(2) + 2.0 * decel * d).sqrt());
        d += step;
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curvature_of_a_circle() {
        let r = 12.0;
        let p = |a: f32| Vec3::new(r - r * a.cos(), 3.0, -r * a.sin());
        let (k, right) = curvature(p(0.0), p(0.2), p(0.4));
        assert!((k - 1.0 / r).abs() < 1e-4, "{k}");
        assert!(
            right,
            "clockwise from above, heading −Z then +X, is a right turn"
        );
        let (_, right) = curvature(p(0.4), p(0.2), p(0.0));
        assert!(!right);
        assert_eq!(curvature(Vec3::ZERO, Vec3::X, Vec3::X * 2.0).0, 0.0);
    }

    #[test]
    fn corner_speed_follows_grip_and_banking() {
        let flat = corner_speed(1.0 / 20.0, 30.0, 0.0);
        assert!((flat - (30.0f32 * 20.0).sqrt()).abs() < 1e-3);
        assert_eq!(
            corner_speed(1.0 / 20.0, 30.0, 0.26),
            flat,
            "the lock limit ignores banking"
        );
        assert!(corner_speed(1.0 / 20.0, 30.0, -0.26) < flat, "off-camber");
        assert!(corner_speed(1.0 / 20.0, 15.0, 0.0) < flat, "less grip");
        assert_eq!(corner_speed(0.0, 30.0, 0.0), UNCAPPED_UPS);
    }

    #[test]
    fn brake_pass_allows_braking_into_every_corner() {
        let mut v = vec![UNCAPPED_UPS; 400];
        v[60] = 10.0;
        let air = vec![false; 400];
        brake_pass(&mut v, &air, 1.0, 25.0);
        for i in 0..399 {
            assert!(
                v[i] * v[i] - v[i + 1] * v[i + 1] <= 2.0 * 25.0 + 1e-3,
                "at {i}"
            );
        }
        assert!((v[59] - (100.0f32 + 50.0).sqrt()).abs() < 1e-3);
        assert_eq!(v[61], UNCAPPED_UPS, "no limit after the corner");
        // A loop: the end of the lap brakes for the corner at the start of the next one.
        assert!((v[399] - (100.0f32 + 50.0 * 61.0).sqrt()).abs() < 1e-3);
    }

    #[test]
    fn no_braking_in_the_air() {
        let mut v = vec![UNCAPPED_UPS; 50];
        v[30] = 12.0;
        let mut air = vec![false; 50];
        air[25..30].fill(true);
        brake_pass(&mut v, &air, 1.0, 25.0);
        for (i, speed) in v.iter().enumerate().take(30).skip(25) {
            assert_eq!(*speed, 12.0, "airborne sample {i} must already be slow");
        }
        assert!(v[24] > 12.0);
    }

    #[test]
    fn brake_point_sees_corners_ahead() {
        // A 10 u/s corner 20 u ahead: from 35 u/s at 25 u/s² the car must brake now.
        let target = |d: f32| if d >= 20.0 { 10.0 } else { 40.0 };
        let v = brake_point_speed(35.0, 25.0, 1.0, target);
        assert!(
            (v - (100.0f32 + 2.0 * 25.0 * 20.0).sqrt()).abs() < 1e-3,
            "{v}"
        );
        assert_eq!(
            brake_point_speed(30.0, 25.0, 1.0, target),
            40.0,
            "not yet at 30 u/s"
        );
        let far = |d: f32| if d >= 200.0 { 10.0 } else { 40.0 };
        assert_eq!(
            brake_point_speed(30.0, 25.0, 1.0, far),
            40.0,
            "beyond braking reach"
        );
    }

    #[test]
    fn caps_come_from_stars() {
        let stars = CarStars {
            speed: 4,
            accel: 5,
            handling: 2,
            weight: 3,
        };
        let caps = CarCaps::from_stars(stars, &[1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(caps.top_speed, 33.0);
        assert_eq!(caps.zero_to_top, 2.5);
        assert_eq!(caps.lateral_accel, 2.0);
        // Integrating the acceleration reaches 99 % of the top speed in zero_to_top.
        let (mut v, mut t) = (0.0, 0.0);
        while v < 0.99 * caps.top_speed {
            v += caps.accel_at(v, caps.top_speed) / 600.0;
            t += 1.0 / 600.0;
        }
        assert!((t - caps.zero_to_top).abs() < 0.05, "0 → 99 % in {t:.2} s");
        assert_eq!(
            caps.accel_at(40.0, 33.0),
            0.0,
            "no drive above the top speed"
        );
    }
}
