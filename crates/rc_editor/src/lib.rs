//! # rc_editor — the track editor (BIN layer, above the game)
//!
//! - **Responsibilities:** the edited document ([`doc`]: layout + decor) with undo/redo; every
//!   edit is built in all four variants before it is accepted; the editing session — the game's
//!   own race session on the track ([`TrackDraft`], so unsaved edits are what loads), held in
//!   `RacePhase::Setup` (no cars) while editing and reloaded after each edit; test drives (F5);
//!   saving the layout and decor files (Ctrl+S); the inspector ([`fields`]); the 3D view
//!   ([`view`]: fly camera, handles, picking, dragging); the [`EditorStatus`] React shows.
//! - **Inputs:** keyboard and mouse, rc_ui's `editor.*` messages, `Track` (lints, projection),
//!   `Catalog` (name, allowed times and weather), `RaceConfig` (session layout),
//!   `LoadingFailed`, `UserPaths::assets_dir`.
//! - **Outputs:** [`TrackDraft`] (sole writer), [`EditorStatus`] (sole writer), `GameCommand`s
//!   (start, restart, change track), a `UiNav::Confirm` that skips a test drive's intro, the
//!   `GameCamera`'s `Transform` while editing, gizmos, the track and decor files on save.
//! - **Must never:** run inside the game binary, write a data file except on an explicit save,
//!   let a failed write truncate it, or hand the session a layout that does not build.
//!
//! Design and plan: `docs/design/rc_editor.md`. Run it: `cargo run -p rc_editor -- <track>`.

pub mod catalog;
pub mod doc;
pub mod fields;
pub mod view;

use std::path::{Path, PathBuf};
use std::time::Instant;

use bevy::ecs::system::SystemParam;
use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use bevy::text::EditableText;
use bevy::window::PrimaryWindow;
use rc_core::prelude::*;
use rc_game::{GameCommand, RaceRequest, TrackPick};
use rc_render::{ENVIRONMENTS_DIR, SHOWROOM_ENV};
use rc_track::{DECOR_DIR, PropCatalog, TRACKS_DIR, Track, TrackDef, TrackDraft, decor_to_ron};
pub use rc_ui::editor::{
    EditorAdd, EditorCommand, EditorCommandMsg, EditorItem, EditorOptions, EditorSelect,
    EditorSelection, EditorSet, EditorSetOptions, EditorStatus,
};

pub use crate::doc::Doc;
use crate::doc::{LAYERS, validate};
pub use crate::view::EditorViewPlugin;

/// Undo steps kept.
const UNDO_DEPTH: usize = 200;

/// The editor's logic on `track` (no window needed; [`EditorViewPlugin`] adds the 3D view).
pub struct EditorPlugin {
    /// Track being edited.
    pub track: TrackId,
    /// Its layout, decor and catalog entry as loaded.
    pub doc: Doc,
    /// Prop kinds (validation, the kind list).
    pub props: PropCatalog,
    /// `catalog.ron` as loaded (the entry is rewritten in this text).
    pub catalog_text: String,
    /// Environment ids.
    pub environments: Vec<String>,
}

