//! `editor.*` channels of the track editor (`rc_editor`): its status for React and its commands.
//! rc_editor writes the [`EditorStatus`] resource and observes the messages; the game has neither,
//! so `editor.snapshot` answers `null` there and React shows the game UI.

use bevy::prelude::*;
use bevy_react::prelude::*;

use crate::content::UiTimeOfDay;

/// Something in the layout the editor can select.
#[derive(
    serde::Serialize, serde::Deserialize, ts_rs::TS, Debug, Clone, Copy, PartialEq, Eq, Hash,
)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(tag = "kind", rename_all = "camelCase")]
#[allow(missing_docs)] // the variants name `TrackDef`'s lists; indices into them
pub enum EditorItem {
    /// Track-wide settings (style, start, heights, bounds) and the catalog entry.
    Track,
    Point {
        index: u32,
    },
    Gap {
        index: u32,
    },
    Ground {
        index: u32,
    },
    Vertex {
        ground: u32,
        index: u32,
    },
    ItemRow {
        index: u32,
    },
    Prop {
        index: u32,
    },
    Shortcut {
        index: u32,
    },
    PathPoint {
        shortcut: u32,
        index: u32,
    },
    Battery,
    Decor {
        index: u32,
    },
    /// An environment point or spot light (index into `EnvironmentDef::lights`).
    Light {
        index: u32,
    },
    /// The sun (key light) of a time of day's look.
    Sun {
        time: UiTimeOfDay,
    },
    /// The fill light of a time of day's look (it may have none).
    Fill {
        time: UiTimeOfDay,
    },
    /// A time of day's look (exposure, fog, grading …) and the room's lamp grid: the Lighting
    /// tab's fields.
    Look {
        time: UiTimeOfDay,
    },
}

/// A field's value as the inspector edits it.
#[derive(serde::Serialize, serde::Deserialize, ts_rs::TS, Debug, Clone, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(tag = "type", rename_all = "camelCase")]
#[allow(missing_docs)]
pub enum FieldValue {
    Number {
        value: f32,
    },
    Integer {
        value: i32,
    },
    Toggle {
        value: bool,
    },
    /// One of `options` (enum variant names, prop kinds).
    Choice {
        value: String,
        options: Vec<String>,
    },
    /// 2 or 3 components.
    Vector {
        value: Vec<f32>,
    },
    Text {
        value: String,
    },
}

/// One field of the selected item.
#[derive(serde::Serialize, ts_rs::TS, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct EditorField {
    /// Field name as in the data file (`width`, `wall_left`).
    pub key: String,
    /// Current value; for an unset optional field, the value it inherits or defaults to.
    pub value: FieldValue,
    /// May be unset (inherited from the previous control point, or absent).
    pub optional: bool,
    /// Set explicitly (always true for non-optional fields).
    pub set: bool,
}

/// The selected item and its fields.
#[derive(serde::Serialize, ts_rs::TS, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct EditorSelection {
    /// What it is.
    pub item: EditorItem,
    /// Heading ("Point 12 · T1").
    pub title: String,
    /// Its fields, in file order.
    pub fields: Vec<EditorField>,
    /// The library asset it shows (props and decor: what a swap replaces).
    pub asset: Option<EditorAsset>,
}

/// A library asset: what placing adds and swapping puts in place of the selection.
#[derive(serde::Serialize, serde::Deserialize, ts_rs::TS, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(tag = "kind", rename_all = "camelCase")]
pub enum EditorAsset {
    /// A prop kind (`assets/data/props/*.ron`).
    Prop {
        /// The kind's id.
        id: String,
    },
    /// A decorative model.
    Decor {
        /// glb path under the assets root (`models/props/toy/sign.glb`).
        model: String,
    },
    /// An item pickup row.
    ItemRow,
    /// The Golden Battery (one per track).
    Battery,
    /// An environment light: a spot (aimed 5 u below when placed) or a point light.
    Light {
        /// A spot light.
        spot: bool,
    },
    /// Selects the previewed time's sun, aimed from where it is placed.
    Sun,
    /// The previewed time's fill light (added if it has none).
    Fill,
}

/// One entry of the asset library.
#[derive(serde::Serialize, ts_rs::TS, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct LibraryEntry {
    /// Shown name ("Toy block cube").
    pub name: String,
    /// What it places.
    pub asset: EditorAsset,
}

/// A group of the asset library ("Props · toy").
#[derive(serde::Serialize, ts_rs::TS, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct LibraryGroup {
    /// Heading.
    pub title: String,
    /// Expanded at first (the track's own environment, gameplay items).
    pub open: bool,
    /// Its entries, by name.
    pub entries: Vec<LibraryEntry>,
}

/// The asset library (response of `editor.library`), built once when the editor starts.
#[derive(serde::Serialize, ts_rs::TS, Resource, Debug, Clone, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct EditorLibrary {
    /// Groups: the track's own environment first.
    pub groups: Vec<LibraryGroup>,
}

