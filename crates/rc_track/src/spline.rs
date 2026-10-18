//! Centreline (pure): a closed centripetal Catmull-Rom spline through the control points, cut at
//! active gaps — a gap is a straight flight line, and lips/landings get one-sided tangents so the
//! takeoff angle is the ramp's own slope. Resampled every ~[`SAMPLE_SPACING_U`] of arc length
//! with banked frames (helices fine; frames tilting past [`MIN_UP_Y`] are rejected: no loops).

use bevy::math::Vec3;

use crate::layout::Point;

/// Target spacing of centreline samples (u).
pub const SAMPLE_SPACING_U: f32 = 0.5;
/// Step of the dense pre-sampling used for arc length (u).
const DENSE_STEP_U: f32 = 0.05;
/// Frames whose up vector has less vertical component than this are rejected (≈ 84°).
pub const MIN_UP_Y: f32 = 0.1;

/// One centreline sample with its banked frame. `right = forward × up`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Sample {
    pub s: f32,
    pub pos: Vec3,
    pub forward: Vec3,
    pub right: Vec3,
    pub up: Vec3,
    pub half_width: f32,
    /// Radians, positive raises the left edge.
    pub bank: f32,
    /// Control segment (index of its first point).
    pub seg: usize,
    /// Inside an active gap (no ribbon below).
    pub in_gap: bool,
}

/// The resampled loop: `samples[k].s = k * ds`; the sample after the last one is sample 0.
#[derive(Debug, Clone)]
pub(crate) struct Centreline {
    pub samples: Vec<Sample>,
    pub ds: f32,
    pub length: f32,
    /// Exact frames at each control point: (arriving, leaving). They differ only at gap ends.
    pub at_points: Vec<(Sample, Sample)>,
}

/// Right and up vectors of a frame looking along `forward` (unit) with `bank` (radians).
pub(crate) fn frame(forward: Vec3, bank: f32) -> Option<(Vec3, Vec3)> {
    let up0 = (Vec3::Y - forward * forward.y).try_normalize()?;
    let right0 = forward.cross(up0);
    let up = up0 * bank.cos() + right0 * bank.sin();
    Some((forward.cross(up), up))
}

