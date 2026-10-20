//! The transform gizmo (D7): drawn with `Gizmos` on the selection, its handles picked in screen
//! space. Move = three arrows + three plane squares; rotate = three rings; scale = three
//! cube-tipped axes + a centre cube (uniform). Each item shows only what its data stores
//! ([`Doc::pose`]): props turn about the vertical only, points and rows only move; a mode the
//! item lacks shows the move gizmo.
//!
//! G · R · C pick the mode, L the space (world or the item's own axes; scale is always local).
//! Ctrl during a drag snaps (0.5 u, 15°, 0.1). A drag writes rc_track's `DragPreview` (props and
//! decor follow it live; markers and the control polygon are drawn from a copy of the document;
//! layout handles get a `TrackPreview`, [`crate::live`]) and commits one edit on release. The
//! preview stays until the commit is in the session (in place: the same frame the session's
//! `Track` is replaced, [`crate::live::apply`]), so the item never flashes back to where it was.

use bevy::gizmos::transform_gizmo::{
    intersect_plane, point_to_ring_screen_dist, point_to_segment_dist, translation_plane_normal,
};
use bevy::prelude::*;
use rc_core::prelude::*;
use rc_track::{DragPreview, DragSource};
use rc_ui::editor::{EditorGizmo, EditorItem, EditorTab};

use crate::doc::{Doc, Pose, Turn};
use crate::{Cmd, Editor, EditorCommand, Mode};

/// The gizmo is this fraction of its distance from the camera (constant on screen).
const SIZE_PER_U: f32 = 0.2;
/// Handle geometry, in gizmo sizes.
const ARROW_FROM: f32 = 0.15;
const PLANE_AT: f32 = 0.35;
const PLANE_HALF: f32 = 0.09;
const RING: f32 = 0.85;
const STRETCH: f32 = 0.85;
const TIP_CUBE: f32 = 0.1;
const CENTRE_CUBE: f32 = 0.16;
/// A press picks handles this close to the cursor (logical px).
const PICK_PX: f32 = 10.0;
/// Ctrl snaps to these steps.
const SNAP_U: f32 = 0.5;
const SNAP_DEG: f32 = 15.0;
const SNAP_SCALE: f32 = 0.1;
/// Smallest scale a drag makes.
const MIN_SCALE: f32 = 0.01;
/// The uniform-scale cube doubles the scale per this many px of drag (right/up grows).
const UNIFORM_PX_PER_DOUBLING: f32 = 150.0;

const AXES: [Color; 3] = [
    Color::srgb(0.95, 0.2, 0.3),
    Color::srgb(0.3, 0.9, 0.3),
    Color::srgb(0.25, 0.5, 1.0),
];
const HOT: Color = Color::srgb(1.0, 0.9, 0.1);
const CENTRE: Color = Color::srgb(0.9, 0.9, 0.9);

/// A part of the gizmo; `usize` is an axis (0 = X, 1 = Y, 2 = Z).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Handle {
    /// Move along the axis.
    Arrow(usize),
    /// Move in the plane normal to the axis.
    Plane(usize),
    /// Turn about the axis.
    Ring(usize),
    /// Scale along the (local) axis.
    Stretch(usize),
    /// Scale uniformly.
    Uniform,
}

/// Where a mouse drag grabbed its handle.
#[derive(Debug, Clone, Copy)]
struct Grab {
    handle: Handle,
    /// The gizmo's axes and size at the press.
    frame: Quat,
    /// Along the axis (arrows, stretch), the plane hit (planes) or the direction from the centre
    /// (rings), at the press.
    at: Vec3,
    press: Vec2,
}

/// A live drag of the selection.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Drag {
    item: EditorItem,
    tab: EditorTab,
    start: Pose,
    pub(crate) live: Pose,
    /// `None`: scripted (`EditorCommand::Drag { hold }`), not following the mouse.
    grab: Option<Grab>,
}

