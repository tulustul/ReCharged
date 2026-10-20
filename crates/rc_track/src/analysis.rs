//! Analysis of a built track (pure): racing line and speed profile, jump flights, authoring
//! lints and the geometry hash.

use bevy::math::{Vec2, Vec3};
use bevy::transform::components::Transform;
use rc_core::SurfaceKind;

use crate::def::CHOKE_WIDTH_U;
use crate::geometry::runs;
use crate::mesh::style_profile;
use crate::place::{LINE_LOCK_AFTER_U, LINE_LOCK_BEFORE_U};
use crate::props::Primitive;
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
/// Lint: the top of a ribbon's edge (ToyTrack lip, Asphalt curb, where barriers stand) must show
/// at least this much above the floor it lies on (u). Banking tilts around the centreline, so the
/// inner edge sinks; the authoring rule (`rc_track.md` §A5) keeps 0.15 at the control points,
/// which leaves room for the spline between them. A ribbon lying flat whose edge rises less (a
/// carpet runner) needs only its own rise.
pub const EDGE_CLEARANCE_U: f32 = 0.1;
/// Lint: a static prop (no drivable surface) must stay out of the racing width between this
/// height above the driving surface (u; lower is a bump the wheels, r ≈ 0.23 u, roll over)…
pub const CORRIDOR_STEP_U: f32 = 0.25;
/// …and this one (u): the car's body and a little more.
pub const CORRIDOR_HEIGHT_U: f32 = 1.5;
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
    let (profile, edge, thickness) = style_profile(track.style);
    if !profile.is_empty() {
        // The edge's top corner: (beyond the half width, above the surface).
        let lip = profile
            .iter()
            .map(|p| p.at(0.0))
            .filter(|p| (p.x - edge).abs() < 1e-6)
            .map(|p| p.y)
            .fold(0.0, f32::max);
        let edge_top = Vec2::new(edge, lip);
        for z in zones(track, |n| sunken_edge(track, &n.frame, edge_top, thickness)) {
            out.push(format!(
                "s = {z} u: the banked ribbon's inner edge sinks to within {EDGE_CLEARANCE_U} u of the floor below it; raise the ribbon there or bank less (rc_track.md §A5)"
            ));
        }
    }
    out.extend(self_overlaps(track));
    out.extend(corridor_intrusions(track));
    out
}

/// A collider primitive seen from above: its outline (convex polygon, or a circle as one point
/// and a radius) and its height range.
struct Footprint {
    outline: Vec<Vec2>,
    radius: f32,
    low: f32,
    high: f32,
}

impl Footprint {
    fn of(prop: &Transform, primitive: &Primitive) -> Self {
        let (offset, rot) = primitive.pose();
        let at = |local: Vec3| prop.transform_point(offset + rot * local);
        let boxed = |half: Vec3| {
            let corners: Vec<Vec3> = (0..8)
                .map(|i| {
                    let sign = |bit: usize| if i & bit == 0 { -1.0 } else { 1.0 };
                    at(half * Vec3::new(sign(1), sign(2), sign(4)))
                })
                .collect();
            let low = corners.iter().map(|c| c.y).fold(f32::MAX, f32::min);
            let high = corners.iter().map(|c| c.y).fold(f32::MIN, f32::max);
            let plan: Vec<Vec2> = corners.iter().map(|c| Vec2::new(c.x, c.z)).collect();
            Self {
                outline: hull(plan),
                radius: 0.0,
                low,
                high,
            }
        };
        let round = |radius: f32, half_height: f32| {
            let c = at(Vec3::ZERO);
            Self {
                outline: vec![Vec2::new(c.x, c.z)],
                radius,
                low: c.y - half_height,
                high: c.y + half_height,
            }
        };
        match *primitive {
            Primitive::Box { half, .. } | Primitive::Wedge { half, .. } => boxed(half),
            // Upright: a circle; tilted (a wheel on its side): its bounding box.
            Primitive::Cylinder {
                radius,
                half_height,
                rot,
                ..
            } => {
                if rot.x.abs() < 1e-3 && rot.z.abs() < 1e-3 {
                    round(radius, half_height)
                } else {
                    boxed(Vec3::new(radius, half_height, radius))
                }
            }
            Primitive::Ball { radius, .. } => round(radius, radius),
        }
    }

