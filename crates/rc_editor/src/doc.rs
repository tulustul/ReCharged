//! The edited document (layout + decor + environment): handles of the items, structural edits
//! (add, insert, delete, duplicate) with index fix-ups, and validation (placing and swapping
//! library assets: [`crate::library`]; lights, sun, fill and time looks: [`crate::lights`]).
//! Pure; the ECS side is in `lib.rs`.

use bevy::math::{EulerRot, Quat, Vec2, Vec3};
use rc_core::prelude::*;
use rc_render::EnvironmentDef;
use rc_track::{
    ControlPoint, DecorItem, Dir, Gap, Ground, ItemRowDef, PropCatalog, RibbonStyle, ShortcutDef,
    Track, TrackDef,
};
pub use rc_ui::editor::{EditorAdd, EditorCounts, EditorItem};

/// Duplicates land this far to the side (u).
const DUPLICATE_OFFSET_U: f32 = 2.0;
/// A new ground area is a square this wide (u).
const NEW_GROUND_U: f32 = 10.0;
/// A new shortcut leaves and rejoins the track this far before and after the focus (u).
const NEW_SHORTCUT_REACH_U: f32 = 12.0;

/// Where an item is, as the gizmo moves it (scale is along the item's own axes).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pose {
    /// Position (u).
    pub pos: Vec3,
    /// Rotation.
    pub rot: Quat,
    /// Scale.
    pub scale: Vec3,
}

/// How an item may turn (D9: props stay upright).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Turn {
    /// Not at all (points, rows, the battery).
    No,
    /// About the vertical (props).
    Yaw,
    /// Any way (decor).
    Free,
    /// Aimed about its position (spot lights: the target swings around the light).
    Aim,
    /// Aimed only, never moved (sun and fill: elevation and azimuth).
    Sky,
}

/// Rounds to the millimetre-ish grid saved files use (dragging makes long fractions).
pub(crate) fn tidy(v: f32) -> f32 {
    (v * 1000.0).round() / 1000.0 + 0.0 // + 0.0: no "-0.0" in the file
}

/// Layout, decor, environment and catalog entry of one track.
#[derive(Debug, Clone, PartialEq)]
pub struct Doc {
    /// The layout.
    pub def: TrackDef,
    /// The decorative models.
    pub decor: Vec<DecorItem>,
    /// Its lighting (`environment.ron`).
    pub env: EnvironmentDef,
    /// Its entry in `catalog.ron`.
    pub info: TrackInfo,
}

fn at(index: u32) -> usize {
    index as usize
}

fn idx(index: usize) -> u32 {
    index as u32
}

impl Doc {
    /// Item counts for the item list.
    pub fn counts(&self) -> EditorCounts {
        let d = &self.def;
        EditorCounts {
            points: idx(d.points.len()),
            gaps: idx(d.gaps.len()),
            ground: idx(d.ground.len()),
            item_rows: idx(d.item_rows.len()),
            props: idx(d.props.len()),
            shortcuts: idx(d.shortcuts.len()),
            decor: idx(self.decor.len()),
            battery: d.golden_battery.is_some(),
        }
    }

    /// The item exists in this document.
    pub fn exists(&self, item: EditorItem) -> bool {
        let d = &self.def;
        match item {
            EditorItem::Track => true,
            EditorItem::Point { index } => at(index) < d.points.len(),
            EditorItem::Gap { index } => at(index) < d.gaps.len(),
            EditorItem::Ground { index } => at(index) < d.ground.len(),
            EditorItem::Vertex { ground, index } => d
                .ground
                .get(at(ground))
                .is_some_and(|g| at(index) < g.points.len()),
            EditorItem::ItemRow { index } => at(index) < d.item_rows.len(),
            EditorItem::Prop { index } => at(index) < d.props.len(),
            EditorItem::Shortcut { index } => at(index) < d.shortcuts.len(),
            EditorItem::PathPoint { shortcut, index } => d
                .shortcuts
                .get(at(shortcut))
                .is_some_and(|s| at(index) < s.path.len()),
            EditorItem::Battery => d.golden_battery.is_some(),
            EditorItem::Decor { index } => at(index) < self.decor.len(),
            EditorItem::Light { index } => at(index) < self.env.lights.len(),
            EditorItem::Fill { .. } => self.sky(item).is_some(),
            EditorItem::Sun { .. } | EditorItem::Look { .. } => true,
        }
    }

