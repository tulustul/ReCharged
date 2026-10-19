//! Way-finding marks along the track (GDD §7.3), built per layout from rc_track's frames and
//! corners, so reverse and mirror get their own; decorative only (no colliders):
//!
//! - **Tape** along both racing edges where the ribbon is `Invisible` and lies on a floor polygon
//!   (visible ribbons show their own edges; barriers mark theirs).
//! - **Floor arrows** before every corner, bending the corner's way in the HUD turn icon's shape
//!   (slight, square, hairpin); sharp corners get a second one earlier.
//! - **Chevron boards** on the outside of corners, one per 45° of turn, facing the approaching
//!   cars: behind a barrier, at the legal corridor's edge on open floors (pulled in when a prop,
//!   the room or another part of the track is in the way), or just past a raised ribbon's edge.
//! - **Crosses** taped on the floor where it runs on straight ahead of a corner, just past the
//!   legal corridor: the way the track does not go.
//!
//! Library roles: `guide_tape` (tape, arrows, crosses), `guide_board` (boards and posts),
//! `guide_chevron` (the chevrons).

use bevy::math::{Mat3, Quat, Vec2, Vec3};
use rc_core::TrackTurn;
use rc_track::{MeshData, Primitive, RibbonStyle, Track, WALL_THICKNESS_U};

use super::barriers::{append, runs, wall_edge};
use crate::props::shapes::chamfer_box;

/// Floor tape width (u).
const TAPE_WIDTH_U: f32 = 0.5;
/// Floor marks lie this far above their floor (u).
const MARK_LIFT_U: f32 = 0.02;
/// Marks go only where a floor polygon lies this close to the track's surface (u): not on prop
/// ramps, tables they do not touch, or in the air.
const FLOOR_FIT_U: f32 = 0.12;
/// Corners turning at least this much get floor arrows (degrees)…
const ARROW_MIN_DEG: f32 = 40.0;
/// …ending this far before the corner starts (u)…
const ARROW_LEAD_U: f32 = 5.0;
/// …this long (u); sharp corners get a second one this much earlier (u).
const ARROW_SIZE_U: f32 = 5.0;
const SECOND_ARROW_U: f32 = 14.0;
/// Stroke of the arrows (the turn icon's 5.5 / 48 box).
const ARROW_STROKE: f32 = 5.5 / 48.0;
/// Corners turning at least this much get chevron boards (degrees), one per `BOARD_STEP_DEG`.
const BOARD_MIN_DEG: f32 = 45.0;
const BOARD_STEP_DEG: f32 = 45.0;
const MAX_BOARDS: usize = 4;
/// Half size of a board (u: across, up, thickness).
const BOARD_HALF: Vec3 = Vec3::new(1.1, 0.66, 0.05);
/// A board's bottom above the surface beside an open edge, and above a barrier's top (u).
const BOARD_LIFT_U: f32 = 0.45;
const BOARD_OVER_WALL_U: f32 = 0.15;
/// Boards turn toward the centreline this far before them (u).
const BOARD_AIM_U: f32 = 15.0;
/// On open floors boards stand at most this far beyond the racing edge (u)…
const BOARD_MAX_OUT_U: f32 = 8.0;
/// …and at least this far (u: just past the tape); beside a raised ribbon, this far past its lip.
const BOARD_MIN_OUT_U: f32 = 1.4;
const RIBBON_OUT_U: f32 = 1.0;
/// Room for a board around its spot (u): props, room walls, other parts of the track.
const BOARD_ROOM_U: f32 = 1.0;
/// Another part of the track: farther than this along it (u)…
const OTHER_PART_U: f32 = 20.0;
/// …within this height of it (u).
const OTHER_PART_HEIGHT_U: f32 = 4.0;
/// Crosses: the ray straight ahead of a corner is followed this far (u) in these steps (u)…
const CROSS_REACH_U: f32 = 40.0;
const CROSS_STEP_U: f32 = 0.5;
/// …and the cross lies this far past the corridor's edge (u), this big (u).
const CROSS_PAST_U: f32 = 1.5;
const CROSS_HALF_U: f32 = 1.5;

