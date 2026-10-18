//! Geometry buffers (pure). Ribbon runs and walls are a closed cross-section profile swept along
//! frames ("rings"); ground polygons are ear-clipped. Flat-shaded, CCW = front (Bevy and
//! parry agree), outward normals. UVs are world units: `v` = centreline arc length, `u` = lateral
//! position on driving surfaces, perimeter distance elsewhere.

use bevy::math::{Vec2, Vec3};

use crate::def::RibbonStyle;

/// Triangle buffers of one mesh (Bevy `Mesh` attribute layout; rapier takes the same data).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MeshData {
    /// Vertex positions (world u).
    pub positions: Vec<[f32; 3]>,
    /// Unit vertex normals.
    pub normals: Vec<[f32; 3]>,
    /// Texture coordinates in world units.
    pub uvs: Vec<[f32; 2]>,
    /// Triangle list, counter-clockwise front faces.
    pub indices: Vec<u32>,
}

impl MeshData {
    /// Number of triangles.
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// No triangles.
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    fn push(&mut self, p: Vec3, n: Vec3, uv: [f32; 2]) -> u32 {
        self.positions.push(p.to_array());
        self.normals.push(n.to_array());
        self.uvs.push(uv);
        (self.positions.len() - 1) as u32
    }

    /// Vertices and triangles of several meshes as one collider soup.
    pub(crate) fn soup(parts: &[&MeshData]) -> (Vec<Vec3>, Vec<[u32; 3]>) {
        let (mut v, mut t) = (Vec::new(), Vec::new());
        for m in parts {
            let base = v.len() as u32;
            v.extend(m.positions.iter().map(|p| Vec3::from_array(*p)));
            t.extend(
                m.indices
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .map(|c| c.map(|i| base + i)),
            );
        }
        (v, t)
    }
}

/// A cross-section frame along a run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Ring {
    /// Texture `v` (arc length, unwrapped along the run).
    pub v: f32,
    pub pos: Vec3,
    pub forward: Vec3,
    pub right: Vec3,
    pub up: Vec3,
    pub half_width: f32,
}

/// Which mesh of a ribbon run a profile edge belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Part {
    /// The driving surface.
    Top,
    /// Lips and curbs.
    Trim,
    /// Sides, chamfers, bottom, end caps.
    Body,
}

/// Profile vertex at lateral `k · half_width + x`, height `y`; `edge` = part of the edge to the
/// next vertex. Vertices run counter-clockwise in the (lateral, height) plane.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ProfilePoint {
    k: f32,
    x: f32,
    y: f32,
    edge: Part,
}

const fn pp(k: f32, x: f32, y: f32, edge: Part) -> ProfilePoint {
    ProfilePoint { k, x, y, edge }
}

impl ProfilePoint {
    pub(crate) fn at(&self, half_width: f32) -> Vec2 {
        Vec2::new(self.k * half_width + self.x, self.y)
    }
}

/// Cross-section of a ribbon style, plus where walls stand: `(profile, wall edge beyond the half
/// width, ribbon thickness below the surface)`. Invisible has no profile.
pub(crate) fn style_profile(style: RibbonStyle) -> (Vec<ProfilePoint>, f32, f32) {
    use Part::*;
    match style {
        RibbonStyle::ToyTrack => {
            // 0.25 thick, lips 0.25 high × 0.35 wide, 0.7 u outer chamfers.
            let (t, l, w, c) = (0.25, 0.25, 0.35, 0.7);
            let p = vec![
                pp(-1.0, -w - c, -t, Body),
                pp(1.0, w + c, -t, Body),
                pp(1.0, w, l, Trim),
                pp(1.0, 0.0, l, Trim),
                pp(1.0, 0.0, 0.0, Top),
                pp(-1.0, 0.0, 0.0, Trim),
                pp(-1.0, 0.0, l, Trim),
                pp(-1.0, -w, l, Body),
            ];
            (p, w, t)
        }
        RibbonStyle::Asphalt => {
            // 0.3 thick slab, sloped curbs 0.9 wide × 0.08 high, 1 u chamfers.
            let (t, wc, hc, c) = (0.3, 0.9, 0.08, 1.0);
            let p = vec![
                pp(-1.0, -wc - c, -t, Body),
                pp(1.0, wc + c, -t, Body),
                pp(1.0, wc, hc, Trim),
                pp(1.0, 0.0, 0.0, Top),
                pp(-1.0, 0.0, 0.0, Trim),
                pp(-1.0, -wc, hc, Body),
            ];
            (p, wc, t)
        }
        RibbonStyle::CarpetRunner => {
            let (t, c) = (0.06, 0.15);
            let p = vec![
                pp(-1.0, -c, -t, Body),
                pp(1.0, c, -t, Body),
                pp(1.0, 0.0, 0.0, Top),
                pp(-1.0, 0.0, 0.0, Body),
            ];
            (p, 0.0, t)
        }
        RibbonStyle::Invisible => (Vec::new(), 0.0, 0.2),
    }
}

