//! Meshes of the water ground areas: the area's polygon plus a ring inset by [`SHORE_U`], with the
//! shore coordinate in UV_1 (x: 0 on the outline → 1 on the inset ring) for the foam of
//! `water.wesl`.

use bevy::prelude::*;
use rc_track::MeshData;

/// Width of the shore band (u); narrower on small areas.
const SHORE_U: f32 = 0.45;

/// The water mesh of a ground area's mesh (vertices = its outline in order, as rc_track builds
/// ground meshes) and each vertex's UV_1.
pub(super) fn water_mesh(ground: &MeshData) -> (MeshData, Vec<[f32; 2]>) {
    let outline: Vec<Vec2> = ground
        .positions
        .iter()
        .map(|p| Vec2::new(p[0], p[2]))
        .collect();
    let n = outline.len();
    let y = ground.positions.first().map_or(0.0, |p| p[1]);
    let (min, max) = outline.iter().fold((Vec2::MAX, Vec2::MIN), |(lo, hi), p| {
        (lo.min(*p), hi.max(*p))
    });
    let width = SHORE_U.min(0.2 * (max - min).min_element());
    let area2: f32 = (0..n)
        .map(|i| outline[i].perp_dot(outline[(i + 1) % n]))
        .sum();
    // Inward = left of each edge for a counter-clockwise outline.
    let inward = |a: Vec2, b: Vec2| (b - a).perp().normalize_or_zero() * area2.signum();
    let inset: Vec<Vec2> = (0..n)
        .map(|i| {
            let (prev, cur, next) = (outline[(i + n - 1) % n], outline[i], outline[(i + 1) % n]);
            let (n1, n2) = (inward(prev, cur), inward(cur, next));
            let m = (n1 + n2).normalize_or(n2);
            cur + m * (width / m.dot(n2).max(0.25))
        })
        .collect();

    let mut mesh = MeshData::default();
    for p in outline.iter().chain(&inset) {
        mesh.positions.push([p.x, y, p.y]);
        mesh.normals.push([0.0, 1.0, 0.0]);
        mesh.uvs.push([p.x, p.y]);
    }
    // The inset polygon keeps the outline's triangulation; the ring joins the two.
    mesh.indices
        .extend(ground.indices.iter().map(|i| i + n as u32));
    let up = |[a, b, c]: [u32; 3]| {
        let p = |i: u32| Vec3::from(mesh.positions[i as usize]);
        if (p(b) - p(a)).cross(p(c) - p(a)).y >= 0.0 {
            [a, b, c]
        } else {
            [a, c, b]
        }
    };
    let ring: Vec<u32> = (0..n as u32)
        .flat_map(|i| {
            let j = (i + 1) % n as u32;
            let (a, b, ia, ib) = (i, j, i + n as u32, j + n as u32);
            [up([a, b, ib]), up([a, ib, ia])].concat()
        })
        .collect();
    mesh.indices.extend(ring);
    let shore = (0..2 * n)
        .map(|i| [if i < n { 0.0 } else { 1.0 }, 0.0])
        .collect();
    (mesh, shore)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ground(outline: &[[f32; 2]], indices: &[u32]) -> MeshData {
        MeshData {
            positions: outline.iter().map(|p| [p[0], 0.06, p[1]]).collect(),
            normals: vec![[0.0, 1.0, 0.0]; outline.len()],
            uvs: outline.to_vec(),
            indices: indices.to_vec(),
        }
    }

    #[test]
    fn the_shore_ring_lies_inside_and_every_triangle_faces_up() {
        // Both windings of a 4 × 4 square (ground meshes' triangles face up either way).
        for (outline, indices) in [
            (
                [[0.0, 0.0], [0.0, 4.0], [4.0, 4.0], [4.0, 0.0]],
                [0, 1, 2, 0, 2, 3],
            ),
            (
                [[0.0, 0.0], [4.0, 0.0], [4.0, 4.0], [0.0, 4.0]],
                [0, 2, 1, 0, 3, 2],
            ),
        ] {
            let (mesh, shore) = water_mesh(&ground(&outline, &indices));
            assert_eq!(mesh.positions.len(), 8);
            for p in &mesh.positions[4..] {
                let inside = p[0] > 0.3 && p[0] < 3.7 && p[2] > 0.3 && p[2] < 3.7;
                assert!(inside, "inset vertex {p:?}");
                assert_eq!(p[1], 0.06);
            }
            assert_eq!(&shore[..4], &[[0.0, 0.0]; 4]);
            assert_eq!(&shore[4..], &[[1.0, 0.0]; 4]);
            assert_eq!(mesh.indices.len(), 6 + 4 * 6);
            for t in mesh.indices.chunks(3) {
                let p = |i: u32| Vec3::from(mesh.positions[i as usize]);
                let normal = (p(t[1]) - p(t[0])).cross(p(t[2]) - p(t[0]));
                assert!(normal.y > 0.0, "triangle {t:?} faces down");
            }
        }
    }
}
