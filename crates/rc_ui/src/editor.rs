//! `editor.*` channels of the track editor (`rc_editor`): its status for React and its commands.
//! rc_editor writes the [`EditorStatus`] resource and observes the messages; the game has neither,
//! so `editor.snapshot` answers `null` there and React shows the game UI.

use bevy::prelude::*;
use bevy_react::prelude::*;

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
}

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
    /// Item kinds with visible (pickable) handles: `points`, `props`, `ground`, `rows`,
    /// `shortcuts`, `decor`.
    pub layers: Vec<String>,
}

/// React → Bevy request `editor.snapshot`: the [`EditorStatus`], `null` outside the editor.
#[react_request(name = "editor.snapshot", response = Option<EditorStatus>)]
pub struct EditorSnapshot;

/// Something added at the view's focus (the point under the screen centre).
#[derive(serde::Serialize, serde::Deserialize, ts_rs::TS, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
#[allow(missing_docs)]
pub enum EditorAdd {
    Prop,
    ItemRow,
    Ground,
    Shortcut,
    Gap,
    Battery,
    Decor,
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
    /// Show or hide one kind of handle.
    ToggleLayer {
        /// Layer name (see [`EditorStatus::layers`]).
        layer: String,
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

/// React → Bevy message `editor.set`: set a field of the selection (`null` = unset an optional
/// field). Rejected (with `error`) when the layout would not build.
#[react_message(name = "editor.set")]
#[derive(Debug, Clone)]
pub struct EditorSet {
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

pub(crate) fn register(app: &mut App) {
    app.add_react_event::<EditorStatus>()
        .add_react_message::<EditorCommandMsg>()
        .add_react_message::<EditorSelect>()
        .add_react_message::<EditorSet>()
        .add_react_message::<EditorSetOptions>()
        .add_react_request_handler(
            |request: On<Request<EditorSnapshot>>, status: Option<Res<EditorStatus>>| {
                request.respond(status.map(|s| s.clone()));
            },
        );
}

/// Sends `editor.status` when rc_editor changed it.
pub(crate) fn push_status(status: Option<Res<EditorStatus>>, events: ReactEvents) {
    if let Some(status) = status.filter(|s| s.is_changed()) {
        events.send(&*status);
    }
}
