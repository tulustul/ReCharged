//! The [`Track`] session resource: everything built from a layout plus the query API used by
//! race rules, AI, items, cameras and graphics. Built by [`Track::build`] (see `build.rs`).

use std::collections::BTreeMap;

use bevy::math::{Mat3, Quat, Vec2, Vec3};
use bevy::prelude::{Resource, Transform};
use rc_core::{SurfaceKind, TrackSummary, TrackVariant};

use crate::def::RibbonStyle;
use crate::geometry::TrackGeometry;
use crate::props::PropDef;

/// Half-window of a hinted projection search (u). Cars move < 1 u per tick.
pub const PROJECT_WINDOW_U: f32 = 30.0;
/// A point counts as on a drivable surface from this far below it…
pub const DRIVABLE_BELOW_U: f32 = 1.0;
/// …to this far above it (u): wheels, chassis and small hops.
pub const DRIVABLE_ABOVE_U: f32 = 3.0;

/// A banked frame on the centreline. `right = forward × up`; `forward` points along travel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrackFrame {
    /// Arc length from the start line (u), in `[0, length)`.
    pub s: f32,
    /// Centreline point on the driving surface (u).
    pub position: Vec3,
    /// Travel direction (unit).
    pub forward: Vec3,
    /// Right across the (banked) surface (unit).
    pub right: Vec3,
    /// Surface normal (unit).
    pub up: Vec3,
    /// Half the racing width (u).
    pub half_width: f32,
    /// Bank (radians; positive raises the left edge).
    pub bank: f32,
    /// Surface under the centreline (for Invisible ribbons: the ground polygon there).
    pub surface: SurfaceKind,
    /// Off-track tolerance beyond the edge (u, GDD §5.4).
    pub offroad_margin: f32,
    /// Inside a jump's gap (no ribbon below).
    pub in_gap: bool,
}

impl TrackFrame {
    /// Rotation whose −Z is `forward` and +Y is `up`.
    pub fn rotation(&self) -> Quat {
        Quat::from_mat3(&Mat3::from_cols(self.right, self.up, -self.forward))
    }

    /// Pose at the centreline facing along the track.
    pub fn transform(&self) -> Transform {
        Transform::from_translation(self.position).with_rotation(self.rotation())
    }

    /// The point `lateral` u to the right and `height` u above the centreline.
    pub fn point(&self, lateral: f32, height: f32) -> Vec3 {
        self.position + self.right * lateral + self.up * height
    }
}

/// A world position expressed in track coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrackPos {
    /// Arc length of the closest centreline point (u), in `[0, length)`.
    pub s: f32,
    /// Offset to the right of the centreline along the banked surface (u).
    pub lateral: f32,
    /// Offset above the surface along its normal (u).
    pub height: f32,
}

/// A respawn point (GDD §5.4): on the centreline every ≈ 15 u, never in or just before a gap.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RespawnPoint {
    /// Arc length (u).
    pub s: f32,
    /// Car pose: on the surface + a small drop, facing forward.
    pub transform: Transform,
}

/// A row of Volt Cubes across the track.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemRow {
    /// Arc length of the row (u).
    pub s: f32,
    /// Cube centres, left to right (u).
    pub slots: Vec<Vec3>,
}

/// A prop instance of this layout.
#[derive(Debug, Clone, PartialEq)]
pub struct PlacedProp {
    /// Kind id from `props/*.ron` (see [`Track::prop_def`]).
    pub kind: String,
    /// Spawn pose (ground-contact centre).
    pub transform: Transform,
}

/// A flat drivable polygon.
#[derive(Debug, Clone, PartialEq)]
pub struct GroundArea {
    /// Surface.
    pub surface: SurfaceKind,
    /// Surface height (u).
    pub height: f32,
    /// Outline (x, z).
    pub polygon: Vec<Vec2>,
    pub(crate) min: Vec2,
    pub(crate) max: Vec2,
}

impl GroundArea {
    pub(crate) fn new(surface: SurfaceKind, height: f32, polygon: Vec<Vec2>) -> Self {
        let min = polygon.iter().copied().fold(Vec2::MAX, Vec2::min);
        let max = polygon.iter().copied().fold(Vec2::MIN, Vec2::max);
        Self {
            surface,
            height,
            polygon,
            min,
            max,
        }
    }

