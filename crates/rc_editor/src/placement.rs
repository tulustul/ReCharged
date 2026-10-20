//! Placement mode: a press on a library entry arms it (`editor.place`; the same entry again
//! disarms, another re-arms), and a ghost of the asset follows the ray through the cursor onto any
//! fixed collider (else y = 0; over the UI it waits at the view's focus). Releasing the left button with the cursor off the UI adds the
//! asset there as one edit and selects it; released over the UI it stays armed, so a click on an
//! entry then a click in the view places too. Esc, a right click, a tab switch, undo/redo, delete
//! and a test drive disarm it.
//!
//! bevy-react's `PointerCapture` stays set for a whole press that began in a pane, so this mode
//! ignores it and reads `ButtonInput<MouseButton>` and the cursor itself; "over the UI" = an
//! interactive React node (the panes, the toolbar, the toast) in the `HoverMap`. While armed,
//! picking is off ([`crate::view::pointer`]): the placing click never also selects.

use bevy::picking::hover::HoverMap;
use bevy::picking::pointer::PointerId;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_rapier3d::prelude::ReadRapierContext;
use bevy_react::ReactInteraction;
use rc_core::prelude::*;
use rc_ui::editor::{EditorAsset, EditorPlace};

use crate::view::{ViewFocus, surface};
use crate::{Editor, Mode};

/// What happens to placement mode.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Input {
    /// A press on a library entry.
    Entry(EditorAsset),
    /// Esc, a right click, a tab switch …
    Cancel,
    /// The left button was let go.
    Release {
        /// Over a pane, the toolbar or the toast.
        over_ui: bool,
    },
}

/// The armed asset after `input`, and the asset to place now.
pub(crate) fn step(
    armed: Option<EditorAsset>,
    input: Input,
) -> (Option<EditorAsset>, Option<EditorAsset>) {
    match input {
        Input::Entry(asset) if armed.as_ref() == Some(&asset) => (None, None),
        Input::Entry(asset) => (Some(asset), None),
        Input::Cancel => (None, None),
        Input::Release { over_ui: true } => (armed, None),
        Input::Release { over_ui: false } => (None, armed),
    }
}

/// `editor.place`: arm (or disarm) placement mode, or with `at`, place at once (automation).
pub(crate) fn on_place(msg: On<EditorPlace>, mut editor: ResMut<Editor>) {
    if let Some(at) = msg.at {
        editor.place(&msg.asset, Vec3::from(at));
    } else if editor.mode == Mode::Editing && editor.bake.is_none() {
        let (armed, _) = step(editor.placing.clone(), Input::Entry(msg.asset.clone()));
        editor.placing = armed;
    }
}

/// The ghost's model and where it stands.
#[derive(Resource, Debug, Default)]
pub(crate) struct Ghost {
    /// Where the asset would land (the cursor's ray; the view's focus while the cursor is over the
    /// UI or outside the window).
    pub(crate) at: Option<Vec3>,
    /// The asset shown and its model entity (none: a meshless item, or no model file).
    shown: Option<(EditorAsset, Option<Entity>)>,
}

/// What [`placement`] reads of the mouse and the UI.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct Mouse<'w, 's> {
    buttons: Option<Res<'w, ButtonInput<MouseButton>>>,
    windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    cameras: Query<'w, 's, (&'static Camera, &'static GlobalTransform), With<GameCamera>>,
    hovers: Option<Res<'w, HoverMap>>,
    interactive: Query<'w, 's, (), With<ReactInteraction>>,
}

impl Mouse<'_, '_> {
    /// The cursor's ray into the scene.
    fn ray(&self) -> Option<Ray3d> {
        let cursor = self.windows.single().ok()?.cursor_position()?;
        let (camera, at) = self.cameras.single().ok()?;
        camera.viewport_to_world(at, cursor).ok()
    }

    fn over_ui(&self) -> bool {
        let hits = self.hovers.as_ref().and_then(|h| h.get(&PointerId::Mouse));
        hits.is_some_and(|hits| hits.keys().any(|e| self.interactive.contains(*e)))
    }
}

