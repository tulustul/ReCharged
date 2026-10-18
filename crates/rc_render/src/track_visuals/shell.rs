//! The room around an indoor track — walls with window openings, frames, skirting boards, the
//! ceiling and a floor under everything — at `Track::bounds()` (floor at y = 0, ceiling at the
//! environment's `room.height`); outdoors a ground plane reaching past the play area so the world
//! never ends at its edge. Walls cast shadows (the window openings make sun patches); the ceiling
//! does not, so the shadowed key light reaches the whole room like a skylight (T13a's lighting
//! model). rc_render's lighting glazes the openings.

use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use rc_track::{MeshData, Track};

use crate::content::mesh_from_data;
use crate::lighting::{ActiveEnvironment, RoomDef, WindowDef, WindowWall};
use crate::materials::{MaterialLibrary, SurfaceMaterial};
use crate::props::shapes::chamfer_box;

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
            let Some(mesh) = mesh_from_data(&data) else {
                return;
            };
            let mut e = commands.spawn((
                Name::new(name),
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(material),
                ChildOf(root),
            ));
            if !casts {
                e.insert(NotShadowCaster);
            }
        };
    let Some(room) = &env.def.room else {
        let reach = Vec3::new(OUTDOOR_REACH_U, 0.0, OUTDOOR_REACH_U);
        add(
            slab(min - reach, max + reach),
            "outdoor ground",
            role("floor"),
            false,
        );
        return;
    };
    add(slab(min, max), "room floor", role("floor"), false);
    let [walls, trim] = room_meshes(min, max, room);
    add(walls, "room walls", role("wall"), true);
    add(trim, "room trim", role("trim"), true);
    let lid = Vec3::new(WALL_U, 0.0, WALL_U);
    let ceiling = chamfer_box(
        Vec3::new(
            (min.x + max.x) * 0.5,
            room.height + WALL_U * 0.5,
            (min.z + max.z) * 0.5,
        ),
        ((max - min) * 0.5 + lid).with_y(WALL_U * 0.5),
        0.0,
    );
    add(ceiling, "room ceiling", role("ceiling"), false);
}

/// A thin floor slab over the x/z box with its top just below y = 0.
fn slab(min: Vec3, max: Vec3) -> MeshData {
    let centre = Vec3::new(
        (min.x + max.x) * 0.5,
        -FLOOR_DROP_U - 0.05,
        (min.z + max.z) * 0.5,
    );
    chamfer_box(centre, ((max - min) * 0.5).with_y(0.05), 0.0)
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

/// Openings of one wall as pieces `[a0, a1, sill, top]`, sorted, clamped inside the wall (a
/// frame's width from its ends) and without overlaps (later ones are dropped with a warning).
fn openings(windows: &[&WindowDef], length: f32, height: f32) -> Vec<Piece> {
    let mut out: Vec<Piece> = windows
        .iter()
        .map(|w| {
            let half = (w.width * 0.5).min(length * 0.5 - FRAME_U * 2.0).max(0.0);
            let c = (w.center * length).clamp(half + FRAME_U, length - half - FRAME_U);
            [
                c - half,
                c + half,
                w.sill,
                (w.sill + w.height).min(height - FRAME_U),
            ]
        })
        .filter(|o| o[1] > o[0] && o[3] > o[2])
        .collect();
    out.sort_by(|a, b| a[0].total_cmp(&b[0]));
    let mut kept: Vec<Piece> = Vec::new();
    for o in out {
        if kept.last().is_some_and(|k| o[0] < k[1] + FRAME_U * 2.0) {
            warn!("room: overlapping windows; one at {:.1} u is skipped", o[0]);
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
        let holes = openings(&windows, wall.length, h);
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
        for &[a0, a1, sill, top] in &holes {
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
        };
        let windows = [w(0.0, 10.0), w(0.5, 10.0), w(0.52, 10.0)];
        let refs: Vec<&WindowDef> = windows.iter().collect();
        let holes = openings(&refs, 60.0, 16.0);
        assert_eq!(holes.len(), 2, "the overlapping third window is dropped");
        assert!(
            holes[0][0] >= FRAME_U - 1e-4,
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
