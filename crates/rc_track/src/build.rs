//! Building a [`Track`] (pure): layout → centreline → checks → geometry (`geometry.rs`) →
//! placements (`place.rs`) → racing line, jumps, lints and hash (`analysis.rs`) → bounds.

use std::collections::BTreeMap;
use std::path::Path;

use bevy::math::{Vec2, Vec3};
use rc_core::{SurfaceKind, TrackVariant};

use crate::analysis::{geometry_hash, lints};
use crate::def::*;
use crate::geometry::geometry;
use crate::layout::Layout;
use crate::mesh::{ground_mesh, self_intersects, style_profile};
use crate::props::{MAX_DYNAMIC_PROPS, PropCatalog};
use crate::spline::{Centreline, Sample, centreline};
use crate::track::*;
use crate::turns::turns;

/// Barrier thickness (u; the spec asks for ≥ 0.5 so fast cars cannot tunnel).
pub const WALL_THICKNESS_U: f32 = 0.6;
/// Automatic bounds: content box plus this margin (u).
const BOUNDS_MARGIN_U: f32 = 15.0;
/// Headroom of the shell walls above the highest content (u).
const BOUNDS_HEADROOM_U: f32 = 30.0;
/// The kill floor's top lies this far below the kill height (u).
pub const KILL_FLOOR_DEPTH_U: f32 = 3.0;
/// The kill height must be at least this far below every drivable point (u).
const KILL_CLEARANCE_U: f32 = 2.0;
/// Shortest allowed lap (u).
const MIN_LENGTH_U: f32 = 50.0;
/// Turn radius must exceed the ribbon's reach on the inside by this much (u).
const TURN_CLEARANCE_U: f32 = 0.25;

impl Track {
    /// Reads `data/tracks/<id>.ron` and `data/props/*.ron` under `assets_dir`, builds `variant`.
    pub fn load(assets_dir: &Path, variant: &TrackVariant) -> Result<Self, String> {
        let def = TrackDef::load(assets_dir, variant.track.as_str())?;
        let props = PropCatalog::load(assets_dir)?;
        Self::build(&def, &props, variant).map_err(|e| {
            format!(
                "{TRACKS_DIR}/{}.ron ({}): {e}",
                variant.track,
                variant_name(variant)
            )
        })
    }

    /// Builds `variant` of `def` (whose `variant.track` id is only recorded).
    pub fn build(
        def: &TrackDef,
        catalog: &PropCatalog,
        variant: &TrackVariant,
    ) -> Result<Self, String> {
        def.validate()?;
        for (i, p) in def.props.iter().enumerate() {
            if catalog.get(&p.kind).is_none() {
                return Err(format!(
                    "props[{i}].kind: \"{}\" is not in {}/*.ron",
                    p.kind,
                    crate::PROPS_DIR
                ));
            }
        }
        let layout = Layout::new(def, variant.reverse, variant.mirror);
        let line = centreline(&layout.points, &layout.gaps)?;
        if line.length < MIN_LENGTH_U {
            return Err(format!(
                "lap is {:.0} u, need at least {MIN_LENGTH_U}",
                line.length
            ));
        }
        let ground = ground_areas(&layout)?;
        check_turns(&layout, &line)?;
        let nodes = line
            .samples
            .iter()
            .map(|s| node(&layout, &ground, s))
            .collect();
        let prop_defs: BTreeMap<_, _> = layout
            .props
            .iter()
            .filter_map(|p| catalog.get(&p.kind).map(|d| (p.kind.clone(), d.clone())))
            .collect();
        let dynamic = layout
            .props
            .iter()
            .filter(|p| prop_defs[&p.kind].dynamic)
            .count();
        if dynamic > MAX_DYNAMIC_PROPS {
            return Err(format!(
                "{dynamic} dynamic props, at most {MAX_DYNAMIC_PROPS}"
            ));
        }
        let mut track = Track {
            variant: variant.clone(),
            style: layout.style,
            nodes,
            ds: line.ds,
            length: line.length,
            checkpoints: Vec::new(),
            respawns: Vec::new(),
            grid: Vec::new(),
            item_rows: Vec::new(),
            props: Vec::new(),
            prop_defs,
            geometry: geometry(&layout, &line, &ground)?,
            ground,
            shortcuts: Vec::new(),
            jumps: Vec::new(),
            turns: Vec::new(),
            golden_battery: layout.golden_battery,
            kill_height: layout.kill_height,
            wall_height: layout.wall_height,
            bounds: (Vec3::ZERO, Vec3::ZERO),
            geometry_hash: 0,
            lints: Vec::new(),
        };
        let gaps: Vec<(f32, f32)> = layout
            .gaps
            .iter()
            .map(|&(a, b)| (line.at_points[a].0.s, line.at_points[b].0.s))
            .collect();
        track.place(&layout, &gaps)?;
        track.fit_racing_line(&gaps);
        track.turns = turns(&track.nodes, track.ds);
        track.jumps = layout
            .gaps
            .iter()
            .map(|&(a, b)| {
                // The exact frame arriving at the lip: interpolated frames smear the kink.
                let lip = line.at_points[a].0;
                let speed = track.target_speed_at(lip.s - 0.01);
                track.fly(lip.pos, lip.forward, lip.s, line.at_points[b].0.s, speed)
            })
            .collect();
        track.bounds = bounds(&track, &layout)?;
        track.lints = lints(&track);
        track.geometry_hash = geometry_hash(&track);
        Ok(track)
    }
}

