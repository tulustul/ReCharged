//! The room around an indoor track — walls with window openings, frames, skirting boards, the
//! ceiling and a floor under everything — at `Track::bounds()` (floor at y = 0, ceiling at the
//! environment's `room.height`); outdoors a ground plane reaching past the play area so the world
//! never ends at its edge. Walls cast shadows (the window openings make sun patches); the ceiling
//! does not, so the shadowed key light reaches the whole room like a skylight (T13a's lighting
//! model), and is one face looking down, so views from above see into the room. rc_render's
//! lighting glazes the openings.

use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use rc_track::{MeshData, Track};

use crate::content::mesh_from_data;
use crate::lighting::{ActiveEnvironment, RoomDef, WindowDef, WindowWall};
use crate::materials::{MaterialLibrary, SurfaceMaterial};
use crate::props::shapes::{chamfer_box, face};

/// Shell wall thickness (u); the inner faces stand on the bounds.
const WALL_U: f32 = 1.0;
/// Skirting board height and depth (a 10 cm board × 4.44).
const SKIRTING_HEIGHT_U: f32 = 0.45;
const SKIRTING_DEPTH_U: f32 = 0.08;
/// Window frame width around the opening and its depth into the room.
const FRAME_U: f32 = 0.35;
const FRAME_DEPTH_U: f32 = 0.2;
/// Depth of a window sill ledge into the room.
const SILL_DEPTH_U: f32 = 0.45;
/// Mullions: thickness, depth (from the inner face into the wall) and the widest pane.
const MULLION_U: f32 = 0.25;
const MULLION_DEPTH_U: f32 = 0.3;
const MAX_PANE_U: f32 = 7.0;
/// The base floor's top lies this far below y = 0, so ground polygons always cover it.
const FLOOR_DROP_U: f32 = 0.03;
/// How far the outdoor ground plane reaches beyond the bounds (u).
const OUTDOOR_REACH_U: f32 = 600.0;

/// A solid part of a wall: (start, end) along it and (bottom, top) heights.
pub(crate) type Piece = [f32; 4];

/// Spawns the shell under `root`.
pub(super) fn spawn(
    commands: &mut Commands,
    root: Entity,
    track: &Track,
    env: &ActiveEnvironment,
    library: &MaterialLibrary,
    meshes: &mut Assets<Mesh>,
) {
    let (min, max) = track.bounds();
    let role = |name: &str| library.named(name).unwrap_or_default();
    let mut add =
        |data: MeshData, name: &'static str, material: Handle<SurfaceMaterial>, casts: bool| {
            let mesh = mesh_from_data(&data)?;
            let mut e = commands.spawn((
                Name::new(name),
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(material),
                ChildOf(root),
            ));
            if !casts {
                e.insert(NotShadowCaster);
            }
            Some(e.id())
        };
    let Some(room) = &env.def.room else {
        // A ring at road level reaching to the horizon, plus an inner floor under the play area
        // low enough that sunken ground (a storm-drain dish) shows through its opening.
        for (lo, hi) in outdoor_ring(min, max, OUTDOOR_REACH_U) {
            add(slab(lo, hi, 0.0), "outdoor ground", role("floor"), false);
        }
        let inner = inner_floor_top(track.ground().iter().map(|g| g.height));
        add(
            slab(min, max, inner),
            "outdoor inner floor",
            role("floor"),
            false,
        );
        return;
    };
    add(slab(min, max, 0.0), "room floor", role("floor"), false);
    let [walls, trim] = room_meshes(min, max, room);
    add(walls, "room walls", role("wall"), true);
    add(trim, "room trim", role("trim"), true);
    add(
        ceiling(min, max, room.height),
        "room ceiling",
        role("ceiling"),
        false,
    );
}

/// The ceiling: one face looking down over the room and the walls, culled from above, so views
/// from outside the room (the editor, cutaway previews) see in.
fn ceiling(min: Vec3, max: Vec3, height: f32) -> MeshData {
    let (lo, hi) = (min - WALL_U, max + WALL_U);
    let corners = [(lo.x, lo.z), (hi.x, lo.z), (hi.x, hi.z), (lo.x, hi.z)];
    let mut m = MeshData::default();
    face(
        &mut m,
        &corners.map(|(x, z)| Vec3::new(x, height, z)),
        Vec3::new(0.0, height + 1.0, 0.0),
    );
    m
}