    /// `(x, z)` lies inside the outline.
    pub fn contains_xz(&self, p: Vec2) -> bool {
        if p.cmplt(self.min).any() || p.cmpgt(self.max).any() {
            return false;
        }
        let n = self.polygon.len();
        let mut inside = false;
        for i in 0..n {
            let (a, b) = (self.polygon[i], self.polygon[(i + 1) % n]);
            if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x) {
                inside = !inside;
            }
        }
        inside
    }

    /// `pos` is over the area within the drivable height window.
    pub fn contains(&self, pos: Vec3) -> bool {
        let h = pos.y - self.height;
        (-DRIVABLE_BELOW_U..=DRIVABLE_ABOVE_U).contains(&h)
            && self.contains_xz(Vec2::new(pos.x, pos.z))
    }
}

/// An alternative path (for AI and checkpoint placement).
#[derive(Debug, Clone, PartialEq)]
pub struct Shortcut {
    /// Path in travel order (u).
    pub path: Vec<Vec3>,
    /// Usable width (u).
    pub width: f32,
    /// Arc length where it leaves the main line.
    pub entry_s: f32,
    /// Arc length where it rejoins.
    pub exit_s: f32,
}

/// A jump of this layout, flown at the reference target speed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Jump {
    /// Arc length of the lip.
    pub lip_s: f32,
    /// Arc length where the landing ribbon starts.
    pub landing_s: f32,
    /// Lip position (u).
    pub takeoff_position: Vec3,
    /// Travel direction at the lip (the ramp's slope, unit).
    pub takeoff_direction: Vec3,
    /// Target speed at the lip (u/s).
    pub takeoff_speed: f32,
    /// Where the ballistic arc touches down (`None`: it never reaches the landing).
    pub touchdown_s: Option<f32>,
    /// Flight time to touchdown (s).
    pub air_time: f32,
}

/// Per-sample data (every `ds` u of arc length).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Node {
    pub frame: TrackFrame,
    pub walls: (bool, bool),
    /// Racing line lateral offset (u).
    pub line: f32,
    /// Target speed (u/s).
    pub speed: f32,
}

/// The session's track (resource; built in `OnEnter(AppState::Loading)`, removed in
/// `OnEnter(AppState::Unloading)`). Arc lengths `s` run from the start line in the travel
/// direction of the variant and wrap at [`Track::length`].
#[derive(Resource, Debug, Clone)]
pub struct Track {
    pub(crate) variant: TrackVariant,
    pub(crate) style: RibbonStyle,
    pub(crate) nodes: Vec<Node>,
    pub(crate) ds: f32,
    pub(crate) length: f32,
    pub(crate) checkpoints: Vec<TrackFrame>,
    pub(crate) respawns: Vec<RespawnPoint>,
    pub(crate) grid: Vec<Transform>,
    pub(crate) item_rows: Vec<ItemRow>,
    pub(crate) props: Vec<PlacedProp>,
    pub(crate) prop_defs: BTreeMap<String, PropDef>,
    pub(crate) ground: Vec<GroundArea>,
    pub(crate) shortcuts: Vec<Shortcut>,
    pub(crate) jumps: Vec<Jump>,
    pub(crate) golden_battery: Option<Vec3>,
    pub(crate) kill_height: f32,
    pub(crate) wall_height: f32,
    pub(crate) bounds: (Vec3, Vec3),
    pub(crate) geometry: TrackGeometry,
    pub(crate) geometry_hash: u64,
    pub(crate) lints: Vec<String>,
}

impl Track {
    /// Lap length (u).
    pub fn length(&self) -> f32 {
        self.length
    }

    /// `s` wrapped into `[0, length)`.
    pub fn wrap(&self, s: f32) -> f32 {
        let w = s.rem_euclid(self.length);
        if w >= self.length { 0.0 } else { w }
    }

    /// Shortest signed arc distance from `from` to `to` (positive = `to` is ahead), across the
    /// start line.
    pub fn delta_s(&self, from: f32, to: f32) -> f32 {
        let d = (to - from).rem_euclid(self.length);
        if d > 0.5 * self.length {
            d - self.length
        } else {
            d
        }
    }

    fn lerp_nodes(&self, s: f32) -> (&Node, &Node, f32) {
        let x = self.wrap(s) / self.ds;
        let i = (x.floor() as usize).min(self.nodes.len() - 1);
        (
            &self.nodes[i],
            &self.nodes[(i + 1) % self.nodes.len()],
            x - i as f32,
        )
    }