    /// Where the item's handle is (a gap's: the middle of its jump, it does not move); `None` for
    /// the track and for shortcuts and ground areas as a whole (their points carry the handles).
    pub fn handle(&self, item: EditorItem) -> Option<Vec3> {
        let d = &self.def;
        match item {
            EditorItem::Point { index } => d.points.get(at(index)).map(|p| p.pos),
            EditorItem::Vertex { ground, index } => d.ground.get(at(ground)).and_then(|g| {
                g.points
                    .get(at(index))
                    .map(|v| Vec3::new(v.x, g.height, v.y))
            }),
            EditorItem::Gap { index } => d
                .gaps
                .get(at(index))
                .and_then(|g| Some((d.points.get(g.from)?.pos + d.points.get(g.to)?.pos) / 2.0)),
            EditorItem::ItemRow { index } => d.item_rows.get(at(index)).map(|r| r.pos),
            EditorItem::Prop { index } => d.props.get(at(index)).map(|p| p.pos),
            EditorItem::PathPoint { shortcut, index } => d
                .shortcuts
                .get(at(shortcut))
                .and_then(|s| s.path.get(at(index)).copied()),
            EditorItem::Battery => d.golden_battery.map(|b| b.pos),
            EditorItem::Decor { index } => self.decor.get(at(index)).map(|d| d.pos),
            EditorItem::Light { .. } | EditorItem::Sun { .. } | EditorItem::Fill { .. } => {
                self.light_handle(item)
            }
            EditorItem::Track
            | EditorItem::Ground { .. }
            | EditorItem::Shortcut { .. }
            | EditorItem::Look { .. } => None,
        }
    }

    /// Items without a mesh in the (forward) editing session, which the view marks with an icon:
    /// item rows, the battery, lights, the sun and fill of `time`, props and decor of reverse
    /// layouts only and lights off at `time` (`true`: shown dimmed); with `layout` also control
    /// points, gaps, ground vertices and shortcut path points (the Track tab's).
    pub fn markers(&self, layout: bool, time: TimeOfDay) -> Vec<(EditorItem, Vec3, bool)> {
        let d = &self.def;
        let reverse_only = |dir: Dir| !dir.active(false);
        let mut items: Vec<EditorItem> = (0..d.item_rows.len())
            .map(|i| EditorItem::ItemRow { index: idx(i) })
            .chain(d.golden_battery.map(|_| EditorItem::Battery))
            .collect();
        let dimmed: Vec<EditorItem> = (d.props.iter().enumerate())
            .filter(|(_, p)| reverse_only(p.dir))
            .map(|(i, _)| EditorItem::Prop { index: idx(i) })
            .chain(
                (self.decor.iter().enumerate())
                    .filter(|(_, p)| reverse_only(p.dir))
                    .map(|(i, _)| EditorItem::Decor { index: idx(i) }),
            )
            .collect();
        if layout {
            items.extend((0..d.points.len()).map(|i| EditorItem::Point { index: idx(i) }));
            items.extend((0..d.gaps.len()).map(|i| EditorItem::Gap { index: idx(i) }));
            for (g, area) in d.ground.iter().enumerate() {
                items.extend((0..area.points.len()).map(|i| EditorItem::Vertex {
                    ground: idx(g),
                    index: idx(i),
                }));
            }
            for (s, cut) in d.shortcuts.iter().enumerate() {
                items.extend((0..cut.path.len()).map(|i| EditorItem::PathPoint {
                    shortcut: idx(s),
                    index: idx(i),
                }));
            }
        }
        let marker = |dim| move |item| self.handle(item).map(|pos| (item, pos, dim));
        let mut out: Vec<_> = items.into_iter().filter_map(marker(false)).collect();
        out.extend(dimmed.into_iter().filter_map(marker(true)));
        out.extend(self.light_markers(time));
        out
    }

