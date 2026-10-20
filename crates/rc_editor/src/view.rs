//! The 3D view while editing: the fly camera, picking, markers, the view's focus (the point
//! under the screen centre, where additions land) and the cutaway.
//!
//! Picking runs on demand, on pointer moves and presses only, and never while bevy-react's
//! `PointerCapture` holds the pointer: a left press takes the selection's gizmo handle
//! ([`gizmo`]), else the marker or model under the cursor (or clears the selection); with Ctrl in
//! the Track tab, a press on the road inserts a control point there. Models are hit with
//! `MeshRayCast` against meshes under a `PropSource` / `DecorSource` (the road and ground carry
//! none); items without a mesh in the editing session (item rows, the battery, the Track tab's
//! points and vertices, reverse-only props and decor, dimmed) are screen-sized markers picked in
//! screen space. A model wins over a marker only when the marker is farther away.
//! Right drag looks around; WASD move, Q/E down/up, Shift faster.

use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_rapier3d::prelude::{QueryFilter, ReadRapierContext};
use bevy_react::PointerCapture;
use rc_core::prelude::*;
use rc_render::DecorSource;
use rc_track::{DragPreview, PropSource, Track};
use rc_ui::editor::{EditorItem, EditorTab};

use crate::doc::Doc;
use crate::gizmo::{self, Eye, Gizmo};
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
/// A press picks markers within this distance of the cursor (logical px).
const PICK_RADIUS_PX: f32 = 12.0;
/// The focus ray (and Ctrl+press on the road) reaches this far (u).
const FOCUS_REACH_U: f32 = 400.0;
/// Marker radius per unit of distance from the camera (constant on screen); the selected one
/// is this much bigger.
const MARKER_PER_U: f32 = 0.012;
const SELECTED_GROWTH: f32 = 1.6;
/// Markers of items the editing session does not show (reverse-only) are this opaque.
const DIM_ALPHA: f32 = 0.45;

const LINE: Color = Color::srgb(0.1, 0.85, 1.0);
const GAP: Color = Color::srgb(1.0, 0.25, 0.3);
const GROUND: Color = Color::srgb(0.3, 0.9, 0.4);
const SHORTCUT: Color = Color::srgb(1.0, 0.6, 0.1);
const PROP: Color = Color::srgb(0.95, 0.95, 0.95);
const ROW: Color = Color::srgb(0.4, 0.6, 1.0);
const BATTERY: Color = Color::srgb(1.0, 0.82, 0.3);
const DECOR: Color = Color::srgb(0.9, 0.3, 0.9);
const SELECTED: Color = Color::srgb(0.95, 0.88, 0.06);
const HOVERED: Color = Color::WHITE;
const START: Color = Color::WHITE;

/// The 3D view of the editor (needs a window, gizmos and rapier; [`crate::EditorPlugin`] first).
pub struct EditorViewPlugin;

impl Plugin for EditorViewPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Gizmo>()
            .init_resource::<Hover>()
            .init_resource::<DragPreview>()
            .add_systems(Startup, gizmo_look)
            .add_systems(Update, cutaway.run_if(in_state(AppState::InRace)))
            .add_systems(
                Update,
                (focus_point, pointer, fly, draw)
                    .chain()
                    .after(CameraSet)
                    .run_if(in_state(AppState::InRace))
                    .run_if(editing),
            )
            // Also between sessions: commands may come while one loads, and a committed drag's
            // preview waits for the reload.
            .add_systems(
                Update,
                (gizmo::commands.before(pointer), gizmo::sync.after(draw)),
            );
        crate::tint::build(app);
    }
}

/// The prop, decor item or marked item under the cursor.
#[derive(Resource, Debug, Default, PartialEq)]
pub(crate) struct Hover(pub Option<EditorItem>);

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

    /// Put the camera here.
    pub fn set(&mut self, pose: Transform) {
        self.pose = Some(pose);
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

/// What [`pointer`] reads of the input.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct Pointer<'w, 's> {
    buttons: Option<Res<'w, ButtonInput<MouseButton>>>,
    keys: Option<Res<'w, ButtonInput<KeyCode>>>,
    capture: Option<Res<'w, PointerCapture>>,
    windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    cameras: Query<'w, 's, (&'static Camera, &'static GlobalTransform), With<GameCamera>>,
}

/// What [`pointer`] hits in the scene.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct Scene<'w, 's> {
    meshes: MeshRayCast<'w, 's>,
    tags: Query<'w, 's, AnyOf<(&'static PropSource, &'static DecorSource)>>,
    parents: Query<'w, 's, &'static ChildOf>,
    rapier: ReadRapierContext<'w, 's>,
    pieces: Query<'w, 's, &'static TrackPiece>,
}