    /// Plan distance from `p` (0 inside).
    fn distance(&self, p: Vec2) -> f32 {
        let o = &self.outline;
        if o.len() == 1 {
            return (o[0].distance(p) - self.radius).max(0.0);
        }
        let mut inside = true;
        let mut best = f32::MAX;
        for (i, &a) in o.iter().enumerate() {
            let b = o[(i + 1) % o.len()];
            let ab = b - a;
            inside &= ab.perp_dot(p - a) >= 0.0; // counter-clockwise hull
            let t = ((p - a).dot(ab) / ab.length_squared().max(1e-12)).clamp(0.0, 1.0);
            best = best.min((a + ab * t).distance(p));
        }
        if inside { 0.0 } else { best }
    }
}

/// Convex hull, counter-clockwise (monotone chain).
fn hull(mut points: Vec<Vec2>) -> Vec<Vec2> {
    points.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    points.dedup_by(|a, b| a.distance(*b) < 1e-5);
    if points.len() < 3 {
        return points;
    }
    let mut out: Vec<Vec2> = Vec::with_capacity(points.len() * 2);
    for pass in [false, true] {
        let start = out.len();
        let iter: Box<dyn Iterator<Item = &Vec2>> = if pass {
            Box::new(points.iter().rev())
        } else {
            Box::new(points.iter())
        };
        for &p in iter {
            while out.len() >= start + 2 {
                let (a, b) = (out[out.len() - 2], out[out.len() - 1]);
                if (b - a).perp_dot(p - a) > 0.0 {
                    break;
                }
                out.pop();
            }
            out.push(p);
        }
        out.pop();
    }
    out
}

/// Static props without a drivable surface whose colliders reach into the racing width between
/// [`CORRIDOR_STEP_U`] and [`CORRIDOR_HEIGHT_U`] above the surface: one line per prop, at its
/// deepest point. Ramps and box jumps (drivable surface), knockables, bumps (a cable), props
/// above the car (a table top, a bench seat) and placements marked `on_line` are fine.
fn corridor_intrusions(track: &Track) -> Vec<String> {
    let nodes: Vec<&TrackFrame> = track
        .nodes
        .iter()
        .step_by(2)
        .map(|n| &n.frame)
        .filter(|f| !f.in_gap)
        .collect();
    let mut out = Vec::new();
    for prop in &track.props {
        let Some(def) = track.prop_def(&prop.kind) else {
            continue;
        };
        if def.dynamic || def.surface.is_some() || prop.on_line {
            continue;
        }
        let mut worst: Option<(f32, f32)> = None; // (depth, s)
        for footprint in def
            .colliders
            .iter()
            .map(|c| Footprint::of(&prop.transform, c))
        {
            let centre = footprint.outline.iter().sum::<Vec2>() / footprint.outline.len() as f32;
            let reach = footprint
                .outline
                .iter()
                .map(|p| p.distance(centre))
                .fold(footprint.radius, f32::max);
            for f in &nodes {
                let p = Vec2::new(f.position.x, f.position.z);
                let allowed = f.half_width;
                if p.distance(centre) > reach + allowed
                    || footprint.high <= f.position.y + CORRIDOR_STEP_U
                    || footprint.low >= f.position.y + CORRIDOR_HEIGHT_U
                {
                    continue;
                }
                let depth = allowed - footprint.distance(p);
                if depth > 0.0 && worst.is_none_or(|(d, _)| depth > d) {
                    worst = Some((depth, f.s));
                }
            }
        }
        if let Some((depth, s)) = worst {
            let at = prop.transform.translation;
            out.push(format!(
                "s = {s:.0} u: prop {} at ({:.1}, {:.1}) reaches {depth:.2} u into the racing width; move it out or make it knockable",
                prop.kind, at.x, at.z
            ));
        }
    }
    out
}

/// The top of a visible ribbon's edge (`edge_top` = beyond the half width, above the surface)
/// lies less than [`EDGE_CLEARANCE_U`] above a floor the ribbon lies on: a ground polygon under
/// the edge that is not above the ribbon's underside at the centreline (a table top the ribbon
/// passes under does not count).
fn sunken_edge(track: &Track, frame: &TrackFrame, edge_top: Vec2, thickness: f32) -> bool {
    if frame.in_gap {
        return false;
    }
    let clearance = EDGE_CLEARANCE_U.min(thickness + edge_top.y) - 1e-3;
    let underside = frame.position.y - thickness + 1e-2;
    [-1.0, 1.0].into_iter().any(|side| {
        let top = frame.point(side * (frame.half_width + edge_top.x), edge_top.y);
        track
            .ground
            .iter()
            .filter(|g| g.height <= underside && g.contains_xz(Vec2::new(top.x, top.z)))
            .any(|g| top.y < g.height + clearance)
    })
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
