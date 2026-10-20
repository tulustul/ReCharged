//! The track file format, `assets/data/tracks/<id>.ron` (authoring guide:
//! `docs/design/rc_track.md`). Pure data + field-level validation; geometry checks happen when the
//! track is built ([`crate::Track::build`]).
//!
//! Control-point fields other than `pos` and `bank_deg` are optional and **inherited**: a point
//! without `width` keeps the previous point's width, and so on (point 0 falls back to the
//! defaults below). `surface`, walls and `offroad_margin` apply to the segment from a point to the
//! next one; `width` and `bank_deg` are interpolated between points.

use std::path::Path;

use bevy::math::{Vec2, Vec3};
use rc_core::SurfaceKind;
use serde::{Deserialize, Serialize};

/// Track files relative to the assets root: `data/tracks/<id>.ron`.
pub const TRACKS_DIR: &str = "data/tracks";
/// Racing width of point 0 when it gives none (u).
pub const DEFAULT_WIDTH_U: f32 = 10.0;
/// Off-road margin of point 0 when it gives none (u, GDD §5.4).
pub const DEFAULT_OFFROAD_MARGIN_U: f32 = 6.0;
/// Barrier height above the driving surface when the track gives none (u).
pub const DEFAULT_WALL_HEIGHT_U: f32 = 1.2;
/// Narrowest allowed racing width (u, GDD §7.1 chokepoints).
pub const MIN_WIDTH_U: f32 = 5.0;
/// Narrower than this counts as a chokepoint (u, GDD §7.1).
pub const CHOKE_WIDTH_U: f32 = 8.0;
/// Steepest bank a file may ask for (degrees; > 30° is linted except on helices/half-pipes).
pub const MAX_BANK_DEG: f32 = 60.0;

/// Which layouts a feature exists in. Forward = the file's point order.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Dir {
    /// Forward and reverse.
    #[default]
    Both,
    /// Forward layout only.
    Fwd,
    /// Reverse layout only.
    Rev,
}

fn is_both(dir: &Dir) -> bool {
    *dir == Dir::Both
}

fn is_zero(value: &f32) -> bool {
    *value == 0.0
}

impl Dir {
    /// Whether the feature exists when the layout is driven `reverse`.
    pub fn active(self, reverse: bool) -> bool {
        match self {
            Self::Both => true,
            Self::Fwd => !reverse,
            Self::Rev => reverse,
        }
    }
}

/// Look and cross-section of the main ribbon.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RibbonStyle {
    /// Plastic toy-track pieces with raised lips (0.25 u thick).
    #[default]
    ToyTrack,
    /// Road slab with sloped curbs (0.3 u thick).
    Asphalt,
    /// Thin rug runner (0.06 u thick).
    CarpetRunner,
    /// No ribbon geometry or collider: the drivable surface comes from `ground` polygons and
    /// props (open floors, lawns). The centreline still drives checkpoints, AI and respawns.
    Invisible,
}

impl RibbonStyle {
    /// Surface of point 0 when it gives none.
    pub fn default_surface(self) -> SurfaceKind {
        match self {
            Self::ToyTrack => SurfaceKind::ToyTrack,
            Self::Asphalt => SurfaceKind::Asphalt,
            Self::CarpetRunner => SurfaceKind::Carpet,
            Self::Invisible => SurfaceKind::Wood,
        }
    }
}

/// One control point of the closed centreline loop. `pos.y` is the height of the driving
/// surface; the ribbon body hangs below it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlPoint {
    /// Centreline position (world u).
    pub pos: Vec3,
    /// Racing width (u, ≥ 5; < 8 is a chokepoint). Inherited.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<f32>,
    /// Bank in degrees; positive raises the **left** edge (for right-hand turns). Not inherited.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub bank_deg: f32,
    /// Ribbon surface from this point to the next. Inherited.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surface: Option<SurfaceKind>,
    /// Barrier on the left edge from this point to the next. Inherited.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wall_left: Option<bool>,
    /// Barrier on the right edge from this point to the next. Inherited.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wall_right: Option<bool>,
    /// Off-track tolerance beyond the edge from this point to the next (u, GDD §5.4). Inherited.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offroad_margin: Option<f32>,
    /// Designer's note shown by the track editor ("T1", "jump: lip"). Not inherited; no effect.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// A jump: no ribbon between control points `from` and `to` (forward order, may wrap). The
/// takeoff is at the first point in the direction of travel; where the gap is not active
/// (`dir`), the ribbon is continuous (a hump).
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Gap {
    /// Control point at the lip (forward order).
    pub from: usize,
    /// Control point where the landing starts (forward order).
    pub to: usize,
    /// Layouts with the gap (the jump direction).
    #[serde(default, skip_serializing_if = "is_both")]
    pub dir: Dir,
}