/// The marks of a layout, one mesh per library role.
#[derive(Debug, Clone, Default)]
pub(crate) struct Guides {
    /// Tape, floor arrows and crosses (`guide_tape`).
    pub tape: MeshData,
    /// Boards and posts (`guide_board`).
    pub boards: MeshData,
    /// Chevrons on the boards (`guide_chevron`).
    pub chevrons: MeshData,
}

/// Every mark of `track`.
pub(crate) fn guides(track: &Track) -> Guides {
    let mut g = Guides::default();
    if track.style() == RibbonStyle::Invisible {
        tape(track, &mut g.tape);
    }
    for turn in track.turns() {
        arrows(track, turn, &mut g.tape);
        boards(track, turn, &mut g);
        if track.style() == RibbonStyle::Invisible {
            cross(track, turn, &mut g.tape);
        }
    }
    g
}

/// Height of the topmost floor polygon under `p` within [`FLOOR_FIT_U`] of its height.
fn floor_at(track: &Track, p: Vec3) -> Option<f32> {
    let xz = Vec2::new(p.x, p.z);
    track
        .ground()
        .iter()
        .filter(|g| (g.height - p.y).abs() <= FLOOR_FIT_U && g.contains_xz(xz))
        .map(|g| g.height)
        .reduce(f32::max)
}

/// Where a post standing at `p` reaches down to: the topmost floor polygon under it (at most a
/// little above `p`), else the lowest floor of the layout.
fn support_below(track: &Track, p: Vec3) -> f32 {
    let xz = Vec2::new(p.x, p.z);
    let ground = track.ground();
    ground
        .iter()
        .filter(|g| g.height <= p.y + 0.3 && g.contains_xz(xz))
        .map(|g| g.height)
        .reduce(f32::max)
        .or_else(|| ground.iter().map(|g| g.height).reduce(f32::min))
        .unwrap_or(p.y - 3.0)
        .min(p.y)
}

/// A flat quad strip along `a` → `b` pairs (one quad per consecutive pair), facing `up`.
fn strip(m: &mut MeshData, pairs: &[(Vec3, Vec3)], up: Vec3) {
    let base = m.positions.len() as u32;
    for &(a, b) in pairs {
        for p in [a, b] {
            m.positions.push(p.to_array());
            m.normals.push(up.to_array());
            m.uvs.push([p.x, p.z]);
        }
    }
    for k in 0..pairs.len().saturating_sub(1) as u32 {
        let (a0, b0, a1, b1) = (
            base + 2 * k,
            base + 2 * k + 1,
            base + 2 * k + 2,
            base + 2 * k + 3,
        );
        // Counter-clockwise seen from `up`, whichever side `a` lies on.
        let (p, q, r) = (
            Vec3::from_array(m.positions[a0 as usize]),
            Vec3::from_array(m.positions[b0 as usize]),
            Vec3::from_array(m.positions[a1 as usize]),
        );
        if (q - p).cross(r - p).dot(up) >= 0.0 {
            m.indices.extend([a0, b0, a1, a1, b0, b1]);
        } else {
            m.indices.extend([a0, a1, b0, b0, a1, b1]);
        }
    }
}

/// A flat triangle facing `up`.
fn triangle(m: &mut MeshData, pts: [Vec3; 3], up: Vec3) {
    let base = m.positions.len() as u32;
    for p in pts {
        m.positions.push(p.to_array());
        m.normals.push(up.to_array());
        m.uvs.push([p.x, p.z]);
    }
    let ccw = (pts[1] - pts[0]).cross(pts[2] - pts[0]).dot(up) >= 0.0;
    m.indices
        .extend(if ccw { [0, 1, 2] } else { [0, 2, 1] }.map(|i| base + i));
}