fn smoothstep(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// Centripetal (α = ½) Catmull-Rom between `p1` and `p2` at `u` ∈ [0, 1] (Barry–Goldman).
fn catmull_rom(p: [Vec3; 4], u: f32) -> Vec3 {
    let knot = |a: Vec3, b: Vec3| a.distance(b).max(1e-4).sqrt();
    let t1 = knot(p[0], p[1]);
    let t2 = t1 + knot(p[1], p[2]);
    let t3 = t2 + knot(p[2], p[3]);
    let t = t1 + (t2 - t1) * u;
    let mix = |a: Vec3, b: Vec3, ta: f32, tb: f32| {
        a * ((tb - t) / (tb - ta)) + b * ((t - ta) / (tb - ta))
    };
    let a1 = mix(p[0], p[1], 0.0, t1);
    let a2 = mix(p[1], p[2], t1, t2);
    let a3 = mix(p[2], p[3], t2, t3);
    let b1 = mix(a1, a2, 0.0, t2);
    let b2 = mix(a2, a3, t1, t3);
    mix(b1, b2, t1, t2)
}

/// Marks gap segments; errors on overlapping or touching gaps and a start point in the air.
fn gap_segments(n: usize, gaps: &[(usize, usize)]) -> Result<Vec<Option<usize>>, String> {
    let mut owner = vec![None; n];
    for (g, &(from, to)) in gaps.iter().enumerate() {
        let mut seg = from;
        while seg != to {
            if owner[seg].replace(g).is_some() {
                return Err(format!("gaps overlap at segment {seg}"));
            }
            seg = (seg + 1) % n;
        }
    }
    for i in 0..n {
        if let (Some(a), Some(b)) = (owner[i], owner[(i + 1) % n])
            && a != b
        {
            return Err(format!(
                "two gaps meet at a control point ({} in travel order); merge them",
                (i + 1) % n
            ));
        }
    }
    if owner[0].is_some() || owner[n - 1].is_some() {
        return Err("the start point is a gap lip, landing or inside a gap".into());
    }
    Ok(owner)
}

/// Builds the resampled centreline of a layout's points (index 0 = start) and active gaps.
pub(crate) fn centreline(points: &[Point], gaps: &[(usize, usize)]) -> Result<Centreline, String> {
    let n = points.len();
    let gap_of = gap_segments(n, gaps)?;
    let pos = |i: usize| points[i % n].pos;
    // Dense polyline: (position, segment, local parameter).
    let mut dense: Vec<(Vec3, usize, f32)> = Vec::new();
    let mut seg_start = Vec::with_capacity(n);
    for i in 0..n {
        seg_start.push(dense.len());
        let (p1, p2) = (pos(i), pos(i + 1));
        let steps = ((p1.distance(p2) / DENSE_STEP_U).ceil() as usize).max(8);
        let ctrl = if gap_of[i].is_some() {
            None
        } else {
            let p0 = if gap_of[(i + n - 1) % n].is_some() {
                2.0 * p1 - p2
            } else {
                pos(i + n - 1)
            };
            let p3 = if gap_of[(i + 1) % n].is_some() {
                2.0 * p2 - p1
            } else {
                pos(i + 2)
            };
            Some([p0, p1, p2, p3])
        };
        for k in 0..steps {
            let u = k as f32 / steps as f32;
            let p = ctrl.map_or_else(|| p1.lerp(p2, u), |c| catmull_rom(c, u));
            dense.push((p, i, u));
        }
    }
    dense.push((pos(0), n - 1, 1.0));
    let mut cum = vec![0.0f32; dense.len()];
    for j in 1..dense.len() {
        cum[j] = cum[j - 1] + dense[j].0.distance(dense[j - 1].0);
    }
    let length = cum[dense.len() - 1];
    let count = ((length / SAMPLE_SPACING_U).round() as usize).max(16);
    let ds = length / count as f32;

    let make = |s: f32, p: Vec3, tangent: Vec3, seg: usize, u: f32| -> Result<Sample, String> {
        let a = points[seg];
        let b = points[(seg + 1) % n];
        let w = smoothstep(u.clamp(0.0, 1.0));
        let bank = a.bank + (b.bank - a.bank) * w;
        let forward = tangent.normalize();
        let (right, up) = frame(forward, bank)
            .filter(|(_, up)| up.y >= MIN_UP_Y)
            .ok_or_else(|| {
                format!(
                    "s = {s:.1} u: the track tilts past vertical (no loops; check slopes and banks)"
                )
            })?;
        Ok(Sample {
            s,
            pos: p,
            forward,
            right,
            up,
            half_width: 0.5 * (a.width + (b.width - a.width) * w),
            bank,
            seg,
            in_gap: gap_of[seg].is_some(),
        })
    };

    let mut samples = Vec::with_capacity(count);
    let mut j = 0;
    for k in 0..count {
        let s = k as f32 * ds;
        while j + 2 < dense.len() && cum[j + 1] <= s {
            j += 1;
        }
        let (p0, seg, u0) = dense[j];
        let (p1, seg1, u1) = dense[j + 1];
        let f = ((s - cum[j]) / (cum[j + 1] - cum[j]).max(1e-6)).clamp(0.0, 1.0);
        let u1 = if seg1 == seg { u1 } else { 1.0 };
        samples.push(make(s, p0.lerp(p1, f), p1 - p0, seg, u0 + (u1 - u0) * f)?);
    }
    let mut at_points = Vec::with_capacity(n);
    for (i, &j) in seg_start.iter().enumerate() {
        let before = if j == 0 { dense.len() - 2 } else { j - 1 };
        let arriving_seg = (i + n - 1) % n;
        let arriving = make(
            cum[j],
            dense[j].0,
            dense[j].0 - dense[before].0,
            arriving_seg,
            1.0,
        )?;
        let leaving = make(cum[j], dense[j].0, dense[j + 1].0 - dense[j].0, i, 0.0)?;
        at_points.push((arriving, leaving));
    }
    Ok(Centreline {
        samples,
        ds,
        length,
        at_points,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rc_core::SurfaceKind;
    use std::f32::consts::TAU;

    pub(crate) fn point(pos: Vec3) -> Point {
        Point {
            pos,
            width: 10.0,
            bank: 0.0,
            surface: SurfaceKind::ToyTrack,
            wall_left: false,
            wall_right: false,
            margin: 6.0,
        }
    }

    fn circle(r: f32, n: usize, climb: f32) -> Vec<Point> {
        (0..n)
            .map(|i| {
                let a = i as f32 / n as f32 * TAU;
                point(Vec3::new(r * a.cos(), climb * i as f32, -r * a.sin()))
            })
            .collect()
    }

    #[test]
    fn circle_length_and_frames() {
        let line = centreline(&circle(30.0, 12, 0.0), &[]).expect("builds");
        assert!(
            (line.length - TAU * 30.0).abs() < 0.5,
            "length {}",
            line.length
        );
        assert!((line.ds - SAMPLE_SPACING_U).abs() < 0.01);
        for s in &line.samples {
            assert!((s.pos.length() - 30.0).abs() < 0.2, "on the circle");
            assert!(s.up.abs_diff_eq(Vec3::Y, 1e-4));
            assert!((s.right.dot(s.forward)).abs() < 1e-4);
            assert!(s.forward.cross(s.up).abs_diff_eq(s.right, 1e-4));
        }
    }

    #[test]
    fn loop_closes_smoothly() {
        let line = centreline(&circle(30.0, 8, 0.0), &[]).expect("builds");
        let first = line.samples[0];
        let last = line.samples[line.samples.len() - 1];
        assert!(
            (first.pos.distance(last.pos) - line.ds).abs() < 0.01,
            "seam spacing"
        );
        assert!(
            first.forward.dot(last.forward) > 0.99,
            "no kink at the seam"
        );
    }

    #[test]
    fn positive_bank_raises_the_left_edge() {
        let mut points = circle(30.0, 8, 0.0);
        points.iter_mut().for_each(|p| p.bank = 20f32.to_radians());
        let line = centreline(&points, &[]).expect("builds");
        let s = line.samples[10];
        let left = s.pos - s.right * 5.0;
        let right = s.pos + s.right * 5.0;
        assert!(left.y > right.y + 3.0);
    }

    #[test]
    fn gaps_are_straight_with_one_sided_tangents() {
        // Straight ramp up to a lip at point 1, flight to a lower landing at point 2.
        let pts = [
            (0.0, 0.0, 0.0),
            (0.0, 2.0, -14.0),
            (0.0, 0.5, -24.0),
            (40.0, 0.0, -24.0),
            (40.0, 0.0, 10.0),
        ];
        let points: Vec<Point> = pts
            .iter()
            .map(|&(x, y, z)| point(Vec3::new(x, y, z)))
            .collect();
        let line = centreline(&points, &[(1, 2)]).expect("builds");
        let (arriving, leaving) = line.at_points[1];
        let ramp = (points[1].pos - points[0].pos).normalize();
        assert!(
            arriving.forward.dot(ramp) > 0.999,
            "takeoff follows the ramp slope"
        );
        let flight = (points[2].pos - points[1].pos).normalize();
        assert!(
            leaving.forward.dot(flight) > 0.999,
            "gap is a straight line"
        );
        assert!(line.samples.iter().any(|s| s.in_gap));
        assert!(
            centreline(&points, &[(0, 1)]).is_err(),
            "start point on a lip"
        );
        assert!(
            centreline(&points, &[(1, 2), (2, 3)]).is_err(),
            "touching gaps"
        );
    }

    #[test]
    fn helix_frames_stay_upright() {
        let line = centreline(&circle(16.0, 8, 0.6), &[]);
        // The last segment drops back down 4.2 u: steep but not a loop.
        let line = line.expect("builds");
        assert!(line.samples.iter().all(|s| s.up.y > 0.9));
    }

    #[test]
    fn loops_are_rejected() {
        let pts = [
            (0.0, 0.0, 0.0),
            (0.0, 0.0, -20.0),
            (0.0, 10.0, -30.0),
            (0.0, 20.0, -20.0),
            (0.0, 10.0, -10.0),
            (20.0, 0.0, 0.0),
        ];
        let points: Vec<Point> = pts
            .iter()
            .map(|&(x, y, z)| point(Vec3::new(x, y, z)))
            .collect();
        assert!(centreline(&points, &[]).unwrap_err().contains("no loops"));
    }
}
