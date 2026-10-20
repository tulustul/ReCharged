//! The 3D view while editing: the fly camera, the handles drawn as gizmos, picking and dragging
//! them, and the view's focus (the point under the screen centre, where additions land).
//!
//! Mouse: left click picks the nearest handle (or clears the selection), left drag moves it on
//! its horizontal plane (Alt: up/down; Ctrl: snap to [`SNAP_U`]); right drag looks around.
//! Keys: WASD move, Q/E down/up, Shift faster.

use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_rapier3d::prelude::{QueryFilter, ReadRapierContext};
use bevy_react::PointerCapture;
use rc_core::prelude::*;
use rc_track::Track;
use rc_ui::editor::EditorItem;

use crate::doc::Doc;
use crate::{Editor, Mode, Typing};

/// Fly speed (u/s; ×[`FAST`] with Shift).
const FLY_SPEED: f32 = 24.0;
/// Shift multiplier of the fly speed.
const FAST: f32 = 4.0;
/// Mouse look (radians per pixel of motion while the right button is held).
const LOOK_RAD_PER_PX: f32 = 0.004;
/// The first view: this far behind and above the first grid slot (u).
const START_VIEW_BACK_U: f32 = 22.0;
const START_VIEW_UP_U: f32 = 14.0;
/// Focus (F) and new views: this far behind and above the target (u).
const FOCUS_BACK_U: f32 = 14.0;
const FOCUS_UP_U: f32 = 9.0;
/// The top view (T) is this high above its target (u).
const TOP_VIEW_U: f32 = 70.0;
/// A click picks handles within this distance of the cursor (logical px).
const PICK_RADIUS_PX: f32 = 16.0;
/// The cursor must move this far before a press becomes a drag (logical px).
const DRAG_START_PX: f32 = 4.0;
/// Ctrl-drag snaps to this grid (u).
pub const SNAP_U: f32 = 0.5;
/// The focus ray reaches this far (u).
const FOCUS_REACH_U: f32 = 400.0;
/// Handle sizes (u).
const HANDLE_U: f32 = 0.45;
const SELECTED_U: f32 = 0.8;

const LINE: Color = Color::srgb(0.1, 0.85, 1.0);
const GAP: Color = Color::srgb(1.0, 0.25, 0.3);
const GROUND: Color = Color::srgb(0.3, 0.9, 0.4);
const SHORTCUT: Color = Color::srgb(1.0, 0.6, 0.1);
const PROP: Color = Color::srgb(0.95, 0.95, 0.95);
const ROW: Color = Color::srgb(0.4, 0.6, 1.0);
const BATTERY: Color = Color::srgb(1.0, 0.82, 0.3);
const DECOR: Color = Color::srgb(0.9, 0.3, 0.9);
const SELECTED: Color = Color::srgb(0.95, 0.88, 0.06);
const START: Color = Color::WHITE;

/// The 3D view of the editor (needs a window, gizmos and rapier; [`crate::EditorPlugin`] first).
pub struct EditorViewPlugin;

impl Plugin for EditorViewPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Dragging>()
            .add_systems(Startup, gizmo_look)
            .add_systems(Update, cutaway.run_if(in_state(AppState::InRace)))
            .add_systems(
                Update,
                (focus_point, pick_and_drag, fly, draw)
                    .chain()
                    .after(CameraSet)
                    .run_if(in_state(AppState::InRace))
                    .run_if(editing),
            );
    }
}

fn editing(editor: Res<Editor>) -> bool {
    editor.mode == Mode::Editing
}

/// The fly camera's pose (kept across test drives and session reloads).
#[derive(Resource, Debug, Default)]
pub struct View {
    pose: Option<Transform>,
}

impl View {
    /// Look straight down at `target` (else the track's middle), north up.
    pub fn top(&mut self, target: Option<Vec3>, track: Option<&Track>) {
        let target = target.or_else(|| track.map(|t| (t.bounds().0 + t.bounds().1) / 2.0));
        if let Some(target) = target {
            self.pose = Some(
                Transform::from_translation(target + Vec3::Y * TOP_VIEW_U)
                    .looking_to(Vec3::NEG_Y, Vec3::NEG_Z),
            );
        }
    }