/// The item a model entity (or one of its ancestors) shows.
fn source(
    tags: &Query<AnyOf<(&PropSource, &DecorSource)>>,
    parents: &Query<&ChildOf>,
    mut e: Entity,
) -> Option<EditorItem> {
    loop {
        match tags.get(e) {
            Ok((Some(p), _)) => return Some(EditorItem::Prop { index: p.0 as u32 }),
            Ok((_, Some(d))) => return Some(EditorItem::Decor { index: d.0 as u32 }),
            _ => e = parents.get(e).ok()?.parent(),
        }
    }
}

impl Scene<'_, '_> {
    /// The model under the ray and its distance, if the nearest visible mesh is one (the road,
    /// a deck or a wall in front of a prop hides it).
    fn model(&mut self, ray: Ray3d) -> Option<(EditorItem, f32)> {
        let settings = MeshRayCastSettings::default().always_early_exit();
        let &(e, ref hit) = self.meshes.cast_ray(ray, &settings).first()?;
        Some((source(&self.tags, &self.parents, e)?, hit.distance))
    }

    /// Where the ray meets the road (rapier, as the view's focus), if the road is what it meets.
    fn road(&self, ray: Ray3d) -> Option<Vec3> {
        let filter = QueryFilter::only_fixed().exclude_sensors();
        let (e, toi) = (self.rapier.single().ok()?).cast_ray(
            ray.origin,
            *ray.direction,
            FOCUS_REACH_U,
            true,
            filter,
        )?;
        let ribbon = self.pieces.get(e).ok()?.kind == TrackPieceKind::Ribbon;
        ribbon.then(|| ray.get_point(toi))
    }
}

/// Hover and left presses: the gizmo, picking, Ctrl+press insertion (see the module docs).
pub(crate) fn pointer(
    pointer: Pointer,
    mut scene: Scene,
    mut editor: ResMut<Editor>,
    mut gizmo: ResMut<Gizmo>,
    mut hover: ResMut<Hover>,
    mut last: Local<Option<Vec2>>,
) {
    let (Some(buttons), Some(keys)) = (pointer.buttons.as_deref(), pointer.keys.as_deref()) else {
        return;
    };
    let cursor = (pointer.windows.single().ok()).and_then(Window::cursor_position);
    let (Some(cursor), Ok((camera, at))) = (cursor, pointer.cameras.single()) else {
        hover.set_if_neq(Hover(None));
        return;
    };
    let Ok(ray) = camera.viewport_to_world(at, cursor) else {
        return;
    };
    let eye = Eye { camera, at };
    let ctrl = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    if gizmo.drag.is_some() {
        // A drag owns the mouse until the release, over the UI too.
        if buttons.pressed(MouseButton::Left) {
            gizmo::follow(&editor, &mut gizmo, ray, cursor, ctrl);
        } else {
            gizmo::release(&mut editor, &mut gizmo);
        }
        return;
    }
    let captured = pointer.capture.as_ref().is_some_and(|c| c.is_captured());
    if captured || editor.bake.is_some() {
        hover.set_if_neq(Hover(None));
        gizmo.hot = None;
        return;
    }
    let press = buttons.just_pressed(MouseButton::Left);
    if !press && *last == Some(cursor) {
        return;
    }
    *last = Some(cursor);
    gizmo.hot = gizmo::handle_at(&editor, &gizmo, &eye, cursor);
    if let Some(handle) = gizmo.hot {
        hover.set_if_neq(Hover(None));
        if press {
            gizmo::press(&editor, &mut gizmo, handle, ray, cursor);
        }
        return;
    }
    let target = pick(&editor.doc, editor.tab, &eye, &mut scene, ray, cursor);
    hover.set_if_neq(Hover(target));
    if !press {
        return;
    }
    let road = (ctrl && target.is_none() && editor.tab == EditorTab::Track)
        .then(|| scene.road(ray))
        .flatten();
    match (target, road) {
        (Some(item), _) => editor.select(Some(item)),
        (None, Some(at)) => {
            let points: Vec<Vec3> = editor.doc.def.points.iter().map(|p| p.pos).collect();
            let after = nearest_segment(&points, at) as u32;
            editor.edit(|doc, sel| {
                let new = doc.insert_after(EditorItem::Point { index: after })?;
                doc.set_handle(new, at);
                *sel = Some(new);
                Ok(())
            });
        }
        (None, None) => editor.select(None),
    }
}

