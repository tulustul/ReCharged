//! # rc_editor — the track editor (BIN layer, above the game)
//!
//! - **Responsibilities:** the edited document ([`doc`]: layout + decor + environment) with
//!   undo/redo; every edit is built in all four variants before it is accepted; the editing
//!   session — the game's own race session on the track ([`TrackDraft`] and `EnvironmentDraft`,
//!   so unsaved edits are what loads), held in
//!   `RacePhase::Setup` (no cars) while editing and updated in place after each edit (reloaded
//!   only for the reload list, [`live`]); test drives (F5);
//!   saving the layout, decor and environment files (Ctrl+S); the inspector ([`fields`]); the
//!   asset library ([`library`]: placing and swapping props, decor, gameplay items and lights;
//!   lights, sun, fill and time looks: [`lights`]); the 3D view
//!   ([`view`]: fly camera, markers, picking, hover/selection tint; [`gizmo`]: the transform
//!   gizmo); the [`EditorStatus`] React shows.
//! - **Inputs:** keyboard and mouse, rc_ui's `editor.*` messages, `Track` (lints, projection),
//!   `Catalog` (name, allowed times and weather), `RaceConfig` (session layout),
//!   `LoadingFailed`, `UserPaths::assets_dir`.
//! - **Outputs:** [`TrackDraft`], `DragPreview`, `TrackPreview`, `EnvironmentDraft` and
//!   `EnvPreview` (sole writer), the session's
//!   `Track` and `TrackSummary` on an in-place commit, [`EditorStatus`] and [`EditorLibrary`]
//!   (sole writer), a placement ghost while placing,
//!   `GameCommand`s (start, restart, change track), a `UiNav::Confirm` that skips a test
//!   drive's intro, the `GameCamera`'s `Transform` while editing, `Overhead` visibility, tint
//!   materials on the hovered and selected models, gizmos, the track, decor and environment files
//!   on save.
//! - **Must never:** run inside the game binary, write a data file except on an explicit save,
//!   let a failed write truncate it, or hand the session a layout that does not build.
//!
//! Design and plan: `docs/design/rc_editor.md`. Run it: `cargo run -p rc_editor -- <track>`.

pub mod bake;
mod baking;
pub mod catalog;
pub mod doc;
pub mod fields;
pub mod gizmo;
pub mod library;
pub mod lights;
mod live;
mod placement;
mod save;
mod tint;
pub mod view;

use std::path::Path;
use std::time::Instant;

use bevy::ecs::system::SystemParam;
use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use bevy::text::EditableText;
use bevy::window::PrimaryWindow;
use rc_core::prelude::*;
use rc_game::{GameCommand, RaceRequest, TrackPick};
use rc_render::{BakedGi, BakedLight, EnvironmentDef, EnvironmentDraft, volume_file};
use rc_track::{PropCatalog, Track, TrackDef, TrackDraft};
pub use rc_ui::editor::{
    EditorAdd, EditorAsset, EditorCommand, EditorCommandMsg, EditorGizmo, EditorItem,
    EditorLibrary, EditorOptions, EditorPlace, EditorSelect, EditorSelection, EditorSet,
    EditorSetOptions, EditorSetTab, EditorStatus, EditorTab,
};

pub use crate::doc::Doc;
use crate::doc::validate;
pub use crate::view::EditorViewPlugin;

/// Bake once the editing session is up, then quit: `time` = only that time of day (normal
/// layout), `None` = every allowed time in both layouts.
pub fn bake_on_start(app: &mut App, time: Option<TimeOfDay>) {
    app.insert_resource(baking::BakeOnStart(time));
}

/// Undo steps kept.
const UNDO_DEPTH: usize = 200;

