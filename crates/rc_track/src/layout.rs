//! A [`TrackDef`] resolved for one variant (pure): inherited point fields filled in, the start
//! point rotated to index 0, only the features active in the travel direction, reversed and/or
//! mirrored. Everything downstream builds from a `Layout` as if it were a forward layout.
//!
//! - **Reverse** keeps the start point and walks the loop backwards; per-segment fields move to
//!   the segment's other end, left and right swap and banks change sign (same physical track).
//! - **Mirror** negates X; left/right and bank signs swap again, yaws are mirrored (prop meshes
//!   are never mirrored), polygon winding flips (triangulation accepts both).

use bevy::math::{Vec2, Vec3};
use rc_core::SurfaceKind;

use crate::def::*;

/// A fully resolved control point. Segment fields (`surface`, walls, `margin`) apply from this
/// point to the next one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Point {
    pub pos: Vec3,
    pub width: f32,
    /// Radians; positive raises the left edge.
    pub bank: f32,
    pub surface: SurfaceKind,
    pub wall_left: bool,
    pub wall_right: bool,
    pub margin: f32,
}

/// A placed prop of a layout (yaw in radians).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LayoutProp {
    pub kind: String,
    pub pos: Vec3,
    pub yaw: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Layout {
    pub style: RibbonStyle,
    /// Index 0 is the start point.
    pub points: Vec<Point>,
    /// Active gaps as (lip, landing) point indices in travel order.
    pub gaps: Vec<(usize, usize)>,
    pub ground: Vec<(SurfaceKind, f32, Vec<Vec2>)>,
    /// (position, cube count).
    pub item_rows: Vec<(Vec3, u8)>,
    pub props: Vec<LayoutProp>,
    /// (path in travel order, width).
    pub shortcuts: Vec<(Vec<Vec3>, f32)>,
    pub golden_battery: Option<Vec3>,
    pub kill_height: f32,
    pub wall_height: f32,
    /// (min, max) in (x, z).
    pub bounds: Option<(Vec2, Vec2)>,
}

impl Layout {
    /// Resolves `def` (already validated) for a variant.
    pub fn new(def: &TrackDef, reverse: bool, mirror: bool) -> Self {
        let n = def.points.len();
        let resolved = resolve_points(def);
        // Rotate so the start point is index 0.
        let points: Vec<Point> = (0..n).map(|k| resolved[(def.start + k) % n]).collect();
        let shift = |i: usize| (i + n - def.start) % n;
        let mut layout = Self {
            style: def.style,
            points,
            gaps: def
                .gaps
                .iter()
                .filter(|g| g.dir.active(reverse))
                .map(|g| (shift(g.from), shift(g.to)))
                .collect(),
            ground: def
                .ground
                .iter()
                .filter(|g| g.dir.active(reverse))
                .map(|g| (g.surface, g.height, g.points.clone()))
                .collect(),
            item_rows: def
                .item_rows
                .iter()
                .filter(|r| r.dir.active(reverse))
                .map(|r| (r.pos, r.count))
                .collect(),
            props: def
                .props
                .iter()
                .filter(|p| p.dir.active(reverse))
                .map(|p| LayoutProp {
                    kind: p.kind.clone(),
                    pos: p.pos,
                    yaw: p.yaw.to_radians(),
                })
                .collect(),
            shortcuts: def
                .shortcuts
                .iter()
                .filter(|s| s.dir.active(reverse))
                .map(|s| (s.path.clone(), s.width))
                .collect(),
            golden_battery: def
                .golden_battery
                .filter(|b| b.dir.active(reverse))
                .map(|b| b.pos),
            kill_height: def.kill_height,
            wall_height: def.wall_height,
            bounds: def.bounds.map(|b| (b.min, b.max)),
        };
        if reverse {
            layout.reverse();
        }
        if mirror {
            layout.mirror();
        }
        layout
    }

    /// Same loop driven backwards from the same start point.
    fn reverse(&mut self) {
        let n = self.points.len();
        let old = std::mem::take(&mut self.points);
        self.points = (0..n)
            .map(|k| {
                let at = old[(n - k) % n];
                // The segment new[k] → new[k+1] is the old segment ending at `at`.
                let seg = old[(2 * n - k - 1) % n];
                Point {
                    pos: at.pos,
                    width: at.width,
                    bank: -at.bank,
                    surface: seg.surface,
                    wall_left: seg.wall_right,
                    wall_right: seg.wall_left,
                    margin: seg.margin,
                }
            })
            .collect();
        for gap in &mut self.gaps {
            *gap = ((n - gap.1) % n, (n - gap.0) % n);
        }
        for (path, _) in &mut self.shortcuts {
            path.reverse();
        }
    }

