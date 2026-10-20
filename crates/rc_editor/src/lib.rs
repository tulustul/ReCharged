//! # rc_editor — the track editor (E)
//!
//! - **Responsibilities:** the editing session: the game's own race session on the edited track
//!   ([`TrackDraft`], so unsaved edits are what loads), held in `RacePhase::Setup` (no cars) while
//!   editing; the fly camera; test drives (F5 starts the race, F5 again restarts into editing);
//!   saving the layout to its track file (Ctrl+S); the [`EditorStatus`] React shows.
//! - **Inputs:** keyboard and mouse, rc_ui's `editor.*` messages ([`EditorSave`], [`EditorDrive`]),
//!   `Track` (lints, the start grid), `Catalog`, `LoadingFailed`, `UserPaths::assets_dir`.
//! - **Outputs:** [`TrackDraft`] (sole writer), [`EditorStatus`] (sole writer), `GameCommand`s
//!   (start, restart), the `GameCamera`'s `Transform` while editing, the track file on save.
//! - **Must never:** run inside the game binary, write a track file except on an explicit save, or
//!   let a failed write truncate it.
//!
//! Design and plan: `docs/design/rc_editor.md`. Run it: `cargo run -p rc_editor -- <track>`.

use std::path::{Path, PathBuf};

use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_react::PointerCapture;
use rc_core::prelude::*;
use rc_game::{GameCommand, RaceRequest};
use rc_track::{TRACKS_DIR, Track, TrackDef, TrackDraft};
pub use rc_ui::editor::{EditorDrive, EditorSave, EditorStatus};

/// Fly speed (u/s; ×[`FAST`] with Shift).
const FLY_SPEED: f32 = 24.0;
/// Shift multiplier of the fly speed.
const FAST: f32 = 4.0;
/// Mouse look (radians per pixel of motion while the right button is held).
const LOOK_RAD_PER_PX: f32 = 0.004;
/// The first view: this far behind and above the first grid slot (u).
const START_VIEW_BACK_U: f32 = 22.0;
const START_VIEW_UP_U: f32 = 14.0;

/// The editor on `track`, starting from layout `def` (the file's content).
pub struct EditorPlugin {
    /// Track being edited.
    pub track: TrackId,
    /// Its layout as loaded.
    pub def: TrackDef,
}

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TrackDraft {
            id: self.track.clone(),
            def: self.def.clone(),
        })
        .insert_resource(Editor {
            track: self.track.clone(),
            saved: self.def.clone(),
            mode: Mode::Editing,
            failed: None,
            save_error: None,
        })
        .init_resource::<EditorStatus>()
        .add_message::<Action>()
        // Editing holds the session in Setup: the phase driver lives in SimSet::Race.
        .configure_sets(FixedUpdate, SimSet::Race.run_if(driving))
        .add_observer(|_: On<EditorSave>, mut out: MessageWriter<Action>| {
            out.write(Action::Save);
        })
        .add_observer(|_: On<EditorDrive>, mut out: MessageWriter<Action>| {
            out.write(Action::Drive);
        })
        .add_systems(Startup, window_title)
        .add_systems(OnEnter(AppState::Menu), start_session)
        .add_systems(
            Update,
            (keys, act, note_failures, push_status)
                .chain()
                .before(FlowSet),
        )
        .add_systems(
            Update,
            fly.after(CameraSet)
                .run_if(in_state(AppState::InRace))
                .run_if(not(driving)),
        );
    }
}

/// Editing or test driving.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Editing,
    Driving,
}

#[derive(Resource, Debug)]
struct Editor {
    track: TrackId,
    /// The layout as last loaded or saved (dirty = the draft differs).
    saved: TrackDef,
    mode: Mode,
    /// Why the session failed to load (no new session starts until the draft changes).
    failed: Option<String>,
    save_error: Option<String>,
}

/// What a key or a React button asked for.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Drive,
    Save,
}

fn driving(editor: Res<Editor>) -> bool {
    editor.mode == Mode::Driving
}

fn window_title(editor: Res<Editor>, mut windows: Query<&mut Window, With<PrimaryWindow>>) {
    for mut window in &mut windows {
        window.title = format!("ReCharged Editor — {}", editor.track);
    }
}

/// `OnEnter(Menu)`: the editor has no menus; (re)open the editing session on the draft. Also
/// reached when a test drive quits to the menu.
fn start_session(mut editor: ResMut<Editor>, mut out: MessageWriter<GameCommand>) {
    editor.mode = Mode::Editing;
    if editor.failed.is_some() {
        return;
    }
    out.write(GameCommand::StartRace(RaceRequest {
        mode: GameMode::QuickRace,
        variant: TrackVariant::forward(editor.track.clone()),
        ai: Some(0),
        ..default()
    }));
}

fn keys(keys: Option<Res<ButtonInput<KeyCode>>>, mut out: MessageWriter<Action>) {
    let Some(keys) = keys else {
        return;
    };
    if keys.just_pressed(KeyCode::F5) {
        out.write(Action::Drive);
    }
    let ctrl = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    if ctrl && keys.just_pressed(KeyCode::KeyS) {
        out.write(Action::Save);
    }
}