/// A thin floor slab over the x/z box with its top just below `top`.
fn slab(min: Vec3, max: Vec3, top: f32) -> MeshData {
    let centre = Vec3::new(
        (min.x + max.x) * 0.5,
        top - FLOOR_DROP_U - 0.05,
        (min.z + max.z) * 0.5,
    );
    chamfer_box(centre, ((max - min) * 0.5).with_y(0.05), 0.0)
}

/// Height of the outdoor inner floor: the lowest ground polygon, never above y = 0.
fn inner_floor_top(heights: impl Iterator<Item = f32>) -> f32 {
    heights.fold(0.0, f32::min)
}

/// Four x/z boxes framing `[min, max]` out to `reach` on every side (north, south, west, east).
fn outdoor_ring(min: Vec3, max: Vec3, reach: f32) -> [(Vec3, Vec3); 4] {
    let (x0, x1, z0, z1) = (min.x - reach, max.x + reach, min.z - reach, max.z + reach);
    [
        (Vec3::new(x0, 0.0, z0), Vec3::new(x1, 0.0, min.z)),
        (Vec3::new(x0, 0.0, max.z), Vec3::new(x1, 0.0, z1)),
        (Vec3::new(x0, 0.0, min.z), Vec3::new(min.x, 0.0, max.z)),
        (Vec3::new(max.x, 0.0, min.z), Vec3::new(x1, 0.0, max.z)),
    ]
}

/// One wall of the shell: where it starts, the direction along it, its length and the normal
/// pointing into the room.
struct Wall {
    start: Vec3,
    along: Vec3,
    length: f32,
    inward: Vec3,
}

impl Wall {
    fn of(side: WindowWall, min: Vec3, max: Vec3) -> Self {
        let (dx, dz) = (max.x - min.x, max.z - min.z);
        let (start, along, length, inward) = match side {
            WindowWall::North => (Vec3::new(min.x, 0.0, min.z), Vec3::X, dx, Vec3::Z),
            WindowWall::South => (Vec3::new(min.x, 0.0, max.z), Vec3::X, dx, Vec3::NEG_Z),
            WindowWall::West => (Vec3::new(min.x, 0.0, min.z), Vec3::Z, dz, Vec3::X),
            WindowWall::East => (Vec3::new(max.x, 0.0, min.z), Vec3::Z, dz, Vec3::NEG_X),
        };
        Self {
            start,
            along,
            length,
            inward,
        }
    }

    /// A box over `[a0, a1]` along the wall and `[y0, y1]` up, from `n0` to `n1` along the
    /// inward normal (negative = inside the wall).
    fn block(&self, [a0, a1, y0, y1]: Piece, n0: f32, n1: f32) -> MeshData {
        let centre = self.start
            + self.along * ((a0 + a1) * 0.5)
            + Vec3::Y * ((y0 + y1) * 0.5)
            + self.inward * ((n0 + n1) * 0.5);
        let half = self.along.abs() * ((a1 - a0) * 0.5)
            + Vec3::Y * ((y1 - y0) * 0.5)
            + self.inward.abs() * ((n1 - n0).abs() * 0.5);
        chamfer_box(centre, half, 0.0)
    }
}

/// Openings of one wall as pieces `[a0, a1, sill, top]` with their door flag, sorted, clamped
/// inside the wall (a frame's width from its ends) and without overlaps (later ones are dropped
/// with a warning).
fn openings(windows: &[&WindowDef], length: f32, height: f32) -> Vec<(Piece, bool)> {
    let mut out: Vec<(Piece, bool)> = windows
        .iter()
        .map(|w| {
            let half = (w.width * 0.5).min(length * 0.5 - FRAME_U * 2.0).max(0.0);
            let c = (w.center * length).clamp(half + FRAME_U, length - half - FRAME_U);
            let top = (w.sill + w.height).min(height - FRAME_U);
            ([c - half, c + half, w.sill, top], w.door)
        })
        .filter(|(o, _)| o[1] > o[0] && o[3] > o[2])
        .collect();
    out.sort_by(|a, b| a.0[0].total_cmp(&b.0[0]));
    let mut kept: Vec<(Piece, bool)> = Vec::new();
    for o in out {
        if kept.last().is_some_and(|k| o.0[0] < k.0[1] + FRAME_U * 2.0) {
            warn!(
                "room: overlapping windows; one at {:.1} u is skipped",
                o.0[0]
            );
            continue;
        }
        kept.push(o);
    }
    kept
}

