//! Shortcuts (`Track::shortcuts`): every brain plans once which shortcuts are faster for its car —
//! the path's own corners and surfaces against the racing line over the same span, both driven
//! with the car's acceleration and braking from the same entry speed to the same exit speed — and
//! a driver at Pro or above takes one on a lap with its difficulty's chance × its aggression.
//! On a shortcut the planned line, the speed plan and the legal corridor come from its plan.

use bevy::math::Vec3;
use rc_core::prelude::*;
use rc_track::{RibbonStyle, Shortcut, Track};

use crate::profile::{CarCaps, SpeedProfile, corner_speed, curvature};

/// Samples of a plan along the centreline (u).
const PLAN_DS_U: f32 = 1.0;
/// The path is evaluated at points at most this far apart (u).
const PATH_STEP_U: f32 = 1.0;
/// Path points either side used for its curvature (points ≈ u).
const CURVATURE_SPAN: usize = 3;
/// The car blends from the racing line onto the shortcut (and back) over this distance (u, about
/// the pursuit lookahead at speed): the path's curvature at its ends leans on the racing line
/// this far before the entry and after the exit.
const JOIN_U: f32 = 12.0;
/// A shortcut is planned only when it saves at least this share of the span's time.
const MIN_SAVING: f32 = 0.03;
/// Narrower shortcuts (planks, pallet tops) are never planned: one slip and the car falls (u).
pub(crate) const MIN_WIDTH_U: f32 = 5.0;
/// On a shortcut the car keeps this far inside its width's edges (u).
const EDGE_U: f32 = 1.0;
/// Whether to take a shortcut is decided once per lap, this far before its entry (u).
pub(crate) const DECIDE_U: f32 = 40.0;
/// Knockable (dynamic) props this close beyond a shortcut's width rule it out (u).
const KNOCKABLE_REACH_U: f32 = 0.5;
/// Speeds below this (u/s) count as this in the time estimates (no division by a standstill).
const MIN_ESTIMATE_UPS: f32 = 1.0;
/// Chance per lap of taking a shortcut that is faster for the car, by difficulty (Rookie, Pro,
/// Elite, Elite+), before the personality scaling (× 0.5 + aggression): never at Rookie.
// ponytail: a code table keeps `ai.ron`'s format readable by builds made before it; move it into
// `difficulty.*` there with the next format change.
const CHANCE_BY_DIFFICULTY: [f32; 4] = [0.0, 0.3, 0.5, 0.6];
/// A car this far (u) outside a shortcut's corridor while on its span has missed it: drop it.
const LOST_U: f32 = 3.0;

/// A shortcut that is faster for one car, sampled along the centreline from its entry.
#[derive(Debug, Clone, PartialEq)]
pub struct ShortcutPlan {
    /// Centreline arc length of the entry (u).
    pub entry_s: f32,
    /// Centreline length it spans (u).
    pub span: f32,
    /// Lateral room either side of its line (u).
    pub half_width: f32,
    /// Seconds it saves over the racing line (estimate).
    pub saving: f32,
    lateral: Vec<f32>,
    corner: Vec<f32>,
    kappa: Vec<f32>,
    surface: Vec<SurfaceKind>,
}

/// Seconds to drive through points with speed limits `limit` (u/s: corners, top speed) and top
/// speeds `tops`, `len[i]` u apart, entering at `v_in` and leaving at `v_out` at most: full
/// throttle with the car's acceleration, braking at `brake` (u/s²) into slower points.
pub(crate) fn traverse_secs(
    len: &[f32],
    limit: &[f32],
    tops: &[f32],
    (v_in, v_out): (f32, f32),
    caps: &CarCaps,
    brake: f32,
) -> f32 {
    let n = limit.len();
    if n < 2 || len.len() != n - 1 {
        return 0.0;
    }
    let mut v = limit.to_vec();
    v[0] = v[0].min(v_in);
    v[n - 1] = v[n - 1].min(v_out);
    for i in 0..n - 1 {
        let reach = (v[i] * v[i] + 2.0 * caps.accel_at(v[i], tops[i]) * len[i]).sqrt();
        v[i + 1] = v[i + 1].min(reach);
    }
    for i in (0..n - 1).rev() {
        v[i] = v[i].min((v[i + 1] * v[i + 1] + 2.0 * brake * len[i]).sqrt());
    }
    len.iter()
        .enumerate()
        .map(|(i, l)| l / (0.5 * (v[i] + v[i + 1])).max(MIN_ESTIMATE_UPS))
        .sum()
}