    /// Look at `target` from behind and above, keeping the heading.
    pub fn look_at(&mut self, target: Vec3) {
        let heading = self
            .pose
            .map(|p| p.forward().as_vec3().with_y(0.0).normalize_or(Vec3::NEG_Z))
            .unwrap_or(Vec3::NEG_Z);
        self.pose = Some(
            Transform::from_translation(target - heading * FOCUS_BACK_U + Vec3::Y * FOCUS_UP_U)
                .looking_at(target, Vec3::Y),
        );
    }
}

/// Where the view looks: the first surface under the screen centre (else the floor plane).
#[derive(Resource, Debug, Default, PartialEq)]
pub struct ViewFocus(pub Option<Vec3>);

/// A point of `item` to look at: its handle, or a representative point of items without one.
pub fn anchor(doc: &Doc, item: EditorItem) -> Option<Vec3> {
    let d = &doc.def;
    doc.handle(item).or_else(|| match item {
        EditorItem::Track => d.points.get(d.start).map(|p| p.pos),
        EditorItem::Gap { index } => d
            .gaps
            .get(index as usize)
            .and_then(|g| d.points.get(g.from))
            .map(|p| p.pos),
        EditorItem::Ground { index } => d.ground.get(index as usize).map(|g| {
            let c = g.points.iter().sum::<Vec2>() / g.points.len().max(1) as f32;
            Vec3::new(c.x, g.height, c.y)
        }),
        EditorItem::Shortcut { index } => d
            .shortcuts
            .get(index as usize)
            .and_then(|s| s.path.get(s.path.len() / 2).copied()),
        _ => None,
    })
}

/// A handle being dragged.
#[derive(Debug, Clone, Copy)]
struct Drag {
    item: EditorItem,
    /// The handle where the press began.
    from: Vec3,
    /// Where it would go now.
    to: Vec3,
    /// Handle minus the cursor's ground point at the press (keeps the grab point under the
    /// cursor).
    offset: Vec3,
    press: Vec2,
    moved: bool,
}

#[derive(Resource, Debug, Default)]
struct Dragging(Option<Drag>);

/// Editing looks into rooms from above: the ceiling and lamp fixtures hide (they cast no shadows,
/// so the lighting stays the same); test drives show them.
fn cutaway(editor: Res<Editor>, mut overhead: Query<&mut Visibility, With<rc_render::Overhead>>) {
    let shown = if editor.mode == Mode::Editing {
        Visibility::Hidden
    } else {
        Visibility::Inherited
    };
    for mut visibility in &mut overhead {
        visibility.set_if_neq(shown);
    }
}

/// Gizmos draw on top of the scene, a little thicker than the default.
fn gizmo_look(mut store: ResMut<GizmoConfigStore>) {
    let (config, _) = store.config_mut::<DefaultGizmoConfigGroup>();
    config.depth_bias = -1.0;
    config.line.width = 3.0;
}

fn focus_point(
    cameras: Query<&GlobalTransform, With<GameCamera>>,
    rapier: ReadRapierContext,
    mut focus: ResMut<ViewFocus>,
) {
    let Ok(camera) = cameras.single() else {
        return;
    };
    let (origin, dir) = (camera.translation(), camera.forward().as_vec3());
    let filter = QueryFilter::only_fixed().exclude_sensors();
    let hit = rapier
        .single()
        .ok()
        .and_then(|c| c.cast_ray(origin, dir, FOCUS_REACH_U, true, filter))
        .map(|(_, toi)| origin + dir * toi)
        .or_else(|| (dir.y < -0.05).then(|| origin + dir * (-origin.y / dir.y)));
    focus.set_if_neq(ViewFocus(hit));
}

/// What [`pick_and_drag`] reads.
#[derive(bevy::ecs::system::SystemParam)]
struct Pointer<'w, 's> {
    buttons: Option<Res<'w, ButtonInput<MouseButton>>>,
    keys: Option<Res<'w, ButtonInput<KeyCode>>>,
    capture: Option<Res<'w, PointerCapture>>,
    windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    cameras: Query<'w, 's, (&'static Camera, &'static GlobalTransform), With<GameCamera>>,
}

