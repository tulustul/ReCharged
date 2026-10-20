//! The editing session on the real sim and flow: editing holds the race in Setup with no cars,
//! a test drive runs the race, ending it returns to Setup without reloading, and unsaved edits
//! are what the session builds.

use bevy::prelude::*;
use rc_core::prelude::*;
use rc_editor::{EditorDrive, EditorPlugin, EditorStatus};
use rc_testkit::*;
use rc_track::{Track, TrackDef, TrackDraft};

fn editor_app(name: &str) -> App {
    let def = TrackDef::load(&assets_dir(), "test_oval").expect("test_oval");
    let data_dir = std::env::temp_dir().join(format!("rc_editor-it-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&data_dir);
    let paths = move |app: &mut App| {
        app.insert_resource(UserPaths {
            assets_dir: assets_dir(),
            data_dir: data_dir.clone(),
        })
        .insert_resource(AutomationConfig {
            unlock_all: true,
            ..default()
        });
    };
    headless_app((
        paths,
        rc_track::TrackPlugin,
        rc_vehicle::VehiclePlugin,
        rc_weather::WeatherPlugin,
        rc_race::RacePlugin,
        rc_ai::AiPlugin,
        rc_items::ItemsPlugin,
        rc_game::GameFlowPlugin,
        EditorPlugin {
            track: "test_oval".into(),
            def,
        },
    ))
}

fn racers(app: &mut App) -> usize {
    count_entities::<With<Racer>>(app)
}

#[test]
fn editing_holds_setup_and_test_drives_come_back_to_it() {
    let mut app = editor_app("drive");
    step_until(&mut app, "the editing session", |w| {
        *w.resource::<State<AppState>>().get() == AppState::InRace
    });
    step(&mut app, 120);
    assert_eq!(
        race_phase(&app),
        Some(RacePhase::Setup),
        "editing holds Setup"
    );
    assert_eq!(racers(&mut app), 0);
    assert!(!app.world().resource::<EditorStatus>().driving);

    app.world_mut().trigger(EditorDrive);
    step_until(&mut app, "the test drive", |w| {
        w.get_resource::<State<RacePhase>>()
            .is_some_and(|p| *p.get() == RacePhase::Racing)
    });
    assert_eq!(racers(&mut app), 1, "only the player's car");
    assert!(app.world().resource::<EditorStatus>().driving);

    app.world_mut().trigger(EditorDrive);
    step(&mut app, 120);
    assert_eq!(race_phase(&app), Some(RacePhase::Setup), "back to editing");
    assert_eq!(racers(&mut app), 0);
    assert!(!app.world().resource::<EditorStatus>().driving);
}

#[test]
fn the_session_builds_the_draft_and_reports_it_unsaved() {
    let mut app = editor_app("draft");
    let lifted = {
        let mut draft = app.world_mut().resource_mut::<TrackDraft>();
        draft.def.kill_height -= 1.0;
        draft.def.kill_height
    };
    step_until(&mut app, "the editing session", |w| {
        w.get_resource::<Track>().is_some()
    });
    assert_eq!(app.world().resource::<Track>().kill_height(), lifted);
    step(&mut app, 1);
    assert!(app.world().resource::<EditorStatus>().dirty);
}
