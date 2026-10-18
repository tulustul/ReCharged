//! Headless lifecycle tests: no window, no GPU; audio output forced off (or probed with the
//! master volume at 0, so a machine with a sound card stays silent).

use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use rc_audio::{GameAudioPlugin, OutputState};
use rc_core::prelude::*;
use rc_testkit::*;

fn race() -> RaceConfig {
    RaceConfig {
        variant: TrackVariant::forward("test_oval"),
        laps: 2,
        weather: WeatherSetup {
            weather: WeatherChoice::Fixed(WeatherKind::Storm),
            time: TimeOfDay::Night,
        },
        ..default()
    }
}

fn muted_app() -> App {
    headless_app((
        |app: &mut App| {
            app.insert_resource(OutputState::Unavailable);
        },
        AssetPlugin::default(),
        GameAudioPlugin,
    ))
}

#[test]
fn without_a_device_the_whole_lifecycle_runs_muted_and_loading_completes() {
    let mut app = muted_app();
    load_race(&mut app, race()).expect("the \"audio\" key completes without a device");
    set_phase(&mut app, RacePhase::Intro);
    set_phase(&mut app, RacePhase::Countdown);
    set_phase(&mut app, RacePhase::Racing);
    let world = app.world_mut();
    world.write_message(UiSound(UiCue::Select));
    world.write_message(RaceEvent {
        racer: None,
        kind: RaceEventKind::Go,
    });
    world.write_message(WeatherEvent::Lightning { intensity: 1.0 });
    world.write_message(MusicStinger(MusicCue::CupVictory));
    step(&mut app, 3);
    pause(&mut app);
    resume(&mut app);
    restart(&mut app);
    set_phase(&mut app, RacePhase::Finished);
    next_race(&mut app, None).expect("next race loads again");
    quit_to_menu(&mut app);
    step(&mut app, 3);
    assert_eq!(app_state(&app), AppState::Menu);
    assert_eq!(
        *app.world().resource::<OutputState>(),
        OutputState::Unavailable
    );
}

#[test]
fn the_device_probe_resolves_within_a_few_frames() {
    let mut app = headless_app((
        |app: &mut App| {
            let mut settings = Settings::default();
            settings.audio.master = 0.0;
            app.insert_resource(settings);
        },
        AssetPlugin::default(),
        GameAudioPlugin,
    ));
    step(&mut app, 10);
    let state = *app.world().resource::<OutputState>();
    assert_ne!(
        state,
        OutputState::Probing,
        "probe still running after 10 frames"
    );
    // Whatever the machine has, a race session must still load.
    load_race(&mut app, race()).expect("loads with or without a device");
    quit_to_menu(&mut app);
}