    /// The item's pose and how it may turn and scale; `None` for items the gizmo does not move
    /// (the track, gaps, whole ground areas and shortcuts).
    pub fn pose(&self, item: EditorItem) -> Option<(Pose, Turn, bool)> {
        if matches!(
            item,
            EditorItem::Light { .. } | EditorItem::Sun { .. } | EditorItem::Fill { .. }
        ) {
            return self.light_pose(item);
        }
        let pos = self.handle(item)?;
        let plain = Pose {
            pos,
            rot: Quat::IDENTITY,
            scale: Vec3::ONE,
        };
        Some(match item {
            EditorItem::Gap { .. } => return None,
            EditorItem::Prop { index } => {
                let yaw = self.def.props[at(index)].yaw.to_radians();
                let rot = Quat::from_rotation_y(yaw);
                (Pose { rot, ..plain }, Turn::Yaw, false)
            }
            EditorItem::Decor { index } => {
                let d = &self.decor[at(index)];
                let r = d.rot * std::f32::consts::PI / 180.0;
                let rot = Quat::from_euler(EulerRot::YXZ, r.y, r.x, r.z);
                (
                    Pose {
                        pos,
                        rot,
                        scale: d.scale,
                    },
                    Turn::Free,
                    true,
                )
            }
            _ => (plain, Turn::No, false),
        })
    }

    /// Moves, turns and scales the item as far as its data allows. Angles are rewritten only
    /// when the rotation changed (a move keeps a file's `yaw: 270`).
    pub fn set_pose(&mut self, item: EditorItem, pose: Pose) {
        let Some((old, ..)) = self.pose(item) else {
            return;
        };
        let turned = !old.rot.abs_diff_eq(pose.rot, 1e-6);
        if matches!(
            item,
            EditorItem::Light { .. } | EditorItem::Sun { .. } | EditorItem::Fill { .. }
        ) {
            return self.set_light_pose(item, pose, turned);
        }
        self.set_handle(item, pose.pos.map(tidy));
        let (yaw, pitch, roll) = pose.rot.to_euler(EulerRot::YXZ);
        let deg = |r: f32| (r.to_degrees() * 100.0).round() / 100.0 + 0.0;
        match item {
            EditorItem::Prop { index } if turned => self.def.props[at(index)].yaw = deg(yaw),
            EditorItem::Decor { index } => {
                let d = &mut self.decor[at(index)];
                if turned {
                    d.rot = Vec3::new(deg(pitch), deg(yaw), deg(roll));
                }
                d.scale = pose.scale.map(tidy);
            }
            _ => {}
        }
    }

    /// Moves the item's handle to `pos` (a ground vertex keeps its area's height).
    pub fn set_handle(&mut self, item: EditorItem, pos: Vec3) {
        let d = &mut self.def;
        match item {
            EditorItem::Point { index } => {
                if let Some(p) = d.points.get_mut(at(index)) {
                    p.pos = pos;
                }
            }
            EditorItem::Vertex { ground, index } => {
                if let Some(v) = d
                    .ground
                    .get_mut(at(ground))
                    .and_then(|g| g.points.get_mut(at(index)))
                {
                    *v = Vec2::new(pos.x, pos.z);
                }
            }
            EditorItem::ItemRow { index } => {
                if let Some(r) = d.item_rows.get_mut(at(index)) {
                    r.pos = pos;
                }
            }
            EditorItem::Prop { index } => {
                if let Some(p) = d.props.get_mut(at(index)) {
                    p.pos = pos;
                }
            }
            EditorItem::PathPoint { shortcut, index } => {
                if let Some(p) = d
                    .shortcuts
                    .get_mut(at(shortcut))
                    .and_then(|s| s.path.get_mut(at(index)))
                {
                    *p = pos;
                }
            }
            EditorItem::Battery => {
                if let Some(b) = &mut d.golden_battery {
                    b.pos = pos;
                }
            }
            EditorItem::Decor { index } => {
                if let Some(item) = self.decor.get_mut(at(index)) {
                    item.pos = pos;
                }
            }
            EditorItem::Light { index } => self.move_light(index, pos),
            EditorItem::Track
            | EditorItem::Gap { .. }
            | EditorItem::Ground { .. }
            | EditorItem::Shortcut { .. }
            | EditorItem::Sun { .. }
            | EditorItem::Fill { .. }
            | EditorItem::Look { .. } => {}
        }
    }

