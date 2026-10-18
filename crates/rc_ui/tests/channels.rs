//! React messages reach the app flow: what the menu and race buttons send (without a JS runtime:
//! the messages are triggered the way bevy-react delivers them).

use bevy::prelude::*;
use rc_core::prelude::*;
use rc_game::GameFlowPlugin;
use rc_testkit::*;
use rc_ui::race::{QuitRace, RestartRace, StartRace, UiMode, UiPlayerChoice};

fn app() -> App {
    headless_app((GameFlowPlugin, rc_ui::register_bindings))
}

fn wait_for_phase(app: &mut App, phase: RacePhase) {
    step_until(app, &format!("{phase:?}"), |w| {
        w.get_resource::<State<RacePhase>>()
            .is_some_and(|s| *s.get() == phase)
    });
}

#[test]
fn race_buttons_start_restart_and_quit_without_reloading() {
    let mut app = app();
    app.world_mut().trigger(StartRace {
        mode: UiMode::QuickRace,
        track: "test_oval".into(),
        reverse: false,
        mirror: false,
        players: vec![UiPlayerChoice {
            slot: 0,
            car: "zip".into(),
            paint: 1,
        }],
        ai: Some(2),
        difficulty: None,
        laps: None,
        weather: None,
        time: None,
        items: None,
    });
    wait_for_phase(&mut app, RacePhase::Countdown);
    let config = app.world().resource::<RaceConfig>().clone();
    assert_eq!(
        (config.racers.len(), config.racers[0].car.as_str()),
        (3, "zip")
    );

    app.world_mut().trigger(RestartRace);
    wait_for_phase(&mut app, RacePhase::Setup);
    assert_eq!(
        app_state(&app),
        AppState::InRace,
        "restart stays in the session (no reload)"
    );
    assert_eq!(app.world().resource::<RaceConfig>(), &config);
    wait_for_phase(&mut app, RacePhase::Countdown);

    app.world_mut().trigger(QuitRace);
    step_until(&mut app, "the menu", |w| {
        *w.resource::<State<AppState>>().get() == AppState::Menu
    });
}