impl EditorPlugin {
    /// Reads everything about `track` under `assets_dir`.
    pub fn load(assets_dir: &Path, track: &str) -> Result<Self, String> {
        let catalog_text = std::fs::read_to_string(assets_dir.join(CATALOG_FILE))
            .map_err(|e| format!("{CATALOG_FILE}: {e}"))?;
        let catalog =
            Catalog::parse(&catalog_text, Path::new(CATALOG_FILE)).map_err(|e| e.to_string())?;
        let id = TrackId::new(track);
        let info = catalog.track(&id).cloned().ok_or_else(|| {
            format!("{track} is not in {CATALOG_FILE} (new tracks: rc_editor new {track} --env …)")
        })?;
        let mut environments: Vec<String> = std::fs::read_dir(assets_dir.join(ENVIRONMENTS_DIR))
            .map_err(|e| format!("{ENVIRONMENTS_DIR}: {e}"))?
            .flatten()
            .filter_map(|e| Some(e.path().file_stem()?.to_str()?.to_owned()))
            .filter(|env| env != SHOWROOM_ENV)
            .collect();
        environments.sort();
        Ok(Self {
            track: id,
            doc: Doc {
                def: TrackDef::load(assets_dir, track)?,
                decor: rc_track::load_decor(assets_dir, track)?,
                info,
            },
            props: PropCatalog::load(assets_dir)?,
            catalog_text,
            environments,
        })
    }
}

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        let doc = self.doc.clone();
        app.insert_resource(TrackDraft {
            id: self.track.clone(),
            def: doc.def.clone(),
            decor: doc.decor.clone(),
        })
        .insert_resource(Editor {
            track: self.track.clone(),
            choices: fields::Choices {
                kinds: self.props.props.keys().cloned().collect(),
                environments: self.environments.clone(),
            },
            props: self.props.clone(),
            catalog_text: self.catalog_text.clone(),
            saved: doc.clone(),
            doc,
            undo: Vec::new(),
            redo: Vec::new(),
            selection: None,
            mode: Mode::Editing,
            options: EditorOptions {
                time: fields::name(&TimeOfDay::Noon),
                weather: fields::name(&WeatherKind::Clear),
                reverse: false,
                mirror: false,
            },
            layers: ["points", "props", "rows", "shortcuts"]
                .map(String::from)
                .to_vec(),
            error: None,
            failed: None,
            wanted: 0,
            built: 0,
            sent: false,
        })
        .init_resource::<EditorStatus>()
        .init_resource::<view::ViewFocus>()
        .init_resource::<view::View>()
        .add_message::<Cmd>()
        // Editing holds the session in Setup: the phase driver lives in SimSet::Race.
        .configure_sets(FixedUpdate, SimSet::Race.run_if(driving))
        .add_observer(|msg: On<EditorCommandMsg>, mut out: MessageWriter<Cmd>| {
            out.write(Cmd(msg.command.clone()));
        })
        .add_observer(|msg: On<EditorSelect>, mut editor: ResMut<Editor>| {
            editor.select(msg.item);
        })
        .add_observer(on_set)
        .add_observer(on_options)
        .add_systems(Startup, window_title)
        .add_systems(OnEnter(AppState::Menu), |mut editor: ResMut<Editor>| {
            editor.mode = Mode::Editing; // no menus: `reload` reopens the editing session
            editor.sent = false;
        })
        .add_systems(OnEnter(AppState::Loading), |mut editor: ResMut<Editor>| {
            editor.built = editor.wanted; // the draft is in sync: this session builds it
            editor.sent = false;
        })
        .add_systems(OnEnter(RacePhase::Intro), skip_intro)
        .add_systems(
            Update,
            (
                keys,
                commands,
                reload,
                note_failures,
                sync_draft,
                sync_catalog,
                push_status,
            )
                .chain()
                .before(FlowSet),
        );
    }
}

/// Editing or test driving.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    Editing,
    Driving,
}

/// The editor's state (one per run).
#[derive(Resource, Debug)]
pub(crate) struct Editor {
    pub(crate) track: TrackId,
    pub(crate) props: PropCatalog,
    /// Prop kinds and environments to choose from.
    pub(crate) choices: fields::Choices,
    /// `catalog.ron` as last loaded or saved.
    catalog_text: String,
    /// The document being edited (what the session builds).
    pub(crate) doc: Doc,
    /// The document as last loaded or saved.
    saved: Doc,
    undo: Vec<Doc>,
    redo: Vec<Doc>,
    pub(crate) selection: Option<EditorItem>,
    pub(crate) mode: Mode,
    options: EditorOptions,
    pub(crate) layers: Vec<String>,
    /// The last rejected edit or failed save.
    error: Option<String>,
    /// Why the session failed to load (no new session starts until the document changes).
    failed: Option<String>,
    /// Generation of the document and options the session should show …
    wanted: u32,
    /// … and of the ones the running (or loading) session was built from.
    built: u32,
    /// A start or reload command is on its way.
    sent: bool,
}

