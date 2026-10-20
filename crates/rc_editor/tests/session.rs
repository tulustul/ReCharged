//! The editing session on the real sim and flow: editing holds the race in Setup with no cars,
//! a test drive runs the race, ending it returns to Setup without reloading, accepted edits
//! update the session in place (only reload-list fields reload), rejected ones change nothing,
//! undo restores; library assets are placed, armed and swapped.

use bevy::prelude::*;
use rc_core::prelude::*;
use rc_editor::{
    EditorAsset, EditorCommand, EditorCommandMsg, EditorItem, EditorLibrary, EditorPlace,
    EditorPlugin, EditorSelect, EditorSet, EditorStatus,
};
use rc_testkit::*;
use rc_track::{Track, TrackDraft};
use rc_ui::editor::FieldValue;

fn editor_app(name: &str) -> App {
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
        EditorPlugin::load(&assets_dir(), "test_oval").expect("test_oval"),
    ))
}

fn drive(app: &mut App) {
    app.world_mut().trigger(EditorCommandMsg {
        command: EditorCommand::Drive,
    });
}

fn in_race_setup(w: &World) -> bool {
    *w.resource::<State<AppState>>().get() == AppState::InRace
        && w.get_resource::<State<RacePhase>>()
            .is_some_and(|p| *p.get() == RacePhase::Setup)
        && !w.resource::<EditorStatus>().loading
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

    drive(&mut app);
    step_until(&mut app, "the test drive", |w| {
        w.get_resource::<State<RacePhase>>()
            .is_some_and(|p| *p.get() == RacePhase::Racing)
    });
    assert_eq!(racers(&mut app), 1, "only the player's car");
    assert!(app.world().resource::<EditorStatus>().driving);

    drive(&mut app);
    step(&mut app, 120);
    assert_eq!(race_phase(&app), Some(RacePhase::Setup), "back to editing");
    assert_eq!(racers(&mut app), 0);
    assert!(!app.world().resource::<EditorStatus>().driving);
}

/// The session's lap length (a moved control point changes it).
fn length(app: &App) -> f32 {
    app.world().resource::<Track>().length()
}

#[test]
fn an_edit_updates_the_session_in_place_a_bad_one_is_rejected_and_undo_restores() {
    let mut app = editor_app("edit");
    step_until(&mut app, "the editing session", in_race_setup);
    let point = EditorItem::Point { index: 3 };
    app.world_mut().trigger(EditorSelect { item: Some(point) });
    let before = app.world().resource::<TrackDraft>().def.points[3].pos;
    let length_before = length(&app);
    let moved = before + Vec3::X * 0.5;
    app.world_mut().trigger(EditorSet {
        key: "pos".into(),
        value: Some(FieldValue::Vector {
            value: moved.to_array().to_vec(),
        }),
    });
    for _ in 0..3 {
        step(&mut app, 1);
        assert!(
            in_race_setup(app.world()),
            "no reload, not even for a frame"
        );
    }
    assert_eq!(
        app.world().resource::<TrackDraft>().def.points[3].pos,
        moved
    );
    assert_ne!(
        length(&app),
        length_before,
        "the session's track is the edited one"
    );
    let status = app.world().resource::<EditorStatus>();
    assert!(status.dirty && status.can_undo && status.error.is_none());
    assert_eq!(status.selection.as_ref().map(|s| s.item), Some(point));

    app.world_mut().trigger(EditorSet {
        key: "width".into(),
        value: Some(FieldValue::Number { value: 1.0 }),
    });
    step(&mut app, 1);
    let status = app.world().resource::<EditorStatus>();
    assert!(status.error.is_some(), "a 1 u wide track does not build");
    assert!(!status.loading);
    assert_eq!(
        app.world().resource::<TrackDraft>().def.points[3].width,
        None
    );

    app.world_mut().trigger(EditorCommandMsg {
        command: EditorCommand::Undo,
    });
    step(&mut app, 2);
    assert!(in_race_setup(app.world()), "undo is in place too");
    assert_eq!(
        app.world().resource::<TrackDraft>().def.points[3].pos,
        before
    );
    assert_eq!(length(&app), length_before);
    assert!(!app.world().resource::<EditorStatus>().dirty);

    // The catalog's `indoor` is on the reload list.
    app.world_mut().trigger(EditorSelect {
        item: Some(EditorItem::Track),
    });
    app.world_mut().trigger(EditorSet {
        key: "indoor".into(),
        value: Some(FieldValue::Toggle { value: true }),
    });
    step(&mut app, 2);
    assert!(app.world().resource::<EditorStatus>().loading, "reloading");
    step_until(&mut app, "the reloaded session", in_race_setup);
}

#[test]
fn library_assets_are_placed_selected_and_swapped_in_place() {
    let mut app = editor_app("place");
    step_until(&mut app, "the editing session", in_race_setup);
    assert!(!app.world().resource::<EditorLibrary>().groups.is_empty());
    let props = app.world().resource::<TrackDraft>().def.props.len();
    let crate_small = EditorAsset::Prop {
        id: "crate_small".into(),
    };
    // Without a position: arms placement mode (the same asset again disarms).
    let arm = || EditorPlace {
        asset: crate_small.clone(),
        at: None,
    };
    app.world_mut().trigger(arm());
    step(&mut app, 1);
    let placing = |app: &App| app.world().resource::<EditorStatus>().placing.clone();
    assert_eq!(placing(&app), Some(crate_small.clone()));
    app.world_mut().trigger(arm());
    step(&mut app, 1);
    assert_eq!(placing(&app), None);

    let at = Vec3::new(3.0, 0.0, -4.0);
    app.world_mut().trigger(EditorPlace {
        asset: crate_small.clone(),
        at: Some(at.to_array()),
    });
    step(&mut app, 2);
    assert!(in_race_setup(app.world()), "placed in place");
    let draft = &app.world().resource::<TrackDraft>().def;
    assert_eq!(draft.props.len(), props + 1);
    let placed = EditorItem::Prop {
        index: props as u32,
    };
    let status = app.world().resource::<EditorStatus>();
    let selection = status.selection.as_ref().expect("the new prop is selected");
    assert_eq!(
        (selection.item, selection.asset.clone()),
        (placed, Some(crate_small))
    );

    app.world_mut().trigger(EditorCommandMsg {
        command: EditorCommand::Swap {
            asset: EditorAsset::Prop {
                id: "tyre_stack".into(),
            },
        },
    });
    step(&mut app, 2);
    let prop = &app.world().resource::<TrackDraft>().def.props[props];
    assert_eq!((prop.kind.as_str(), prop.pos), ("tyre_stack", at));
    assert!(in_race_setup(app.world()));
}