/// "forward", "reverse", "mirror" or "reverse mirror".
pub fn variant_name(v: &TrackVariant) -> &'static str {
    match (v.reverse, v.mirror) {
        (false, false) => "forward",
        (true, false) => "reverse",
        (false, true) => "mirror",
        (true, true) => "reverse mirror",
    }
}

fn ground_areas(layout: &Layout) -> Result<Vec<GroundArea>, String> {
    layout
        .ground
        .iter()
        .enumerate()
        .map(|(i, (surface, height, polygon))| {
            if self_intersects(polygon) || ground_mesh(polygon, *height).is_none() {
                return Err(format!("ground[{i}]: the outline is not a simple polygon"));
            }
            Ok(GroundArea::new(*surface, *height, polygon.clone()))
        })
        .collect()
}

fn topmost_ground(ground: &[GroundArea], pos: Vec3) -> Option<SurfaceKind> {
    ground
        .iter()
        .filter(|g| g.contains(pos))
        .max_by(|a, b| a.height.total_cmp(&b.height))
        .map(|g| g.surface)
}

fn node(layout: &Layout, ground: &[GroundArea], s: &Sample) -> Node {
    let p = layout.points[s.seg];
    let surface = match layout.style {
        RibbonStyle::Invisible => topmost_ground(ground, s.pos).unwrap_or(p.surface),
        _ => p.surface,
    };
    Node {
        frame: TrackFrame {
            s: s.s,
            position: s.pos,
            forward: s.forward,
            right: s.right,
            up: s.up,
            half_width: s.half_width,
            bank: s.bank,
            surface,
            offroad_margin: p.margin,
            in_gap: s.in_gap,
        },
        walls: (p.wall_left && !s.in_gap, p.wall_right && !s.in_gap),
        line: 0.0,
        speed: 0.0,
    }
}

/// Lateral reach of the ribbon beyond its half width (profile edge + chamfer).
fn reach(style: RibbonStyle) -> f32 {
    let (profile, _, _) = style_profile(style);
    let hw = 100.0;
    let max = profile.iter().map(|p| p.at(hw).x.abs()).fold(hw, f32::max);
    max - hw
}

/// The inner edge of a turn must not fold over itself.
fn check_turns(layout: &Layout, line: &Centreline) -> Result<(), String> {
    let n = line.samples.len();
    let extra = reach(layout.style);
    let (_, edge, _) = style_profile(layout.style);
    for k in 0..n {
        let (a, s, b) = (
            &line.samples[(k + n - 1) % n],
            &line.samples[k],
            &line.samples[(k + 1) % n],
        );
        if a.in_gap || s.in_gap || b.in_gap {
            continue;
        }
        let bend = (b.forward - a.forward).dot(s.right) / (2.0 * line.ds);
        let p = layout.points[s.seg];
        let wall = if bend > 0.0 {
            p.wall_right
        } else {
            p.wall_left
        };
        let side = if wall { edge + WALL_THICKNESS_U } else { 0.0 };
        let inner = s.half_width + side.max(extra);
        if bend.abs() * (inner + TURN_CLEARANCE_U) > 1.0 {
            return Err(format!(
                "s = {:.1} u: turn radius {:.1} u is tighter than the track's inner reach {:.1} u; widen the turn or narrow the track",
                s.s,
                1.0 / bend.abs(),
                inner + TURN_CLEARANCE_U
            ));
        }
    }
    Ok(())
}

fn bounds(track: &Track, layout: &Layout) -> Result<(Vec3, Vec3), String> {
    let mut lo = Vec3::MAX;
    let mut hi = Vec3::MIN;
    let mut grow = |p: Vec3, r: f32| {
        lo = lo.min(p - Vec3::splat(r));
        hi = hi.max(p + Vec3::splat(r));
    };
    for n in &track.nodes {
        grow(n.frame.position, n.frame.half_width + 3.0);
    }
    for g in &track.ground {
        g.polygon
            .iter()
            .for_each(|p| grow(Vec3::new(p.x, g.height, p.y), 0.0));
    }
    track
        .props
        .iter()
        .for_each(|p| grow(p.transform.translation, 5.0));
    let low = track
        .nodes
        .iter()
        .filter(|n| !n.frame.in_gap)
        .map(|n| n.frame.position.y)
        .chain(track.ground.iter().map(|g| g.height))
        .fold(f32::MAX, f32::min);
    if layout.kill_height > low - KILL_CLEARANCE_U {
        return Err(format!(
            "kill_height: {} must be at least {KILL_CLEARANCE_U} u below the lowest drivable point ({low:.1})",
            layout.kill_height
        ));
    }
    let (min, max) = match layout.bounds {
        Some((min, max)) => (min, max),
        None => (
            Vec2::new(lo.x, lo.z) - BOUNDS_MARGIN_U,
            Vec2::new(hi.x, hi.z) + BOUNDS_MARGIN_U,
        ),
    };
    let floor = layout.kill_height - KILL_FLOOR_DEPTH_U - 1.0;
    Ok((
        Vec3::new(min.x, floor, min.y),
        Vec3::new(max.x, hi.y + BOUNDS_HEADROOM_U, max.y),
    ))
}