    /// Inserts a point, ground vertex or shortcut path point halfway to the next one; returns
    /// the new item.
    pub fn insert_after(&mut self, item: EditorItem) -> Result<EditorItem, String> {
        let d = &mut self.def;
        match item {
            EditorItem::Point { index } => {
                let i = at(index);
                let n = d.points.len();
                let (a, b) = (&d.points[i], &d.points[(i + 1) % n]);
                let point = ControlPoint {
                    pos: (a.pos + b.pos) / 2.0,
                    bank_deg: (a.bank_deg + b.bank_deg) / 2.0,
                    ..ControlPoint::default()
                };
                let k = i + 1;
                d.points.insert(k, point);
                let shift = |j: &mut usize| *j += usize::from(*j >= k);
                shift(&mut d.start);
                for gap in &mut d.gaps {
                    shift(&mut gap.from);
                    shift(&mut gap.to);
                }
                Ok(EditorItem::Point { index: idx(k) })
            }
            EditorItem::Vertex { ground, index } => {
                let g = &mut d.ground[at(ground)];
                let i = at(index);
                let mid = (g.points[i] + g.points[(i + 1) % g.points.len()]) / 2.0;
                g.points.insert(i + 1, mid);
                Ok(EditorItem::Vertex {
                    ground,
                    index: index + 1,
                })
            }
            EditorItem::PathPoint { shortcut, index } => {
                let path = &mut d.shortcuts[at(shortcut)].path;
                let i = at(index);
                let next = path.get(i + 1).copied().unwrap_or(path[i]);
                path.insert(i + 1, (path[i] + next) / 2.0);
                Ok(EditorItem::PathPoint {
                    shortcut,
                    index: index + 1,
                })
            }
            _ => Err(
                "select a control point, ground vertex or shortcut point to insert after".into(),
            ),
        }
    }

    /// Removes the item; returns what to select next.
    pub fn delete(&mut self, item: EditorItem) -> Result<Option<EditorItem>, String> {
        let d = &mut self.def;
        let next = |len: usize, i: usize| (len > 0).then(|| idx(i.min(len - 1)));
        Ok(match item {
            EditorItem::Track => return Err("the track itself cannot be deleted".into()),
            EditorItem::Point { index } => {
                let k = at(index);
                if d.points.len() <= 4 {
                    return Err("a track needs at least 4 control points".into());
                }
                if let Some(g) = d.gaps.iter().position(|g| g.from == k || g.to == k) {
                    return Err(format!(
                        "point {k} is an end of gap {g}: delete the gap first"
                    ));
                }
                d.points.remove(k);
                if d.start > k {
                    d.start -= 1;
                } else if d.start == d.points.len() {
                    d.start = 0; // the start was the last point: its successor wraps around
                }
                for gap in &mut d.gaps {
                    gap.from -= usize::from(gap.from > k);
                    gap.to -= usize::from(gap.to > k);
                }
                next(d.points.len(), k).map(|index| EditorItem::Point { index })
            }
            EditorItem::Gap { index } => {
                d.gaps.remove(at(index));
                None
            }
            EditorItem::Ground { index } => {
                d.ground.remove(at(index));
                None
            }
            EditorItem::Vertex { ground, index } => {
                let g = &mut d.ground[at(ground)];
                if g.points.len() <= 3 {
                    return Err("a ground area needs at least 3 vertices: delete the area".into());
                }
                g.points.remove(at(index));
                next(g.points.len(), at(index)).map(|index| EditorItem::Vertex { ground, index })
            }
            EditorItem::ItemRow { index } => {
                d.item_rows.remove(at(index));
                None
            }
            EditorItem::Prop { index } => {
                d.props.remove(at(index));
                None
            }
            EditorItem::Shortcut { index } => {
                d.shortcuts.remove(at(index));
                None
            }
            EditorItem::PathPoint { shortcut, index } => {
                let path = &mut d.shortcuts[at(shortcut)].path;
                if path.len() <= 2 {
                    return Err("a shortcut needs at least 2 points: delete the shortcut".into());
                }
                path.remove(at(index));
                next(path.len(), at(index)).map(|index| EditorItem::PathPoint { shortcut, index })
            }
            EditorItem::Battery => {
                d.golden_battery = None;
                None
            }
            EditorItem::Decor { index } => {
                self.decor.remove(at(index));
                None
            }
            EditorItem::Light { .. }
            | EditorItem::Sun { .. }
            | EditorItem::Fill { .. }
            | EditorItem::Look { .. } => {
                self.delete_light(item)?;
                None
            }
        })
    }