fn pick_and_drag(pointer: Pointer, mut dragging: ResMut<Dragging>, mut editor: ResMut<Editor>) {
    let (Some(buttons), Some(keys)) = (pointer.buttons.as_deref(), pointer.keys.as_deref()) else {
        return;
    };
    let cursor = pointer
        .windows
        .single()
        .ok()
        .and_then(Window::cursor_position);
    let (Some(cursor), Ok((camera, eye))) = (cursor, pointer.cameras.single()) else {
        return;
    };
    let ground_hit = |at: Vec2, height: f32| {
        let ray = camera.viewport_to_world(eye, at).ok()?;
        let t = ray.intersect_plane(Vec3::Y * height, InfinitePlane3d::new(Vec3::Y))?;
        Some(ray.get_point(t))
    };
    if buttons.just_pressed(MouseButton::Left) {
        if pointer.capture.as_ref().is_some_and(|c| c.is_captured()) {
            return;
        }
        let hit = editor
            .doc
            .handles(&editor.layers)
            .into_iter()
            .filter_map(|(item, pos)| {
                let screen = camera.world_to_viewport(eye, pos).ok()?;
                Some((item, pos, screen.distance(cursor)))
            })
            .filter(|(.., d)| *d <= PICK_RADIUS_PX)
            .min_by(|a, b| a.2.total_cmp(&b.2));
        match hit {
            Some((item, pos, _)) => {
                editor.select(Some(item));
                let offset = ground_hit(cursor, pos.y).map_or(Vec3::ZERO, |g| pos - g);
                dragging.0 = Some(Drag {
                    item,
                    from: pos,
                    to: pos,
                    offset: offset.with_y(0.0),
                    press: cursor,
                    moved: false,
                });
            }
            None => editor.select(None),
        }
        return;
    }
    let Some(drag) = &mut dragging.0 else {
        return;
    };
    if buttons.pressed(MouseButton::Left) {
        drag.moved |= cursor.distance(drag.press) > DRAG_START_PX;
        if !drag.moved {
            return;
        }
        let snap = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
        let snapped = |v: f32| {
            if snap {
                (v / SNAP_U).round() * SNAP_U
            } else {
                v
            }
        };
        if keys.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]) {
            // Up/down on the vertical plane through the handle, facing the camera.
            let normal = eye.forward().as_vec3().with_y(0.0).normalize_or(Vec3::Z);
            let Ok(ray) = camera.viewport_to_world(eye, cursor) else {
                return;
            };
            if let Some(t) = ray.intersect_plane(drag.from, InfinitePlane3d::new(normal)) {
                drag.to = drag.from.with_y(snapped(ray.get_point(t).y));
            }
        } else if let Some(hit) = ground_hit(cursor, drag.from.y) {
            let p = hit + drag.offset;
            drag.to = Vec3::new(snapped(p.x), drag.from.y, snapped(p.z));
        }
    } else {
        let drag = dragging.0.take().expect("checked above");
        if drag.moved && drag.to != drag.from {
            editor.edit(|doc, _| {
                doc.set_handle(drag.item, drag.to);
                Ok(())
            });
        }
    }
}

/// What [`fly`] reads.
#[derive(bevy::ecs::system::SystemParam)]
struct FlyInput<'w, 's> {
    time: Res<'w, Time<Real>>,
    keys: Option<Res<'w, ButtonInput<KeyCode>>>,
    buttons: Option<Res<'w, ButtonInput<MouseButton>>>,
    motion: Option<Res<'w, AccumulatedMouseMotion>>,
    capture: Option<Res<'w, PointerCapture>>,
    typing: Typing<'w, 's>,
}

/// Right mouse button: look; WASD: move; Q/E: down/up; Shift: faster. Writes the game camera.
fn fly(
    input: FlyInput,
    track: Option<Res<Track>>,
    mut view: ResMut<View>,
    mut looking: Local<bool>,
    mut cameras: Query<&mut Transform, With<GameCamera>>,
) {
    let (Some(keys), Some(buttons)) = (input.keys.as_deref(), input.buttons.as_deref()) else {
        return;
    };
    if view.pose.is_none() {
        view.pose = track.as_deref().map(start_view);
    }
    let Some(pose) = view.pose.as_mut() else {
        return;
    };
    if buttons.just_pressed(MouseButton::Right) {
        *looking = !input.capture.as_ref().is_some_and(|c| c.is_captured());
    }
    *looking &= buttons.pressed(MouseButton::Right);
    if *looking && let Some(motion) = &input.motion {
        let (yaw, pitch, _) = pose.rotation.to_euler(EulerRot::YXZ);
        let yaw = yaw - motion.delta.x * LOOK_RAD_PER_PX;
        let pitch = (pitch - motion.delta.y * LOOK_RAD_PER_PX).clamp(-1.55, 1.55);
        pose.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);
    }
    let ctrl = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    if !ctrl && !input.typing.active() {
        let axis = |plus: KeyCode, minus: KeyCode| {
            f32::from(u8::from(keys.pressed(plus))) - f32::from(u8::from(keys.pressed(minus)))
        };
        let dir = pose.forward() * axis(KeyCode::KeyW, KeyCode::KeyS)
            + pose.right() * axis(KeyCode::KeyD, KeyCode::KeyA)
            + Vec3::Y * axis(KeyCode::KeyE, KeyCode::KeyQ);
        let fast = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
        let speed = FLY_SPEED * if fast { FAST } else { 1.0 };
        pose.translation += dir.normalize_or_zero() * speed * input.time.delta_secs();
    }
    for mut camera in &mut cameras {
        *camera = *pose;
    }
}