fn act(
    mut actions: MessageReader<Action>,
    state: Res<State<AppState>>,
    draft: Res<TrackDraft>,
    paths: Res<UserPaths>,
    mut editor: ResMut<Editor>,
    mut out: MessageWriter<GameCommand>,
) {
    for action in actions.read() {
        match action {
            Action::Drive if *state.get() == AppState::InRace => match editor.mode {
                Mode::Editing => {
                    info!("editor: test drive");
                    editor.mode = Mode::Driving;
                }
                Mode::Driving => {
                    info!("editor: back to editing");
                    editor.mode = Mode::Editing;
                    out.write(GameCommand::Restart);
                }
            },
            Action::Drive => {}
            Action::Save => match save(&paths.assets_dir, &draft) {
                Ok(path) => {
                    info!("editor: saved {}", path.display());
                    editor.saved = draft.def.clone();
                    editor.save_error = None;
                }
                Err(e) => {
                    error!("editor: {e}");
                    editor.save_error = Some(e);
                }
            },
        }
    }
}

/// Writes the draft to `<assets>/data/tracks/<id>.ron` through a temporary file, so a failed
/// write never truncates the track. What it writes must parse back.
fn save(assets_dir: &Path, draft: &TrackDraft) -> Result<PathBuf, String> {
    let ron = draft.def.to_ron()?;
    TrackDef::parse(&ron).map_err(|e| format!("the saved layout would not load: {e}"))?;
    let text = format!(
        "// Written by the track editor: cargo run -p rc_editor -- {}\n{ron}",
        draft.id
    );
    let path = assets_dir
        .join(TRACKS_DIR)
        .join(format!("{}.ron", draft.id));
    let tmp = path.with_extension("ron.tmp");
    std::fs::write(&tmp, text)
        .and_then(|()| std::fs::rename(&tmp, &path))
        .map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path)
}

fn note_failures(mut failed: MessageReader<LoadingFailed>, mut editor: ResMut<Editor>) {
    if let Some(f) = failed.read().last() {
        editor.failed = Some(format!("loading failed ({}): {}", f.key, f.error));
    }
}

fn push_status(
    editor: Res<Editor>,
    draft: Res<TrackDraft>,
    catalog: Option<Res<Catalog>>,
    track: Option<Res<Track>>,
    mut status: ResMut<EditorStatus>,
) {
    let name = catalog
        .as_ref()
        .and_then(|c| c.tracks.iter().find(|t| t.id == editor.track))
        .map_or_else(|| editor.track.to_string(), |t| t.name.clone());
    let lints = match &track {
        Some(track) => track.lints().to_vec(),
        None => status.lints.clone(), // between sessions: keep the last ones
    };
    status.set_if_neq(EditorStatus {
        track: editor.track.to_string(),
        name,
        dirty: draft.def != editor.saved,
        driving: editor.mode == Mode::Driving,
        error: editor.failed.clone().or_else(|| editor.save_error.clone()),
        lints,
    });
}

/// Inputs of the fly camera (absent in headless runs).
#[derive(bevy::ecs::system::SystemParam)]
struct FlyInput<'w> {
    time: Res<'w, Time<Real>>,
    keys: Option<Res<'w, ButtonInput<KeyCode>>>,
    buttons: Option<Res<'w, ButtonInput<MouseButton>>>,
    motion: Option<Res<'w, AccumulatedMouseMotion>>,
    capture: Option<Res<'w, PointerCapture>>,
}

/// Right mouse button: look; WASD: move; Q/E: down/up; Shift: faster. The view survives test
/// drives and session reloads.
fn fly(
    input: FlyInput,
    track: Option<Res<Track>>,
    mut view: Local<Option<Transform>>,
    mut looking: Local<bool>,
    mut cameras: Query<&mut Transform, With<GameCamera>>,
) {
    let (Some(keys), Some(buttons)) = (input.keys, input.buttons) else {
        return;
    };
    if view.is_none() {
        *view = track.as_deref().map(start_view);
    }
    let Some(view) = view.as_mut() else {
        return;
    };
    if buttons.just_pressed(MouseButton::Right) {
        *looking = !input.capture.is_some_and(|c| c.is_captured());
    }
    *looking &= buttons.pressed(MouseButton::Right);
    if *looking && let Some(motion) = input.motion {
        let (yaw, pitch, _) = view.rotation.to_euler(EulerRot::YXZ);
        let yaw = yaw - motion.delta.x * LOOK_RAD_PER_PX;
        let pitch = (pitch - motion.delta.y * LOOK_RAD_PER_PX).clamp(-1.54, 1.54);
        view.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);
    }
    let axis = |plus: KeyCode, minus: KeyCode| {
        f32::from(u8::from(keys.pressed(plus))) - f32::from(u8::from(keys.pressed(minus)))
    };
    let dir = view.forward() * axis(KeyCode::KeyW, KeyCode::KeyS)
        + view.right() * axis(KeyCode::KeyD, KeyCode::KeyA)
        + Vec3::Y * axis(KeyCode::KeyE, KeyCode::KeyQ);
    let fast = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let speed = FLY_SPEED * if fast { FAST } else { 1.0 };
    view.translation += dir.normalize_or_zero() * speed * input.time.delta_secs();
    for mut camera in &mut cameras {
        *camera = *view;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_writes_a_file_that_loads_back_and_replaces_the_old_one() {
        let assets = std::env::temp_dir().join(format!("rc_editor-save-{}", std::process::id()));
        let dir = assets.join(TRACKS_DIR);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("oval.ron"), "old").unwrap();
        let real = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let mut def = TrackDef::load(&real, "test_oval").unwrap();
        def.points[1].pos.x += 0.5;
        let draft = TrackDraft {
            id: "oval".into(),
            def: def.clone(),
        };
        let path = save(&assets, &draft).unwrap();
        assert_eq!(TrackDef::load(&assets, "oval").unwrap(), def);
        assert!(!path.with_extension("ron.tmp").exists());
        let _ = std::fs::remove_dir_all(&assets);
    }
}