/// A flat drivable area at any height: floors, lawns, mats, furniture tops. Polygons may
/// overlap; the higher one is driven on.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ground {
    /// Surface.
    pub surface: SurfaceKind,
    /// Height of the surface (u).
    pub height: f32,
    /// Simple polygon outline, world (x, z), either winding, ≥ 3 points.
    pub points: Vec<Vec2>,
    /// Layouts with this area.
    #[serde(default, skip_serializing_if = "is_both")]
    pub dir: Dir,
}

fn default_row_count() -> u8 {
    5
}

/// A row of Volt Cubes across the track near `pos` (projected onto the centreline).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemRowDef {
    /// A point on or near the centreline (u).
    pub pos: Vec3,
    /// Cubes in the row, 4–5.
    #[serde(default = "default_row_count")]
    pub count: u8,
    /// Layouts with the row.
    #[serde(default, skip_serializing_if = "is_both")]
    pub dir: Dir,
}

/// A prop from `assets/data/props/*.ron` placed in the world.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PropPlacement {
    /// Prop kind (key in a `props/*.ron` file).
    pub kind: String,
    /// Ground-contact centre (u).
    pub pos: Vec3,
    /// Rotation about +Y in degrees (0 = the prop's front faces −Z).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub yaw: f32,
    /// Layouts with the prop.
    #[serde(default, skip_serializing_if = "is_both")]
    pub dir: Dir,
}

fn default_shortcut_width() -> f32 {
    6.0
}

/// An alternative path off the main line (forward order, entry → exit). It has no geometry of
/// its own: ground polygons and props carry it. AI and checkpoint placement use it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShortcutDef {
    /// Path points (u); the first and last lie on the main track.
    pub path: Vec<Vec3>,
    /// Usable width (u).
    #[serde(default = "default_shortcut_width")]
    pub width: f32,
    /// Layouts with the shortcut.
    #[serde(default, skip_serializing_if = "is_both")]
    pub dir: Dir,
}

fn fwd() -> Dir {
    Dir::Fwd
}

/// The track's collectible (GDD §7.2: forward layouts only, off the main line).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoldenBatteryDef {
    /// Position of its centre (u).
    pub pos: Vec3,
    /// Must be `Fwd`.
    #[serde(default = "fwd")]
    pub dir: Dir,
}

/// Explicit world bounds (x, z) for the shell walls and the room visuals.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundsDef {
    /// Minimum corner (x, z).
    pub min: Vec2,
    /// Maximum corner (x, z).
    pub max: Vec2,
}

fn default_wall_height() -> f32 {
    DEFAULT_WALL_HEIGHT_U
}

/// A track layout file. The loop's shape is the same in both directions; everything that
/// works only one way carries a [`Dir`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrackDef {
    /// Ribbon style of the main loop.
    pub style: RibbonStyle,
    /// Closed centreline loop in forward order (≥ 4 points).
    pub points: Vec<ControlPoint>,
    /// Control point on the start/finish line (same place in reverse).
    #[serde(default)]
    pub start: usize,
    /// Cars below this height are respawned (u).
    pub kill_height: f32,
    /// Barrier height above the surface (u).
    #[serde(default = "default_wall_height")]
    pub wall_height: f32,
    /// Jumps.
    #[serde(default)]
    pub gaps: Vec<Gap>,
    /// Drivable areas.
    #[serde(default)]
    pub ground: Vec<Ground>,
    /// Volt Cube rows (GDD §6.1: 4–6 per lap).
    #[serde(default)]
    pub item_rows: Vec<ItemRowDef>,
    /// Placed props.
    #[serde(default)]
    pub props: Vec<PropPlacement>,
    /// Alternative paths.
    #[serde(default)]
    pub shortcuts: Vec<ShortcutDef>,
    /// The Golden Battery (forward layouts).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub golden_battery: Option<GoldenBatteryDef>,
    /// World bounds; `None` = the content's bounding box plus a margin.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bounds: Option<BoundsDef>,
}

/// One value as single-line RON (`(pos: (1.0, 0.0, 2.0), width: 12.0)`).
pub(crate) fn ron_value<T: Serialize>(v: &T) -> Result<String, String> {
    let pretty = ron::ser::PrettyConfig::new()
        .compact_structs(true)
        .compact_arrays(true);
    ron_options()
        .to_string_pretty(v, pretty)
        .map_err(|e| e.to_string())
}

