//! Barriers in the environment's style (GDD §7.3, art bible §3): where rc_track has a barrier,
//! a row of chamfered blocks — foam blocks in the toy store, red/white kart blocks on the test
//! grounds, kerb stones on the street, edging stones in the garden — occupying exactly the
//! collider's box (same frames, inner face, thickness, depth and height as rc_track's sweep).
//! Environments without a block style draw the collider sweep in their `barrier` material.

use bevy::math::{Mat3, Quat, Vec3};
use rc_track::{MeshData, RibbonStyle, Track, WALL_THICKNESS_U};

use crate::props::shapes::chamfer_box;

/// Edge chamfer of barrier blocks (u): catches highlights at chase-camera distance.
const BLOCK_CHAMFER_U: f32 = 0.05;
/// Most a block's straight chord may leave the curved barrier line (u).
const MAX_DEVIATION_U: f32 = 0.03;

/// A barrier look: blocks `length` long with `gap` between them, cycling through library
/// materials.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct BlockStyle {
    pub length: f32,
    pub gap: f32,
    pub materials: &'static [&'static str],
}

/// The block style of an environment (`None` = the plain collider sweep).
pub(crate) fn block_style(environment: &str) -> Option<BlockStyle> {
    let style = |length, gap, materials| {
        Some(BlockStyle {
            length,
            gap,
            materials,
        })
    };
    match environment {
        "toy_store" => style(
            2.0,
            0.06,
            &[
                "plastic_red",
                "plastic_yellow",
                "plastic_blue",
                "plastic_green",
            ][..],
        ),
        "test" => style(1.5, 0.0, &["plastic_red", "plastic_white"][..]),
        // Kerb stones and brick edging in the environment's own barrier material.
        "street" => style(4.0, 0.05, &["barrier"][..]),
        "garden" => style(1.2, 0.06, &["barrier"][..]),
        _ => None,
    }
}

/// Where rc_track stands its barriers (`docs/design/rc_track.md`, ribbon styles): the inner
/// face's reach beyond the half width, and the depth below the driving surface.
fn wall_edge(style: RibbonStyle) -> (f32, f32) {
    match style {
        RibbonStyle::ToyTrack => (0.35, 0.25),
        RibbonStyle::Asphalt => (0.9, 0.3),
        RibbonStyle::CarpetRunner => (0.0, 0.06),
        RibbonStyle::Invisible => (0.0, 0.2),
    }
}

/// Maximal runs `(first sample, sample count)` of a cyclic flag over `n` samples.
fn runs(n: usize, has: impl Fn(usize) -> bool) -> Vec<(usize, usize)> {
    let Some(start) = (0..n).find(|&i| has(i) && !has((i + n - 1) % n)) else {
        return if n > 0 && has(0) {
            vec![(0, n)]
        } else {
            Vec::new()
        };
    };
    let mut out = Vec::new();
    let mut i = start;
    let mut seen = 0;
    while seen < n {
        if has(i % n) {
            let first = i % n;
            let mut len = 0;
            while len < n && has((first + len) % n) {
                len += 1;
            }
            out.push((first, len));
            i += len;
            seen += len;
        } else {
            i += 1;
            seen += 1;
        }
    }
    out
}

/// Largest distance of the points strictly between `a` and `b` from the chord `a`–`b`.
fn chord_deviation(points: &[Vec3], a: usize, b: usize) -> f32 {
    let (pa, pb) = (points[a], points[b]);
    let d = pb - pa;
    let len2 = d.length_squared().max(1e-9);
    points[a + 1..b]
        .iter()
        .map(|&p| p.distance(pa + d * ((p - pa).dot(d) / len2).clamp(0.0, 1.0)))
        .fold(0.0, f32::max)
}