impl Editor {
    /// Applies `edit` to a copy of the document (it may move the selection); keeps it only if
    /// every variant builds. A rejected edit leaves everything as it was and shows why.
    pub(crate) fn edit(
        &mut self,
        edit: impl FnOnce(&mut Doc, &mut Option<EditorItem>) -> Result<(), String>,
    ) {
        let mut doc = self.doc.clone();
        let mut selection = self.selection;
        let result = edit(&mut doc, &mut selection).and_then(|()| {
            if doc == self.doc {
                return Ok(false);
            }
            let started = Instant::now();
            validate(&doc, &self.props, &self.track)?;
            if doc.info != self.doc.info {
                catalog::with_entry(&self.catalog_text, &doc.info)?;
            }
            debug!("editor: edit validated in {:?}", started.elapsed());
            Ok(true)
        });
        match result {
            Ok(changed) => {
                if changed {
                    self.undo.push(std::mem::replace(&mut self.doc, doc));
                    if self.undo.len() > UNDO_DEPTH {
                        self.undo.remove(0);
                    }
                    self.redo.clear();
                    self.changed();
                }
                self.selection = selection;
                self.error = None;
            }
            Err(e) => {
                info!("editor: edit rejected: {e}");
                self.error = Some(e);
            }
        }
    }

    fn changed(&mut self) {
        self.wanted += 1;
        self.failed = None;
        if self.selection.is_some_and(|s| !self.doc.exists(s)) {
            self.selection = None;
        }
    }

    fn step(&mut self, back: bool) {
        let (from, to) = if back {
            (&mut self.undo, &mut self.redo)
        } else {
            (&mut self.redo, &mut self.undo)
        };
        if let Some(doc) = from.pop() {
            to.push(std::mem::replace(&mut self.doc, doc));
            self.error = None;
            self.changed();
        }
    }

    pub(crate) fn select(&mut self, item: Option<EditorItem>) {
        self.selection = item.filter(|i| self.doc.exists(*i));
    }

    /// The layout and look the session should run.
    fn pick(&self) -> TrackPick {
        let driving = self.mode == Mode::Driving;
        TrackPick {
            variant: TrackVariant {
                track: self.track.clone(),
                reverse: driving && self.options.reverse,
                mirror: driving && self.options.mirror,
            },
            ai: Some(0),
            time: fields::parse(&self.options.time).ok(),
            weather: fields::parse(&self.options.weather)
                .ok()
                .map(WeatherChoice::Fixed),
            ..default()
        }
    }
}

/// A command from a key or a React button.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
struct Cmd(EditorCommand);

fn driving(editor: Res<Editor>) -> bool {
    editor.mode == Mode::Driving
}

fn window_title(editor: Res<Editor>, mut windows: Query<&mut Window, With<PrimaryWindow>>) {
    for mut window in &mut windows {
        window.title = format!("ReCharged Editor — {}", editor.track);
    }
}

/// `OnEnter(Intro)` of a test drive: skip the fly-by.
fn skip_intro(editor: Res<Editor>, mut nav: MessageWriter<UiNav>) {
    if editor.mode == Mode::Driving {
        nav.write(UiNav {
            action: NavAction::Confirm,
            slot: None,
            device: DeviceKind::default(),
            repeat: false,
        });
    }
}

/// A bevy-react text field has the input focus (the inspector): keys are for it, not for us.
#[derive(SystemParam)]
pub(crate) struct Typing<'w, 's> {
    focus: Option<Res<'w, InputFocus>>,
    fields: Query<'w, 's, (), With<EditableText>>,
}

impl Typing<'_, '_> {
    pub(crate) fn active(&self) -> bool {
        self.focus
            .as_ref()
            .and_then(|f| f.get())
            .is_some_and(|e| self.fields.contains(e))
    }
}