/// While editing, before picking: moves the ghost's drop point, places on a left release off the
/// UI, disarms on a right click.
pub(crate) fn placement(
    mouse: Mouse,
    rapier: ReadRapierContext,
    focus: Res<ViewFocus>,
    mut editor: ResMut<Editor>,
    mut ghost: ResMut<Ghost>,
) {
    let (Some(armed), Some(buttons)) = (editor.placing.clone(), mouse.buttons.as_deref()) else {
        ghost.at = None;
        return;
    };
    let (ray, over_ui) = (mouse.ray(), mouse.over_ui());
    // Over the panes (or with no cursor) the ghost previews at the view's focus.
    ghost.at = match ray.filter(|_| !over_ui) {
        Some(ray) => surface(&rapier, ray),
        None => focus.0,
    };
    if buttons.just_pressed(MouseButton::Right) {
        editor.placing = step(Some(armed), Input::Cancel).0;
        return;
    }
    if !buttons.just_released(MouseButton::Left) || ray.is_none() {
        return;
    }
    if let (armed, Some(asset)) = step(Some(armed), Input::Release { over_ui })
        && let Some(at) = ghost.at
    {
        editor.placing = armed;
        editor.place(&asset, at);
    }
}

/// Every frame: the ghost model of the armed asset (loaded like the real spawn, through
/// `rc_render::load_scene`) stands at the drop point; gone when disarmed or not editing.
pub(crate) fn ghost(
    editor: Res<Editor>,
    state: Res<State<AppState>>,
    paths: Option<Res<UserPaths>>,
    server: Res<AssetServer>,
    mut ghost: ResMut<Ghost>,
    mut models: Query<(&mut Transform, &mut Visibility)>,
    mut commands: Commands,
) {
    let editing = editor.mode == Mode::Editing && *state.get() == AppState::InRace;
    let want = editor.placing.clone().filter(|_| editing);
    if ghost.shown.as_ref().map(|(a, _)| a) != want.as_ref() {
        if let Some((_, Some(e))) = ghost.shown.take() {
            commands.entity(e).try_despawn();
        }
        ghost.shown = want.map(|asset| {
            let model = match &asset {
                EditorAsset::Prop { id } => editor.props.get(id).map(|d| d.model.clone()),
                EditorAsset::Decor { model } => Some(model.clone()),
                EditorAsset::ItemRow
                | EditorAsset::Battery
                | EditorAsset::Light { .. }
                | EditorAsset::Sun
                | EditorAsset::Fill => None,
            };
            let exists = |m: &String| {
                paths
                    .as_ref()
                    .is_some_and(|p| p.assets_dir.join(m).is_file())
            };
            let entity = model.filter(exists).map(|m| {
                let scene = rc_render::load_scene(&server, &m);
                let ghost = (Name::new("placement ghost"), WorldAssetRoot(scene));
                commands
                    .spawn((ghost, Transform::default(), Visibility::Hidden))
                    .id()
            });
            (asset, entity)
        });
    }
    if let Some((_, Some(e))) = &ghost.shown
        && let Ok((mut transform, mut visibility)) = models.get_mut(*e)
    {
        match ghost.at {
            Some(at) => {
                transform.translation = at;
                visibility.set_if_neq(Visibility::Inherited);
            }
            None => {
                visibility.set_if_neq(Visibility::Hidden);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bear() -> EditorAsset {
        EditorAsset::Prop {
            id: "toy_bear".into(),
        }
    }

    #[test]
    fn placement_arms_rearms_places_and_cancels() {
        let row = EditorAsset::ItemRow;
        // A press on an entry arms; the same entry again disarms; another re-arms.
        let (armed, place) = step(None, Input::Entry(bear()));
        assert_eq!((armed.clone(), place), (Some(bear()), None));
        assert_eq!(step(armed.clone(), Input::Entry(bear())), (None, None));
        assert_eq!(
            step(armed.clone(), Input::Entry(row.clone())),
            (Some(row), None)
        );
        // Released over a pane (the arming click itself, the search box): still armed.
        assert_eq!(
            step(armed.clone(), Input::Release { over_ui: true }),
            (Some(bear()), None)
        );
        // Released over the view: placed once, disarmed.
        assert_eq!(
            step(armed.clone(), Input::Release { over_ui: false }),
            (None, Some(bear()))
        );
        assert_eq!(step(armed, Input::Cancel), (None, None));
        // Not armed: a release is an ordinary click.
        assert_eq!(step(None, Input::Release { over_ui: false }), (None, None));
    }
}