/// Tape along both racing edges wherever the ribbon lies on a floor and no barrier stands.
fn tape(track: &Track, m: &mut MeshData) {
    let ds = track.sample_spacing();
    let n = (track.length() / ds).round() as usize;
    for side in [-1.0_f32, 1.0] {
        let at = |i: usize| {
            let f = track.frame_at(i as f32 * ds);
            let (left, right) = track.walls_at(f.s);
            let wall = if side < 0.0 { left } else { right };
            if f.in_gap || wall {
                return None;
            }
            // Both edges of the tape on the same floor (never hanging over a curb or a rug's edge).
            let [inner, outer] =
                [f.half_width - TAPE_WIDTH_U, f.half_width].map(|l| f.point(side * l, 0.0));
            let floor = floor_at(track, inner)?;
            let y = floor + MARK_LIFT_U;
            (floor_at(track, outer) == Some(floor)).then(|| (inner.with_y(y), outer.with_y(y)))
        };
        for (first, count) in runs(n, |i| at(i).is_some()) {
            let closed = count == n;
            let pairs: Vec<(Vec3, Vec3)> = (0..count + usize::from(closed))
                .filter_map(|k| at((first + k) % n))
                .collect();
            strip(m, &pairs, Vec3::Y);
        }
    }
}

/// Right-hand stems of the HUD's turn icons (`ui/src/hud/turnIcon.ts`, a 48 box, y down).
fn stem(angle_deg: f32) -> &'static [[f32; 2]] {
    if angle_deg >= 135.0 {
        &[
            [11., 45.],
            [11., 22.],
            [14., 14.],
            [20., 10.],
            [27., 10.],
            [33., 14.],
            [36., 21.],
            [36., 27.],
        ]
    } else if angle_deg >= 60.0 {
        &[[13., 45.], [13., 25.], [15., 19.], [20., 16.], [30., 16.]]
    } else {
        &[[17., 45.], [17., 31.], [19., 24.], [23., 19.], [28., 15.]]
    }
}

/// The floor arrows before `turn`.
fn arrows(track: &Track, turn: &TrackTurn, m: &mut MeshData) {
    let degrees = turn.angle.to_degrees();
    if degrees < ARROW_MIN_DEG {
        return;
    }
    let mut ends = vec![turn.start_s - ARROW_LEAD_U];
    if turn.severity >= 3 {
        ends.push(turn.start_s - ARROW_LEAD_U - SECOND_ARROW_U);
    }
    for end in ends {
        arrow(track, end, degrees, turn.right, m);
    }
}