fn keys(keys: Option<Res<ButtonInput<KeyCode>>>, typing: Typing, mut out: MessageWriter<Cmd>) {
    let Some(keys) = keys else {
        return;
    };
    if typing.active() {
        return;
    }
    let ctrl = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let pressed = |key| keys.just_pressed(key);
    let command = if pressed(KeyCode::F5) {
        EditorCommand::Drive
    } else if ctrl && pressed(KeyCode::KeyS) {
        EditorCommand::Save
    } else if ctrl && (pressed(KeyCode::KeyY) || shift && pressed(KeyCode::KeyZ)) {
        EditorCommand::Redo
    } else if ctrl && pressed(KeyCode::KeyZ) {
        EditorCommand::Undo
    } else if ctrl && pressed(KeyCode::KeyD) {
        EditorCommand::Duplicate
    } else if pressed(KeyCode::Delete) {
        EditorCommand::Delete
    } else if pressed(KeyCode::Insert) {
        EditorCommand::Insert
    } else if !ctrl && pressed(KeyCode::KeyT) {
        EditorCommand::TopView
    } else if !ctrl && pressed(KeyCode::KeyF) {
        EditorCommand::Focus
    } else {
        return;
    };
    out.write(Cmd(command));
}

/// What [`commands`] reads and moves besides the editor.
#[derive(SystemParam)]
struct Session<'w> {
    state: Res<'w, State<AppState>>,
    config: Option<Res<'w, RaceConfig>>,
    track: Option<Res<'w, Track>>,
    paths: Res<'w, UserPaths>,
    focus: Res<'w, view::ViewFocus>,
    view: ResMut<'w, view::View>,
}

fn commands(
    mut commands: MessageReader<Cmd>,
    mut session: Session,
    mut editor: ResMut<Editor>,
    mut out: MessageWriter<GameCommand>,
) {
    for Cmd(command) in commands.read() {
        let selection = editor.selection;
        match command {
            EditorCommand::Save => match save(&session.paths.assets_dir, &editor) {
                Ok((path, catalog_text)) => {
                    info!("editor: saved {}", path.display());
                    editor.saved = editor.doc.clone();
                    editor.catalog_text = catalog_text;
                    editor.error = None;
                }
                Err(e) => {
                    error!("editor: {e}");
                    editor.error = Some(e);
                }
            },
            EditorCommand::Drive => drive(&mut editor, &session, &mut out),
            EditorCommand::Undo => editor.step(true),
            EditorCommand::Redo => editor.step(false),
            EditorCommand::Delete => editor.edit(|doc, sel| {
                *sel = doc.delete(sel.ok_or("nothing is selected")?)?;
                Ok(())
            }),
            EditorCommand::Insert => editor.edit(|doc, sel| {
                *sel = Some(doc.insert_after(sel.ok_or("nothing is selected")?)?);
                Ok(())
            }),
            EditorCommand::Duplicate => editor.edit(|doc, sel| {
                *sel = Some(doc.duplicate(sel.ok_or("nothing is selected")?)?);
                Ok(())
            }),
            EditorCommand::Add { what } => {
                let Some(focus) = session.focus.0 else {
                    editor.error = Some("look at the floor or the track to add there".into());
                    continue;
                };
                let props = editor.props.clone();
                let track = session.track.as_deref();
                editor.edit(|doc, sel| {
                    *sel = Some(doc.add(*what, focus, selection, track, &props)?);
                    Ok(())
                });
            }
            EditorCommand::TopView => {
                let target = selection
                    .and_then(|s| view::anchor(&editor.doc, s))
                    .or(session.focus.0);
                session.view.top(target, session.track.as_deref());
            }
            EditorCommand::Focus => {
                if let Some(target) = selection.and_then(|s| view::anchor(&editor.doc, s)) {
                    session.view.look_at(target);
                }
            }
            EditorCommand::ToggleLayer { layer } => {
                if !LAYERS.contains(&layer.as_str()) {
                    warn!("editor: no layer {layer}");
                } else if let Some(i) = editor.layers.iter().position(|l| l == layer) {
                    editor.layers.remove(i);
                } else {
                    editor.layers.push(layer.clone());
                }
            }
        }
    }
}

/// F5: start a test drive, or end it. A forward drive just lifts the Setup hold; a reverse or
/// mirrored one reloads the session in that layout (and back to forward afterwards).
fn drive(editor: &mut Editor, session: &Session, out: &mut MessageWriter<GameCommand>) {
    if *session.state.get() != AppState::InRace {
        return;
    }
    let forward = |c: &RaceConfig| !c.variant.reverse && !c.variant.mirror;
    match editor.mode {
        Mode::Editing => {
            info!("editor: test drive");
            editor.mode = Mode::Driving;
            if editor.options.reverse || editor.options.mirror {
                editor.wanted += 1;
            }
        }
        Mode::Driving => {
            info!("editor: back to editing");
            editor.mode = Mode::Editing;
            if session.config.as_deref().is_some_and(forward) {
                out.write(GameCommand::Restart);
            } else {
                editor.wanted += 1;
            }
        }
    }
}