/// The editor's logic on `track` (no window needed; [`EditorViewPlugin`] adds the 3D view).
pub struct EditorPlugin {
    /// Track being edited.
    pub track: TrackId,
    /// Its layout, decor, environment and catalog entry as loaded.
    pub doc: Doc,
    /// Prop kinds (validation, the kind list).
    pub props: PropCatalog,
    /// `catalog.ron` as loaded (the entry is rewritten in this text).
    pub catalog_text: String,
    /// Environment ids.
    pub environments: Vec<String>,
    /// The asset library.
    pub library: EditorLibrary,
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
        // Environment kinds (styles, ambience); the lighting itself is the map's environment.ron.
        let mut environments: Vec<String> = catalog
            .tracks
            .iter()
            .map(|t| t.environment.clone())
            .collect();
        environments.sort();
        environments.dedup();
        let props = PropCatalog::load(assets_dir)?;
        Ok(Self {
            track: id,
            library: library::build(assets_dir, &props, &info.environment),
            doc: Doc {
                def: TrackDef::load(assets_dir, track)?,
                decor: rc_track::load_decor(assets_dir, track)?,
                env: EnvironmentDef::load(assets_dir, track)?,
                info,
            },
            props,
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
        .insert_resource(EnvironmentDraft {
            id: self.track.clone(),
            def: doc.env.clone(),
        })
        .insert_resource(Editor {
            track: self.track.clone(),
            choices: fields::Choices {
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
            tab: EditorTab::default(),
            error: None,
            failed: None,
            wanted: 0,
            built: 0,
            sent: false,
            pending: None,
            commits: 0,
            applied: 0,
            stale: Vec::new(),
            bake: None,
            placing: None,
            live: None,
        })
        .insert_resource(self.library.clone())
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
        .add_observer(|msg: On<EditorSetTab>, mut editor: ResMut<Editor>| {
            if editor.tab != msg.tab {
                editor.tab = msg.tab;
                editor.placing = None; // a tab switch cancels placement
            }
        })
        .add_observer(placement::on_place)
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
                baking::bake_on_start.run_if(resource_exists::<baking::BakeOnStart>),
                baking::run.run_if(|editor: Res<Editor>| editor.bake.is_some()),
                reload,
                note_failures,
                sync_draft,
                sync_catalog,
                live::apply,
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
    /// The open left-pane tab (the Track tab shows the layout handles).
    pub(crate) tab: EditorTab,
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
    /// The editing variant's track of an accepted change, to commit in place ([`live::apply`]).
    pending: Option<Track>,
    /// Accepted changes so far …
    pub(crate) commits: u64,
    /// … and how many of them the session has (in place or by a reload being asked for).
    pub(crate) applied: u64,
    /// Volumes (mirror, time) edited since they were baked (in memory; shown when the file
    /// exists).
    stale: Vec<(bool, TimeOfDay)>,
    /// A bake queue is running (editing waits).
    bake: Option<baking::BakeRun>,
    /// Placement mode is armed with this library asset ([`placement`]).
    pub(crate) placing: Option<EditorAsset>,
    /// A slider drag of a field ([`Editor::live_set`]).
    pub(crate) live: Option<LiveField>,
}

/// The field a slider drag is changing.
#[derive(Debug)]
pub(crate) struct LiveField {
    pub(crate) item: EditorItem,
    key: String,
    /// A layout change: the document as the drag has it (the road preview's; the document
    /// itself changes on release, when it is validated).
    pub(crate) layout: Option<Doc>,
}

impl Editor {
    /// Applies `edit` to a copy of the document (it may move the selection); keeps it only if
    /// every variant builds. A rejected edit leaves everything as it was and shows why.
    pub(crate) fn edit(
        &mut self,
        edit: impl FnOnce(&mut Doc, &mut Option<EditorItem>) -> Result<(), String>,
    ) {
        self.live = None;
        self.apply_edit(false, edit);
    }

    /// A slider drag's frame of field `key` of `item`: a layout change becomes the road preview
    /// (built off-thread, [`live::preview_track`]); a change the session takes in place is applied
    /// now, one undo step for the whole drag; one that reloads waits for the release.
    pub(crate) fn live_set(
        &mut self,
        item: EditorItem,
        key: &str,
        set: impl FnOnce(&mut Doc, &mut Option<EditorItem>) -> Result<(), String>,
    ) {
        let same = |l: &LiveField| l.item == item && l.key == key;
        let merge = self.live.as_ref().is_some_and(same);
        let mut doc = self.doc.clone();
        let mut selection = self.selection;
        if self.bake.is_some() || set(&mut doc, &mut selection).is_err() {
            return; // the release shows why
        }
        let layout = if doc.def != self.doc.def {
            Some(doc)
        } else {
            if !live::reloads(&self.doc, &doc) {
                self.apply_edit(merge, |d, _| {
                    *d = doc;
                    Ok(())
                });
            }
            None
        };
        self.live = Some(LiveField {
            item,
            key: key.into(),
            layout,
        });
    }

    fn apply_edit(
        &mut self,
        merge: bool,
        edit: impl FnOnce(&mut Doc, &mut Option<EditorItem>) -> Result<(), String>,
    ) {
        if self.bake.is_some() {
            self.error = Some("baking: editing waits until it is done".into());
            return;
        }
        let mut doc = self.doc.clone();
        let mut selection = self.selection;
        let result = edit(&mut doc, &mut selection).and_then(|()| {
            if doc == self.doc {
                return Ok(None);
            }
            let started = Instant::now();
            // Only a layout edit needs building (an environment edit leaves the track as it is).
            let track = (doc.def != self.doc.def)
                .then(|| validate(&doc, &self.props, &self.track))
                .transpose()?;
            if doc.info != self.doc.info {
                catalog::with_entry(&self.catalog_text, &doc.info)?;
            }
            if doc.env != self.doc.env {
                doc.env.validate()?;
            }
            info!("editor: edit validated in {:?}", started.elapsed());
            Ok(Some(track))
        });
        match result {
            Ok(built) => {
                if let Some(track) = built {
                    let old = std::mem::replace(&mut self.doc, doc);
                    self.changed(&old, track);
                    // A drag's later frames: undo goes back to before the drag.
                    if !merge {
                        self.undo.push(old);
                        if self.undo.len() > UNDO_DEPTH {
                            self.undo.remove(0);
                        }
                        self.redo.clear();
                    }
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

    /// The document changed from `old` (`track`: its editing variant, built if the layout
    /// changed): reload when a reload-list field changed (or a changed layout did not build),
    /// else commit in place ([`live`]); every volume is stale now.
    fn changed(&mut self, old: &Doc, track: Option<Track>) {
        self.failed = None;
        self.commits += 1;
        let reload = live::reloads(old, &self.doc)
            | live::fall_back(&mut self.options, &self.doc.info)
            || (track.is_none() && old.def != self.doc.def);
        if reload {
            self.wanted += 1;
            self.pending = None;
            self.applied = self.commits;
        } else {
            self.pending = track;
        }
        for mirror in [false, true] {
            for &time in &self.doc.info.allowed_times {
                if !self.stale.contains(&(mirror, time)) {
                    self.stale.push((mirror, time));
                }
            }
        }
        if self.selection.is_some_and(|s| !self.doc.exists(s)) {
            self.selection = None;
        }
    }

    fn step(&mut self, back: bool) {
        self.live = None;
        let popped = if back {
            self.undo.pop()
        } else {
            self.redo.pop()
        };
        let Some(doc) = popped else {
            return;
        };
        let old = std::mem::replace(&mut self.doc, doc);
        let track = (old.def != self.doc.def)
            .then(|| Track::build(&self.doc.def, &self.props, &live::editing_variant(self)))
            .and_then(|built| {
                built
                    .inspect_err(|e| warn!("editor: undo/redo: {e}; reloading"))
                    .ok()
            });
        self.changed(&old, track);
        if back {
            self.redo.push(old);
        } else {
            self.undo.push(old);
        }
        self.error = None;
    }

    /// A bake wrote the volume of `mirror` × `time`: it is current again.
    pub(crate) fn baked(&mut self, mirror: bool, time: TimeOfDay) {
        self.stale.retain(|v| *v != (mirror, time));
    }

    /// Adds library `asset` at `at` as one edit and selects it.
    pub(crate) fn place(&mut self, asset: &EditorAsset, at: Vec3) {
        let (props, time) = (self.props.clone(), self.time());
        self.edit(|doc, sel| {
            *sel = Some(doc.place(asset, at, &props, time)?);
            Ok(())
        });
    }

    pub(crate) fn select(&mut self, item: Option<EditorItem>) {
        self.selection = item.filter(|i| self.doc.exists(*i));
        if self
            .live
            .as_ref()
            .is_some_and(|l| Some(l.item) != self.selection)
        {
            self.live = None;
        }
    }

    /// The time of day being previewed (its sun, fill and look are the ones edited).
    pub(crate) fn time(&self) -> TimeOfDay {
        fields::parse(&self.options.time).unwrap_or(TimeOfDay::Noon)
    }

    /// Starts baking (see [`EditorCommand::Bake`]; `time` overrides the session's time of day for
    /// a single bake); `exit` quits when done.
    pub(crate) fn start_bake(&mut self, all: bool, time: Option<TimeOfDay>, exit: bool) {
        if self.bake.is_some() || self.mode != Mode::Editing {
            return;
        }
        let time =
            time.unwrap_or_else(|| fields::parse(&self.options.time).unwrap_or(TimeOfDay::Noon));
        let run = baking::BakeRun::new(&self.doc.info.allowed_times, time, all, exit);
        info!(
            "editor: baking ({})",
            if all {
                "every time, both layouts"
            } else {
                "this time"
            }
        );
        self.bake = Some(run);
        self.selection = None; // no tint material in the baked light
        self.placing = None; // nor a ghost
        self.error = None;
        self.wanted += 1;
    }

    /// The next bake job's session (or, after the last, the editing one) should load.
    pub(crate) fn bake_next(&mut self) {
        self.wanted += 1;
    }

    pub(crate) fn bake_done(&mut self) {
        self.wanted += 1;
    }

    /// The running session is built from the current document and options.
    pub(crate) fn session_current(&self) -> bool {
        self.built == self.wanted && !self.sent
    }

    pub(crate) fn set_error(&mut self, error: String) {
        self.error = Some(error);
    }

    /// The layout and look the session should run.
    fn pick(&self) -> TrackPick {
        if let Some(job) = self.bake.as_ref().and_then(baking::BakeRun::current) {
            return TrackPick {
                variant: TrackVariant {
                    track: self.track.clone(),
                    reverse: false,
                    mirror: job.mirror,
                },
                ai: Some(0),
                time: Some(job.time),
                weather: Some(WeatherChoice::Fixed(WeatherKind::Clear)),
                ..default()
            };
        }
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
#[derive(Message, Debug, Clone, PartialEq)]
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
    } else if !ctrl && pressed(KeyCode::KeyP) {
        EditorCommand::ToggleProbes
    } else if !ctrl && pressed(KeyCode::KeyG) {
        EditorCommand::Gizmo {
            mode: EditorGizmo::Move,
        }
    } else if !ctrl && pressed(KeyCode::KeyR) {
        EditorCommand::Gizmo {
            mode: EditorGizmo::Rotate,
        }
    } else if !ctrl && pressed(KeyCode::KeyC) {
        EditorCommand::Gizmo {
            mode: EditorGizmo::Scale,
        }
    } else if !ctrl && pressed(KeyCode::KeyL) {
        EditorCommand::GizmoSpace
    } else if pressed(KeyCode::Escape) {
        EditorCommand::Cancel
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
    probes: Option<ResMut<'w, BakedLight>>,
}

fn commands(
    mut commands: MessageReader<Cmd>,
    mut session: Session,
    mut editor: ResMut<Editor>,
    mut out: MessageWriter<GameCommand>,
) {
    for Cmd(command) in commands.read() {
        let selection = editor.selection;
        if editor.bake.is_some() {
            continue; // baking: the session is the baker's
        }
        match command {
            EditorCommand::Bake { all } => editor.start_bake(*all, None, false),
            EditorCommand::ToggleProbes => {
                if let Some(probes) = session.probes.as_deref_mut() {
                    probes.enabled = !probes.enabled;
                    info!(
                        "editor: light probes {}",
                        if probes.enabled { "on" } else { "off" }
                    );
                }
            }
            EditorCommand::Save => match save::save(
                &session.paths.assets_dir,
                &editor.track,
                &editor.doc,
                &editor.saved,
                &editor.catalog_text,
            ) {
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
                let track = session.track.as_deref();
                editor.edit(|doc, sel| {
                    *sel = Some(doc.add(*what, focus, selection, track)?);
                    Ok(())
                });
            }
            EditorCommand::Swap { asset } => {
                let props = editor.props.clone();
                editor.edit(|doc, sel| doc.swap(sel.ok_or("nothing is selected")?, asset, &props));
            }
            EditorCommand::TopView => {
                let target = selection
                    .and_then(|s| view::anchor(&editor.doc, s))
                    .or(session.focus.0);
                session.view.top(target, session.track.as_deref());
            }
            EditorCommand::LookAt { from, at } => {
                session.view.set(
                    Transform::from_translation(Vec3::from(*from))
                        .looking_at(Vec3::from(*at), Vec3::Y),
                );
            }
            EditorCommand::Focus => {
                if let Some(target) = selection.and_then(|s| view::anchor(&editor.doc, s)) {
                    session.view.look_at(target);
                }
            }
            // The view's (gizmo::commands).
            EditorCommand::Gizmo { .. }
            | EditorCommand::GizmoSpace
            | EditorCommand::Cancel
            | EditorCommand::Drag { .. } => {}
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
    let Some(item) = msg.item.or(editor.selection) else {
        editor.error = Some("nothing is selected".into());
        return;
    };
    let set = |doc: &mut Doc, _: &mut Option<EditorItem>| {
        fields::set_field(doc, item, &msg.key, msg.value.clone(), &choices, track)
    };
    if msg.live {
        editor.live_set(item, &msg.key, set);
    } else {
        editor.edit(set);
    }
}

fn on_options(msg: On<EditorSetOptions>, mut editor: ResMut<Editor>) {
    if editor.options != msg.options {
        editor.options = msg.options.clone();
        editor.wanted += 1;
        // A selected sun or fill is the previewed time's.
        let time = editor.time().into();
        let selection = editor.selection.map(|item| match item {
            EditorItem::Sun { .. } => EditorItem::Sun { time },
            EditorItem::Fill { .. } => EditorItem::Fill { time },
            other => other,
        });
        editor.select(selection);
    }
}

fn note_failures(mut failed: MessageReader<LoadingFailed>, mut editor: ResMut<Editor>) {
    if let Some(f) = failed.read().last() {
        editor.failed = Some(format!("loading failed ({}): {}", f.key, f.error));
    }
}

/// The session builds what the editor holds (in the frame [`live::apply`] commits it).
fn sync_draft(
    editor: Res<Editor>,
    mut draft: ResMut<TrackDraft>,
    mut env: ResMut<EnvironmentDraft>,
) {
    if !editor.is_changed() {
        return;
    }
    if draft.def != editor.doc.def || draft.decor != editor.doc.decor {
        draft.def = editor.doc.def.clone();
        draft.decor = editor.doc.decor.clone();
    }
    if env.def != editor.doc.env {
        env.def = editor.doc.env.clone();
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

/// The stale volumes that exist ("Noon", "Night · mirror").
fn stale_bake(editor: &Editor, assets_dir: &Path) -> Vec<String> {
    editor
        .stale
        .iter()
        .filter(|(mirror, time)| {
            volume_file(assets_dir, editor.track.as_str(), *mirror, *time).is_file()
        })
        .map(|(mirror, time)| format!("{time:?}{}", if *mirror { " · mirror" } else { "" }))
        .collect()
}

/// Whether the session has a baked volume, and whether it is in use.
#[derive(SystemParam)]
struct Probes<'w, 's> {
    setting: Option<Res<'w, BakedLight>>,
    volumes: Query<'w, 's, Ref<'static, BakedGi>>,
}

impl Probes<'_, '_> {
    fn changed(&self) -> bool {
        self.setting.as_ref().is_some_and(|s| s.is_changed())
            || self.volumes.iter().any(|v| v.is_added())
    }

    fn state(&self) -> Option<bool> {
        (!self.volumes.is_empty()).then(|| self.setting.as_ref().is_none_or(|s| s.enabled))
    }
}

fn push_status(
    editor: Res<Editor>,
    paths: Res<UserPaths>,
    state: Res<State<AppState>>,
    track: Option<Res<Track>>,
    probes: Probes,
    mut status: ResMut<EditorStatus>,
) {
    let track_changed = track.as_ref().is_some_and(|t| t.is_changed());
    let probes_changed = probes.changed() || (status.probes.is_some() && probes.volumes.is_empty());
    if !editor.is_changed() && !state.is_changed() && !track_changed && !probes_changed {
        return;
    }
    let info = &editor.doc.info;
    let shown = |item| {
        let (title, fields) = fields::fields(&editor.doc, item, &editor.choices, track.as_deref())?;
        Some(EditorSelection {
            item,
            title,
            fields,
            asset: editor.doc.asset(item),
        })
    };
    let selection = editor.selection.and_then(shown);
    let look = shown(EditorItem::Look {
        time: editor.time().into(),
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
        baking: editor.bake.as_ref().map(|b| b.status.clone()),
        probes: probes.state(),
        stale_bake: stale_bake(&editor, &paths.assets_dir),
        error: editor.failed.clone().or_else(|| editor.error.clone()),
        lints,
        can_undo: !editor.undo.is_empty(),
        can_redo: !editor.redo.is_empty(),
        counts: editor.doc.counts(),
        selection,
        options: editor.options.clone(),
        times: info.allowed_times.iter().map(fields::name).collect(),
        weathers: info.allowed_weather.iter().map(fields::name).collect(),
        tab: editor.tab,
        placing: editor.placing.clone(),
        look,
    });
}