/// Mode, space and the live drag.
#[derive(Resource, Debug, Default)]
pub(crate) struct Gizmo {
    mode: EditorGizmo,
    /// The item's own axes instead of the world's (move and rotate).
    local: bool,
    pub(crate) drag: Option<Drag>,
    /// The handle under the cursor.
    pub(crate) hot: Option<Handle>,
    /// A drag was committed: the preview stays until the commit is in the session.
    settling: bool,
}

impl Gizmo {
    /// The document as the live drag shows it.
    pub(crate) fn preview(&self, doc: &Doc) -> Option<Doc> {
        let drag = self.drag.as_ref()?;
        let mut copy = doc.clone();
        copy.set_pose(drag.item, drag.live);
        Some(copy)
    }

    /// The document as a live drag of a layout handle (control point, ground vertex, shortcut
    /// point) that has moved shows it: what the road preview is built from.
    pub(crate) fn layout_preview(&self, doc: &Doc) -> Option<Doc> {
        let drag = self.drag.as_ref().filter(|d| d.live != d.start)?;
        let layout = matches!(
            drag.item,
            EditorItem::Point { .. } | EditorItem::Vertex { .. } | EditorItem::PathPoint { .. }
        );
        layout.then(|| self.preview(doc)).flatten()
    }

    /// A committed drag's preview waits for the commit to reach the session.
    pub(crate) fn settling(&self) -> bool {
        self.settling
    }

    fn cancel(&mut self) -> bool {
        self.drag.take().is_some()
    }
}

/// The handles of `mode` for an item that turns `turn` and may scale.
fn handles(mode: EditorGizmo, turn: Turn, scales: bool) -> Vec<Handle> {
    match mode {
        EditorGizmo::Rotate if turn == Turn::Yaw => vec![Handle::Ring(1)],
        EditorGizmo::Rotate if turn == Turn::Free => (0..3).map(Handle::Ring).collect(),
        EditorGizmo::Scale if scales => {
            let mut h: Vec<_> = (0..3).map(Handle::Stretch).collect();
            h.push(Handle::Uniform);
            h
        }
        _ => (0..3)
            .map(Handle::Arrow)
            .chain((0..3).map(Handle::Plane))
            .collect(),
    }
}

/// The axes `handle` works in: scale always the item's, a yaw ring the world's.
fn frame(handle: Handle, local: bool, turn: Turn, pose: &Pose) -> Quat {
    match handle {
        Handle::Stretch(_) | Handle::Uniform => pose.rot,
        Handle::Ring(_) if turn == Turn::Yaw => Quat::IDENTITY,
        _ if local => pose.rot,
        _ => Quat::IDENTITY,
    }
}

fn axis(frame: Quat, i: usize) -> Vec3 {
    frame * Vec3::AXES[i]
}

/// Where along `axis` (through `origin`) the ray points, on the plane through the axis that
/// faces the camera most.
pub(crate) fn along_axis(ray: Ray3d, origin: Vec3, axis: Vec3) -> Option<f32> {
    let hit = intersect_plane(ray, translation_plane_normal(ray, axis), origin)?;
    Some((hit - origin).dot(axis))
}

/// The direction from `origin` to where the ray crosses the plane normal to `axis`.
pub(crate) fn ring_dir(ray: Ray3d, origin: Vec3, axis: Vec3) -> Option<Vec3> {
    let hit = intersect_plane(ray, axis, origin)?;
    (hit - origin).try_normalize()
}

/// Signed angle (radians) from `from` to `to` about `axis`.
pub(crate) fn angle_about(from: Vec3, to: Vec3, axis: Vec3) -> f32 {
    axis.dot(from.cross(to)).atan2(from.dot(to))
}

fn snap(v: f32, step: f32) -> f32 {
    (v / step).round() * step
}

/// The press grabs `handle` of the gizmo at `start`.
fn grab(handle: Handle, frame: Quat, start: &Pose, ray: Ray3d, cursor: Vec2) -> Option<Grab> {
    let at = match handle {
        Handle::Arrow(i) | Handle::Stretch(i) => {
            Vec3::splat(along_axis(ray, start.pos, axis(frame, i))?)
        }
        Handle::Plane(i) => intersect_plane(ray, axis(frame, i), start.pos)?,
        Handle::Ring(i) => ring_dir(ray, start.pos, axis(frame, i))?,
        Handle::Uniform => Vec3::ZERO,
    };
    Some(Grab {
        handle,
        frame,
        at,
        press: cursor,
    })
}