/// Barrier cross-section on one side: inner face at `half_width + edge`, `thickness` wide, from
/// `depth` below the surface to `height` above it.
pub(crate) fn wall_profile(
    left: bool,
    edge: f32,
    thickness: f32,
    depth: f32,
    height: f32,
) -> Vec<ProfilePoint> {
    use Part::Body;
    if left {
        vec![
            pp(-1.0, -edge - thickness, -depth, Body),
            pp(-1.0, -edge, -depth, Body),
            pp(-1.0, -edge, height, Body),
            pp(-1.0, -edge - thickness, height, Body),
        ]
    } else {
        vec![
            pp(1.0, edge, -depth, Body),
            pp(1.0, edge + thickness, -depth, Body),
            pp(1.0, edge + thickness, height, Body),
            pp(1.0, edge, height, Body),
        ]
    }
}

/// Sweeps `profile` along `rings` into one mesh per [`Part`] (`[Top, Trim, Body]`), with end
/// caps where `caps` says so (start, end).
pub(crate) fn sweep(rings: &[Ring], profile: &[ProfilePoint], caps: (bool, bool)) -> [MeshData; 3] {
    let mut out: [MeshData; 3] = Default::default();
    let n = profile.len();
    if rings.len() < 2 || n < 3 {
        return out;
    }
    for e in 0..n {
        let (pa, pb) = (profile[e], profile[(e + 1) % n]);
        let mesh = &mut out[pa.edge as usize];
        let base = mesh.positions.len() as u32;
        for ring in rings {
            let (a, b) = (pa.at(ring.half_width), pb.at(ring.half_width));
            let d = b - a;
            let n2 = Vec2::new(d.y, -d.x).normalize_or_zero();
            let normal = ring.right * n2.x + ring.up * n2.y;
            let (ua, ub) = if pa.edge == Part::Top {
                (a.x, b.x)
            } else {
                let start = perimeter_to(profile, e, ring.half_width);
                (start, start + d.length())
            };
            mesh.push(
                ring.pos + ring.right * a.x + ring.up * a.y,
                normal,
                [ua, ring.v],
            );
            mesh.push(
                ring.pos + ring.right * b.x + ring.up * b.y,
                normal,
                [ub, ring.v],
            );
        }
        for r in 0..rings.len() as u32 - 1 {
            let (a0, b0, a1, b1) = (
                base + 2 * r,
                base + 2 * r + 1,
                base + 2 * r + 2,
                base + 2 * r + 3,
            );
            mesh.indices.extend([a0, b1, b0, a0, a1, b1]);
        }
    }
    let ends = [
        (caps.0, rings[0], -1.0),
        (caps.1, rings[rings.len() - 1], 1.0),
    ];
    for (cap, ring, sign) in ends {
        if cap {
            add_cap(
                &mut out[Part::Body as usize],
                profile,
                &ring,
                ring.forward * sign,
            );
        }
    }
    out
}

fn perimeter_to(profile: &[ProfilePoint], edge: usize, half_width: f32) -> f32 {
    (0..edge)
        .map(|e| {
            profile[e]
                .at(half_width)
                .distance(profile[e + 1].at(half_width))
        })
        .sum()
}