/// Uniform Catmull-Rom point between `p1` and `p2` at `t` (0–1).
fn catmull_rom(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
    let (t2, t3) = (t * t, t * t * t);
    0.5 * (2.0 * p1
        + (p2 - p0) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
        + (3.0 * p1 - p0 - 3.0 * p2 + p3) * t3)
}

/// The authored waypoints `path` as a smooth curve through them, a point every ≤ `step` u. The
/// ends keep the authored first and last segments' directions: those thread the barrier openings
/// the path enters and leaves by.
fn smooth(path: &[Vec3], step: f32) -> Vec<Vec3> {
    let (Some(&first), Some(&last)) = (path.first(), path.last()) else {
        return Vec::new();
    };
    let before = 2.0 * first - path.get(1).copied().unwrap_or(first);
    let after = 2.0 * last
        - path
            .get(path.len().wrapping_sub(2))
            .copied()
            .unwrap_or(last);
    let ctrl: Vec<Vec3> = std::iter::once(before)
        .chain(path.iter().copied())
        .chain(std::iter::once(after))
        .collect();
    let mut out = Vec::new();
    for w in ctrl.windows(4) {
        let pieces = (w[1].distance(w[2]) / step).ceil().max(1.0) as usize;
        out.extend(
            (0..pieces).map(|k| catmull_rom(w[0], w[1], w[2], w[3], k as f32 / pieces as f32)),
        );
    }
    out.extend(path.last().copied());
    out
}

/// The surface under a path point at track coordinates (`s`, `lateral`): the ribbon where the
/// point is on it, else the topmost ground polygon under it, else the centreline's surface.
fn surface_under(track: &Track, p: Vec3, s: f32, lateral: f32) -> SurfaceKind {
    let frame = track.frame_at(s);
    let ribbon = track.style() != RibbonStyle::Invisible && !frame.in_gap;
    if ribbon && lateral.abs() <= frame.half_width {
        return frame.surface;
    }
    track
        .ground()
        .iter()
        .filter(|g| g.contains(p))
        .max_by(|a, b| a.height.total_cmp(&b.height))
        .map_or(frame.surface, |g| g.surface)
}

