//! Simple closed meshes with world-unit UVs (pure): the fallback look of props without a model
//! (their collider primitives, with chamfered edges that catch highlights) and the boxes of the
//! room shell. Flat-shaded faces, counter-clockwise fronts, outward normals; UVs are planar
//! projections of the vertex positions (u along the face, v up), so the `MaterialLibrary`'s
//! world-unit tiling applies unchanged.

use std::f32::consts::TAU;

use bevy::math::{Quat, Vec3};
use rc_track::{MeshData, Primitive};

/// Segments around cylinders and balls.
const ROUND_SEGMENTS: usize = 32;
/// Rings from pole to pole of a ball.
const BALL_RINGS: usize = 16;

/// Edge chamfer of a fallback shape: 6 % of its smallest half extent, within art-bible bevel
/// widths scaled for readability (0.02–0.12 u).
pub(crate) fn chamfer_for(half: Vec3) -> f32 {
    (half.min_element() * 0.06).clamp(0.02, 0.12)
}

/// Planar texture coordinates of `p` on a face with normal `n` (world units).
fn planar_uv(p: Vec3, n: Vec3) -> [f32; 2] {
    let a = n.abs();
    if a.y >= a.x && a.y >= a.z {
        [p.x, p.z]
    } else if a.x >= a.z {
        [p.z, p.y]
    } else {
        [p.x, p.y]
    }
}

fn push(m: &mut MeshData, p: Vec3, n: Vec3, uv: [f32; 2]) -> u32 {
    m.positions.push(p.to_array());
    m.normals.push(n.to_array());
    m.uvs.push(uv);
    (m.positions.len() - 1) as u32
}

/// Appends a flat convex polygon (any winding) facing away from the point `inside`.
fn face(m: &mut MeshData, pts: &[Vec3], inside: Vec3) {
    let count = pts.len();
    let centroid = pts.iter().copied().sum::<Vec3>() / count as f32;
    // Newell's normal: robust for any planar polygon.
    let mut n = Vec3::ZERO;
    for i in 0..count {
        let (a, b) = (pts[i], pts[(i + 1) % count]);
        n += Vec3::new(
            (a.y - b.y) * (a.z + b.z),
            (a.z - b.z) * (a.x + b.x),
            (a.x - b.x) * (a.y + b.y),
        );
    }
    let Some(mut n) = n.try_normalize() else {
        return;
    };
    let flip = n.dot(centroid - inside) < 0.0;
    if flip {
        n = -n;
    }
    let base = m.positions.len() as u32;
    for &p in pts {
        push(m, p, n, planar_uv(p, n));
    }
    for i in 1..count as u32 - 1 {
        let (b, c) = (base + i, base + i + 1);
        m.indices
            .extend(if flip { [base, c, b] } else { [base, b, c] });
    }
}

/// Appends the triangle `a b c`, wound so its front faces along `outward`.
fn tri(m: &mut MeshData, [a, b, c]: [u32; 3], outward: Vec3) {
    let p = |i: u32| Vec3::from_array(m.positions[i as usize]);
    let front = (p(b) - p(a)).cross(p(c) - p(a)).dot(outward) >= 0.0;
    m.indices.extend(if front { [a, b, c] } else { [a, c, b] });
}

/// A box around `center` with half extents `half` and edges chamfered by `chamfer` (0 = sharp).
pub(crate) fn chamfer_box(center: Vec3, half: Vec3, chamfer: f32) -> MeshData {
    let c = chamfer.clamp(0.0, half.min_element() * 0.5);
    let inner = half - Vec3::splat(c);
    let mut m = MeshData::default();
    // The corner point in the octant `sign` that lies on the face across `axis`.
    let corner = |sign: [f32; 3], axis: usize| {
        let mut p = Vec3::from_array(sign) * inner;
        p[axis] = sign[axis] * half[axis];
        center + p
    };
    let signed = |pairs: [(usize, f32); 3]| {
        let mut s = [0.0; 3];
        for (axis, v) in pairs {
            s[axis] = v;
        }
        s
    };
    const QUAD: [(f32, f32); 4] = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];
    for axis in 0..3 {
        let (u, v) = ((axis + 1) % 3, (axis + 2) % 3);
        for sign in [-1.0, 1.0] {
            let pts = QUAD.map(|(su, sv)| corner(signed([(axis, sign), (u, su), (v, sv)]), axis));
            face(&mut m, &pts, center);
        }
    }
    if c <= 0.0 {
        return m;
    }
    for (i, j) in [(0, 1), (1, 2), (0, 2)] {
        let k = 3 - i - j;
        for si in [-1.0, 1.0] {
            for sj in [-1.0, 1.0] {
                let at = |axis, sk| corner(signed([(i, si), (j, sj), (k, sk)]), axis);
                face(
                    &mut m,
                    &[at(i, -1.0), at(i, 1.0), at(j, 1.0), at(j, -1.0)],
                    center,
                );
            }
        }
    }
    for sx in [-1.0, 1.0] {
        for sy in [-1.0, 1.0] {
            for sz in [-1.0, 1.0] {
                let s = [sx, sy, sz];
                face(&mut m, &[corner(s, 0), corner(s, 1), corner(s, 2)], center);
            }
        }
    }
    m
}