/// The pose a drag from `start` shows with the cursor at `cursor` (ray `ray`); `snap`: Ctrl.
fn dragged(
    start: &Pose,
    turn: Turn,
    g: &Grab,
    ray: Ray3d,
    cursor: Vec2,
    snaps: bool,
) -> Option<Pose> {
    let mut pose = *start;
    let world = g.frame == Quat::IDENTITY;
    // Snaps the moved coordinates to the grid (world axes) or the move itself (the item's).
    let snap_move = |pos: Vec3, moved: [bool; 3]| {
        if !snaps {
            return pos;
        }
        let snapped = |v: Vec3| {
            Vec3::from_array(std::array::from_fn(|k| {
                if moved[k] { snap(v[k], SNAP_U) } else { v[k] }
            }))
        };
        if world {
            snapped(pos)
        } else {
            start.pos + g.frame * snapped(g.frame.inverse() * (pos - start.pos))
        }
    };
    match g.handle {
        Handle::Arrow(i) => {
            let d = along_axis(ray, start.pos, axis(g.frame, i))? - g.at.x;
            let moved = std::array::from_fn(|k| k == i);
            pose.pos = snap_move(start.pos + axis(g.frame, i) * d, moved);
        }
        Handle::Plane(i) => {
            let hit = intersect_plane(ray, axis(g.frame, i), start.pos)?;
            let moved = std::array::from_fn(|k| k != i);
            pose.pos = snap_move(start.pos + hit - g.at, moved);
        }
        Handle::Ring(i) => {
            let a = axis(g.frame, i);
            let mut angle = angle_about(g.at, ring_dir(ray, start.pos, a)?, a);
            if snaps && turn == Turn::Yaw {
                // Props: whole steps of yaw, not of the turn.
                let yaw = start.rot.to_euler(EulerRot::YXZ).0 + angle;
                angle += snap(yaw.to_degrees(), SNAP_DEG).to_radians() - yaw;
            } else if snaps {
                angle = snap(angle.to_degrees(), SNAP_DEG).to_radians();
            }
            pose.rot = (Quat::from_axis_angle(a, angle) * start.rot).normalize();
        }
        Handle::Stretch(i) => {
            let now = along_axis(ray, start.pos, axis(g.frame, i))?;
            if g.at.x.abs() < 1e-4 {
                return None;
            }
            let s = (start.scale[i] * now / g.at.x).max(MIN_SCALE);
            pose.scale[i] = if snaps {
                snap(s, SNAP_SCALE).max(SNAP_SCALE)
            } else {
                s
            };
        }
        Handle::Uniform => {
            let d = cursor - g.press;
            let factor = ((d.x - d.y) / UNIFORM_PX_PER_DOUBLING).exp2();
            let s = (start.scale * factor).max(Vec3::splat(MIN_SCALE));
            pose.scale = if snaps {
                s.map(|v| snap(v, SNAP_SCALE).max(SNAP_SCALE))
            } else {
                s
            };
        }
    }
    Some(pose)
}

/// The camera, for drawing and picking.
pub(crate) struct Eye<'a> {
    pub camera: &'a Camera,
    pub at: &'a GlobalTransform,
}

impl Eye<'_> {
    fn size(&self, origin: Vec3) -> f32 {
        self.at.translation().distance(origin) * SIZE_PER_U
    }

    fn screen(&self, p: Vec3) -> Option<Vec2> {
        self.camera.world_to_viewport(self.at, p).ok()
    }
}

