//! In-place session updates (P7): which committed changes reload the session and which replace
//! its `Track` in place, the in-place commit itself, and the off-thread `TrackPreview` builds of a
//! live control-point drag.
//!
//! - **Routing:** after every accepted edit, undo or redo, the reload-list fields of the old and
//!   new document are compared ([`reloads`]); only a difference (or the previewed time or weather
//!   leaving the allowed lists) bumps `wanted`, which reloads. Everything else is committed in
//!   place: the draft follows ([`crate::sync_draft`]), then [`apply`] inserts the current
//!   variant's `Track` and `TrackSummary` (built by the validation), and rc_track / rc_render
//!   diff against what they built.
//! - **Point drags:** at most one `Track::build` of the dragged layout in flight on the
//!   `AsyncComputeTaskPool`; the latest pose wins, results older than the last commit are dropped,
//!   a failed build keeps the last good preview and shows its error.

use std::time::{Duration, Instant};

use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, block_on};
use rc_core::prelude::*;
use rc_track::{DragPreview, Track, TrackDef, TrackPreview};

use crate::doc::Doc;
use crate::gizmo::Gizmo;
use crate::{Editor, Mode, fields};

/// Changing these rebuilds the session (they are read once per session: materials and barrier
/// style, ambience, music; P9 adds the environment's).
pub(crate) fn reloads(old: &Doc, new: &Doc) -> bool {
    let (a, b) = (&old.info, &new.info);
    a.environment != b.environment || a.indoor != b.indoor || a.music != b.music
}

/// The session's time and weather fall back to the first allowed value when the entry no longer
/// allows them; true when one did.
pub(crate) fn fall_back(options: &mut rc_ui::editor::EditorOptions, info: &TrackInfo) -> bool {
    let mut changed = false;
    let time = fields::parse::<TimeOfDay>(&options.time).ok();
    if let Some(first) = info.allowed_times.first()
        && time.is_none_or(|t| !info.allowed_times.contains(&t))
    {
        options.time = fields::name(first);
        changed = true;
    }
    let weather = fields::parse::<WeatherKind>(&options.weather).ok();
    if let Some(first) = info.allowed_weather.first()
        && weather.is_none_or(|w| !info.allowed_weather.contains(&w))
    {
        options.weather = fields::name(first);
        changed = true;
    }
    changed
}

/// The session's variant while editing (always forward, unmirrored).
pub(crate) fn editing_variant(editor: &Editor) -> TrackVariant {
    TrackVariant::forward(editor.track.as_str())
}

/// Commits the pending in-place change: the session gets the edited `Track` (+ its summary) and
/// loses the drag previews in the same frame, before rc_track and rc_render diff (`FlowSet`
/// onward), so nothing shows the old state for a frame. A session that is not the edited one
/// (loading, test driving, reloading) reloads instead.
pub(crate) fn apply(
    mut commands: Commands,
    mut editor: ResMut<Editor>,
    state: Res<State<AppState>>,
    preview: Option<ResMut<DragPreview>>,
) {
    if editor.pending.is_none() {
        return; // (no `take` through `ResMut`: it would mark the editor changed every frame)
    }
    let Some(track) = editor.pending.take() else {
        return;
    };
    let live = *state.get() == AppState::InRace
        && editor.session_current()
        && editor.mode == Mode::Editing;
    if live {
        info!("editor: committed in place");
        commands.insert_resource(track.summary());
        commands.insert_resource(track);
        commands.remove_resource::<TrackPreview>();
        if let Some(mut preview) = preview {
            preview.set_if_neq(DragPreview(None));
        }
    } else {
        editor.wanted += 1;
    }
    editor.applied = editor.commits;
}

/// A finished preview build: the track (or why it did not build) and how long it took.
type Built = (Result<Track, String>, Duration);

/// The live control-point drag's preview build.
#[derive(Resource, Default)]
pub(crate) struct PreviewBuild {
    task: Option<(u64, Task<Built>)>,
    /// The layout last sent to a build.
    asked: Option<TrackDef>,
    /// The error of the last failed build (the toast shows it until a build succeeds).
    failed: Option<String>,
    /// Builds and their total time during this drag (logged when it ends).
    count: u32,
    total: Duration,
}

/// Every frame after the gizmo: keeps one build of the dragged layout in flight and publishes
/// finished ones as `TrackPreview`; the drag's end drops the build and the preview (a commit's
/// own frame removes it, see [`apply`]).
pub(crate) fn preview_track(
    mut commands: Commands,
    mut editor: ResMut<Editor>,
    gizmo: Res<Gizmo>,
    mut build: ResMut<PreviewBuild>,
    preview: Option<Res<TrackPreview>>,
) {
    let live = gizmo.layout_preview(&editor.doc).map(|d| d.def);
    let Some(def) = live else {
        if build.count > 0 {
            info!(
                "editor: drag preview: {} builds, {:.1} ms each",
                build.count,
                build.total.as_secs_f32() * 1000.0 / build.count as f32
            );
        }
        // A cancelled drag takes its preview error along; a commit shows its own outcome.
        if !gizmo.settling() && build.failed.is_some() && editor.error == build.failed {
            editor.error = None;
        }
        *build = PreviewBuild::default();
        if preview.is_some() && !gizmo.settling() {
            commands.remove_resource::<TrackPreview>();
        }
        return;
    };
    if build.task.as_ref().is_some_and(|(_, t)| t.is_finished())
        && let Some((generation, task)) = build.task.take()
    {
        let (result, took) = block_on(task);
        build.count += 1;
        build.total += took;
        debug!("editor: preview built in {took:?}");
        match result {
            // A commit since the build started: its track is newer.
            _ if generation < editor.commits => {}
            Ok(track) => {
                commands.insert_resource(TrackPreview { track, generation });
                if let Some(e) = build.failed.take()
                    && editor.error == Some(e)
                {
                    editor.error = None;
                }
            }
            Err(e) => {
                build.failed = Some(e.clone());
                editor.error = Some(e);
            }
        }
    }
    if build.task.is_none() && build.asked.as_ref() != Some(&def) {
        let props = editor.props.clone();
        let variant = editing_variant(&editor);
        build.asked = Some(def.clone());
        let task = AsyncComputeTaskPool::get().spawn(async move {
            let started = Instant::now();
            (Track::build(&def, &props, &variant), started.elapsed())
        });
        build.task = Some((editor.commits, task));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rc_ui::editor::EditorOptions;

    #[test]
    fn only_reload_list_fields_reload() {
        let old = Doc {
            def: TrackDef::default(),
            decor: Vec::new(),
            info: TrackInfo::default(),
        };
        let mut new = old.clone();
        new.info.name = "Renamed".into();
        new.def.kill_height = -5.0;
        assert!(!reloads(&old, &new));
        new.info.indoor = !old.info.indoor;
        assert!(reloads(&old, &new));
    }

    #[test]
    fn a_removed_time_or_weather_falls_back_to_the_first_allowed() {
        let info = TrackInfo {
            allowed_times: vec![TimeOfDay::Sunset, TimeOfDay::Night],
            allowed_weather: vec![WeatherKind::Clear],
            ..default()
        };
        let mut options = EditorOptions {
            time: fields::name(&TimeOfDay::Night),
            weather: fields::name(&WeatherKind::Clear),
            ..default()
        };
        assert!(!fall_back(&mut options, &info));
        options.time = fields::name(&TimeOfDay::Noon);
        assert!(fall_back(&mut options, &info));
        assert_eq!(options.time, fields::name(&TimeOfDay::Sunset));
    }
}