/// An upright cylinder around the origin with chamfered rims; UVs: arc length × height on the
/// side, planar on the caps.
pub(crate) fn cylinder(radius: f32, half_height: f32, chamfer: f32) -> MeshData {
    let c = chamfer.clamp(0.0, radius.min(half_height) * 0.5);
    let (r, h) = (radius, half_height);
    let profile = [(r - c, -h), (r, -h + c), (r, h - c), (r - c, h)];
    let mut m = MeshData::default();
    let dir = |seg: usize| {
        let a = seg as f32 / ROUND_SEGMENTS as f32 * TAU;
        (a, Vec3::new(a.cos(), 0.0, a.sin()))
    };
    for band in profile.windows(2) {
        let ((r0, y0), (r1, y1)) = (band[0], band[1]);
        if (r1 - r0).abs() + (y1 - y0).abs() < 1e-6 {
            continue;
        }
        // Outward profile normal of an upward segment: (dy, -dr).
        let (nr, ny) = (y1 - y0, r0 - r1);
        let len = (nr * nr + ny * ny).sqrt();
        let base = m.positions.len() as u32;
        for seg in 0..=ROUND_SEGMENTS {
            let (a, d) = dir(seg);
            let n = d * (nr / len) + Vec3::Y * (ny / len);
            push(&mut m, d * r0 + Vec3::Y * y0, n, [a * r, y0]);
            push(&mut m, d * r1 + Vec3::Y * y1, n, [a * r, y1]);
        }
        for seg in 0..ROUND_SEGMENTS as u32 {
            let (a0, a1, b0, b1) = (
                base + 2 * seg,
                base + 2 * seg + 1,
                base + 2 * seg + 2,
                base + 2 * seg + 3,
            );
            let out = dir(seg as usize).1 + dir(seg as usize + 1).1;
            tri(&mut m, [a0, b0, b1], out);
            tri(&mut m, [a0, b1, a1], out);
        }
    }
    for (y, n) in [(h, Vec3::Y), (-h, Vec3::NEG_Y)] {
        let rim: Vec<Vec3> = (0..ROUND_SEGMENTS)
            .map(|seg| dir(seg).1 * (r - c) + Vec3::Y * y)
            .collect();
        face(&mut m, &rim, -n);
    }
    m
}

/// A ball around the origin; UVs: arc lengths along longitude and latitude.
pub(crate) fn ball(radius: f32) -> MeshData {
    let mut m = MeshData::default();
    for ring in 0..=BALL_RINGS {
        let polar = ring as f32 / BALL_RINGS as f32 * std::f32::consts::PI;
        for seg in 0..=ROUND_SEGMENTS {
            let a = seg as f32 / ROUND_SEGMENTS as f32 * TAU;
            let n = Vec3::new(polar.sin() * a.cos(), polar.cos(), polar.sin() * a.sin());
            push(&mut m, n * radius, n, [a * radius, polar * radius]);
        }
    }
    let row = ROUND_SEGMENTS as u32 + 1;
    for ring in 0..BALL_RINGS as u32 {
        for seg in 0..ROUND_SEGMENTS as u32 {
            let (a, b) = (ring * row + seg, (ring + 1) * row + seg);
            let out = Vec3::from_array(m.positions[a as usize])
                + Vec3::from_array(m.positions[b as usize + 1]);
            if ring > 0 {
                tri(&mut m, [a, b, a + 1], out);
            }
            if ring + 1 < BALL_RINGS as u32 {
                tri(&mut m, [a + 1, b, b + 1], out);
            }
        }
    }
    m
}

/// rc_track's ramp: the box `half` with its top sloping from full height at −Z to zero at +Z.
pub(crate) fn wedge(half: Vec3) -> MeshData {
    let [a, b, c, d, e, f] = Primitive::wedge_points(half);
    // The centroid lies inside (the box centre is on the slope).
    let inside = [a, b, c, d, e, f].iter().copied().sum::<Vec3>() / 6.0;
    let mut m = MeshData::default();
    for pts in [
        &[a, b, d, c][..],
        &[a, b, f, e],
        &[e, f, d, c],
        &[a, e, c],
        &[b, f, d],
    ] {
        face(&mut m, pts, inside);
    }
    m
}

/// The fallback mesh of one collider primitive, in prop space (posed by its offset and rotation).
pub(crate) fn primitive(p: &Primitive) -> MeshData {
    let mut m = match *p {
        Primitive::Box { half, .. } => chamfer_box(Vec3::ZERO, half, chamfer_for(half)),
        Primitive::Cylinder {
            radius,
            half_height,
            ..
        } => cylinder(
            radius,
            half_height,
            chamfer_for(Vec3::new(radius, half_height, radius)),
        ),
        Primitive::Ball { radius, .. } => ball(radius),
        Primitive::Wedge { half, .. } => wedge(half),
    };
    let (offset, rot) = p.pose();
    transform(&mut m, offset, rot);
    m
}