/// The gizmo of the selection (where the live drag has it), if it has one.
fn shown(editor: &Editor, gizmo: &Gizmo) -> Option<(EditorItem, Pose, Turn, Vec<Handle>)> {
    let item = editor.selection?;
    let layout = matches!(
        item,
        EditorItem::Point { .. } | EditorItem::Vertex { .. } | EditorItem::PathPoint { .. }
    );
    if layout && editor.tab != EditorTab::Track {
        return None; // the layout's handles are the Track tab's
    }
    let (pose, turn, scales) = editor.doc.pose(item)?;
    let pose = gizmo
        .drag
        .as_ref()
        .filter(|d| d.item == item)
        .map_or(pose, |d| d.live);
    let handles = match gizmo.drag.as_ref().and_then(|d| d.grab) {
        Some(g) => vec![g.handle], // only the dragged one
        None => handles(gizmo.mode, turn, scales),
    };
    Some((item, pose, turn, handles))
}

/// Screen distance from the cursor to `handle` (px), `None` when off screen.
fn distance(eye: &Eye, handle: Handle, frame: Quat, origin: Vec3, cursor: Vec2) -> Option<f32> {
    let s = eye.size(origin);
    let segment = |i: usize, from: f32, to: f32| {
        let a = eye.screen(origin + axis(frame, i) * from * s)?;
        let b = eye.screen(origin + axis(frame, i) * to * s)?;
        Some(point_to_segment_dist(cursor, a, b))
    };
    match handle {
        Handle::Arrow(i) => segment(i, ARROW_FROM, 1.0),
        Handle::Stretch(i) => segment(i, ARROW_FROM, STRETCH + TIP_CUBE),
        Handle::Plane(i) => {
            let centre = origin + plane_centre(frame, i) * s;
            let c = eye.screen(centre)?;
            let corner = eye.screen(
                centre + (axis(frame, (i + 1) % 3) + axis(frame, (i + 2) % 3)) * PLANE_HALF * s,
            )?;
            let d = cursor.distance(c);
            Some(if d <= c.distance(corner) { 0.0 } else { d })
        }
        Handle::Ring(i) => {
            let a = axis(frame, i);
            Some(point_to_ring_screen_dist(
                cursor,
                eye.camera,
                eye.at,
                origin,
                a,
                RING * s,
            ))
        }
        Handle::Uniform => Some(cursor.distance(eye.screen(origin)?)),
    }
}

fn plane_centre(frame: Quat, i: usize) -> Vec3 {
    (axis(frame, (i + 1) % 3) + axis(frame, (i + 2) % 3)) * PLANE_AT
}

