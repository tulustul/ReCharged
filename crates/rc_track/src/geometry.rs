//! Render and collider geometry of a layout (pure): one ribbon mesh set per run of segments
//! with the same surface between gaps, barrier runs, ground polygons.

use rc_core::SurfaceKind;

use crate::build::WALL_THICKNESS_U;
use crate::layout::Layout;
use crate::mesh::{MeshData, Ring, ground_mesh, style_profile, sweep, wall_profile};
use crate::spline::{Centreline, Sample};
use crate::track::GroundArea;

/// One ribbon run (consecutive segments with one surface, between gaps).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RibbonMesh {
    /// Surface of the run.
    pub surface: SurfaceKind,
    /// Driving surface.
    pub top: MeshData,
    /// Lips (ToyTrack) or curbs (Asphalt).
    pub trim: MeshData,
    /// Sides, chamfers, bottom and end caps.
    pub body: MeshData,
}

/// A ground polygon's mesh.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GroundMesh {
    /// Surface.
    pub surface: SurfaceKind,
    /// Upward-facing triangles.
    pub mesh: MeshData,
}

/// Render-ready geometry (rc_render turns `MeshData` into meshes; colliders use the same data).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TrackGeometry {
    /// Ribbon runs (empty for `RibbonStyle::Invisible`).
    pub ribbons: Vec<RibbonMesh>,
    /// Barrier runs (≥ 0.5 u thick).
    pub walls: Vec<MeshData>,
    /// Ground polygons.
    pub ground: Vec<GroundMesh>,
}

/// Splits the cyclic segments `0..n` into maximal runs of equal `key`: `(first point, end
/// point, key)`; a single run over the whole loop is `(0, 0, key)`.
pub(crate) fn runs<K: PartialEq + Copy>(
    n: usize,
    key: impl Fn(usize) -> K,
) -> Vec<(usize, usize, K)> {
    let Some(first) = (0..n).find(|&i| key(i) != key((i + n - 1) % n)) else {
        return vec![(0, 0, key(0))];
    };
    let mut out = Vec::new();
    let mut i = first;
    loop {
        let k = key(i);
        let mut j = (i + 1) % n;
        while j != first && key(j) == k {
            j = (j + 1) % n;
        }
        out.push((i, j, k));
        if j == first {
            return out;
        }
        i = j;
    }
}

fn ring(s: &Sample, v: f32) -> Ring {
    Ring {
        v,
        pos: s.pos,
        forward: s.forward,
        right: s.right,
        up: s.up,
        half_width: s.half_width,
    }
}

/// Frames from control point `a` to `b` (the whole loop when `a == b`): exact frames at both
/// ends, every sample in between.
fn rings(line: &Centreline, a: usize, b: usize) -> Vec<Ring> {
    let (start, end) = (line.at_points[a].1, line.at_points[b].0);
    let s0 = start.s;
    let s1 = if end.s <= s0 {
        end.s + line.length
    } else {
        end.s
    };
    let mut out = vec![ring(&start, s0)];
    let n = line.samples.len();
    let mut k = (s0 / line.ds).floor() as usize + 1;
    while (k as f32) * line.ds < s1 - 0.1 {
        let v = k as f32 * line.ds;
        if v > s0 + 0.1 {
            out.push(ring(&line.samples[k % n], v));
        }
        k += 1;
    }
    out.push(ring(&end, s1));
    out
}

pub(crate) fn geometry(
    layout: &Layout,
    line: &Centreline,
    ground: &[GroundArea],
) -> Result<TrackGeometry, String> {
    let n = layout.points.len();
    let mut in_gap = vec![false; n];
    for &(from, to) in &layout.gaps {
        let mut seg = from;
        while seg != to {
            in_gap[seg] = true;
            seg = (seg + 1) % n;
        }
    }
    let mut geo = TrackGeometry::default();
    let (profile, edge, depth) = style_profile(layout.style);
    if !profile.is_empty() {
        for (a, b, key) in runs(n, |i| (!in_gap[i]).then_some(layout.points[i].surface)) {
            let Some(surface) = key else { continue };
            let caps = (in_gap[(a + n - 1) % n], in_gap[b]);
            let [top, trim, body] = sweep(&rings(line, a, b), &profile, caps);
            geo.ribbons.push(RibbonMesh {
                surface,
                top,
                trim,
                body,
            });
        }
    }
    for left in [true, false] {
        let has = |i: usize| {
            !in_gap[i]
                && if left {
                    layout.points[i].wall_left
                } else {
                    layout.points[i].wall_right
                }
        };
        for (a, b, on) in runs(n, has) {
            if on {
                let open = a != b;
                let wall = wall_profile(left, edge, WALL_THICKNESS_U, depth, layout.wall_height);
                let [_, _, body] = sweep(&rings(line, a, b), &wall, (open, open));
                geo.walls.push(body);
            }
        }
    }
    for g in ground {
        let mesh =
            ground_mesh(&g.polygon, g.height).ok_or("ground polygon cannot be triangulated")?;
        geo.ground.push(GroundMesh {
            surface: g.surface,
            mesh,
        });
    }
    Ok(geo)
}