/// The blocks of every barrier, one mesh per material of `style`. Blocks are chords of the
/// barrier's centre line: `style.length` long on straights, shorter where the wall curves
/// tightly (inside of hairpins), so they never stick out of the collider.
pub(crate) fn block_meshes(track: &Track, style: &BlockStyle) -> Vec<MeshData> {
    let mut out = vec![MeshData::default(); style.materials.len()];
    let ds = track.sample_spacing();
    let n = (track.length() / ds).round() as usize;
    let (edge, depth) = wall_edge(track.style());
    let height = track.wall_height();
    let half_t = WALL_THICKNESS_U * 0.5;
    let mut k = 0;
    for side in [-1.0_f32, 1.0] {
        let has = |i: usize| {
            let (left, right) = track.walls_at(i as f32 * ds);
            if side < 0.0 { left } else { right }
        };
        for (first, count) in runs(n, has) {
            // The wall's centre line and up at every sample of the run, end sample included.
            let (points, ups): (Vec<Vec3>, Vec<Vec3>) = (0..=count)
                .map(|i| {
                    let f = track.frame_at(((first + i) % n) as f32 * ds);
                    let lateral = side * (f.half_width + edge + half_t);
                    (f.point(lateral, (height - depth) * 0.5), f.up)
                })
                .unzip();
            let mut a = 0;
            while a < count {
                let mut b = a + 1;
                while b < count
                    && points[a].distance(points[b + 1]) <= style.length + 1e-3
                    && chord_deviation(&points, a, b + 1) <= MAX_DEVIATION_U
                {
                    b += 1;
                }
                let (pa, pb) = (points[a], points[b]);
                let len = pa.distance(pb).max(1e-3);
                let forward = (pb - pa) / len;
                let right = forward.cross(ups[a] + ups[b]).normalize();
                let rot = Quat::from_mat3(&Mat3::from_cols(right, right.cross(forward), -forward));
                let half = Vec3::new(
                    half_t,
                    (height + depth) * 0.5,
                    ((len - style.gap) * 0.5).max(0.02),
                );
                let mut block = chamfer_box(Vec3::ZERO, half, BLOCK_CHAMFER_U);
                let centre = (pa + pb) * 0.5;
                for p in &mut block.positions {
                    *p = (rot * Vec3::from_array(*p) + centre).to_array();
                }
                for nrm in &mut block.normals {
                    *nrm = (rot * Vec3::from_array(*nrm)).to_array();
                }
                append(&mut out[k % style.materials.len()], block);
                k += 1;
                a = b;
            }
        }
    }
    out
}

fn append(dst: &mut MeshData, src: MeshData) {
    let base = dst.positions.len() as u32;
    dst.positions.extend(src.positions);
    dst.normals.extend(src.normals);
    dst.uvs.extend(src.uvs);
    dst.indices
        .extend(src.indices.into_iter().map(|i| i + base));
}

#[cfg(test)]
mod tests {
    use super::*;
    use rc_core::TrackVariant;

    fn track(id: &str) -> Track {
        let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        Track::load(&assets, &TrackVariant::forward(id)).unwrap_or_else(|e| panic!("{e}"))
    }

    #[test]
    fn runs_wrap_around_the_loop() {
        let flags = [true, true, false, false, true, true];
        assert_eq!(runs(6, |i| flags[i]), vec![(4, 4)]);
        assert_eq!(runs(4, |_| true), vec![(0, 4)]);
        assert!(runs(4, |_| false).is_empty());
        let split = [true, false, true, false];
        assert_eq!(runs(4, |i| split[i]), vec![(0, 1), (2, 1)]);
    }

    /// The blocks stand where rc_track's barrier colliders are: every block's inner bottom edge
    /// lies on the collider's inner face (catches drift if rc_track moves its barriers).
    #[test]
    fn blocks_cover_the_collider_barriers() {
        for id in ["test_oval", "toy_town"] {
            let track = track(id);
            let style = block_style("test").expect("a style");
            let blocks = block_meshes(&track, &style);
            let walls: Vec<Vec3> = track
                .geometry()
                .walls
                .iter()
                .flat_map(|w| w.positions.iter().map(|p| Vec3::from_array(*p)))
                .collect();
            let block_points: Vec<Vec3> = blocks
                .iter()
                .flat_map(|m| m.positions.iter().map(|p| Vec3::from_array(*p)))
                .collect();
            assert!(!block_points.is_empty(), "{id}: no blocks");
            let lo = walls.iter().copied().fold(Vec3::MAX, Vec3::min);
            let hi = walls.iter().copied().fold(Vec3::MIN, Vec3::max);
            let blo = block_points.iter().copied().fold(Vec3::MAX, Vec3::min);
            let bhi = block_points.iter().copied().fold(Vec3::MIN, Vec3::max);
            assert!(
                (blo - lo).abs().max_element() < 0.1 && (bhi - hi).abs().max_element() < 0.1,
                "{id}: blocks {blo}..{bhi} vs colliders {lo}..{hi}"
            );
            // The inner face's bottom edge at every barrier sample is a collider vertex: same
            // offsets as rc_track's sweep.
            let (edge, depth) = wall_edge(track.style());
            let ds = track.sample_spacing();
            for k in (0..(track.length() / ds) as usize).step_by(9) {
                let f = track.frame_at(k as f32 * ds);
                let (left, right) = track.walls_at(k as f32 * ds);
                for (on, side) in [(left, -1.0), (right, 1.0)] {
                    if !on {
                        continue;
                    }
                    let p = f.point(side * (f.half_width + edge), -depth);
                    let d = walls.iter().map(|w| w.distance(p)).fold(f32::MAX, f32::min);
                    assert!(d < 0.02, "{id}: inner face off by {d} at s = {}", f.s);
                }
            }
        }
    }
}