impl ShortcutPlan {
    /// Plans `shortcut` for a car with `caps` cornering on `grip_share` of its grip and braking at
    /// `brake` (u/s²); `None` when it is narrower than [`MIN_WIDTH_U`] or not faster than the
    /// racing line on the car's speed plan `main`.
    pub fn build(
        track: &Track,
        shortcut: &Shortcut,
        caps: &CarCaps,
        grip_share: f32,
        brake: f32,
        main: &SpeedProfile,
    ) -> Option<Self> {
        let length = track.length();
        let span = (shortcut.exit_s - shortcut.entry_s).rem_euclid(length);
        if shortcut.width < MIN_WIDTH_U || span < 2.0 * PLAN_DS_U || span > 0.5 * length {
            return None;
        }
        // The path in track coordinates (distance along the span, lateral), with its corners
        // and surfaces.
        let rl = |s: f32| track.racing_line_at(s);
        let join = JOIN_U;
        let pts = smooth(&shortcut.path, PATH_STEP_U);
        let n = pts.len();
        // Knockable props across the path (a gate of toy blocks): drivers ignore dynamic props,
        // and a block tower hit at speed stops the car dead — not worth the gamble.
        let reach = 0.5 * shortcut.width + KNOCKABLE_REACH_U;
        let blocked = track.props().iter().any(|p| {
            track.prop_def(&p.kind).is_some_and(|d| d.dynamic)
                && pts
                    .iter()
                    .any(|q| q.distance(p.transform.translation) < reach)
        });
        if blocked {
            return None;
        }
        let mut hint = shortcut.entry_s;
        let mut along: Vec<(f32, f32)> = Vec::with_capacity(n);
        for p in &pts {
            let tp = track.project(*p, Some(hint));
            hint = tp.s;
            let d = (tp.s - shortcut.entry_s).rem_euclid(length).min(span);
            let d = along.last().map_or(d, |&(last, _)| d.max(last));
            along.push((d, tp.lateral));
        }
        let surface: Vec<SurfaceKind> = (0..n)
            .map(|i| surface_under(track, pts[i], shortcut.entry_s + along[i].0, along[i].1))
            .collect();
        // Curvature along the path, joined to the racing line before and after it: turning onto
        // and off the shortcut are corners too.
        let joined: Vec<Vec3> = std::iter::once(rl(shortcut.entry_s - join))
            .chain(pts.iter().copied())
            .chain(std::iter::once(rl(shortcut.exit_s + join)))
            .collect();
        let last = joined.len() - 1;
        let kappa: Vec<f32> = (1..=n)
            .map(|j| {
                let a = joined[j.saturating_sub(CURVATURE_SPAN)];
                let c = joined[(j + CURVATURE_SPAN).min(last)];
                let (k, right) = curvature(a, joined[j], c);
                if right { k } else { -k }
            })
            .collect();
        let top_of = |s: SurfaceKind| caps.top_speed * s.props().top_speed;
        let tops: Vec<f32> = surface.iter().map(|&s| top_of(s)).collect();
        let limit: Vec<f32> = (0..n)
            .map(|i| {
                let lateral = caps.lateral_accel * grip_share * surface[i].props().grip;
                corner_speed(kappa[i].abs(), lateral, 0.0).min(tops[i])
            })
            .collect();
        let len: Vec<f32> = pts.windows(2).map(|w| w[0].distance(w[1])).collect();

        // The racing line over the same span, from the same entry speed to the same exit speed.
        let m = ((span / PLAN_DS_U).ceil() as usize).max(2);
        let at = |k: usize| shortcut.entry_s + span * k as f32 / m as f32;
        let line: Vec<Vec3> = (0..=m).map(|k| track.racing_line_at(at(k))).collect();
        let line_tops: Vec<f32> = (0..=m).map(|k| top_of(main.surface_at(at(k)))).collect();
        let line_limit: Vec<f32> = (0..=m)
            .map(|k| main.corner_at(at(k)).min(line_tops[k]))
            .collect();
        let line_len: Vec<f32> = line.windows(2).map(|w| w[0].distance(w[1])).collect();
        let ends = (line_limit[0], line_limit[m]);
        let t_line = traverse_secs(&line_len, &line_limit, &line_tops, ends, caps, brake);
        let t_short = traverse_secs(&len, &limit, &tops, ends, caps, brake);
        if t_short > t_line * (1.0 - MIN_SAVING) {
            return None;
        }

        // What the driver follows, per PLAN_DS_U of span: the corner limits braked into what
        // comes next on the path (in its own distances) and into the exit speed.
        let mut braked = limit.clone();
        braked[n - 1] = braked[n - 1].min(ends.1);
        for i in (0..n - 1).rev() {
            braked[i] = braked[i].min((braked[i + 1].powi(2) + 2.0 * brake * len[i]).sqrt());
        }
        let samples = (span / PLAN_DS_U).floor() as usize + 1;
        let mut plan = Self {
            entry_s: shortcut.entry_s,
            span,
            half_width: (0.5 * shortcut.width - EDGE_U).max(0.5),
            saving: t_line - t_short,
            lateral: Vec::with_capacity(samples),
            corner: Vec::with_capacity(samples),
            kappa: Vec::with_capacity(samples),
            surface: Vec::with_capacity(samples),
        };
        let mut i = 0;
        for j in 0..samples {
            let d = j as f32 * PLAN_DS_U;
            while i + 2 < n && along[i + 1].0 < d {
                i += 1;
            }
            let (d0, d1) = (along[i].0, along[i + 1].0);
            let t = if d1 > d0 {
                ((d - d0) / (d1 - d0)).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let mix = |a: f32, b: f32| a + (b - a) * t;
            plan.lateral.push(mix(along[i].1, along[i + 1].1));
            plan.corner.push(mix(braked[i], braked[i + 1]));
            plan.kappa.push(mix(kappa[i], kappa[i + 1]));
            plan.surface.push(surface[if t < 0.5 { i } else { i + 1 }]);
        }
        Some(plan)
    }

    /// Fractional sample index of `s` when `s` lies on the span.
    fn sample(&self, length: f32, s: f32) -> Option<f32> {
        let d = (s - self.entry_s).rem_euclid(length);
        (d <= self.span).then_some(d / PLAN_DS_U)
    }

    fn lerp(values: &[f32], x: f32) -> f32 {
        let i = (x.floor() as usize).min(values.len() - 1);
        let next = values[(i + 1).min(values.len() - 1)];
        values[i] + (next - values[i]) * (x - i as f32).clamp(0.0, 1.0)
    }

    /// `s` lies between the entry and the exit.
    pub fn covers(&self, length: f32, s: f32) -> bool {
        self.sample(length, s).is_some()
    }

    /// Distance from `s` to the entry along the track (u, in `[0, length)`).
    pub fn ahead_of(&self, length: f32, s: f32) -> f32 {
        (self.entry_s - s).rem_euclid(length)
    }

    /// The shortcut's line (lateral offset, u) at `s`.
    pub fn lateral_at(&self, length: f32, s: f32) -> Option<f32> {
        self.sample(length, s).map(|x| Self::lerp(&self.lateral, x))
    }

    /// Corner- and brake-limited speed (u/s, dry) at `s`.
    pub fn corner_at(&self, length: f32, s: f32) -> Option<f32> {
        self.sample(length, s).map(|x| Self::lerp(&self.corner, x))
    }

    /// Signed curvature of the path (1/u, + = right) at `s`.
    pub fn kappa_at(&self, length: f32, s: f32) -> Option<f32> {
        self.sample(length, s).map(|x| Self::lerp(&self.kappa, x))
    }

    /// Surface under the path at `s`.
    pub fn surface_at(&self, length: f32, s: f32) -> Option<SurfaceKind> {
        self.sample(length, s)
            .map(|x| self.surface[(x.round() as usize).min(self.surface.len() - 1)])
    }
}

/// Chance that a driver of `difficulty` with `aggression` (catalog) takes a shortcut on a lap.
pub(crate) fn take_chance(difficulty: Difficulty, aggression: f32) -> f32 {
    let base = match difficulty {
        Difficulty::Rookie => CHANCE_BY_DIFFICULTY[0],
        Difficulty::Pro => CHANCE_BY_DIFFICULTY[1],
        Difficulty::Elite => CHANCE_BY_DIFFICULTY[2],
        Difficulty::ElitePlus => CHANCE_BY_DIFFICULTY[3],
    };
    (base * (0.5 + aggression)).clamp(0.0, 1.0)
}

/// Which shortcut to drive (index into `plans`) and the entry already decided on: decides once
/// per approach — when an entry is within [`DECIDE_U`] ahead of `s`, for the best plan starting
/// there, with `roll(chance)` — and drops the shortcut once the car is past it or off it.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ShortcutChoice {
    /// The shortcut being driven or approached.
    pub active: Option<usize>,
    /// Entry `s` already decided on (re-armed once no shortcut is near).
    rolled: Option<f32>,
}