/// The gizmo handle under the cursor.
pub(crate) fn handle_at(editor: &Editor, gizmo: &Gizmo, eye: &Eye, cursor: Vec2) -> Option<Handle> {
    let (_, pose, turn, handles) = shown(editor, gizmo)?;
    handles
        .into_iter()
        .filter_map(|h| {
            let d = distance(eye, h, frame(h, gizmo.local, turn, &pose), pose.pos, cursor)?;
            (d <= PICK_PX).then_some((h, d))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(h, _)| h)
}

/// A left press on `handle` starts a drag of the selection.
pub(crate) fn press(editor: &Editor, gizmo: &mut Gizmo, handle: Handle, ray: Ray3d, cursor: Vec2) {
    let Some((item, pose, turn, _)) = shown(editor, gizmo) else {
        return;
    };
    let f = frame(handle, gizmo.local, turn, &pose);
    if let Some(g) = grab(handle, f, &pose, ray, cursor) {
        gizmo.drag = Some(Drag {
            item,
            tab: editor.tab,
            start: pose,
            live: pose,
            grab: Some(g),
        });
    }
}

/// The left button is held: the live pose follows the cursor (`snaps`: Ctrl).
pub(crate) fn follow(editor: &Editor, gizmo: &mut Gizmo, ray: Ray3d, cursor: Vec2, snaps: bool) {
    let Some(drag) = gizmo.drag.as_mut() else {
        return;
    };
    let Some(g) = drag.grab else {
        return; // scripted
    };
    let turn = editor.doc.pose(drag.item).map_or(Turn::No, |p| p.1);
    if let Some(pose) = dragged(&drag.start, turn, &g, ray, cursor, snaps) {
        drag.live = pose;
    }
}

/// The left button was let go: a mouse drag commits.
pub(crate) fn release(editor: &mut Editor, gizmo: &mut Gizmo) {
    if gizmo.drag.is_some_and(|d| d.grab.is_some()) {
        commit(editor, gizmo);
    }
}

/// Applies the live drag as one edit.
fn commit(editor: &mut Editor, gizmo: &mut Gizmo) {
    let Some(drag) = gizmo.drag else {
        return;
    };
    if drag.live != drag.start {
        let commits = editor.commits;
        editor.edit(|doc, _| {
            doc.set_pose(drag.item, drag.live);
            Ok(())
        });
        gizmo.settling = editor.commits != commits; // accepted: it reaches the session soon
    }
    gizmo.drag = None;
}

/// `editor.command`s of the gizmo, and the commands that cancel a live drag or placement first
/// (undo, redo, delete, test drive; the drag also ends when the selection, tab or mode changes: [`sync`]).
pub(crate) fn commands(
    mut commands: MessageReader<Cmd>,
    mut editor: ResMut<Editor>,
    mut gizmo: ResMut<Gizmo>,
) {
    for Cmd(command) in commands.read() {
        match command {
            EditorCommand::Gizmo { mode } => gizmo.mode = *mode,
            EditorCommand::GizmoSpace => gizmo.local = !gizmo.local,
            EditorCommand::Cancel => {
                // Esc in a test drive is the game's pause.
                if editor.placing.is_some() {
                    editor.placing = None;
                } else if !gizmo.cancel() && editor.mode == Mode::Editing {
                    editor.select(None);
                }
            }
            EditorCommand::Undo
            | EditorCommand::Redo
            | EditorCommand::Delete
            | EditorCommand::Drive => {
                gizmo.cancel();
                if editor.placing.is_some() {
                    editor.placing = None;
                }
            }
            EditorCommand::Drag {
                translate,
                rotate,
                scale,
                hold,
            } => scripted(&mut editor, &mut gizmo, *translate, *rotate, *scale, *hold),
            _ => {}
        }
    }
}

/// `EditorCommand::Drag`: what a gizmo drag by these amounts would do.
fn scripted(
    editor: &mut Editor,
    gizmo: &mut Gizmo,
    translate: [f32; 3],
    rotate: [f32; 3],
    scale: [f32; 3],
    hold: bool,
) {
    let Some(item) = editor.selection else {
        return;
    };
    let Some((start, turn, scales)) = editor.doc.pose(item) else {
        return;
    };
    let [pitch, yaw, roll] = rotate.map(f32::to_radians);
    let turned = match turn {
        Turn::No => Quat::IDENTITY,
        Turn::Yaw => Quat::from_rotation_y(yaw),
        Turn::Free => Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll),
    };
    let live = Pose {
        pos: start.pos + Vec3::from(translate),
        rot: start.rot * turned,
        scale: if scales {
            start.scale * Vec3::from(scale)
        } else {
            start.scale
        },
    };
    gizmo.drag = Some(Drag {
        item,
        tab: editor.tab,
        start,
        live,
        grab: None,
    });
    if !hold {
        commit(editor, gizmo);
    }
}

/// Every frame in a session (editing or not): ends a drag the editor left (mode, tab, selection,
/// a bake), and writes the `DragPreview` (kept until a committed drag is in the session).
pub(crate) fn sync(
    editor: Res<Editor>,
    state: Res<State<AppState>>,
    mut gizmo: ResMut<Gizmo>,
    mut preview: ResMut<DragPreview>,
) {
    let stale = |d: &Drag| {
        editor.mode != Mode::Editing
            || editor.bake.is_some()
            || d.tab != editor.tab
            || editor.selection != Some(d.item)
    };
    if gizmo.drag.as_ref().is_some_and(stale) {
        gizmo.drag = None;
    }
    if gizmo.settling {
        let applied = editor.applied == editor.commits;
        if !(applied && editor.session_current() && *state.get() == AppState::InRace) {
            return;
        }
        gizmo.settling = false;
    }
    let shown = gizmo.drag.filter(|d| d.live != d.start).and_then(|d| {
        let source = match d.item {
            EditorItem::Prop { index } => DragSource::Prop(index as usize),
            EditorItem::Decor { index } => DragSource::Decor(index as usize),
            _ => return None,
        };
        Some((
            source,
            Transform::from_translation(d.live.pos)
                .with_rotation(d.live.rot)
                .with_scale(d.live.scale),
        ))
    });
    preview.set_if_neq(DragPreview(shown));
}