    /// Copies a prop, item row, ground area, shortcut or decor model a little to the side.
    pub fn duplicate(&mut self, item: EditorItem) -> Result<EditorItem, String> {
        let shift = Vec3::X * DUPLICATE_OFFSET_U;
        let d = &mut self.def;
        match item {
            EditorItem::Prop { index } => {
                let mut copy = d.props[at(index)].clone();
                copy.pos += shift;
                d.props.push(copy);
                Ok(EditorItem::Prop {
                    index: idx(d.props.len() - 1),
                })
            }
            EditorItem::ItemRow { index } => {
                let mut copy = d.item_rows[at(index)];
                copy.pos += shift;
                d.item_rows.push(copy);
                Ok(EditorItem::ItemRow {
                    index: idx(d.item_rows.len() - 1),
                })
            }
            EditorItem::Ground { index } | EditorItem::Vertex { ground: index, .. } => {
                let mut copy = d.ground[at(index)].clone();
                copy.points.iter_mut().for_each(|p| p.x += shift.x);
                d.ground.push(copy);
                Ok(EditorItem::Ground {
                    index: idx(d.ground.len() - 1),
                })
            }
            EditorItem::Shortcut { index }
            | EditorItem::PathPoint {
                shortcut: index, ..
            } => {
                let mut copy = d.shortcuts[at(index)].clone();
                copy.path.iter_mut().for_each(|p| *p += shift);
                d.shortcuts.push(copy);
                Ok(EditorItem::Shortcut {
                    index: idx(d.shortcuts.len() - 1),
                })
            }
            EditorItem::Decor { index } => {
                let mut copy = self.decor[at(index)].clone();
                copy.pos += shift;
                self.decor.push(copy);
                Ok(EditorItem::Decor {
                    index: idx(self.decor.len() - 1),
                })
            }
            EditorItem::Light { index } => Ok(self.duplicate_light(index)),
            _ => Err(
                "only props, item rows, ground areas, shortcuts, decor and lights can be duplicated"
                    .into(),
            ),
        }
    }