/// React → Bevy request `editor.library`: the [`EditorLibrary`], `null` outside the editor.
#[react_request(name = "editor.library", response = Option<EditorLibrary>)]
pub struct EditorLibraryRequest;

/// How many of each item the layout has (the item list).
#[derive(serde::Serialize, ts_rs::TS, Debug, Clone, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
#[allow(missing_docs)]
pub struct EditorCounts {
    pub points: u32,
    pub gaps: u32,
    pub ground: u32,
    pub item_rows: u32,
    pub props: u32,
    pub shortcuts: u32,
    pub decor: u32,
    pub battery: bool,
}

/// The editor's left-pane tab. Bevy needs it: the Track tab shows the layout handles.
#[derive(
    serde::Serialize, serde::Deserialize, ts_rs::TS, Debug, Clone, Copy, Default, PartialEq, Eq,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
#[allow(missing_docs)]
pub enum EditorTab {
    Assets,
    #[default]
    Track,
    Lighting,
}

/// Session look and test-drive layout (rc_core names: `Noon`, `Rain`).
#[derive(serde::Serialize, serde::Deserialize, ts_rs::TS, Debug, Clone, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct EditorOptions {
    /// Time of day of the session.
    pub time: String,
    /// Weather preset of the session.
    pub weather: String,
    /// Test drives run the reverse layout (editing stays forward).
    pub reverse: bool,
    /// Test drives run the mirrored layout.
    pub mirror: bool,
}

/// Bevy → React event `editor.status` (also the `editor.snapshot` response): what the editor
/// shows. A resource written by rc_editor; sent whenever it changes.
#[react_event(name = "editor.status")]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct EditorStatus {
    /// Track id (file stem).
    pub track: String,
    /// Display name from the catalog.
    pub name: String,
    /// The layout or decor differs from the files.
    pub dirty: bool,
    /// Test driving (the game's race UI is up) instead of editing.
    pub driving: bool,
    /// A session (re)load is pending or running.
    pub loading: bool,
    /// Baking progress ("Baking Noon · mirror (5/8): 42 %"); editing waits meanwhile.
    pub baking: Option<String>,
    /// The session's baked light probes are in use (`null`: this layout and time have none).
    pub probes: Option<bool>,
    /// Baked volumes edited since they were baked ("Noon", "Night · mirror"); the bake is stale.
    pub stale_bake: Vec<String>,
    /// The last failure (an edit that would not build, load, save), if any.
    pub error: Option<String>,
    /// Build warnings of the session's track.
    pub lints: Vec<String>,
    /// Undo / redo are possible.
    pub can_undo: bool,
    #[allow(missing_docs)]
    pub can_redo: bool,
    /// Item counts.
    pub counts: EditorCounts,
    /// The selected item.
    pub selection: Option<EditorSelection>,
    /// Session options.
    pub options: EditorOptions,
    /// Allowed times of day of the track.
    pub times: Vec<String>,
    /// Allowed weather presets of the track.
    pub weathers: Vec<String>,
    /// The open left-pane tab.
    pub tab: EditorTab,
    /// The library asset placement mode is armed with (the next click in the view adds it).
    pub placing: Option<EditorAsset>,
    /// The previewed time's look and the room's lamp grid (the Lighting tab; set with
    /// `editor.set` naming its item).
    pub look: Option<EditorSelection>,
}

/// React → Bevy request `editor.snapshot`: the [`EditorStatus`], `null` outside the editor.
#[react_request(name = "editor.snapshot", response = Option<EditorStatus>)]
pub struct EditorSnapshot;

/// A layout feature added at the view's focus (the point under the screen centre; library assets
/// are placed instead, [`EditorPlace`]).
#[derive(serde::Serialize, serde::Deserialize, ts_rs::TS, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
#[allow(missing_docs)]
pub enum EditorAdd {
    Ground,
    Shortcut,
    Gap,
}

/// What the transform gizmo does to the selection.
#[derive(
    serde::Serialize, serde::Deserialize, ts_rs::TS, Debug, Clone, Copy, Default, PartialEq, Eq,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
#[allow(missing_docs)]
pub enum EditorGizmo {
    #[default]
    Move,
    Rotate,
    Scale,
}

