//! Analysis of a built track (pure): racing line and speed profile, jump flights, authoring
//! lints and the geometry hash.

use bevy::math::Vec3;
use rc_core::SurfaceKind;

use crate::def::CHOKE_WIDTH_U;
use crate::geometry::runs;
use crate::place::{LINE_LOCK_AFTER_U, LINE_LOCK_BEFORE_U};
use crate::racing::{LineInput, SpeedInput, SpeedModel, curvature, racing_line, speed_profile};
use crate::track::*;

/// Guideline: steepest slope (degrees).
pub const MAX_SLOPE_DEG: f32 = 25.0;
/// Guideline: steepest bank outside helices and half-pipes (degrees).
pub const MAX_BANK_GUIDE_DEG: f32 = 30.0;
/// Guideline: chokepoints per lap (GDD §7.1).
pub const MAX_CHOKEPOINTS: usize = 2;
/// Two parts of the track closer than this vertically must not overlap in plan (u): a car and
/// the lower barriers fit under the upper ribbon.
pub const STACK_CLEARANCE_U: f32 = 3.5;
/// Samples either side used for the racing line's curvature (2 u apart).
const CURVATURE_SPAN: usize = 4;

impl Track {
    /// Racing line offsets and the reference speed profile on every node.
    pub(crate) fn fit_racing_line(&mut self, gaps: &[(f32, f32)]) {
        let locked = |s: f32| {
            gaps.iter().any(|&(lip, landing)| {
                let span = (landing - lip).rem_euclid(self.length);
                let d = (s - lip + LINE_LOCK_BEFORE_U).rem_euclid(self.length);
                d <= span + LINE_LOCK_BEFORE_U + LINE_LOCK_AFTER_U
            })
        };
        let input: Vec<LineInput> = self
            .nodes
            .iter()
            .map(|n| LineInput {
                centre: n.frame.position,
                right: n.frame.right,
                half_width: n.frame.half_width,
                locked: locked(n.frame.s),
            })
            .collect();
        for (node, offset) in self.nodes.iter_mut().zip(racing_line(&input)) {
            node.line = offset;
        }
        let n = self.nodes.len();
        let pts: Vec<Vec3> = self
            .nodes
            .iter()
            .map(|n| n.frame.point(n.line, 0.0))
            .collect();
        let speed_in: Vec<SpeedInput> = (0..n)
            .map(|k| {
                let f = &self.nodes[k].frame;
                // Horizontal curvature: crests, dips and jump lips are not corners.
                let flat = |p: Vec3| Vec3::new(p.x, 0.0, p.z);
                let [a, c] = [n - CURVATURE_SPAN, CURVATURE_SPAN].map(|d| flat(pts[(k + d) % n]));
                let (kappa, right_turn) = curvature(a, flat(pts[k]), c, Vec3::Y);
                let props = f.surface.props();
                SpeedInput {
                    step: pts[k].distance(pts[(k + 1) % n]),
                    curvature: kappa,
                    bank_help: f.bank.sin() * if right_turn { 1.0 } else { -1.0 },
                    grip: props.grip,
                    top: props.top_speed,
                    airborne: f.in_gap,
                }
            })
            .collect();
        for (node, v) in self
            .nodes
            .iter_mut()
            .zip(speed_profile(&speed_in, &SpeedModel::REFERENCE))
        {
            node.speed = v;
        }
    }

    /// Flies a jump ballistically ([`rc_core::GRAVITY`]) from the lip at `position` along
    /// `direction` at `speed`, until it touches the ribbon (past `landing`: a landing) or falls
    /// below the flight line inside the gap (short).
    pub(crate) fn fly(
        &self,
        position: Vec3,
        direction: Vec3,
        lip: f32,
        landing: f32,
        speed: f32,
    ) -> Jump {
        const DT: f32 = 1.0 / 240.0;
        /// Ignore contacts in the first moments (the lip itself).
        const ARM_S: f32 = 0.05;
        let (mut pos, mut vel, mut s, mut t) = (position, direction * speed, lip, 0.0);
        let span = (landing - lip).rem_euclid(self.length);
        let mut touchdown_s = None;
        while t < 4.0 {
            t += DT;
            vel.y -= rc_core::GRAVITY * DT;
            pos += vel * DT;
            let tp = self.project(pos, Some(s));
            s = tp.s;
            if t > ARM_S && tp.height <= 0.0 {
                touchdown_s = ((s - lip).rem_euclid(self.length) > span).then_some(s);
                break;
            }
        }
        Jump {
            lip_s: lip,
            landing_s: landing,
            takeoff_position: position,
            takeoff_direction: direction,
            takeoff_speed: speed,
            touchdown_s,
            air_time: t,
        }
    }
}

/// Zones (s ranges) where `bad` holds, as "a–b" strings.
fn zones(track: &Track, bad: impl Fn(&Node) -> bool) -> Vec<String> {
    let n = track.nodes.len();
    runs(n, |i| bad(&track.nodes[i]))
        .into_iter()
        .filter(|&(_, _, b)| b)
        .map(|(a, b, _)| {
            let end = if b == 0 {
                track.length
            } else {
                b as f32 * track.ds
            };
            format!("{:.0}–{:.0}", a as f32 * track.ds, end)
        })
        .collect()
}