impl ShortcutChoice {
    /// One tick at track position (`s`, `lateral`).
    pub fn step(
        &mut self,
        plans: &[ShortcutPlan],
        length: f32,
        (s, lateral): (f32, f32),
        chance: f32,
        roll: impl FnOnce(f32) -> bool,
    ) {
        let near = |p: &ShortcutPlan| p.ahead_of(length, s) <= DECIDE_U || p.covers(length, s);
        let off = |p: &ShortcutPlan| {
            p.lateral_at(length, s)
                .is_some_and(|c| (lateral - c).abs() > p.half_width + LOST_U)
        };
        if self
            .active
            .is_some_and(|i| plans.get(i).is_none_or(|p| !near(p) || off(p)))
        {
            self.active = None;
        }
        let approaching = plans
            .iter()
            .enumerate()
            .filter(|(_, p)| !p.covers(length, s) && p.ahead_of(length, s) <= DECIDE_U)
            .max_by(|a, b| a.1.saving.total_cmp(&b.1.saving));
        match approaching {
            Some((i, p)) if self.rolled != Some(p.entry_s) => {
                self.rolled = Some(p.entry_s);
                if roll(chance) {
                    self.active = Some(i);
                }
            }
            Some(_) => {}
            None if !plans.iter().any(near) => self.rolled = None,
            None => {}
        }
    }

