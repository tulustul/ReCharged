//! In-place session updates (P7): which committed changes reload the session and which replace
//! its `Track` in place, the in-place commit itself, and the off-thread `TrackPreview` builds of a
//! live control-point drag.
//!
//! - **Routing:** after every accepted edit, undo or redo, the reload-list fields of the old and
//!   new document are compared ([`reloads`]); only a difference (or the previewed time or weather
//!   leaving the allowed lists) bumps `wanted`, which reloads. Everything else is committed in
//!   place: the drafts follow ([`crate::sync_draft`]), then [`apply`] inserts the current
//!   variant's `Track` and `TrackSummary` (built by the validation, for a layout edit) and clears
//!   the drag previews, and rc_track / rc_render diff against what they built (an environment
//!   edit: rc_render copies the draft into the session's environment and diffs its lights).
//! - **Point drags** (and slider drags of layout fields, [`crate::Editor::live_set`]): at most one
//!   `Track::build` of the dragged layout in flight on the
//!   `AsyncComputeTaskPool`; the latest pose wins, results older than the last commit are dropped,
//!   a failed build keeps the last good preview and shows its error.

use std::time::{Duration, Instant};

use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, block_on};
use rc_core::prelude::*;
use rc_render::EnvPreview;
use rc_track::{DragPreview, Track, TrackDef, TrackPreview};

use crate::doc::Doc;
use crate::gizmo::Gizmo;
use crate::{Editor, Mode, fields};

/// Changing these rebuilds the session (they are read once per session: materials and barrier
/// style, ambience, music; prop lamps (`practicals`), lamp fixtures (`fixture_materials` and,
/// with any, the `lamps` level they glow at), `shadow_distance`, the sky).
pub(crate) fn reloads(old: &Doc, new: &Doc) -> bool {
    let (a, b) = (&old.info, &new.info);
    let catalog = a.environment != b.environment || a.indoor != b.indoor || a.music != b.music;
    let (a, b) = (&old.env, &new.env);
    let looks = TimeOfDay::ALL.iter().any(|&t| {
        let (x, y) = (a.look(t), b.look(t));
        x.practicals != y.practicals || (!b.fixture_materials.is_empty() && x.lamps != y.lamps)
    });
    catalog
        || looks
        || a.fixture_materials != b.fixture_materials
        || a.shadow_distance != b.shadow_distance
        || a.sky != b.sky
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

/// Commits the pending in-place change: the session gets the edited `Track` (+ its summary; a
/// layout edit) and loses the drag previews in the same frame the drafts changed, before rc_track
/// and rc_render diff (`FlowSet` onward), so nothing shows the old state for a frame. A session
/// that is not the edited one (loading, test driving, reloading) reloads instead.
pub(crate) fn apply(
    mut commands: Commands,
    mut editor: ResMut<Editor>,
    state: Res<State<AppState>>,
    previews: (Option<ResMut<DragPreview>>, Option<ResMut<EnvPreview>>),
) {
    if editor.applied == editor.commits {
        return; // (no `take` through `ResMut`: it would mark the editor changed every frame)
    }
    let track = editor.pending.take();
    let live = *state.get() == AppState::InRace
        && editor.session_current()
        && editor.mode == Mode::Editing;
    if live {
        info!("editor: committed in place");
        if let Some(track) = track {
            commands.insert_resource(track.summary());
            commands.insert_resource(track);
            commands.remove_resource::<TrackPreview>();
        }
        if let Some(mut preview) = previews.0 {
            preview.set_if_neq(DragPreview(None));
        }
        if let Some(mut preview) = previews.1 {
            preview.set_if_neq(EnvPreview(None));
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
    let field = editor.live.as_ref().and_then(|l| l.layout.as_ref());
    let live = (field.cloned())
        .or_else(|| gizmo.layout_preview(&editor.doc))
        .map(|d| d.def);
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
        let old = crate::lights::tests::toy_town();
        let mut new = old.clone();
        new.info.name = "Renamed".into();
        new.def.kill_height = -5.0;
        new.env.noon.exposure += 1.0;
        new.env.noon.sun.azimuth += 10.0;
        new.env.lights[0].pos[0] += 1.0;
        new.env
            .room
            .as_mut()
            .unwrap()
            .lamps
            .as_mut()
            .unwrap()
            .spacing = 20.0;
        assert!(!reloads(&old, &new), "these update in place");
        let edits: [fn(&mut Doc); 5] = [
            |d| d.info.indoor = !d.info.indoor,
            |d| d.env.night.practicals = Some(0.5),
            |d| d.env.fixture_materials.push("tube".into()),
            |d| d.env.shadow_distance = 90.0,
            |d| d.env.sky.stars = 0.0,
        ];
        for (i, edit) in edits.into_iter().enumerate() {
            let mut new = old.clone();
            edit(&mut new);
            assert!(reloads(&old, &new), "edit {i} reloads");
        }
        let mut lamps = old.clone();
        lamps.env.sunset.lamps = 0.2;
        assert!(
            !reloads(&old, &lamps),
            "no fixtures: the room lamps follow live"
        );
        let mut tubes = old.clone();
        tubes.env.fixture_materials.push("tube".into());
        let mut dimmed = tubes.clone();
        dimmed.env.sunset.lamps = 0.2;
        assert!(
            reloads(&tubes, &dimmed),
            "fixtures glow at the lamps level from instancing on"
        );
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