pub(crate) fn lints(track: &Track) -> Vec<String> {
    let mut out = Vec::new();
    let slope = MAX_SLOPE_DEG.to_radians().sin();
    for z in zones(track, |n| {
        !n.frame.in_gap && n.frame.forward.y.abs() > slope
    }) {
        out.push(format!("s = {z} u: slope steeper than {MAX_SLOPE_DEG}°"));
    }
    let bank = MAX_BANK_GUIDE_DEG.to_radians();
    for z in zones(track, |n| n.frame.bank.abs() > bank + 1e-4) {
        out.push(format!(
            "s = {z} u: bank above {MAX_BANK_GUIDE_DEG}° (only for helices/half-pipes)"
        ));
    }
    let chokes = zones(track, |n| 2.0 * n.frame.half_width < CHOKE_WIDTH_U - 1e-3);
    if chokes.len() > MAX_CHOKEPOINTS {
        out.push(format!(
            "{} chokepoints (width < {CHOKE_WIDTH_U} u) at s = {} u; at most {MAX_CHOKEPOINTS} per lap",
            chokes.len(),
            chokes.join(", ")
        ));
    }
    for jump in &track.jumps {
        let (lip, landing) = (jump.lip_s, jump.landing_s);
        let slow = track.fly(
            jump.takeoff_position,
            jump.takeoff_direction,
            lip,
            landing,
            0.8 * jump.takeoff_speed,
        );
        if jump.touchdown_s.is_none() || slow.touchdown_s.is_none() {
            out.push(format!(
                "jump at s = {lip:.0} u: lands short at {} the target speed ({:.0} u/s); shorten the gap, lower the landing or steepen the takeoff",
                if jump.touchdown_s.is_none() { "1.0 ×" } else { "0.8 ×" },
                jump.takeoff_speed
            ));
        }
    }
    out.extend(self_overlaps(track));
    out
}

/// Pairs of distant samples whose ribbons overlap in plan at a similar height (first 3).
// ponytail: O(n²) over every 4th sample (≈ 50 k pairs per lap); a grid if tracks get huge.
fn self_overlaps(track: &Track) -> Vec<String> {
    let n = track.nodes.len();
    let mut out = Vec::new();
    let mut last_hit = None;
    for i in (0..n).step_by(4) {
        for j in (i + 4..n).step_by(4) {
            let (a, b) = (&track.nodes[i].frame, &track.nodes[j].frame);
            let reach = a.half_width + b.half_width + 2.0;
            let apart = (j - i).min(n + i - j) as f32 * track.ds;
            let d = Vec3::new(
                a.position.x - b.position.x,
                0.0,
                a.position.z - b.position.z,
            );
            let close =
                d.length() < reach && (a.position.y - b.position.y).abs() < STACK_CLEARANCE_U;
            if close && apart > 3.0 * reach && last_hit.is_none_or(|k| i > k + 40) {
                last_hit = Some(i);
                out.push(format!(
                    "s = {:.0} u and s = {:.0} u: the track passes within {:.1} u of itself less than {STACK_CLEARANCE_U} u apart in height",
                    a.s,
                    b.s,
                    d.length()
                ));
            }
        }
    }
    out.truncate(3);
    out
}

/// FNV-1a over quantised geometry: stable across Rust versions and platforms.
pub(crate) fn geometry_hash(track: &Track) -> u64 {
    struct Fnv(u64);
    impl Fnv {
        fn bytes(&mut self, b: &[u8]) {
            for x in b {
                self.0 = (self.0 ^ u64::from(*x)).wrapping_mul(0x0100_0000_01b3);
            }
        }
        fn num(&mut self, v: f32) {
            self.bytes(&((v * 1000.0).round() as i32).to_le_bytes());
        }
        fn surface(&mut self, s: SurfaceKind) {
            self.bytes(&[SurfaceKind::ALL.iter().position(|k| *k == s).unwrap_or(0) as u8]);
        }
    }
    let mut h = Fnv(0xcbf2_9ce4_8422_2325);
    for n in &track.nodes {
        let f = &n.frame;
        f.position.to_array().into_iter().for_each(|v| h.num(v));
        h.num(f.half_width);
        h.num(f.bank);
        h.surface(f.surface);
        h.bytes(&[u8::from(n.walls.0), u8::from(n.walls.1), u8::from(f.in_gap)]);
    }
    for g in &track.ground {
        h.surface(g.surface);
        h.num(g.height);
        g.polygon.iter().for_each(|p| {
            h.num(p.x);
            h.num(p.y);
        });
    }
    for p in &track.props {
        h.bytes(p.kind.as_bytes());
        p.transform
            .translation
            .to_array()
            .into_iter()
            .for_each(|v| h.num(v));
        p.transform
            .rotation
            .to_array()
            .into_iter()
            .for_each(|v| h.num(v));
    }
    h.num(track.kill_height);
    h.num(track.wall_height);
    h.0
}