/// Draws the selection's gizmo.
pub(crate) fn draw(editor: &Editor, gizmo: &Gizmo, eye: &Eye, gizmos: &mut Gizmos) {
    let Some((_, pose, turn, handles)) = shown(editor, gizmo) else {
        return;
    };
    let (o, s) = (pose.pos, eye.size(pose.pos));
    let hot = gizmo
        .drag
        .and_then(|d| d.grab)
        .map(|g| g.handle)
        .or(gizmo.hot);
    for h in handles {
        let f = frame(h, gizmo.local, turn, &pose);
        let tint = |i: usize| if hot == Some(h) { HOT } else { AXES[i] };
        match h {
            Handle::Arrow(i) => {
                gizmos.arrow(o + axis(f, i) * ARROW_FROM * s, o + axis(f, i) * s, tint(i));
            }
            Handle::Plane(i) => {
                let (u, v) = (axis(f, (i + 1) % 3), axis(f, (i + 2) % 3));
                let c = o + plane_centre(f, i) * s;
                let r = PLANE_HALF * s;
                gizmos.lineloop(
                    [
                        c + (u + v) * r,
                        c + (u - v) * r,
                        c - (u + v) * r,
                        c - (u - v) * r,
                    ],
                    tint(i),
                );
            }
            Handle::Ring(i) => {
                let iso = Isometry3d::new(o, Quat::from_rotation_arc(Vec3::Z, axis(f, i)));
                gizmos.circle(iso, RING * s, tint(i)).resolution(64);
            }
            Handle::Stretch(i) => {
                let tip = o + axis(f, i) * STRETCH * s;
                gizmos.line(o + axis(f, i) * ARROW_FROM * s, tip, tint(i));
                let cube = Transform::from_translation(tip)
                    .with_rotation(f)
                    .with_scale(Vec3::splat(TIP_CUBE * s));
                gizmos.cube(cube, tint(i));
            }
            Handle::Uniform => {
                let colour = if hot == Some(h) { HOT } else { CENTRE };
                let cube = Transform::from_translation(o)
                    .with_rotation(f)
                    .with_scale(Vec3::splat(CENTRE_CUBE * s));
                gizmos.cube(cube, colour);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A ray from `from` through `to`.
    fn ray(from: Vec3, to: Vec3) -> Ray3d {
        Ray3d::new(from, Dir3::new(to - from).unwrap())
    }

    #[test]
    fn an_axis_drag_follows_the_point_under_the_cursor_along_the_axis() {
        let eye = Vec3::new(3.0, 10.0, 12.0);
        // The cursor over (5, 0, 0) is 5 along X; over (2, 0, 0), 3 along X from (−1, 0, 0).
        let x = |p: Vec3, o: Vec3| along_axis(ray(eye, p), o, Vec3::X).unwrap();
        assert!((x(Vec3::X * 5.0, Vec3::ZERO) - 5.0).abs() < 1e-4);
        assert!((x(Vec3::X * 2.0, Vec3::NEG_X) - 3.0).abs() < 1e-4);
        // A drag of the X arrow from x = 1 to the cursor over x = 4: moved by 3, snapped grid.
        let start = Pose {
            pos: Vec3::new(0.2, 0.0, 0.0),
            rot: Quat::IDENTITY,
            scale: Vec3::ONE,
        };
        let g = grab(
            Handle::Arrow(0),
            Quat::IDENTITY,
            &start,
            ray(eye, Vec3::new(1.2, 0.0, 0.0)),
            Vec2::ZERO,
        )
        .unwrap();
        let to = ray(eye, Vec3::new(4.3, 0.0, 0.0));
        let p = dragged(&start, Turn::No, &g, to, Vec2::ZERO, false).unwrap();
        assert!(p.pos.distance(Vec3::new(3.3, 0.0, 0.0)) < 1e-3, "{p:?}");
        let p = dragged(&start, Turn::No, &g, to, Vec2::ZERO, true).unwrap();
        assert!(
            p.pos.distance(Vec3::new(3.5, 0.0, 0.0)) < 1e-3,
            "snapped: {p:?}"
        );
    }

    #[test]
    fn a_ring_drag_turns_by_the_angle_swept_about_its_axis() {
        let eye = Vec3::new(0.0, 20.0, 0.1);
        let start = Pose {
            pos: Vec3::ZERO,
            rot: Quat::from_rotation_y(0.2),
            scale: Vec3::ONE,
        };
        // From +X to −Z about +Y is a quarter turn the right-handed way.
        let g = grab(
            Handle::Ring(1),
            Quat::IDENTITY,
            &start,
            ray(eye, Vec3::X),
            Vec2::ZERO,
        )
        .unwrap();
        let p = dragged(
            &start,
            Turn::Free,
            &g,
            ray(eye, Vec3::NEG_Z),
            Vec2::ZERO,
            false,
        )
        .unwrap();
        let yaw = p.rot.to_euler(EulerRot::YXZ).0;
        assert!(
            (yaw - (0.2 + std::f32::consts::FRAC_PI_2)).abs() < 1e-3,
            "{yaw}"
        );
        // Props snap their yaw to whole 15° steps.
        let p = dragged(
            &start,
            Turn::Yaw,
            &g,
            ray(eye, Vec3::new(1.0, 0.0, -0.9)),
            Vec2::ZERO,
            true,
        )
        .unwrap();
        let yaw = p.rot.to_euler(EulerRot::YXZ).0.to_degrees();
        assert!((yaw / 15.0 - (yaw / 15.0).round()).abs() < 1e-3, "{yaw}");
        assert!(
            (angle_about(Vec3::X, Vec3::Z, Vec3::Y) + std::f32::consts::FRAC_PI_2).abs() < 1e-5
        );
    }

    #[test]
    fn scale_handles_stretch_one_axis_or_all() {
        let eye = Vec3::new(0.0, 10.0, 10.0);
        let start = Pose {
            pos: Vec3::ZERO,
            rot: Quat::IDENTITY,
            scale: Vec3::new(1.0, 2.0, 1.0),
        };
        let g = grab(
            Handle::Stretch(0),
            Quat::IDENTITY,
            &start,
            ray(eye, Vec3::X),
            Vec2::ZERO,
        )
        .unwrap();
        let p = dragged(
            &start,
            Turn::Free,
            &g,
            ray(eye, Vec3::X * 3.0),
            Vec2::ZERO,
            false,
        )
        .unwrap();
        assert!(p.scale.distance(Vec3::new(3.0, 2.0, 1.0)) < 1e-3, "{p:?}");
        let g = grab(
            Handle::Uniform,
            Quat::IDENTITY,
            &start,
            ray(eye, Vec3::ZERO),
            Vec2::new(100.0, 100.0),
        )
        .unwrap();
        let right = Vec2::new(100.0 + UNIFORM_PX_PER_DOUBLING, 100.0);
        let p = dragged(&start, Turn::Free, &g, ray(eye, Vec3::ZERO), right, false).unwrap();
        assert!(p.scale.distance(start.scale * 2.0) < 1e-3, "{p:?}");
    }

    #[test]
    fn items_show_only_the_handles_their_data_stores() {
        assert_eq!(
            handles(EditorGizmo::Rotate, Turn::Yaw, false),
            vec![Handle::Ring(1)]
        );
        assert_eq!(handles(EditorGizmo::Rotate, Turn::Free, true).len(), 3);
        assert_eq!(handles(EditorGizmo::Scale, Turn::Free, true).len(), 4);
        let moves = handles(EditorGizmo::Move, Turn::No, false);
        assert_eq!(
            handles(EditorGizmo::Rotate, Turn::No, false),
            moves,
            "points only move"
        );
        assert_eq!(
            handles(EditorGizmo::Scale, Turn::Yaw, false),
            moves,
            "props do not scale"
        );
    }
}