    /// Negates X of everything.
    fn mirror(&mut self) {
        let flip3 = |v: &mut Vec3| v.x = -v.x;
        for p in &mut self.points {
            flip3(&mut p.pos);
            p.bank = -p.bank;
            std::mem::swap(&mut p.wall_left, &mut p.wall_right);
        }
        for (_, _, polygon) in &mut self.ground {
            for v in polygon {
                v.x = -v.x;
            }
        }
        for (pos, _) in &mut self.item_rows {
            flip3(pos);
        }
        for prop in &mut self.props {
            flip3(&mut prop.pos);
            prop.yaw = -prop.yaw;
        }
        for (path, _) in &mut self.shortcuts {
            path.iter_mut().for_each(flip3);
        }
        if let Some(b) = &mut self.golden_battery {
            flip3(b);
        }
        if let Some((min, max)) = &mut self.bounds {
            (min.x, max.x) = (-max.x, -min.x);
        }
    }
}

/// Fills inherited fields in file order.
fn resolve_points(def: &TrackDef) -> Vec<Point> {
    let mut width = DEFAULT_WIDTH_U;
    let mut surface = def.style.default_surface();
    let (mut wall_left, mut wall_right) = (false, false);
    let mut margin = DEFAULT_OFFROAD_MARGIN_U;
    def.points
        .iter()
        .map(|p| {
            width = p.width.unwrap_or(width);
            surface = p.surface.unwrap_or(surface);
            wall_left = p.wall_left.unwrap_or(wall_left);
            wall_right = p.wall_right.unwrap_or(wall_right);
            margin = p.offroad_margin.unwrap_or(margin);
            Point {
                pos: p.pos,
                width,
                bank: p.bank_deg.to_radians(),
                surface,
                wall_left,
                wall_right,
                margin,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::default;

    fn def() -> TrackDef {
        let pos = [(0.0, 0.0), (40.0, 0.0), (40.0, -40.0), (0.0, -40.0)];
        let mut def = TrackDef {
            points: pos
                .iter()
                .map(|&(x, z)| ControlPoint {
                    pos: Vec3::new(x, 0.3, z),
                    ..default()
                })
                .collect(),
            start: 1,
            kill_height: -10.0,
            wall_height: 1.0,
            ..default()
        };
        def.points[1].wall_left = Some(true);
        def.points[2].wall_left = Some(false);
        def.points[2].surface = Some(SurfaceKind::Wood);
        def.points[3].surface = Some(SurfaceKind::ToyTrack);
        def.points[2].bank_deg = 20.0;
        def.points[2].width = Some(14.0);
        def.gaps = vec![
            Gap {
                from: 2,
                to: 3,
                dir: Dir::Fwd,
            },
            Gap {
                from: 3,
                to: 0,
                dir: Dir::Rev,
            },
        ];
        def.props = vec![PropPlacement {
            kind: "box".into(),
            pos: Vec3::new(5.0, 0.0, 1.0),
            yaw: 30.0,
            dir: Dir::Both,
        }];
        def
    }

    #[test]
    fn inheritance_and_start_rotation() {
        let l = Layout::new(&def(), false, false);
        // File points 1, 2, 3, 0 in that order.
        assert_eq!(l.points[0].pos.x, 40.0);
        assert!(l.points[0].wall_left && !l.points[1].wall_left);
        assert_eq!(l.points[1].width, 14.0);
        assert_eq!(l.points[2].width, 14.0, "width is inherited");
        assert_eq!(l.points[3].width, 10.0, "file point 0 keeps the default");
        assert_eq!(l.points[3].surface, SurfaceKind::ToyTrack, "inherited");
        assert_eq!(
            l.gaps,
            vec![(1, 2)],
            "only the Fwd gap, indices from the start"
        );
    }

    #[test]
    fn reverse_is_the_same_track_backwards() {
        let fwd = Layout::new(&def(), false, false);
        let rev = Layout::new(&def(), true, false);
        let n = fwd.points.len();
        assert_eq!(rev.points[0].pos, fwd.points[0].pos, "same start point");
        for k in 0..n {
            let f = fwd.points[k];
            let r = rev.points[(n - k) % n];
            assert_eq!(r.pos, f.pos);
            assert_eq!(r.bank, -f.bank);
            // Forward segment k → k+1 is reverse segment (n-k-1) → (n-k).
            let rs = rev.points[(2 * n - k - 1) % n];
            assert_eq!(rs.surface, f.surface);
            assert_eq!((rs.wall_left, rs.wall_right), (f.wall_right, f.wall_left));
        }
        // The Rev gap (file 3 → 0 = forward 2 → 3) becomes (lip 1, landing 2) in reverse.
        assert_eq!(rev.gaps, vec![(1, 2)]);
        let twice = {
            let mut l = rev.clone();
            l.reverse();
            l
        };
        assert_eq!(twice.points, fwd.points, "reversing twice is the identity");
    }

    #[test]
    fn mirror_negates_x_and_swaps_sides() {
        let fwd = Layout::new(&def(), false, false);
        let mir = Layout::new(&def(), false, true);
        for (f, m) in fwd.points.iter().zip(&mir.points) {
            assert_eq!(m.pos, Vec3::new(-f.pos.x, f.pos.y, f.pos.z));
            assert_eq!(m.bank, -f.bank);
            assert_eq!((m.wall_left, m.wall_right), (f.wall_right, f.wall_left));
        }
        assert_eq!(mir.props[0].pos.x, -5.0);
        assert!((mir.props[0].yaw + 30f32.to_radians()).abs() < 1e-6);
    }
}