    /// The centreline frame at `s` (any value; wrapped), interpolated between samples.
    pub fn frame_at(&self, s: f32) -> TrackFrame {
        let (a, b, t) = self.lerp_nodes(s);
        let (fa, fb) = (a.frame, b.frame);
        let forward = fa.forward.lerp(fb.forward, t).normalize();
        let up = fa.up.lerp(fb.up, t);
        let right = forward.cross(up).normalize();
        TrackFrame {
            s: self.wrap(s),
            position: fa.position.lerp(fb.position, t),
            forward,
            right,
            up: right.cross(forward),
            half_width: fa.half_width + (fb.half_width - fa.half_width) * t,
            bank: fa.bank + (fb.bank - fa.bank) * t,
            ..fa
        }
    }

    /// Track coordinates of `pos`. With `hint` (the caller's previous `s`) only the centreline
    /// within ±[`PROJECT_WINDOW_U`] of it is searched — keeps progress continuous on helices,
    /// crossings and across the start line; pass `None` after teleports (global, nearest in
    /// 3D).
    pub fn project(&self, pos: Vec3, hint: Option<f32>) -> TrackPos {
        let n = self.nodes.len() as isize;
        let (first, count) = match hint {
            Some(h) => {
                let w = (PROJECT_WINDOW_U / self.ds) as isize;
                ((self.wrap(h) / self.ds) as isize - w, 2 * w + 1)
            }
            None => (0, n),
        };
        let dist = |i: usize| self.nodes[i].frame.position.distance_squared(pos);
        let best = (first..first + count.min(n))
            .map(|k| k.rem_euclid(n) as usize)
            .min_by(|&a, &b| dist(a).total_cmp(&dist(b)))
            .unwrap_or(0);
        // Refine on the segments before and after the closest sample.
        let refine = |i: usize| {
            let a = self.nodes[i].frame.position;
            let b = self.nodes[(i + 1) % self.nodes.len()].frame.position;
            let t = ((pos - a).dot(b - a) / (b - a).length_squared().max(1e-9)).clamp(0.0, 1.0);
            ((i as f32 + t) * self.ds, a.lerp(b, t).distance_squared(pos))
        };
        let prev = (best + self.nodes.len() - 1) % self.nodes.len();
        let (s1, d1) = refine(best);
        let (s0, d0) = refine(prev);
        let frame = self.frame_at(if d0 < d1 { s0 } else { s1 });
        let d = pos - frame.position;
        TrackPos {
            s: frame.s,
            lateral: d.dot(frame.right),
            height: d.dot(frame.up),
        }
    }

    /// Point of the racing line at centreline arc length `s`.
    pub fn racing_line_at(&self, s: f32) -> Vec3 {
        let (a, b, t) = self.lerp_nodes(s);
        let pa = a.frame.point(a.line, 0.0);
        pa.lerp(b.frame.point(b.line, 0.0), t)
    }

    /// Lateral offset of the racing line at `s` (u, + = right).
    pub fn racing_line_offset_at(&self, s: f32) -> f32 {
        let (a, b, t) = self.lerp_nodes(s);
        a.line + (b.line - a.line) * t
    }

    /// Target speed of the reference car ([`crate::SpeedModel::REFERENCE`]) at `s` (u/s).
    pub fn target_speed_at(&self, s: f32) -> f32 {
        let (a, b, t) = self.lerp_nodes(s);
        a.speed + (b.speed - a.speed) * t
    }

    /// Barriers at `s`: (left, right).
    pub fn walls_at(&self, s: f32) -> (bool, bool) {
        self.lerp_nodes(s).0.walls
    }

    /// Index of the checkpoint segment containing `s`: the last checkpoint at or before it.
    pub fn checkpoint_of(&self, s: f32) -> usize {
        let s = self.wrap(s);
        self.checkpoints
            .partition_point(|c| c.s <= s)
            .saturating_sub(1)
    }

    /// Where to respawn a car at progress `s` whose last validated checkpoint is
    /// `last_valid_checkpoint`: the last respawn point at or before `s`, but never past the
    /// next checkpoint still to validate (skipping a checkpoint by falling or cutting never
    /// gains ground).
    pub fn respawn_before(&self, s: f32, last_valid_checkpoint: usize) -> Transform {
        let last = last_valid_checkpoint % self.checkpoints.len();
        let lo = self.checkpoints[last].s;
        let hi = self.checkpoints.get(last + 1).map_or(self.length, |c| c.s);
        let ahead = (s - lo).rem_euclid(self.length);
        let span = hi - lo;
        let s = if ahead > span && ahead - span < self.length - ahead {
            hi - 0.01
        } else {
            s
        };
        let s = self.wrap(s);
        let k = self.respawns.partition_point(|r| r.s <= s);
        let i = if k == 0 {
            self.respawns.len() - 1
        } else {
            k - 1
        };
        self.respawns[i].transform
    }