/// Opens the session from the menu (the editor has none: at start, after a test drive quit to it,
/// after a failed load once the document changed) and rebuilds it when the document or options
/// changed since it was built.
fn reload(
    state: Res<State<AppState>>,
    mut editor: ResMut<Editor>,
    mut out: MessageWriter<GameCommand>,
) {
    if editor.sent {
        return;
    }
    match state.get() {
        AppState::Menu if editor.failed.is_none() => {
            let pick = editor.pick();
            out.write(GameCommand::StartRace(RaceRequest {
                mode: GameMode::QuickRace,
                variant: pick.variant,
                ai: pick.ai,
                time: pick.time,
                weather: pick.weather,
                ..default()
            }));
        }
        AppState::InRace if editor.built != editor.wanted => {
            out.write(GameCommand::ChangeTrack(Box::new(editor.pick())));
        }
        _ => return,
    }
    editor.bypass_change_detection().sent = true;
}

fn on_set(msg: On<EditorSet>, track: Option<Res<Track>>, mut editor: ResMut<Editor>) {
    let choices = editor.choices.clone();
    let track = track.as_deref();
    editor.edit(|doc, sel| {
        let item = sel.ok_or("nothing is selected")?;
        fields::set_field(doc, item, &msg.key, msg.value.clone(), &choices, track)
    });
}

fn on_options(msg: On<EditorSetOptions>, mut editor: ResMut<Editor>) {
    if editor.options != msg.options {
        editor.options = msg.options.clone();
        editor.wanted += 1;
    }
}

/// Writes `<assets>/data/tracks/<id>.ron`, `<assets>/data/decor/<id>.ron` and (if the entry
/// changed) `catalog.ron` through temporary files, so a failed write never truncates them. What
/// it writes must parse back. No decor and no decor file: none is created. Returns the track
/// file and the catalog text now on disk.
fn save(assets_dir: &Path, editor: &Editor) -> Result<(PathBuf, String), String> {
    let id = &editor.track;
    let ron = editor.doc.def.to_ron()?;
    TrackDef::parse(&ron).map_err(|e| format!("the saved layout would not load: {e}"))?;
    let header = format!("// Written by the track editor: cargo run -p rc_editor -- {id}\n");
    let path = assets_dir.join(TRACKS_DIR).join(format!("{id}.ron"));
    let decor_path = assets_dir.join(DECOR_DIR).join(format!("{id}.ron"));
    write(&path, &format!("{header}{ron}"))?;
    if !editor.doc.decor.is_empty() || decor_path.exists() {
        let decor = decor_to_ron(&editor.doc.decor)?;
        write(&decor_path, &format!("{header}{decor}"))?;
    }
    let mut catalog_text = editor.catalog_text.clone();
    if editor.doc.info != editor.saved.info {
        catalog_text = catalog::with_entry(&catalog_text, &editor.doc.info)?;
        write(&assets_dir.join(CATALOG_FILE), &catalog_text)?;
    }
    Ok((path, catalog_text))
}

fn write(path: &Path, text: &str) -> Result<(), String> {
    let tmp = path.with_extension("ron.tmp");
    std::fs::write(&tmp, text)
        .and_then(|()| std::fs::rename(&tmp, path))
        .map_err(|e| format!("{}: {e}", path.display()))
}

fn note_failures(mut failed: MessageReader<LoadingFailed>, mut editor: ResMut<Editor>) {
    if let Some(f) = failed.read().last() {
        editor.failed = Some(format!("loading failed ({}): {}", f.key, f.error));
    }
}

/// The session builds what the editor holds.
fn sync_draft(editor: Res<Editor>, mut draft: ResMut<TrackDraft>) {
    if editor.is_changed() && (draft.def != editor.doc.def || draft.decor != editor.doc.decor) {
        draft.def = editor.doc.def.clone();
        draft.decor = editor.doc.decor.clone();
    }
}