/// Appends `    name: [` + one line per item (with `// <index>` if `index`) + `    ],`; nothing for
/// an empty list.
pub(crate) fn ron_list<T: Serialize>(
    out: &mut String,
    name: &str,
    items: &[T],
    index: bool,
) -> Result<(), String> {
    if items.is_empty() {
        return Ok(());
    }
    out.push_str(&format!("    {name}: [\n"));
    for (i, item) in items.iter().enumerate() {
        let note = if index {
            format!(" // {i}")
        } else {
            String::new()
        };
        out.push_str(&format!("        {},{note}\n", ron_value(item)?));
    }
    out.push_str("    ],\n");
    Ok(())
}

/// RON options of every rc_track data file: `width: 12` instead of `width: Some(12)`.
pub(crate) fn ron_options() -> ron::Options {
    ron::Options::default().with_default_extension(ron::extensions::Extensions::IMPLICIT_SOME)
}

/// Reads a whole data file; errors name the path.
pub(crate) fn read(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}

impl TrackDef {
    /// Reads and validates `<assets_dir>/data/tracks/<id>.ron`.
    pub fn load(assets_dir: &Path, id: &str) -> Result<Self, String> {
        let path = assets_dir.join(TRACKS_DIR).join(format!("{id}.ron"));
        Self::parse(&read(&path)?).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// Track RON in the shipped files' shape, which [`Self::parse`] reads back: one line per
    /// top-level field and per list entry (control points end with `// <index>`); empty lists and
    /// inherited or default fields are left out. Comments of a hand-written file are not kept.
    pub fn to_ron(&self) -> Result<String, String> {
        let mut out = String::from("(\n");
        out.push_str(&format!("    style: {},\n", ron_value(&self.style)?));
        out.push_str(&format!("    start: {},\n", self.start));
        out.push_str(&format!(
            "    kill_height: {},\n",
            ron_value(&self.kill_height)?
        ));
        out.push_str(&format!(
            "    wall_height: {},\n",
            ron_value(&self.wall_height)?
        ));
        if let Some(bounds) = &self.bounds {
            out.push_str(&format!("    bounds: {},\n", ron_value(bounds)?));
        }
        ron_list(&mut out, "points", &self.points, true)?;
        ron_list(&mut out, "gaps", &self.gaps, false)?;
        ron_list(&mut out, "ground", &self.ground, false)?;
        ron_list(&mut out, "item_rows", &self.item_rows, false)?;
        ron_list(&mut out, "props", &self.props, false)?;
        ron_list(&mut out, "shortcuts", &self.shortcuts, false)?;
        if let Some(battery) = &self.golden_battery {
            out.push_str(&format!("    golden_battery: {},\n", ron_value(battery)?));
        }
        out.push_str(")\n");
        Ok(out)
    }

    /// Parses and validates track RON.
    pub fn parse(text: &str) -> Result<Self, String> {
        let def: Self = ron_options().from_str(text).map_err(|e| e.to_string())?;
        def.validate()?;
        Ok(def)
    }

    /// Field-level checks (counts, ranges, indices, `dir` rules). Errors name the field.
    pub fn validate(&self) -> Result<(), String> {
        let n = self.points.len();
        if n < 4 {
            return Err(format!("points: {n} control points, need at least 4"));
        }
        if self.start >= n {
            return Err(format!(
                "start: {} is not a point index (0..{n})",
                self.start
            ));
        }
        for (i, p) in self.points.iter().enumerate() {
            if !p.pos.is_finite() {
                return Err(format!("points[{i}].pos is not finite"));
            }
            if let Some(w) = p.width.filter(|w| w.is_nan() || *w < MIN_WIDTH_U) {
                return Err(format!(
                    "points[{i}].width: {w} u is below the {MIN_WIDTH_U} u minimum"
                ));
            }
            if p.bank_deg.is_nan() || p.bank_deg.abs() > MAX_BANK_DEG {
                return Err(format!(
                    "points[{i}].bank_deg: {} exceeds ±{MAX_BANK_DEG}°",
                    p.bank_deg
                ));
            }
            if let Some(m) = p.offroad_margin.filter(|m| m.is_nan() || *m < 0.0) {
                return Err(format!("points[{i}].offroad_margin: {m} must be ≥ 0"));
            }
            let next = self.points[(i + 1) % n].pos;
            if p.pos.distance(next) < 1.0 {
                return Err(format!(
                    "points[{i}] and points[{}] are closer than 1 u",
                    (i + 1) % n
                ));
            }
        }
        if self.wall_height.is_nan() || self.wall_height <= 0.0 {
            return Err(format!("wall_height: {} must be > 0", self.wall_height));
        }
        for (i, g) in self.gaps.iter().enumerate() {
            if g.from >= n || g.to >= n || g.from == g.to {
                return Err(format!(
                    "gaps[{i}]: from {} / to {} must be distinct point indices (0..{n})",
                    g.from, g.to
                ));
            }
        }
        for (i, g) in self.ground.iter().enumerate() {
            if g.points.len() < 3 {
                return Err(format!("ground[{i}].points: need at least 3 points"));
            }
            if !g.height.is_finite() || g.points.iter().any(|p| !p.is_finite()) {
                return Err(format!("ground[{i}]: coordinates must be finite"));
            }
        }
        for (i, r) in self.item_rows.iter().enumerate() {
            if !(4..=5).contains(&r.count) {
                return Err(format!(
                    "item_rows[{i}].count: {} (rows hold 4–5 cubes)",
                    r.count
                ));
            }
        }
        for (i, s) in self.shortcuts.iter().enumerate() {
            if s.path.len() < 2 {
                return Err(format!("shortcuts[{i}].path: need at least 2 points"));
            }
            if s.width.is_nan() || s.width < 1.0 {
                return Err(format!("shortcuts[{i}].width: {} must be ≥ 1 u", s.width));
            }
        }
        if let Some(b) = &self.golden_battery
            && b.dir != Dir::Fwd
        {
            return Err("golden_battery.dir: forward layouts only (GDD §7.2), use Fwd".into());
        }
        if let Some(b) = &self.bounds
            && !(b.min.x < b.max.x && b.min.y < b.max.y)
        {
            return Err("bounds: min must be below max on both axes".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square() -> TrackDef {
        let pos = [(0.0, 0.0), (40.0, 0.0), (40.0, -40.0), (0.0, -40.0)];
        TrackDef {
            points: pos
                .iter()
                .map(|&(x, z)| ControlPoint {
                    pos: Vec3::new(x, 0.3, z),
                    ..default()
                })
                .collect(),
            kill_height: -10.0,
            wall_height: DEFAULT_WALL_HEIGHT_U,
            ..default()
        }
    }

    use bevy::prelude::default;

    #[test]
    fn format_round_trips_through_ron() {
        let mut def = square();
        def.points[1].width = Some(12.0);
        def.points[2].bank_deg = 15.0;
        def.points[2].surface = Some(SurfaceKind::Wood);
        def.gaps.push(Gap {
            from: 1,
            to: 2,
            dir: Dir::Fwd,
        });
        def.ground.push(Ground {
            surface: SurfaceKind::Grass,
            height: 0.0,
            points: vec![Vec2::ZERO, Vec2::X, Vec2::Y],
            dir: Dir::Both,
        });
        def.golden_battery = Some(GoldenBatteryDef {
            pos: Vec3::ONE,
            dir: Dir::Fwd,
        });
        let text = ron_options()
            .to_string_pretty(&def, ron::ser::PrettyConfig::default())
            .expect("serializes");
        assert_eq!(TrackDef::parse(&text).expect("parses"), def);
    }

    #[test]
    fn terse_syntax_parses_with_defaults() {
        let def = TrackDef::parse(
            "(style: Asphalt, kill_height: -5, points: [
                (pos: (0, 0, 0), width: 12), (pos: (40, 0, 0), bank_deg: 10),
                (pos: (40, 0, -40)), (pos: (0, 0, -40), wall_left: true),
            ], gaps: [(from: 1, to: 2)])",
        )
        .expect("parses");
        assert_eq!(def.points[0].width, Some(12.0));
        assert_eq!(def.points[1].width, None);
        assert_eq!(def.points[3].wall_left, Some(true));
        assert_eq!(def.gaps[0].dir, Dir::Both);
        assert_eq!(def.wall_height, DEFAULT_WALL_HEIGHT_U);
    }

    #[test]
    fn validation_names_the_field() {
        let mut def = square();
        def.points[2].width = Some(4.0);
        assert!(def.validate().unwrap_err().contains("points[2].width"));
        let mut def = square();
        def.golden_battery = Some(GoldenBatteryDef {
            pos: Vec3::ZERO,
            dir: Dir::Rev,
        });
        assert!(def.validate().unwrap_err().contains("golden_battery"));
        let mut def = square();
        def.gaps.push(Gap {
            from: 2,
            to: 9,
            dir: Dir::Fwd,
        });
        assert!(def.validate().unwrap_err().contains("gaps[0]"));
        let err = TrackDef::parse("(style: ToyTrack, kill_height: 0, points: [], typo: 1)");
        assert!(
            err.unwrap_err().contains("typo"),
            "unknown fields are errors"
        );
    }

    #[test]
    fn dir_filtering() {
        assert!(Dir::Both.active(false) && Dir::Both.active(true));
        assert!(Dir::Fwd.active(false) && !Dir::Fwd.active(true));
        assert!(!Dir::Rev.active(false) && Dir::Rev.active(true));
    }
}