fn add_cap(mesh: &mut MeshData, profile: &[ProfilePoint], ring: &Ring, normal: Vec3) {
    let flat: Vec<Vec2> = profile.iter().map(|p| p.at(ring.half_width)).collect();
    let Some(tris) = triangulate(&flat) else {
        return;
    };
    let world: Vec<Vec3> = flat
        .iter()
        .map(|p| ring.pos + ring.right * p.x + ring.up * p.y)
        .collect();
    let base = mesh.positions.len() as u32;
    for (p, w) in flat.iter().zip(&world) {
        mesh.push(*w, normal, [p.x, p.y]);
    }
    add_facing(mesh, base, &world, &tris, normal);
}

/// Appends `tris` (indices into `world`, offset by `base`) wound to face `normal`.
fn add_facing(mesh: &mut MeshData, base: u32, world: &[Vec3], tris: &[[usize; 3]], normal: Vec3) {
    for &[a, b, c] in tris {
        let facing = (world[b] - world[a]).cross(world[c] - world[a]).dot(normal) >= 0.0;
        let (b, c) = if facing { (b, c) } else { (c, b) };
        mesh.indices.extend([a, b, c].map(|i| base + i as u32));
    }
}

/// Upward-facing mesh of a flat polygon (world x, z) at `height`; `None` if it is not simple.
pub(crate) fn ground_mesh(polygon: &[Vec2], height: f32) -> Option<MeshData> {
    let tris = triangulate(polygon)?;
    let world: Vec<Vec3> = polygon
        .iter()
        .map(|p| Vec3::new(p.x, height, p.y))
        .collect();
    let mut mesh = MeshData::default();
    for (p, w) in polygon.iter().zip(&world) {
        mesh.push(*w, Vec3::Y, [p.x, p.y]);
    }
    add_facing(&mut mesh, 0, &world, &tris, Vec3::Y);
    Some(mesh)
}

fn cross2(a: Vec2, b: Vec2) -> f32 {
    a.x * b.y - a.y * b.x
}

/// Twice the signed area (positive = counter-clockwise).
pub(crate) fn signed_area2(polygon: &[Vec2]) -> f32 {
    (0..polygon.len())
        .map(|i| cross2(polygon[i], polygon[(i + 1) % polygon.len()]))
        .sum()
}

/// Whether non-adjacent edges of `polygon` cross.
pub(crate) fn self_intersects(polygon: &[Vec2]) -> bool {
    let n = polygon.len();
    let edge = |i: usize| (polygon[i], polygon[(i + 1) % n]);
    let crosses = |(a, b): (Vec2, Vec2), (c, d): (Vec2, Vec2)| {
        let side = |p: Vec2, q: Vec2, r: Vec2| cross2(q - p, r - p);
        side(a, b, c) * side(a, b, d) < 0.0 && side(c, d, a) * side(c, d, b) < 0.0
    };
    (0..n).any(|i| ((i + 2)..n).any(|j| (j + 1) % n != i && crosses(edge(i), edge(j))))
}