    /// The shortcut being driven, if it covers `s`.
    pub fn at<'a>(
        &self,
        plans: &'a [ShortcutPlan],
        length: f32,
        s: f32,
    ) -> Option<&'a ShortcutPlan> {
        self.active
            .and_then(|i| plans.get(i))
            .filter(|p| p.covers(length, s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rc_core::{CarStars, TrackVariant};

    fn caps() -> CarCaps {
        let stars = CarStars {
            speed: 3,
            accel: 3,
            handling: 3,
            weight: 3,
        };
        CarCaps::from_stars(stars, &[25.0, 27.5, 30.5, 32.5, 34.5])
    }

    #[test]
    fn traversal_time_follows_the_limits() {
        let c = caps();
        let top = c.top_speed;
        // 100 u flat out at the top speed: 100 / top.
        let len = vec![10.0; 10];
        let free = vec![top; 11];
        let t = traverse_secs(&len, &free, &free, (top, top), &c, 28.0);
        assert!((t - 100.0 / top).abs() < 1e-3, "{t}");
        // A 10 u/s corner in the middle costs braking and acceleration.
        let mut slow = free.clone();
        slow[5] = 10.0;
        let t2 = traverse_secs(&len, &slow, &free, (top, top), &c, 28.0);
        assert!(t2 > t + 0.5, "{t2} vs {t}");
        // Entering slowly costs too.
        assert!(traverse_secs(&len, &free, &free, (5.0, top), &c, 28.0) > t + 0.3);
    }

    fn load(id: &str) -> Track {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        Track::load(&root, &TrackVariant::forward(id)).expect("loads")
    }

    /// A stadium whose west straight bulges 28 u out (an S into a wide arc and back) and a
    /// straight 100 u cut across the bulge on the same asphalt.
    fn bulge() -> Track {
        let ron = "(style: Invisible, start: 0, kill_height: -10, points: [
            (pos: (0, 0, 80), width: 10, offroad_margin: 6),
            (pos: (0, 0, 50), offroad_margin: 26),
            (pos: (-10, 0, 35)), (pos: (-22, 0, 18)), (pos: (-28, 0, 0)), (pos: (-22, 0, -18)),
            (pos: (-10, 0, -35)),
            (pos: (0, 0, -50), offroad_margin: 6),
            (pos: (0, 0, -90)),
            (pos: (4.02, 0, -105)), (pos: (15, 0, -116)), (pos: (30, 0, -120)),
            (pos: (45, 0, -116)), (pos: (55.98, 0, -105)), (pos: (60, 0, -90)),
            (pos: (60, 0, 0)), (pos: (60, 0, 90)),
            (pos: (55.98, 0, 105)), (pos: (45, 0, 116)), (pos: (30, 0, 120)),
            (pos: (15, 0, 116)), (pos: (4.02, 0, 105)), (pos: (0, 0, 90)),
        ],
        ground: [(surface: Asphalt, height: 0, points: [(-45, -135), (75, -135), (75, 135), (-45, 135)])],
        shortcuts: [(path: [(0, 0, 50), (0, 0, 0), (0, 0, -50)], width: 8)])";
        let def = rc_track::TrackDef::parse(ron).expect("parses");
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let props = rc_track::PropCatalog::load(&root).expect("props");
        Track::build(&def, &props, &TrackVariant::forward("bulge")).expect("builds")
    }

    fn plans(track: &Track, c: &CarCaps) -> Vec<Option<ShortcutPlan>> {
        let main = SpeedProfile::build(track, c, 0.96, 28.0);
        track
            .shortcuts()
            .iter()
            .map(|s| ShortcutPlan::build(track, s, c, 0.96, 28.0, &main))
            .collect()
    }

    #[test]
    fn only_shortcuts_that_save_time_are_planned() {
        let c = caps();
        // test_oval's cut is shorter, but grass and the turn onto it make it slower; toy_town's
        // inner cut would save ≈ 0.35 s, but two knockable block gates stand across it.
        assert_eq!(plans(&load("test_oval"), &c), vec![None]);
        assert_eq!(plans(&load("toy_town"), &c), vec![None]);
        let track = bulge();
        let shortcut = &track.shortcuts()[0];
        let plan = plans(&track, &c)[0]
            .clone()
            .expect("the chord beats the bulge");
        assert!(plan.saving > 0.3, "saves {} s", plan.saving);
        let length = track.length();
        // The plan follows the path: the mid point is 28 u right of the bulge's centreline.
        let mid = track.project(shortcut.path[1], Some(shortcut.entry_s + 40.0));
        let lateral = plan.lateral_at(length, mid.s).expect("on the span");
        assert!(
            (lateral - mid.lateral).abs() < 1.0,
            "{lateral} vs {}",
            mid.lateral
        );
        assert!(mid.lateral > 20.0);
        assert!(
            plan.corner
                .iter()
                .all(|&v| v > 0.0 && v <= c.top_speed + 1e-3)
        );
        assert!(plan.covers(length, shortcut.entry_s + 1.0));
        assert!(!plan.covers(length, shortcut.entry_s - 1.0));
        assert_eq!(plan.surface_at(length, mid.s), Some(SurfaceKind::Asphalt));
        // Too narrow to risk: never planned, however fast.
        let mut narrow = shortcut.clone();
        narrow.width = MIN_WIDTH_U - 1.0;
        let main = SpeedProfile::build(&track, &c, 0.96, 28.0);
        assert!(ShortcutPlan::build(&track, &narrow, &c, 0.96, 28.0, &main).is_none());
    }

    #[test]
    fn take_chance_scales_with_aggression_and_never_at_rookie() {
        assert_eq!(take_chance(Difficulty::Rookie, 0.95), 0.0, "Rookie: never");
        let elite = |a| take_chance(Difficulty::Elite, a);
        assert!(elite(0.95) > elite(0.3), "aggressive rivals gamble more");
        assert_eq!(
            elite(0.5),
            CHANCE_BY_DIFFICULTY[2],
            "neutral = the difficulty's"
        );
        assert!(take_chance(Difficulty::Pro, 0.5) < elite(0.5));
    }

    #[test]
    fn a_shortcut_is_decided_once_per_approach_and_dropped_after_it() {
        let track = bulge();
        let plans: Vec<ShortcutPlan> = plans(&track, &caps()).into_iter().flatten().collect();
        let plan = plans.first().expect("the chord is faster");
        let length = track.length();
        let at = |d: f32| (plan.entry_s + d).rem_euclid(length);
        let line = |d: f32| plan.lateral_at(length, at(d)).unwrap_or(0.0);
        let mut choice = ShortcutChoice::default();
        let rolls = std::cell::Cell::new(0);
        let drive = |choice: &mut ShortcutChoice, d: f32, take: bool| {
            choice.step(&plans, length, (at(d), line(d)), 0.5, |_| {
                rolls.set(rolls.get() + 1);
                take
            });
        };
        drive(&mut choice, -60.0, true);
        assert_eq!(choice.active, None, "too far to decide");
        drive(&mut choice, -30.0, true);
        assert_eq!(choice.active, Some(0));
        for d in [-29.0, -10.0, 0.5, 5.0, plan.span - 1.0] {
            drive(&mut choice, d, false);
            assert_eq!(choice.active, Some(0), "kept through the cut at {d}");
        }
        assert!(choice.at(&plans, length, at(5.0)).is_some());
        drive(&mut choice, plan.span + 2.0, false);
        assert_eq!(choice.active, None, "dropped after the exit");
        assert_eq!(rolls.get(), 1, "one roll per approach");
        // Next lap: a new roll (which says no this time).
        drive(&mut choice, -20.0, false);
        assert_eq!((choice.active, rolls.get()), (None, 2));
        // A car far off the cut's corridor (pushed back onto the track) drops it.
        let mut taken = ShortcutChoice::default();
        taken.step(&plans, length, (at(-20.0), 0.0), 1.0, |_| true);
        taken.step(
            &plans,
            length,
            (at(10.0), line(10.0) + plan.half_width + 4.0),
            1.0,
            |_| true,
        );
        assert_eq!(taken.active, None);
    }
}