/// An arrow ending at arc length `end`: the icon laid on the surface, its top toward travel.
fn arrow(track: &Track, end: f32, degrees: f32, right: bool, m: &mut MeshData) {
    let mirror = if right { 1.0 } else { -1.0 };
    // Icon (x, y) → (lateral, along) with the icon's top at `end`.
    let local = |p: [f32; 2]| {
        Vec2::new(
            mirror * (p[0] - 24.0) / 48.0 * ARROW_SIZE_U,
            -p[1] / 48.0 * ARROW_SIZE_U,
        )
    };
    let invisible = track.style() == RibbonStyle::Invisible;
    let world = |q: Vec2| -> Option<(Vec3, Vec3)> {
        let f = track.frame_at(end + q.y);
        if f.in_gap {
            return None;
        }
        if invisible {
            let p = f.point(q.x, 0.0);
            floor_at(track, p).map(|h| (p.with_y(h + MARK_LIFT_U), Vec3::Y))
        } else {
            Some((f.point(q.x, MARK_LIFT_U), f.up))
        }
    };
    let pts: Vec<Vec2> = stem(degrees).iter().map(|&p| local(p)).collect();
    let (Some(&a), Some(&b)) = (pts.get(pts.len() - 2), pts.last()) else {
        return;
    };
    let dir = (b - a).normalize_or_zero();
    let w = ARROW_STROKE * ARROW_SIZE_U;
    let normal = Vec2::new(-dir.y, dir.x);
    let head = [
        b + dir * 0.19 * ARROW_SIZE_U,
        b + normal * w * 1.5,
        b - normal * w * 1.5,
    ];
    // Every part on the surface, or no arrow at all.
    let mut quads = Vec::new();
    for seg in pts.windows(2) {
        let n = (seg[1] - seg[0]).perp().normalize_or_zero() * w * 0.5;
        // Stretch each segment a little into the joints so they close.
        let along = (seg[1] - seg[0]).normalize_or_zero() * w * 0.3;
        let ends = [seg[0] - along, seg[1] + along];
        let pairs: Option<Vec<(Vec3, Vec3)>> = ends
            .iter()
            .map(|&e| Some((world(e + n)?.0, world(e - n)?.0)))
            .collect();
        quads.push(pairs);
    }
    let tip: Option<Vec<(Vec3, Vec3)>> = head.iter().map(|&h| world(h)).collect();
    let (Some(quads), Some(tip)) = (quads.into_iter().collect::<Option<Vec<_>>>(), tip) else {
        return;
    };
    let up = tip[0].1;
    for pairs in quads {
        strip(m, &pairs, up);
    }
    triangle(m, [tip[0].0, tip[1].0, tip[2].0], up);
}

/// Something stands at `p` (xz, ± `room`): a prop's collider, the room's walls.
fn blocked(track: &Track, p: Vec3, room: f32) -> bool {
    let (lo, hi) = track.bounds();
    if p.x < lo.x + room || p.x > hi.x - room || p.z < lo.z + room || p.z > hi.z - room {
        return true;
    }
    let xz = Vec2::new(p.x, p.z);
    track.props().iter().any(|placed| {
        let Some(def) = track.prop_def(&placed.kind) else {
            return false;
        };
        def.colliders.iter().any(|c| {
            // Centre, reach around it seen from above, and half height (tilted shapes: their
            // whole extent both ways).
            let (offset, reach, up) = match *c {
                Primitive::Box { half, offset, rot } | Primitive::Wedge { half, offset, rot } => {
                    match rot.x == 0.0 && rot.z == 0.0 {
                        true => (offset, Vec2::new(half.x, half.z).length(), half.y),
                        false => (offset, half.length(), half.length()),
                    }
                }
                Primitive::Ball { radius, offset } => (offset, radius, radius),
                Primitive::Cylinder {
                    radius,
                    half_height,
                    offset,
                    rot,
                } => match rot.x == 0.0 && rot.z == 0.0 {
                    true => (offset, radius, half_height),
                    false => (offset, radius.max(half_height), radius.max(half_height)),
                },
            };
            let centre = placed.transform.transform_point(offset);
            // Props entirely below the spot (a rug, a mat) are no obstacle.
            centre.y + up > p.y + 0.1 && Vec2::new(centre.x, centre.z).distance(xz) < reach + room
        })
    })
}

/// `p` lies on another part of the track than arc length `s` (a crossing, a bridge, the next
/// aisle).
fn on_other_part(track: &Track, p: Vec3, s: f32) -> bool {
    let tp = track.project(p, None);
    let f = track.frame_at(tp.s);
    track.delta_s(tp.s, s).abs() > OTHER_PART_U
        && tp.lateral.abs() <= f.half_width + BOARD_ROOM_U
        && tp.height.abs() <= OTHER_PART_HEIGHT_U
}