/// Behind and above the first grid slot, looking down the start straight.
fn start_view(track: &Track) -> Transform {
    let Some(slot) = track.grid().first() else {
        return Transform::from_xyz(0.0, 60.0, 60.0).looking_at(Vec3::ZERO, Vec3::Y);
    };
    let ahead = slot.forward();
    Transform::from_translation(
        slot.translation - ahead * START_VIEW_BACK_U + Vec3::Y * START_VIEW_UP_U,
    )
    .looking_at(slot.translation + ahead * START_VIEW_BACK_U, Vec3::Y)
}

fn color(item: EditorItem) -> Color {
    match item {
        EditorItem::Point { .. } => LINE,
        EditorItem::Vertex { .. } | EditorItem::Ground { .. } => GROUND,
        EditorItem::PathPoint { .. } | EditorItem::Shortcut { .. } => SHORTCUT,
        EditorItem::ItemRow { .. } => ROW,
        EditorItem::Prop { .. } => PROP,
        EditorItem::Battery => BATTERY,
        EditorItem::Decor { .. } => DECOR,
        EditorItem::Track | EditorItem::Gap { .. } => GAP,
    }
}

/// The control polygon, gaps, ground outlines, shortcuts and the handles (the dragged one where
/// it would go).
fn draw(editor: Res<Editor>, dragging: Res<Dragging>, mut gizmos: Gizmos) {
    let preview;
    let doc = match dragging.0.filter(|d| d.moved) {
        Some(drag) => {
            let mut copy = editor.doc.clone();
            copy.set_handle(drag.item, drag.to);
            preview = copy;
            &preview
        }
        None => &editor.doc,
    };
    let d = &doc.def;
    let on = |layer: &str| editor.layers.iter().any(|l| l == layer);
    let selected = editor.selection;
    let n = d.points.len();
    for i in 0..n {
        gizmos.line(d.points[i].pos, d.points[(i + 1) % n].pos, LINE);
    }
    if let Some(start) = d.points.get(d.start) {
        gizmos.line(start.pos, start.pos + Vec3::Y * 4.0, START);
    }
    for (i, gap) in d.gaps.iter().enumerate() {
        let (Some(a), Some(b)) = (d.points.get(gap.from), d.points.get(gap.to)) else {
            continue;
        };
        let hot = selected == Some(EditorItem::Gap { index: i as u32 });
        let lift = Vec3::Y * 0.3;
        gizmos.line(a.pos + lift, b.pos + lift, if hot { SELECTED } else { GAP });
    }
    if on("ground") {
        for (i, g) in d.ground.iter().enumerate() {
            let hot = matches!(selected, Some(EditorItem::Ground { index } | EditorItem::Vertex { ground: index, .. }) if index as usize == i);
            let ring = g.points.iter().chain(g.points.first());
            gizmos.linestrip(
                ring.map(|p| Vec3::new(p.x, g.height + 0.05, p.y)),
                if hot { SELECTED } else { GROUND },
            );
        }
    }
    if on("shortcuts") {
        for (i, s) in d.shortcuts.iter().enumerate() {
            let hot = matches!(selected, Some(EditorItem::Shortcut { index } | EditorItem::PathPoint { shortcut: index, .. }) if index as usize == i);
            gizmos.linestrip(
                s.path.iter().copied(),
                if hot { SELECTED } else { SHORTCUT },
            );
        }
    }
    for (item, pos) in doc.handles(&editor.layers) {
        let (size, color) = if selected == Some(item) {
            (SELECTED_U, SELECTED)
        } else {
            (HANDLE_U, color(item))
        };
        gizmos.sphere(Isometry3d::from_translation(pos), size, color);
    }
}
