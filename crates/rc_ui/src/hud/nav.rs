//! Navigation aids of the HUD (pure): the minimap's fit and drawing data (centreline, racing
//! edges, checkpoint gates, the heading-up zoom), the corner the turn indicator shows, and the
//! angles of the heading-up map and of the arrows (degrees clockwise from straight up).

use bevy::prelude::*;
use rc_core::{TrackSummary, TrackTurn};

/// Most points sent per minimap line.
const OUTLINE_POINTS: usize = 240;
/// Minimap margin inside its 0–1 square.
pub(super) const MAP_PAD: f32 = 0.06;
/// A player's heading-up minimap shows about this much track across (u); smaller tracks whole.
const MAP_VIEW_U: f32 = 150.0;
/// The turn indicator shows a corner this long before the car reaches it (s)…
const TURN_LOOKAHEAD_S: f32 = 2.5;
/// …but never closer than this…
const TURN_LOOKAHEAD_MIN_U: f32 = 30.0;
/// …nor farther than this (u).
const TURN_LOOKAHEAD_MAX_U: f32 = 90.0;

/// Maps world X/Z onto the minimap's 0–1 square: aspect kept, centred, padded by [`MAP_PAD`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MinimapFit {
    origin: Vec2,
    scale: f32,
    offset: Vec2,
}

impl MinimapFit {
    /// The fit of a set of world X/Z points.
    pub fn new(points: &[Vec2]) -> Self {
        let Some(&first) = points.first() else {
            return Self {
                origin: Vec2::ZERO,
                scale: 0.0,
                offset: Vec2::splat(0.5),
            };
        };
        let (min, max) = points
            .iter()
            .fold((first, first), |(lo, hi), &p| (lo.min(p), hi.max(p)));
        let scale = (1.0 - 2.0 * MAP_PAD) / (max - min).max_element().max(1e-3);
        Self {
            origin: min,
            scale,
            offset: (Vec2::ONE - (max - min) * scale) / 2.0,
        }
    }

    /// The fit of a track: its centreline and both racing edges.
    pub fn of(summary: &TrackSummary) -> Self {
        let edges = summary.edges_2d.iter().flatten();
        let points: Vec<Vec2> = summary.outline_2d.iter().chain(edges).copied().collect();
        Self::new(&points)
    }

    /// World X/Z → minimap 0–1.
    pub fn project(&self, p: Vec2) -> Vec2 {
        (p - self.origin) * self.scale + self.offset
    }

    /// How much a player's heading-up minimap magnifies the whole-track fit (≥ 1).
    pub fn zoom(&self) -> f32 {
        if self.scale <= 0.0 {
            return 1.0;
        }
        let extent = (1.0 - 2.0 * MAP_PAD) / self.scale;
        (extent / MAP_VIEW_U).max(1.0)
    }
}

/// What the minimap draws, as flat x, y pairs in its 0–1 square: the centreline, the left and
/// right racing edges (same points as the centreline) and each checkpoint gate (x1, y1, x2, y2).
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct MapLines {
    pub outline: Vec<f32>,
    pub left: Vec<f32>,
    pub right: Vec<f32>,
    pub gates: Vec<f32>,
}

fn round(v: f32) -> f32 {
    (v * 10_000.0).round() / 10_000.0
}

impl MapLines {
    /// The lines of `summary` (at most [`OUTLINE_POINTS`] points each).
    pub(super) fn of(summary: &TrackSummary) -> Self {
        let fit = MinimapFit::of(summary);
        let step = summary.outline_2d.len().div_ceil(OUTLINE_POINTS).max(1);
        let flat = |pts: &mut dyn Iterator<Item = Vec2>| -> Vec<f32> {
            pts.step_by(step)
                .flat_map(|p| {
                    let q = fit.project(p);
                    [round(q.x), round(q.y)]
                })
                .collect()
        };
        let side = |i: usize| -> Vec<f32> {
            match summary.edges_2d.len() == summary.outline_2d.len() {
                true => flat(&mut summary.edges_2d.iter().map(|e| e[i])),
                false => Vec::new(),
            }
        };
        Self {
            outline: flat(&mut summary.outline_2d.iter().copied()),
            left: side(0),
            right: side(1),
            gates: summary
                .gates_2d
                .iter()
                .flat_map(|g| {
                    let [a, b] = g.map(|p| fit.project(p));
                    [a.x, a.y, b.x, b.y].map(round)
                })
                .collect(),
        }
    }
}

/// The corner the turn indicator shows for a car at arc length `s` (any value; wrapped) moving
/// at `speed` u/s: the one it is in, else the nearest one starting within the look-ahead.
pub(super) fn turn_ahead(
    turns: &[TrackTurn],
    length: f32,
    s: f32,
    speed: f32,
) -> Option<&TrackTurn> {
    if length <= 0.0 {
        return None;
    }
    let reach = (speed * TURN_LOOKAHEAD_S).clamp(TURN_LOOKAHEAD_MIN_U, TURN_LOOKAHEAD_MAX_U);
    turns
        .iter()
        .filter_map(|t| {
            let to_start = (t.start_s - s).rem_euclid(length);
            let inside = length - to_start < t.span || to_start == 0.0;
            match (inside, to_start <= reach) {
                (true, _) => Some((0.0, t)),
                (false, true) => Some((to_start, t)),
                _ => None,
            }
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, t)| t)
}