    /// Adds `what` at `focus` (`selected` gives a gap its lip point); `track` is the session's (a
    /// new shortcut leaves and rejoins it). Library assets are placed instead ([`Doc::place`]).
    pub fn add(
        &mut self,
        what: EditorAdd,
        focus: Vec3,
        selected: Option<EditorItem>,
        track: Option<&Track>,
    ) -> Result<EditorItem, String> {
        let d = &mut self.def;
        match what {
            EditorAdd::Ground => {
                let h = NEW_GROUND_U / 2.0;
                let c = Vec2::new(focus.x, focus.z);
                d.ground.push(Ground {
                    surface: d.style.default_surface(),
                    height: focus.y,
                    points: vec![
                        c + Vec2::new(-h, -h),
                        c + Vec2::new(h, -h),
                        c + Vec2::new(h, h),
                        c + Vec2::new(-h, h),
                    ],
                    dir: Dir::Both,
                });
                Ok(EditorItem::Ground {
                    index: idx(d.ground.len() - 1),
                })
            }
            EditorAdd::Shortcut => {
                let track = track.ok_or("the track is still loading")?;
                let s = track.project(focus, None).s;
                let on = |ds: f32| track.frame_at(track.wrap(s + ds)).position;
                d.shortcuts.push(ShortcutDef {
                    path: vec![on(-NEW_SHORTCUT_REACH_U), focus, on(NEW_SHORTCUT_REACH_U)],
                    width: 6.0,
                    dir: Dir::Both,
                });
                Ok(EditorItem::Shortcut {
                    index: idx(d.shortcuts.len() - 1),
                })
            }
            EditorAdd::Gap => {
                let Some(EditorItem::Point { index }) = selected else {
                    return Err("select the control point at the jump's lip first".into());
                };
                let from = at(index);
                d.gaps.push(Gap {
                    from,
                    to: (from + 1) % d.points.len(),
                    dir: Dir::Both,
                });
                Ok(EditorItem::Gap {
                    index: idx(d.gaps.len() - 1),
                })
            }
        }
    }
}

/// New tracks start as an oval this big (u, half-axes) with this many points …
const NEW_OVAL_U: Vec2 = Vec2::new(45.0, 28.0);
const NEW_OVAL_POINTS: usize = 16;
/// … on a floor this far past it (u).
const NEW_FLOOR_MARGIN_U: f32 = 30.0;

/// A new track: a flat oval of `style` on a floor (wood indoors, grass outdoors) with four item
/// rows, lit by `env` (a copy of another map's), and a hidden catalog entry (allowed everything
/// the environment allows).
pub fn new_track(
    id: &TrackId,
    name: &str,
    environment: &str,
    env: EnvironmentDef,
    style: RibbonStyle,
) -> Doc {
    let indoor = env.indoor;
    // The ribbon's top lies on the floor: its body (this thick) hangs below the points.
    let top = match style {
        RibbonStyle::ToyTrack => 0.25,
        RibbonStyle::Asphalt => 0.3,
        RibbonStyle::CarpetRunner => 0.06,
        RibbonStyle::Invisible => 0.0,
    };
    let at = |i: usize| {
        let a = i as f32 / NEW_OVAL_POINTS as f32 * std::f32::consts::TAU;
        let mm = |v: f32| (v * 1000.0).round() / 1000.0 + 0.0; // + 0.0: no "-0.0" in the file
        Vec3::new(mm(NEW_OVAL_U.x * a.cos()), top, mm(NEW_OVAL_U.y * a.sin()))
    };
    let mut points: Vec<ControlPoint> = (0..NEW_OVAL_POINTS)
        .map(|i| ControlPoint {
            pos: at(i),
            ..ControlPoint::default()
        })
        .collect();
    points[0].width = Some(10.0);
    points[0].label = Some("start".into());
    let reach = NEW_OVAL_U + Vec2::splat(NEW_FLOOR_MARGIN_U);
    let floor = if indoor {
        SurfaceKind::Wood
    } else {
        SurfaceKind::Grass
    };
    let def = TrackDef {
        style,
        points,
        start: 0,
        kill_height: -10.0,
        wall_height: rc_track::DEFAULT_WALL_HEIGHT_U,
        ground: vec![Ground {
            surface: floor,
            height: 0.0,
            points: vec![
                Vec2::new(-reach.x, -reach.y),
                Vec2::new(reach.x, -reach.y),
                Vec2::new(reach.x, reach.y),
                Vec2::new(-reach.x, reach.y),
            ],
            dir: Dir::Both,
        }],
        item_rows: (0..4)
            .map(|q| ItemRowDef {
                pos: at(q * NEW_OVAL_POINTS / 4 + NEW_OVAL_POINTS / 8),
                count: 5,
                dir: Dir::Both,
            })
            .collect(),
        ..TrackDef::default()
    };
    let times = vec![
        TimeOfDay::Morning,
        TimeOfDay::Noon,
        TimeOfDay::Sunset,
        TimeOfDay::Night,
    ];
    let mut weather = vec![
        WeatherKind::Clear,
        WeatherKind::Cloudy,
        WeatherKind::Rain,
        WeatherKind::Storm,
    ];
    if !indoor {
        weather.extend([WeatherKind::Fog, WeatherKind::Snow]);
    }
    Doc {
        def,
        decor: Vec::new(),
        env,
        info: TrackInfo {
            id: id.clone(),
            name: name.to_owned(),
            environment: environment.to_owned(),
            indoor,
            difficulty: 1,
            preview: format!("ui/tracks/{id}.png"),
            music: MusicCue::Race1,
            default_laps: 3,
            allowed_times: times,
            allowed_weather: weather,
            unlock: UnlockRule::Start,
            hidden: true,
        },
    }
}