fn transform(m: &mut MeshData, offset: Vec3, rot: Quat) {
    for p in &mut m.positions {
        *p = (rot * Vec3::from_array(*p) + offset).to_array();
    }
    for n in &mut m.normals {
        *n = (rot * Vec3::from_array(*n)).to_array();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every triangle faces away from `inside` and its normals agree with the winding; the
    /// mesh is closed (every edge is shared by exactly two triangles, by position).
    fn assert_closed_outward(m: &MeshData, inside: Vec3) {
        let p = |i: u32| Vec3::from_array(m.positions[i as usize]);
        let key = |v: Vec3| (v * 1e4).round().as_ivec3().to_array();
        let mut edges = std::collections::HashMap::new();
        for t in m.indices.chunks(3) {
            let (a, b, c) = (p(t[0]), p(t[1]), p(t[2]));
            let geo = (b - a).cross(c - a);
            if geo.length() < 1e-9 {
                continue;
            }
            let centroid = (a + b + c) / 3.0;
            assert!(
                geo.dot(centroid - inside) > 0.0,
                "inward triangle at {centroid}"
            );
            let n = Vec3::from_array(m.normals[t[0] as usize]);
            assert!(
                n.dot(geo.normalize()) > 0.5,
                "normal disagrees with winding at {centroid}"
            );
            for (u, v) in [(a, b), (b, c), (c, a)] {
                let (ku, kv) = (key(u), key(v));
                let k = if ku < kv { (ku, kv) } else { (kv, ku) };
                *edges.entry(k).or_insert(0) += 1;
            }
        }
        assert!(
            edges.values().all(|&n| n == 2),
            "open or non-manifold edges"
        );
        assert_eq!(m.positions.len(), m.uvs.len());
        assert_eq!(m.positions.len(), m.normals.len());
    }

    #[test]
    fn chamfered_and_sharp_boxes_are_closed_and_fit_their_extents() {
        let half = Vec3::new(0.7, 0.4, 1.2);
        let centre = Vec3::new(3.0, 1.0, -2.0);
        for c in [0.0, 0.05] {
            let m = chamfer_box(centre, half, c);
            assert_closed_outward(&m, centre);
            let max = m
                .positions
                .iter()
                .map(|p| Vec3::from_array(*p) - centre)
                .fold(Vec3::ZERO, |a, p| a.max(p.abs()));
            assert!((max - half).abs().max_element() < 1e-5, "extents {max}");
        }
        assert_eq!(chamfer_box(centre, half, 0.0).triangle_count(), 12);
        assert_eq!(chamfer_box(centre, half, 0.05).triangle_count(), 44);
    }

    #[test]
    fn box_uvs_are_world_units() {
        let m = chamfer_box(Vec3::new(10.0, 0.0, 0.0), Vec3::new(2.0, 1.0, 1.0), 0.0);
        let us: Vec<f32> = m.uvs.iter().map(|uv| uv[0]).collect();
        assert!(
            us.iter().any(|&u| (u - 12.0).abs() < 1e-5),
            "u follows world x on ±Y/±Z faces"
        );
    }

    #[test]
    fn round_shapes_and_wedges_are_closed() {
        assert_closed_outward(&cylinder(1.3, 1.0, 0.08), Vec3::ZERO);
        assert_closed_outward(&cylinder(1.3, 1.0, 0.0), Vec3::ZERO);
        assert_closed_outward(&ball(0.5), Vec3::ZERO);
        let half = Vec3::new(2.0, 1.25, 4.5);
        let w = wedge(half);
        assert_closed_outward(&w, Vec3::new(0.0, -half.y / 3.0, -half.z / 3.0));
        assert_eq!(w.triangle_count(), 8);
    }

    #[test]
    fn primitives_are_posed_by_their_offset_and_rotation() {
        let p = Primitive::Box {
            half: Vec3::new(1.0, 0.5, 2.0),
            offset: Vec3::new(0.0, 0.5, 0.0),
            // rc_track applies `rot.y` as the yaw.
            rot: Vec3::new(0.0, 90.0, 0.0),
        };
        let m = primitive(&p);
        let max_x = m.positions.iter().map(|q| q[0]).fold(f32::MIN, f32::max);
        assert!(
            (max_x - 2.0).abs() < 1e-4,
            "yawed 90°: the long side lies along x ({max_x})"
        );
        let min_y = m.positions.iter().map(|q| q[1]).fold(f32::MAX, f32::min);
        assert!(
            min_y.abs() < 1e-4,
            "offset lifts it onto the ground ({min_y})"
        );
    }
}