/// A chevron board's spot: the bottom centre and its lift above the surface.
fn board_spot(track: &Track, s: f32, out: f32) -> Option<Vec3> {
    let f = track.frame_at(s);
    if f.in_gap {
        return None;
    }
    let (left, right) = track.walls_at(s);
    let wall = if out < 0.0 { left } else { right };
    let (edge, _) = wall_edge(track.style());
    let candidates: Vec<(f32, f32)> = if wall {
        let behind = f.half_width + edge + WALL_THICKNESS_U + BOARD_HALF.z + 0.25;
        vec![(behind, track.wall_height() + BOARD_OVER_WALL_U)]
    } else if track.style() == RibbonStyle::Invisible {
        let far = f.offroad_margin.clamp(BOARD_MIN_OUT_U, BOARD_MAX_OUT_U);
        let steps = (far - BOARD_MIN_OUT_U).ceil() as usize;
        (0..=steps)
            .map(|k| {
                (
                    f.half_width + (far - k as f32).max(BOARD_MIN_OUT_U),
                    BOARD_LIFT_U,
                )
            })
            .collect()
    } else {
        vec![(f.half_width + edge + RIBBON_OUT_U, BOARD_LIFT_U)]
    };
    candidates.into_iter().find_map(|(lateral, lift)| {
        let base = f.point(out * lateral, 0.0);
        let bottom = base + Vec3::Y * lift;
        let clear = !blocked(track, bottom, BOARD_ROOM_U) && !on_other_part(track, base, s);
        clear.then_some(bottom)
    })
}

/// The chevron boards on the outside of `turn`.
fn boards(track: &Track, turn: &TrackTurn, g: &mut Guides) {
    let degrees = turn.angle.to_degrees();
    if degrees < BOARD_MIN_DEG {
        return;
    }
    let count = ((degrees / BOARD_STEP_DEG).round() as usize).clamp(1, MAX_BOARDS);
    let out = if turn.right { -1.0 } else { 1.0 };
    for k in 0..count {
        let s = turn.start_s + turn.span * (k as f32 + 0.5) / count as f32;
        let Some(bottom) = board_spot(track, s, out) else {
            continue;
        };
        let aim = track.frame_at(s - BOARD_AIM_U).position - bottom;
        let Some(facing) = Vec2::new(aim.x, aim.z).try_normalize() else {
            continue;
        };
        board(
            track,
            bottom,
            Vec3::new(facing.x, 0.0, facing.y),
            turn.right,
            g,
        );
    }
}

/// A board standing at `bottom` (its lower edge's centre) facing `facing` (flat), on a post.
fn board(track: &Track, bottom: Vec3, facing: Vec3, right: bool, g: &mut Guides) {
    let across = Vec3::Y.cross(facing);
    let rot = Quat::from_mat3(&Mat3::from_cols(across, Vec3::Y, facing));
    let place = |mut part: MeshData, at: Vec3| {
        for p in &mut part.positions {
            *p = (rot * Vec3::from_array(*p) + at).to_array();
        }
        for n in &mut part.normals {
            *n = (rot * Vec3::from_array(*n)).to_array();
        }
        part
    };
    let centre = bottom + Vec3::Y * BOARD_HALF.y;
    append(
        &mut g.boards,
        place(chamfer_box(Vec3::ZERO, BOARD_HALF, 0.03), centre),
    );
    let foot = support_below(track, bottom);
    let post = (bottom.y + BOARD_HALF.y - foot).max(0.0);
    if post > 0.05 {
        let half = Vec3::new(0.07, post * 0.5, 0.07);
        let at = Vec3::new(bottom.x, foot + post * 0.5, bottom.z) - facing * (BOARD_HALF.z + 0.07);
        append(
            &mut g.boards,
            place(chamfer_box(Vec3::ZERO, half, 0.02), at),
        );
    }
    // Two chevrons pointing the corner's way (the viewer's right is `across`).
    let side = if right { 1.0 } else { -1.0 };
    let (hx, hy) = (BOARD_HALF.x, BOARD_HALF.y);
    for cx in [-0.36 * hx, 0.36 * hx] {
        let (a, b, t) = (0.22 * hx, 0.65 * hy, 0.19 * hx);
        let p = |x: f32, y: f32| {
            centre + across * (cx + side * x) + Vec3::Y * y + facing * (BOARD_HALF.z + 0.012)
        };
        let (apex, upper, upper_in, apex_in, lower_in, lower) = (
            p(a, 0.0),
            p(-a, b),
            p(-a + t, b),
            p(a - t, 0.0),
            p(-a + t, -b),
            p(-a, -b),
        );
        for tri in [
            [apex, upper, upper_in],
            [apex, upper_in, apex_in],
            [apex, apex_in, lower_in],
            [apex, lower_in, lower],
        ] {
            triangle(&mut g.chevrons, tri, facing);
        }
    }
}