/// The marker or model under the cursor.
fn pick(
    doc: &Doc,
    tab: EditorTab,
    eye: &Eye,
    scene: &mut Scene,
    ray: Ray3d,
    cursor: Vec2,
) -> Option<EditorItem> {
    let marker = (doc.markers(tab == EditorTab::Track).into_iter())
        .filter_map(|(item, pos, _)| {
            let px = eye
                .camera
                .world_to_viewport(eye.at, pos)
                .ok()?
                .distance(cursor);
            (px <= PICK_RADIUS_PX).then(|| (item, px, eye.at.translation().distance(pos)))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1));
    match (marker, scene.model(ray)) {
        (Some((item, _, far)), Some((_, near))) if far <= near => Some(item),
        (_, Some((item, _))) => Some(item),
        (marker, None) => marker.map(|m| m.0),
    }
}

/// The control polygon's segment (by its first point) nearest to `p`.
pub(crate) fn nearest_segment(points: &[Vec3], p: Vec3) -> usize {
    let n = points.len();
    let dist = |i: usize| {
        let (a, b) = (points[i], points[(i + 1) % n]);
        let t = ((p - a).dot(b - a) / (b - a).length_squared().max(1e-6)).clamp(0.0, 1.0);
        p.distance(a + (b - a) * t)
    };
    (0..n)
        .min_by(|&i, &j| dist(i).total_cmp(&dist(j)))
        .unwrap_or(0)
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

/// The layout (Track tab), the markers and the gizmo, as a live drag has them.
fn draw(
    editor: Res<Editor>,
    gizmo: Res<Gizmo>,
    hover: Res<Hover>,
    cameras: Query<(&Camera, &GlobalTransform), With<GameCamera>>,
    mut gizmos: Gizmos,
) {
    let preview = gizmo.preview(&editor.doc);
    let doc = preview.as_ref().unwrap_or(&editor.doc);
    let layout = editor.tab == EditorTab::Track;
    let selected = editor.selection;
    if layout {
        draw_layout(doc, selected, &mut gizmos);
    }
    let Ok((camera, at)) = cameras.single() else {
        return;
    };
    for (item, pos, dim) in doc.markers(layout) {
        let mut size = at.translation().distance(pos) * MARKER_PER_U;
        let color = if selected == Some(item) {
            size *= SELECTED_GROWTH;
            SELECTED
        } else if hover.0 == Some(item) {
            HOVERED
        } else {
            color(item)
        };
        let color = if dim && selected != Some(item) {
            color.with_alpha(DIM_ALPHA)
        } else {
            color
        };
        gizmos.sphere(Isometry3d::from_translation(pos), size, color);
    }
    gizmo::draw(&editor, &gizmo, &Eye { camera, at }, &mut gizmos);
}

/// The control polygon, the start, gaps, ground outlines and shortcuts (the Track tab).
fn draw_layout(doc: &Doc, selected: Option<EditorItem>, gizmos: &mut Gizmos) {
    let d = &doc.def;
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
    for (i, g) in d.ground.iter().enumerate() {
        let hot = matches!(selected, Some(EditorItem::Ground { index } | EditorItem::Vertex { ground: index, .. }) if index as usize == i);
        let ring = g.points.iter().chain(g.points.first());
        gizmos.linestrip(
            ring.map(|p| Vec3::new(p.x, g.height + 0.05, p.y)),
            if hot { SELECTED } else { GROUND },
        );
    }
    for (i, s) in d.shortcuts.iter().enumerate() {
        let hot = matches!(selected, Some(EditorItem::Shortcut { index } | EditorItem::PathPoint { shortcut: index, .. }) if index as usize == i);
        gizmos.linestrip(
            s.path.iter().copied(),
            if hot { SELECTED } else { SHORTCUT },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ctrl_press_inserts_on_the_nearest_segment() {
        let square = [
            Vec3::ZERO,
            Vec3::X * 10.0,
            Vec3::new(10.0, 0.0, 10.0),
            Vec3::Z * 10.0,
        ];
        assert_eq!(nearest_segment(&square, Vec3::new(5.0, 0.0, -1.0)), 0);
        assert_eq!(nearest_segment(&square, Vec3::new(11.0, 0.0, 5.0)), 1);
        assert_eq!(
            nearest_segment(&square, Vec3::new(-1.0, 0.0, 4.0)),
            3,
            "the closing one"
        );
        // A helix: the level the point is on, not the one above it.
        let stacked = [
            Vec3::ZERO,
            Vec3::X * 10.0,
            Vec3::new(10.0, 4.0, 0.1),
            Vec3::new(0.0, 4.0, 0.1),
        ];
        assert_eq!(nearest_segment(&stacked, Vec3::new(5.0, 3.9, 0.0)), 2);
    }
}