/// The layout builds in every variant (what the game will race); the error names the variant.
/// Returns the forward layout's track (the editing session's).
pub fn validate(doc: &Doc, props: &PropCatalog, track: &TrackId) -> Result<Track, String> {
    let mut forward = None;
    for (reverse, mirror) in [(false, false), (true, false), (false, true), (true, true)] {
        let variant = TrackVariant {
            track: track.clone(),
            reverse,
            mirror,
        };
        let built = Track::build(&doc.def, props, &variant)
            .map_err(|e| format!("{} layout: {e}", rc_track::variant_name(&variant)))?;
        forward.get_or_insert(built);
    }
    Ok(forward.expect("the forward layout is built first"))
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;

    fn assets() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
    }

    fn oval() -> (Doc, PropCatalog) {
        let def = TrackDef::load(&assets(), "test_oval").unwrap();
        (
            Doc {
                def,
                decor: Vec::new(),
                env: EnvironmentDef::load(&assets(), "test_oval").unwrap(),
                info: TrackInfo::default(),
            },
            PropCatalog::load(&assets()).unwrap(),
        )
    }

    #[test]
    fn inserting_and_deleting_points_keeps_start_and_gaps_on_their_points() {
        let (mut doc, props) = oval();
        let (start, gap) = (doc.def.start, doc.def.gaps[0]);
        let start_pos = doc.def.points[start].pos;
        let gap_pos = (doc.def.points[gap.from].pos, doc.def.points[gap.to].pos);
        let new = doc.insert_after(EditorItem::Point { index: 0 }).unwrap();
        assert_eq!(new, EditorItem::Point { index: 1 });
        assert_eq!(doc.def.points[doc.def.start].pos, start_pos);
        let g = doc.def.gaps[0];
        assert_eq!(
            (doc.def.points[g.from].pos, doc.def.points[g.to].pos),
            gap_pos
        );
        validate(&doc, &props, &"test_oval".into()).unwrap();

        doc.delete(new).unwrap();
        assert_eq!(doc.def.points[doc.def.start].pos, start_pos);
        let g = doc.def.gaps[0];
        assert_eq!(
            (doc.def.points[g.from].pos, doc.def.points[g.to].pos),
            gap_pos
        );
        let err = doc
            .delete(EditorItem::Point { index: idx(g.from) })
            .unwrap_err();
        assert!(err.contains("gap 0"), "{err}");
    }

    #[test]
    fn new_tracks_build_in_every_style() {
        let (_, props) = oval();
        for style in [
            RibbonStyle::ToyTrack,
            RibbonStyle::Asphalt,
            RibbonStyle::CarpetRunner,
            RibbonStyle::Invisible,
        ] {
            for map in ["test_oval", "toy_town"] {
                let id = "fresh".into();
                let env = EnvironmentDef::load(&assets(), map).unwrap();
                let doc = new_track(&id, "Fresh", "toy_store", env, style);
                validate(&doc, &props, &id).unwrap_or_else(|e| panic!("{style:?}: {e}"));
            }
        }
    }

    #[test]
    fn a_layout_that_does_not_build_is_rejected_with_its_variant() {
        let (mut doc, props) = oval();
        doc.def.points[5].pos = doc.def.points[6].pos; // two points on one spot
        let err = validate(&doc, &props, &"test_oval".into()).unwrap_err();
        assert!(err.contains("layout"), "{err}");
    }

    #[test]
    fn markers_show_layout_only_with_layout_and_reverse_only_items_dimmed() {
        let (mut doc, _) = oval();
        let is_point =
            |(item, ..): &(EditorItem, Vec3, bool)| matches!(item, EditorItem::Point { .. });
        assert_eq!(
            doc.markers(false, TimeOfDay::Noon)
                .iter()
                .filter(|h| is_point(h))
                .count(),
            0
        );
        let points = doc
            .markers(true, TimeOfDay::Noon)
            .into_iter()
            .filter(is_point)
            .count();
        assert_eq!(points, doc.def.points.len());
        let row = EditorItem::ItemRow { index: 0 };
        assert!(
            doc.markers(false, TimeOfDay::Noon)
                .iter()
                .any(|(item, ..)| *item == row)
        );
        let prop = EditorItem::Prop { index: 0 };
        let marked = |doc: &Doc| {
            doc.markers(false, TimeOfDay::Noon)
                .into_iter()
                .find(|(i, ..)| *i == prop)
        };
        assert_eq!(marked(&doc), None, "a prop in the session has its mesh");
        doc.def.props[0].dir = Dir::Rev;
        assert_eq!(marked(&doc), Some((prop, doc.def.props[0].pos, true)));
        let vertex = EditorItem::Vertex {
            ground: 0,
            index: 0,
        };
        doc.set_handle(vertex, Vec3::new(1.0, 99.0, 2.0));
        let h = doc.def.ground[0].height;
        assert_eq!(
            doc.handle(vertex),
            Some(Vec3::new(1.0, h, 2.0)),
            "keeps the height"
        );
        let copy = doc.duplicate(EditorItem::Prop { index: 0 }).unwrap();
        assert_eq!(
            doc.handle(copy).unwrap(),
            doc.def.props[0].pos + Vec3::X * DUPLICATE_OFFSET_U
        );
    }

    #[test]
    fn poses_set_only_what_the_data_stores() {
        let (mut doc, _) = oval();
        doc.decor.push(DecorItem {
            model: "a.glb".into(),
            pos: Vec3::ZERO,
            rot: Vec3::new(0.0, 270.0, 0.0),
            scale: Vec3::ONE,
            dir: Dir::Both,
        });
        let decor = EditorItem::Decor { index: 0 };
        let (pose, turn, scales) = doc.pose(decor).unwrap();
        assert_eq!((turn, scales), (Turn::Free, true));
        let moved = Pose {
            pos: Vec3::new(1.23456, 0.0, -2.0),
            ..pose
        };
        doc.set_pose(decor, moved);
        assert_eq!(doc.decor[0].pos, Vec3::new(1.235, 0.0, -2.0));
        assert_eq!(
            doc.decor[0].rot.y, 270.0,
            "a move keeps the angles as written"
        );
        let turned = Pose {
            rot: Quat::from_euler(EulerRot::YXZ, 0.5, 0.25, -0.1),
            scale: Vec3::new(1.0, 2.0, 0.5),
            ..moved
        };
        doc.set_pose(decor, turned);
        let back = doc.pose(decor).unwrap().0;
        assert!(back.rot.abs_diff_eq(turned.rot, 1e-3), "{back:?}");
        assert_eq!(doc.decor[0].scale, Vec3::new(1.0, 2.0, 0.5));

        let prop = EditorItem::Prop { index: 0 };
        let (pose, turn, scales) = doc.pose(prop).unwrap();
        assert_eq!((turn, scales), (Turn::Yaw, false));
        doc.set_pose(
            prop,
            Pose {
                rot: Quat::from_rotation_y(90f32.to_radians()),
                scale: Vec3::splat(3.0),
                ..pose
            },
        );
        assert_eq!(doc.def.props[0].yaw, 90.0);
        assert_eq!(doc.pose(EditorItem::Gap { index: 0 }), None);
    }
}