/// A cross on the floor straight ahead of `turn`, where the floor runs on past the corridor.
fn cross(track: &Track, turn: &TrackTurn, m: &mut MeshData) {
    if turn.angle.to_degrees() < BOARD_MIN_DEG {
        return;
    }
    let entry = track.frame_at(turn.start_s);
    let Some(d) = Vec2::new(entry.forward.x, entry.forward.z).try_normalize() else {
        return;
    };
    let d = Vec3::new(d.x, 0.0, d.y);
    let steps = (CROSS_REACH_U / CROSS_STEP_U) as usize;
    let Some((at, s)) = (1..=steps).find_map(|k| {
        let q = entry.position + d * (k as f32 * CROSS_STEP_U);
        let tp = track.project(q, Some(turn.start_s));
        let f = track.frame_at(tp.s);
        (tp.lateral.abs() > f.half_width + f.offroad_margin).then_some((q, tp))
    }) else {
        return;
    };
    let (left, right) = track.walls_at(s.s);
    if (s.lateral < 0.0 && left) || (s.lateral > 0.0 && right) {
        return; // a barrier closes it
    }
    let centre = at + d * CROSS_PAST_U;
    let Some(floor) = floor_at(track, centre) else {
        return;
    };
    if blocked(track, centre.with_y(floor), CROSS_HALF_U) || on_other_part(track, centre, s.s) {
        return;
    }
    let side = Vec3::Y.cross(d);
    let y = floor + MARK_LIFT_U;
    for diag in [(d + side).normalize(), (d - side).normalize()] {
        let n = Vec3::Y.cross(diag) * TAPE_WIDTH_U * 0.75;
        let (a, b) = (centre - diag * CROSS_HALF_U, centre + diag * CROSS_HALF_U);
        strip(
            m,
            &[(a + n, a - n), (b + n, b - n)].map(|(p, q)| (p.with_y(y), q.with_y(y))),
            Vec3::Y,
        );
    }
}

#[cfg(test)]
mod tests {
    use rc_core::TrackVariant;

    use super::*;