/// Bearing of a world X/Z direction on a north-up map (degrees clockwise from up, i.e. from −Z).
pub(super) fn bearing(dir: Vec2) -> f32 {
    dir.x.atan2(-dir.y).to_degrees()
}

/// Rotation of a heading-up map (degrees clockwise) that turns `heading` (world X/Z) up.
pub(super) fn map_rotation(heading: Vec2) -> f32 {
    -bearing(heading)
}

/// Bearing of world point `to` seen from a car at `pose` (degrees clockwise from straight ahead).
pub(super) fn bearing_from(pose: &Transform, to: Vec3) -> f32 {
    let local = pose.rotation.inverse() * (to - pose.translation);
    local.x.atan2(-local.z).to_degrees()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn turn(start_s: f32, span: f32) -> TrackTurn {
        TrackTurn {
            start_s,
            span,
            right: true,
            angle: 1.0,
            severity: 2,
        }
    }

    #[test]
    fn the_minimap_keeps_the_aspect_and_centres_the_track() {
        let fit = MinimapFit::new(&[
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 5.0),
            Vec2::new(0.0, 5.0),
        ]);
        let near = |a: Vec2, b: Vec2| a.distance(b) < 1e-5;
        assert!(near(fit.project(Vec2::new(5.0, 2.5)), Vec2::splat(0.5)));
        assert!(near(fit.project(Vec2::ZERO), Vec2::new(MAP_PAD, 0.28)));
        assert!(near(
            fit.project(Vec2::new(10.0, 5.0)),
            Vec2::new(1.0 - MAP_PAD, 0.72)
        ));
        assert_eq!(
            MinimapFit::new(&[]).project(Vec2::splat(7.0)),
            Vec2::splat(0.5)
        );
        assert_eq!(fit.zoom(), 1.0, "a 10 u track shows whole");
        let big = MinimapFit::new(&[Vec2::ZERO, Vec2::new(300.0, 100.0)]);
        assert!(
            (big.zoom() - 2.0).abs() < 1e-4,
            "300 u across → 150 u in view"
        );
    }

    #[test]
    fn map_lines_fit_the_edges_and_keep_them_aligned() {
        let summary = TrackSummary {
            outline_2d: vec![Vec2::ZERO; 1000],
            edges_2d: vec![[Vec2::new(-5.0, 0.0), Vec2::new(5.0, 0.0)]; 1000],
            gates_2d: vec![[Vec2::new(-5.0, 0.0), Vec2::new(5.0, 0.0)]; 8],
            ..default()
        };
        let lines = MapLines::of(&summary);
        assert_eq!(lines.outline.len(), 2 * 200);
        assert_eq!(lines.left.len(), lines.outline.len());
        assert_eq!(lines.right.len(), lines.outline.len());
        assert_eq!(lines.gates.len(), 4 * 8);
        assert_eq!(&lines.left[..2], &[MAP_PAD, 0.5], "the edges set the fit");
        assert_eq!(&lines.right[..2], &[1.0 - MAP_PAD, 0.5]);
        let no_edges = MapLines::of(&TrackSummary {
            outline_2d: vec![Vec2::ZERO; 3],
            ..default()
        });
        assert!(no_edges.left.is_empty() && no_edges.outline.len() == 6);
    }

    #[test]
    fn the_turn_indicator_shows_the_corner_ahead_or_the_one_driven() {
        let turns = [turn(50.0, 20.0), turn(150.0, 30.0), turn(390.0, 20.0)];
        let at = |s: f32, speed: f32| turn_ahead(&turns, 400.0, s, speed).map(|t| t.start_s);
        assert_eq!(at(15.0, 0.0), None, "35 u away, look-ahead 30 u");
        assert_eq!(at(25.0, 0.0), Some(50.0));
        assert_eq!(at(15.0, 24.0), Some(50.0), "2.5 s at 24 u/s = 60 u");
        assert_eq!(at(60.0, 30.0), Some(50.0), "inside it");
        assert_eq!(at(76.0, 30.0), Some(150.0), "past it: the next within 75 u");
        assert_eq!(at(71.0, 10.0), None);
        assert_eq!(at(385.0, 10.0), Some(390.0));
        assert_eq!(
            at(5.0, 10.0),
            Some(390.0),
            "a corner running past the start line"
        );
        assert_eq!(at(-5.0, 10.0), Some(390.0), "behind the line on the grid");
        assert_eq!(turn_ahead(&[], 400.0, 0.0, 30.0), None);
    }

    #[test]
    fn bearings_are_clockwise_from_up() {
        assert!((bearing(Vec2::new(0.0, -1.0))).abs() < 1e-4, "north = up");
        assert!(
            (bearing(Vec2::new(1.0, 0.0)) - 90.0).abs() < 1e-4,
            "east = right"
        );
        assert!(
            (map_rotation(Vec2::new(1.0, 0.0)) + 90.0).abs() < 1e-4,
            "east turns up"
        );
        let car = Transform::from_xyz(0.0, 0.0, 0.0).looking_to(Vec3::X, Vec3::Y);
        assert!(
            (bearing_from(&car, Vec3::new(10.0, 0.0, 0.0))).abs() < 1e-3,
            "ahead"
        );
        assert!(
            (bearing_from(&car, Vec3::new(0.0, 0.0, 10.0)) - 90.0).abs() < 1e-3,
            "right of a car facing east is south"
        );
    }
}