    /// `pos` is on a drivable surface: over the ribbon (not Invisible, not in a gap) or a
    /// ground polygon, from [`DRIVABLE_BELOW_U`] below to [`DRIVABLE_ABOVE_U`] above it. Props
    /// are not counted (their colliders carry `Surface` for wheel rays).
    pub fn is_on_drivable(&self, pos: Vec3) -> bool {
        self.ground.iter().any(|g| g.contains(pos)) || self.on_ribbon(pos)
    }

    // ponytail: linear scan over all samples (~1–2 k), fine for a few calls per tick; add a
    // spatial grid if it shows up in profiles.
    fn on_ribbon(&self, pos: Vec3) -> bool {
        if self.style == RibbonStyle::Invisible {
            return false;
        }
        self.nodes.iter().any(|n| {
            let f = &n.frame;
            let d = pos - f.position;
            !f.in_gap
                && d.length_squared() <= (f.half_width + DRIVABLE_ABOVE_U).powi(2)
                && d.dot(f.forward).abs() <= self.ds
                && d.dot(f.right).abs() <= f.half_width
                && (-DRIVABLE_BELOW_U..=DRIVABLE_ABOVE_U).contains(&d.dot(f.up))
        })
    }

    /// Top-down centreline (world x, z) every ≈ 2 u, for minimaps.
    pub fn outline_2d(&self) -> Vec<Vec2> {
        let step = ((2.0 / self.ds).round() as usize).max(1);
        self.nodes
            .iter()
            .step_by(step)
            .map(|n| Vec2::new(n.frame.position.x, n.frame.position.z))
            .collect()
    }

    /// The facts rc_ui and rc_game need.
    pub fn summary(&self) -> TrackSummary {
        TrackSummary {
            outline_2d: self.outline_2d(),
            length: self.length,
            checkpoints: self.checkpoints.len(),
            geometry_hash: self.geometry_hash,
        }
    }

    /// Checkpoint gates in order (≥ 8; index 0 is the start/finish line at `s = 0`).
    pub fn checkpoints(&self) -> &[TrackFrame] {
        &self.checkpoints
    }

    /// Respawn points sorted by `s`.
    pub fn respawn_points(&self) -> &[RespawnPoint] {
        &self.respawns
    }

    /// Starting grid, pole first: 8 poses, 2 staggered columns behind the start line.
    pub fn grid(&self) -> &[Transform] {
        &self.grid
    }

    /// Volt Cube rows of this layout, sorted by `s`.
    pub fn item_rows(&self) -> &[ItemRow] {
        &self.item_rows
    }

    /// Props of this layout.
    pub fn props(&self) -> &[PlacedProp] {
        &self.props
    }

    /// Definition of a prop kind used by this layout.
    pub fn prop_def(&self, kind: &str) -> Option<&PropDef> {
        self.prop_defs.get(kind)
    }

    /// Ground polygons of this layout.
    pub fn ground(&self) -> &[GroundArea] {
        &self.ground
    }

    /// Shortcuts of this layout.
    pub fn shortcuts(&self) -> &[Shortcut] {
        &self.shortcuts
    }

    /// Jumps of this layout.
    pub fn jumps(&self) -> &[Jump] {
        &self.jumps
    }

    /// Golden Battery position (forward layouts only).
    pub fn golden_battery(&self) -> Option<Vec3> {
        self.golden_battery
    }

    /// Cars below this height are respawned (u).
    pub fn kill_height(&self) -> f32 {
        self.kill_height
    }

    /// Barrier height above the surface (u).
    pub fn wall_height(&self) -> f32 {
        self.wall_height
    }

    /// World box enclosed by the shell walls: (min, max), y from below the kill floor up.
    pub fn bounds(&self) -> (Vec3, Vec3) {
        self.bounds
    }

    /// Ribbon style of the main loop.
    pub fn style(&self) -> RibbonStyle {
        self.style
    }

    /// The variant this track was built for.
    pub fn variant(&self) -> &TrackVariant {
        &self.variant
    }

    /// Render-ready meshes.
    pub fn geometry(&self) -> &TrackGeometry {
        &self.geometry
    }

    /// Stable hash of the built geometry (records/ghosts of another hash are discarded).
    pub fn geometry_hash(&self) -> u64 {
        self.geometry_hash
    }

    /// Authoring-guideline warnings (slopes, banks, chokepoints, short jumps); empty for a
    /// layout ready for QA.
    pub fn lints(&self) -> &[String] {
        &self.lints
    }

    /// Distance between centreline samples (u).
    pub fn sample_spacing(&self) -> f32 {
        self.ds
    }
}