/// The solid pieces of a wall `length` × `height` around `openings` (pure).
pub(crate) fn wall_pieces(length: f32, height: f32, openings: &[Piece]) -> Vec<Piece> {
    let mut pieces = Vec::new();
    let mut a = 0.0;
    for &[a0, a1, sill, top] in openings {
        if a0 > a {
            pieces.push([a, a0, 0.0, height]);
        }
        if sill > 0.0 {
            pieces.push([a0, a1, 0.0, sill]);
        }
        if top < height {
            pieces.push([a0, a1, top, height]);
        }
        a = a1;
    }
    if a < length {
        pieces.push([a, length, 0.0, height]);
    }
    pieces
}

fn append(dst: &mut MeshData, src: MeshData) {
    let base = dst.positions.len() as u32;
    dst.positions.extend(src.positions);
    dst.normals.extend(src.normals);
    dst.uvs.extend(src.uvs);
    dst.indices
        .extend(src.indices.into_iter().map(|i| i + base));
}

/// `[walls, trim]` of the room: wall pieces and corner posts; skirting, frames, sills, mullions.
fn room_meshes(min: Vec3, max: Vec3, room: &RoomDef) -> [MeshData; 2] {
    let h = room.height;
    let (mut walls, mut trim) = (MeshData::default(), MeshData::default());
    for side in [
        WindowWall::North,
        WindowWall::South,
        WindowWall::East,
        WindowWall::West,
    ] {
        let wall = Wall::of(side, min, max);
        let windows: Vec<&WindowDef> = room.windows.iter().filter(|w| w.wall == side).collect();
        let openings = openings(&windows, wall.length, h);
        let holes: Vec<Piece> = openings.iter().map(|(o, _)| *o).collect();
        for piece in wall_pieces(wall.length, h, &holes) {
            append(&mut walls, wall.block(piece, -WALL_U, 0.0));
        }
        let skirting: Vec<Piece> = holes
            .iter()
            .filter(|o| o[2] < SKIRTING_HEIGHT_U)
            .map(|o| [o[0], o[1], 0.0, h])
            .collect();
        for [a0, a1, ..] in wall_pieces(wall.length, h, &skirting) {
            append(
                &mut trim,
                wall.block([a0, a1, 0.0, SKIRTING_HEIGHT_U], 0.0, SKIRTING_DEPTH_U),
            );
        }
        for &([a0, a1, sill, top], door) in &openings {
            let f = FRAME_U;
            append(
                &mut trim,
                wall.block([a0 - f, a0, sill, top], -f, FRAME_DEPTH_U),
            );
            append(
                &mut trim,
                wall.block([a1, a1 + f, sill, top], -f, FRAME_DEPTH_U),
            );
            append(
                &mut trim,
                wall.block([a0 - f, a1 + f, top, top + f], -f, FRAME_DEPTH_U),
            );
            if sill > f {
                append(
                    &mut trim,
                    wall.block([a0 - f, a1 + f, sill - f, sill], -f, SILL_DEPTH_U),
                );
            }
            if door {
                continue; // an open door: frame only
            }
            let panes = ((a1 - a0) / MAX_PANE_U).ceil().max(1.0) as usize;
            for i in 1..panes {
                let a = a0 + (a1 - a0) * i as f32 / panes as f32;
                let m = MULLION_U * 0.5;
                append(
                    &mut trim,
                    wall.block([a - m, a + m, sill, top], -MULLION_DEPTH_U, 0.0),
                );
            }
            let mid = (sill + top) * 0.5;
            let m = MULLION_U * 0.5;
            append(
                &mut trim,
                wall.block([a0, a1, mid - m, mid + m], -MULLION_DEPTH_U, 0.0),
            );
        }
    }
    for (x, z) in [
        (min.x, min.z),
        (max.x, min.z),
        (min.x, max.z),
        (max.x, max.z),
    ] {
        let out = Vec3::new(
            if x == min.x { -1.0 } else { 1.0 },
            0.0,
            if z == min.z { -1.0 } else { 1.0 },
        );
        let centre = Vec3::new(x, h * 0.5, z) + out * (WALL_U * 0.5);
        append(
            &mut walls,
            chamfer_box(centre, Vec3::new(WALL_U * 0.5, h * 0.5, WALL_U * 0.5), 0.0),
        );
    }
    [walls, trim]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outdoor_ring_frames_the_bounds_without_covering_them() {
        let (min, max) = (Vec3::new(-10.0, 0.0, -5.0), Vec3::new(20.0, 0.0, 15.0));
        let ring = outdoor_ring(min, max, 100.0);
        let area: f32 = ring
            .iter()
            .map(|(lo, hi)| (hi.x - lo.x) * (hi.z - lo.z))
            .sum();
        let outer = (30.0 + 200.0) * (20.0 + 200.0);
        assert!(
            (area - (outer - 30.0 * 20.0)).abs() < 1e-3,
            "ring area {area}"
        );
        let centre = Vec3::new(5.0, 0.0, 5.0);
        for (lo, hi) in ring {
            let inside = (lo.x..hi.x).contains(&centre.x) && (lo.z..hi.z).contains(&centre.z);
            assert!(!inside, "a ring piece covers the play area");
        }
    }

    #[test]
    fn the_ceiling_faces_down_only() {
        let m = ceiling(Vec3::splat(-10.0), Vec3::splat(10.0), 8.0);
        assert!(m.normals.iter().all(|n| n[1] < -0.999), "{:?}", m.normals);
        // Counter-clockwise seen from below.
        let p = |i: u32| Vec3::from_array(m.positions[i as usize]);
        let [a, b, c] = [0, 1, 2].map(|i| p(m.indices[i]));
        assert!((b - a).cross(c - a).y < 0.0);
    }

    #[test]
    fn inner_floor_follows_the_lowest_ground() {
        assert_eq!(inner_floor_top([0.5, 0.0].into_iter()), 0.0);
        assert_eq!(inner_floor_top([0.2, -2.05, 1.0].into_iter()), -2.05);
        assert_eq!(inner_floor_top(std::iter::empty()), 0.0);
    }

    fn area(pieces: &[Piece]) -> f32 {
        pieces
            .iter()
            .map(|[a0, a1, y0, y1]| (a1 - a0) * (y1 - y0))
            .sum()
    }

    #[test]
    fn a_wall_without_windows_is_one_piece() {
        assert_eq!(wall_pieces(40.0, 16.0, &[]), vec![[0.0, 40.0, 0.0, 16.0]]);
    }

    #[test]
    fn windows_are_cut_out_exactly() {
        let holes = [[5.0, 15.0, 4.0, 12.0], [20.0, 30.0, 0.0, 10.0]];
        let pieces = wall_pieces(40.0, 16.0, &holes);
        let expected = 40.0 * 16.0 - 10.0 * 8.0 - 10.0 * 10.0;
        assert!((area(&pieces) - expected).abs() < 1e-3, "{pieces:?}");
        assert!(
            !pieces
                .iter()
                .any(|p| p[0] < 30.0 && p[1] > 20.0 && p[2] < 10.0),
            "a floor-length opening keeps no piece below it"
        );
    }

    #[test]
    fn openings_stay_inside_their_wall_and_never_overlap() {
        let w = |center, width| WindowDef {
            wall: WindowWall::East,
            center,
            width,
            height: 8.0,
            sill: 4.0,
            door: false,
        };
        let windows = [w(0.0, 10.0), w(0.5, 10.0), w(0.52, 10.0)];
        let refs: Vec<&WindowDef> = windows.iter().collect();
        let holes = openings(&refs, 60.0, 16.0);
        assert_eq!(holes.len(), 2, "the overlapping third window is dropped");
        assert!(
            holes[0].0[0] >= FRAME_U - 1e-4,
            "clamped away from the corner: {holes:?}"
        );
    }

    #[test]
    fn the_room_has_walls_and_trim_with_shadow_casting_ceiling_space() {
        let room = RoomDef {
            height: 16.0,
            windows: vec![WindowDef {
                wall: WindowWall::East,
                center: 0.5,
                width: 20.0,
                height: 9.0,
                sill: 5.0,
                door: false,
            }],
            lamps: None,
        };
        let [walls, trim] = room_meshes(
            Vec3::new(-50.0, -10.0, -40.0),
            Vec3::new(50.0, 40.0, 40.0),
            &room,
        );
        assert!(walls.triangle_count() > 0 && trim.triangle_count() > 0);
        let top = walls
            .positions
            .iter()
            .map(|p| p[1])
            .fold(f32::MIN, f32::max);
        assert!((top - 16.0).abs() < 1e-4, "walls reach the ceiling: {top}");
        let east_inner = walls
            .positions
            .iter()
            .filter(|p| (p[0] - 50.0).abs() < 1e-4 && p[1] > 5.5 && p[1] < 13.5)
            .filter(|p| p[2].abs() < 9.0)
            .count();
        assert_eq!(east_inner, 0, "no wall in the window opening");
    }
}