/// An editor command (also bound to keys in Rust).
#[derive(serde::Serialize, serde::Deserialize, ts_rs::TS, Debug, Clone, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(tag = "kind", rename_all = "camelCase")]
pub enum EditorCommand {
    /// Write the layout and decor files (Ctrl+S).
    Save,
    /// Start a test drive, or end it and edit again (F5).
    Drive,
    /// Ctrl+Z.
    Undo,
    /// Ctrl+Y / Ctrl+Shift+Z.
    Redo,
    /// Remove the selection (Delete).
    Delete,
    /// Insert a point / vertex / path point after the selected one (Insert).
    Insert,
    /// Copy the selected prop, row, ground or decor next to it (Ctrl+D).
    Duplicate,
    /// Add an item at the view's focus.
    Add {
        /// What.
        what: EditorAdd,
    },
    /// Look straight down from above the selection (T).
    TopView,
    /// Fly to the selection (F).
    Focus,
    /// Put the camera at `from` looking at `at` (scripted screenshots).
    LookAt {
        #[allow(missing_docs)]
        from: [f32; 3],
        #[allow(missing_docs)]
        at: [f32; 3],
    },
    /// Bake the irradiance volume of the current time of day (forward layout), or of every allowed
    /// time in both layouts (`all`).
    Bake {
        /// Every allowed time, forward and mirrored.
        all: bool,
    },
    /// Use the session's baked irradiance volume or not (P): compare the lighting with and
    /// without it.
    ToggleProbes,
    /// Gizmo mode (G · R · C).
    Gizmo {
        /// The mode.
        mode: EditorGizmo,
    },
    /// Gizmo space for move and rotate: world ↔ the selection's own axes (L).
    GizmoSpace,
    /// Cancel placement or a live gizmo drag, else clear the selection (Esc).
    Cancel,
    /// The selected prop or decor item becomes `asset` (same type), keeping its transform.
    Swap {
        /// The new asset.
        asset: EditorAsset,
    },
    /// Move, turn and scale the selection as a gizmo drag would (automation): `hold` leaves the
    /// drag live (previewed, uncommitted), else it is committed as one undo step. Only what the
    /// selection's data stores applies (props turn about Y only, points only move).
    Drag {
        /// Added to the position (u, world axes).
        translate: [f32; 3],
        /// Turned by (pitch, yaw, roll) degrees, applied after its rotation.
        rotate: [f32; 3],
        /// Multiplies the scale per axis.
        scale: [f32; 3],
        /// Keep the drag live.
        hold: bool,
    },
}

/// React → Bevy message `editor.command`.
#[react_message(name = "editor.command")]
#[derive(Debug, Clone)]
pub struct EditorCommandMsg {
    /// The command.
    pub command: EditorCommand,
}

/// React → Bevy message `editor.select`: select an item (`null` = nothing).
#[react_message(name = "editor.select")]
#[derive(Debug, Clone)]
pub struct EditorSelect {
    /// The item.
    pub item: Option<EditorItem>,
}

/// React → Bevy message `editor.set`: set a field of the selection, or of `item` (`null` = unset
/// an optional field). Rejected (with `error`) when the layout would not build.
#[react_message(name = "editor.set")]
#[derive(Debug, Clone)]
pub struct EditorSet {
    /// The item (`null`: the selection).
    pub item: Option<EditorItem>,
    /// Field name.
    pub key: String,
    /// New value.
    pub value: Option<FieldValue>,
}

/// React → Bevy message `editor.options`: session time and weather, test-drive layout.
#[react_message(name = "editor.options")]
#[derive(Debug, Clone)]
pub struct EditorSetOptions {
    /// The options.
    pub options: EditorOptions,
}

/// React → Bevy message `editor.place`: arm placement mode with `asset` (the same asset again
/// disarms it; another re-arms), or, with `at`, add it there at once (automation).
#[react_message(name = "editor.place")]
#[derive(Debug, Clone)]
pub struct EditorPlace {
    /// What to place.
    pub asset: EditorAsset,
    /// Add it here now (u) instead of arming.
    pub at: Option<[f32; 3]>,
}

/// React → Bevy message `editor.tab`: the left pane switched tabs.
#[react_message(name = "editor.tab")]
#[derive(Debug, Clone)]
pub struct EditorSetTab {
    /// The tab now open.
    pub tab: EditorTab,
}

pub(crate) fn register(app: &mut App) {
    app.add_react_event::<EditorStatus>()
        .add_react_message::<EditorCommandMsg>()
        .add_react_message::<EditorSelect>()
        .add_react_message::<EditorSet>()
        .add_react_message::<EditorSetOptions>()
        .add_react_message::<EditorSetTab>()
        .add_react_message::<EditorPlace>()
        .add_react_request_handler(
            |request: On<Request<EditorSnapshot>>, status: Option<Res<EditorStatus>>| {
                request.respond(status.map(|s| s.clone()));
            },
        )
        .add_react_request_handler(
            |request: On<Request<EditorLibraryRequest>>, library: Option<Res<EditorLibrary>>| {
                request.respond(library.map(|l| l.clone()));
            },
        );
}

/// Sends `editor.status` when rc_editor changed it.
pub(crate) fn push_status(status: Option<Res<EditorStatus>>, events: ReactEvents) {
    if let Some(status) = status.filter(|s| s.is_changed()) {
        events.send(&*status);
    }
}