    fn variants(id: &str) -> impl Iterator<Item = Track> + '_ {
        let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        [(false, false), (true, false), (false, true), (true, true)]
            .into_iter()
            .map(move |(reverse, mirror)| {
                let v = TrackVariant {
                    track: id.into(),
                    reverse,
                    mirror,
                };
                Track::load(&assets, &v).unwrap_or_else(|e| panic!("{e}"))
            })
    }

    const TRACKS: [&str; 7] = [
        "toy_town",
        "sofa_circuit",
        "backyard_bash",
        "aisle_rush",
        "wrench_works",
        "maple_street",
        "test_oval",
    ];

    fn points(m: &MeshData) -> impl Iterator<Item = Vec3> + '_ {
        m.positions.iter().map(|p| Vec3::from_array(*p))
    }

    #[test]
    fn open_floors_get_tape_and_every_mark_lies_on_its_floor() {
        for id in TRACKS {
            for track in variants(id) {
                let g = guides(&track);
                let invisible = track.style() == RibbonStyle::Invisible;
                // Tape and crosses only exist on floors: every vertex just above a floor polygon.
                if invisible {
                    assert!(g.tape.triangle_count() > 1000, "{id}: tape on the floors");
                    for p in points(&g.tape) {
                        let floor = floor_at(&track, p - Vec3::Y * MARK_LIFT_U);
                        assert!(floor.is_some(), "{id}: a mark floats at {p}");
                    }
                } else {
                    assert!(g.tape.triangle_count() > 0, "{id}: arrows on the ribbon");
                }
                assert_eq!(g.tape.indices.len() % 3, 0);
                assert!(
                    g.boards.triangle_count() > 0 && g.chevrons.triangle_count() > 0,
                    "{id}"
                );
            }
        }
    }

    #[test]
    fn boards_stand_off_the_track_facing_the_cars_that_come() {
        for id in TRACKS {
            let track = variants(id).next().expect("forward");
            let mut seen = 0;
            for turn in track.turns() {
                let mut g = Guides::default();
                boards(&track, turn, &mut g);
                for c in chevron_centres(&g) {
                    seen += 1;
                    let tp = track.project(c, None);
                    let f = track.frame_at(tp.s);
                    assert!(
                        tp.lateral.abs() > f.half_width + 1.0 || tp.height > 1.0,
                        "{id}: a board on the racing width at s = {:.0}",
                        tp.s
                    );
                    // Outside the corner: left of a right-hander (seen from the corner's nearest
                    // point in 3D: a helix passes the same spot on every level).
                    let dist = |s: f32| track.frame_at(s).position.distance(c);
                    let hint = (0..=turn.span as usize)
                        .map(|k| turn.start_s + k as f32)
                        .min_by(|a, b| dist(*a).total_cmp(&dist(*b)));
                    let near = track.project(c, hint);
                    assert_eq!(
                        near.lateral < 0.0,
                        turn.right,
                        "{id}: inside the corner at s = {:.0}",
                        near.s
                    );
                }
            }
            assert!(seen >= 4, "{id}: {seen} boards");
        }
    }

    /// Each board's centre: two chevrons of four triangles (24 vertices) per board.
    fn chevron_centres(g: &Guides) -> Vec<Vec3> {
        let pts: Vec<Vec3> = points(&g.chevrons).collect();
        pts.chunks(24)
            .map(|board| board.iter().copied().sum::<Vec3>() / board.len() as f32)
            .collect()
    }

    #[test]
    fn arrows_bend_the_corners_way() {
        for id in ["test_oval", "aisle_rush", "sofa_circuit"] {
            for track in variants(id) {
                for turn in track
                    .turns()
                    .iter()
                    .filter(|t| t.angle.to_degrees() >= ARROW_MIN_DEG)
                {
                    let mut m = MeshData::default();
                    arrow(
                        &track,
                        turn.start_s - ARROW_LEAD_U,
                        turn.angle.to_degrees(),
                        turn.right,
                        &mut m,
                    );
                    if m.is_empty() {
                        continue; // no floor there
                    }
                    // The head (last triangle) lies on the corner's side of the centreline.
                    let tip = Vec3::from_array(m.positions[m.positions.len() - 3]);
                    let tp = track.project(tip, Some(turn.start_s));
                    assert_eq!(
                        tp.lateral > 0.0,
                        turn.right,
                        "{id}: arrow at s = {:.0}",
                        tp.s
                    );
                }
            }
        }
    }

    #[test]
    fn crosses_close_the_open_floor_ahead_of_corners() {
        let track = variants("aisle_rush").next().expect("forward");
        let mut found = 0;
        for turn in track.turns() {
            let mut m = MeshData::default();
            cross(&track, turn, &mut m);
            if m.is_empty() {
                continue;
            }
            found += 1;
            let centre = points(&m).sum::<Vec3>() / m.positions.len() as f32;
            let tp = track.project(centre, Some(turn.start_s));
            let f = track.frame_at(tp.s);
            assert!(
                tp.lateral.abs() > f.half_width + f.offroad_margin,
                "outside the corridor"
            );
        }
        assert!(found >= 2, "{found} crosses on Aisle Rush");
    }
}
