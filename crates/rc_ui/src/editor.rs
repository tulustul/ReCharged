//! `editor.*` channels of the track editor (`rc_editor`): its status for React and its commands.
//! rc_editor writes the [`EditorStatus`] resource and observes the command messages; the game has
//! neither, so `editor.snapshot` answers `null` there and React shows the game UI.

use bevy::prelude::*;
use bevy_react::prelude::*;

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
    /// The layout differs from the file.
    pub dirty: bool,
    /// Test driving (the game's race UI is up) instead of editing.
    pub driving: bool,
    /// The last failure (load, save), if any.
    pub error: Option<String>,
    /// Build warnings of the session's track.
    pub lints: Vec<String>,
}

/// React → Bevy request `editor.snapshot`: the [`EditorStatus`], `null` outside the editor.
#[react_request(name = "editor.snapshot", response = Option<EditorStatus>)]
pub struct EditorSnapshot;

/// React → Bevy message `editor.save`: write the layout to its track file.
#[react_message(name = "editor.save")]
pub struct EditorSave;

/// React → Bevy message `editor.drive`: start a test drive, or end it and edit again.
#[react_message(name = "editor.drive")]
pub struct EditorDrive;

pub(crate) fn register(app: &mut App) {
    app.add_react_event::<EditorStatus>()
        .add_react_message::<EditorSave>()
        .add_react_message::<EditorDrive>()
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