/// The game's catalog shows the edited entry (environment, allowed presets for the session; the
/// one exception to rc_game being its only writer, editor only).
fn sync_catalog(editor: Res<Editor>, catalog: Option<ResMut<Catalog>>) {
    let Some(mut catalog) = catalog else {
        return;
    };
    if !editor.is_changed() {
        return;
    }
    let info = &editor.doc.info;
    match catalog.tracks.iter().position(|t| t.id == info.id) {
        Some(i) if catalog.tracks[i] != *info => catalog.tracks[i] = info.clone(),
        Some(_) => {}
        None => catalog.tracks.push(info.clone()),
    }
}

fn push_status(
    editor: Res<Editor>,
    state: Res<State<AppState>>,
    track: Option<Res<Track>>,
    mut status: ResMut<EditorStatus>,
) {
    let track_changed = track.as_ref().is_some_and(|t| t.is_changed());
    if !editor.is_changed() && !state.is_changed() && !track_changed {
        return;
    }
    let info = &editor.doc.info;
    let selection = editor.selection.and_then(|item| {
        let (title, fields) = fields::fields(&editor.doc, item, &editor.choices, track.as_deref())?;
        Some(EditorSelection {
            item,
            title,
            fields,
        })
    });
    let lints = match &track {
        Some(track) => track.lints().to_vec(),
        None => status.lints.clone(), // between sessions: keep the last ones
    };
    status.set_if_neq(EditorStatus {
        track: editor.track.to_string(),
        name: info.name.clone(),
        dirty: editor.doc != editor.saved,
        driving: editor.mode == Mode::Driving,
        loading: editor.built != editor.wanted || *state.get() != AppState::InRace,
        error: editor.failed.clone().or_else(|| editor.error.clone()),
        lints,
        can_undo: !editor.undo.is_empty(),
        can_redo: !editor.redo.is_empty(),
        counts: editor.doc.counts(),
        selection,
        options: editor.options.clone(),
        times: info.allowed_times.iter().map(fields::name).collect(),
        weathers: info.allowed_weather.iter().map(fields::name).collect(),
        layers: editor.layers.clone(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_writes_files_that_load_back_and_replace_the_old_ones() {
        let real = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let assets = std::env::temp_dir().join(format!("rc_editor-save-{}", std::process::id()));
        for dir in [TRACKS_DIR, DECOR_DIR, "data"] {
            std::fs::create_dir_all(assets.join(dir)).unwrap();
        }
        std::fs::write(assets.join(TRACKS_DIR).join("test_oval.ron"), "old").unwrap();
        let plugin = EditorPlugin::load(&real, "test_oval").unwrap();
        let mut doc = plugin.doc.clone();
        doc.def.points[1].pos.x += 0.5;
        doc.def.points[1].label = Some("start".into());
        doc.decor = rc_track::load_decor(&real, "toy_town").unwrap();
        doc.info.name = "Renamed Oval".into();
        let editor = Editor {
            track: plugin.track.clone(),
            props: PropCatalog::default(),
            choices: fields::Choices::default(),
            catalog_text: plugin.catalog_text.clone(),
            saved: plugin.doc.clone(),
            doc: doc.clone(),
            undo: Vec::new(),
            redo: Vec::new(),
            selection: None,
            mode: Mode::Editing,
            options: EditorOptions::default(),
            layers: Vec::new(),
            error: None,
            failed: None,
            wanted: 0,
            built: 0,
            sent: false,
        };
        let (path, catalog_text) = save(&assets, &editor).unwrap();
        assert_eq!(TrackDef::load(&assets, "test_oval").unwrap(), doc.def);
        assert_eq!(
            rc_track::load_decor(&assets, "test_oval").unwrap(),
            doc.decor
        );
        let catalog = Catalog::load(&assets).unwrap();
        assert_eq!(catalog.track(&plugin.track), Some(&doc.info));
        assert_eq!(
            std::fs::read_to_string(assets.join(CATALOG_FILE)).unwrap(),
            catalog_text
        );
        assert!(!path.with_extension("ron.tmp").exists());
        let _ = std::fs::remove_dir_all(&assets);
    }
}