/// Ear-clipping triangulation of a simple polygon (either winding). Triangles are
/// counter-clockwise in the polygon's plane. O(n²): fine for hand-made outlines.
pub(crate) fn triangulate(polygon: &[Vec2]) -> Option<Vec<[usize; 3]>> {
    let area = signed_area2(polygon);
    if polygon.len() < 3 || area.abs() < 1e-6 {
        return None;
    }
    let mut idx: Vec<usize> = (0..polygon.len()).collect();
    if area < 0.0 {
        idx.reverse();
    }
    let mut tris = Vec::with_capacity(polygon.len() - 2);
    while idx.len() > 3 {
        let m = idx.len();
        let ear = (0..m).find(|&i| {
            let (a, b, c) = (idx[(i + m - 1) % m], idx[i], idx[(i + 1) % m]);
            let (pa, pb, pc) = (polygon[a], polygon[b], polygon[c]);
            cross2(pb - pa, pc - pb) > 1e-9
                && !idx.iter().any(|&p| {
                    let q = polygon[p];
                    ![pa, pb, pc].contains(&q)
                        && cross2(pb - pa, q - pa) >= 0.0
                        && cross2(pc - pb, q - pb) >= 0.0
                        && cross2(pa - pc, q - pc) >= 0.0
                })
        })?;
        tris.push([idx[(ear + m - 1) % m], idx[ear], idx[(ear + 1) % m]]);
        idx.remove(ear);
    }
    tris.push([idx[0], idx[1], idx[2]]);
    Some(tris)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tri_area(t: [usize; 3], p: &[Vec2]) -> f32 {
        0.5 * cross2(p[t[1]] - p[t[0]], p[t[2]] - p[t[0]])
    }

    #[test]
    fn triangulation_covers_concave_polygons_in_both_windings() {
        // An L shape (area 300).
        let mut l = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(20.0, 0.0),
            Vec2::new(20.0, 10.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(10.0, 20.0),
            Vec2::new(0.0, 20.0),
        ];
        for _ in 0..2 {
            let tris = triangulate(&l).expect("simple polygon");
            assert_eq!(tris.len(), l.len() - 2);
            let total: f32 = tris.iter().map(|&t| tri_area(t, &l)).sum();
            assert!((total - 300.0).abs() < 1e-3, "area {total}");
            assert!(tris.iter().all(|&t| tri_area(t, &l) > 0.0), "all CCW");
            l.reverse();
        }
        assert!(!self_intersects(&l));
        let bowtie = [
            Vec2::ZERO,
            Vec2::new(10.0, 10.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(0.0, 10.0),
        ];
        assert!(self_intersects(&bowtie));
        assert!(
            triangulate(&[Vec2::ZERO, Vec2::X, Vec2::X * 2.0]).is_none(),
            "degenerate"
        );
    }

    #[test]
    fn ground_faces_up_with_world_uvs() {
        let square = [
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 8.0),
            Vec2::new(8.0, 8.0),
            Vec2::new(8.0, 0.0),
        ];
        let mesh = ground_mesh(&square, 2.0).expect("simple");
        assert_eq!(mesh.triangle_count(), 2);
        for t in mesh.indices.as_chunks::<3>().0 {
            let [a, b, c] = [0, 1, 2].map(|k| Vec3::from_array(mesh.positions[t[k] as usize]));
            assert!((b - a).cross(c - a).y > 0.0);
        }
        assert_eq!(mesh.uvs[2], [8.0, 8.0]);
        assert!(mesh.positions.iter().all(|p| p[1] == 2.0));
    }

    /// Every swept triangle faces the way its vertex normals say (outward).
    #[test]
    fn swept_profiles_face_outward() {
        let rings: Vec<Ring> = (0..5)
            .map(|i| {
                let a = i as f32 * 0.2;
                let forward = Vec3::new(a.sin(), 0.0, -a.cos());
                let up = Vec3::new(0.0, 0.94, 0.0) + forward.cross(Vec3::Y) * 0.34;
                let up = up.normalize();
                Ring {
                    v: i as f32,
                    pos: Vec3::new(30.0 * (1.0 - a.cos()), 0.0, -30.0 * a.sin()),
                    forward,
                    right: forward.cross(up),
                    up,
                    half_width: 5.0,
                }
            })
            .collect();
        for style in [
            RibbonStyle::ToyTrack,
            RibbonStyle::Asphalt,
            RibbonStyle::CarpetRunner,
        ] {
            let (profile, edge, depth) = style_profile(style);
            let mut meshes: Vec<MeshData> = sweep(&rings, &profile, (true, true)).into();
            meshes.extend(sweep(
                &rings,
                &wall_profile(true, edge, 0.6, depth, 1.2),
                (true, true),
            ));
            meshes.extend(sweep(
                &rings,
                &wall_profile(false, edge, 0.6, depth, 1.2),
                (true, false),
            ));
            for mesh in &meshes {
                for t in mesh.indices.as_chunks::<3>().0 {
                    let [a, b, c] =
                        [0, 1, 2].map(|k| Vec3::from_array(mesh.positions[t[k] as usize]));
                    let n = Vec3::from_array(mesh.normals[t[0] as usize]);
                    let geo = (b - a).cross(c - a);
                    assert!(
                        geo.length() < 1e-9 || geo.dot(n) > 0.0,
                        "{style:?} triangle faces inward"
                    );
                }
            }
            let top = &sweep(&rings, &profile, (false, false))[Part::Top as usize];
            assert!(
                top.normals
                    .iter()
                    .all(|n| Vec3::from_array(*n).dot(rings[0].up) > 0.5)
            );
            assert_eq!(top.uvs[0][0], 5.0, "u is the lateral position");
        }
    }
}
