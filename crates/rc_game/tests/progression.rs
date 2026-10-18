//! Progression end to end with placeholder racers: a full cup (points, restart reverting points,
//! trophy, unlocks), abandoned cups, saves surviving an app restart, corrupt-file recovery, Time
//! Trial ghosts and records on changed geometry, Golden Batteries, hints and progress reset.

mod common;

use std::fs;
use std::path::Path;

use bevy::prelude::*;
use common::*;
use rc_core::prelude::*;
use rc_game::{
    Content, CupRun, GameCommand, GameNotice, Medal, Profile, RaceOutcome, RaceRequest, RacerKey,
    Trophy,
};
use rc_testkit::*;

const PLAYER: RacerKey = RacerKey::Player(PlayerSlot(0));

fn cup_request(cup: &str) -> RaceRequest {
    RaceRequest {
        mode: GameMode::Championship {
            cup: cup.into(),
            race_index: 0,
        },
        car: Some("zip".into()),
        ..RaceRequest::quick("ignored")
    }
}

fn run(app: &App) -> &CupRun {
    app.world().resource::<CupRun>()
}

fn outcome(app: &App) -> &RaceOutcome {
    app.world().resource::<RaceOutcome>()
}

fn profile(app: &App) -> &Profile {
    app.world().resource::<Profile>()
}

#[test]
fn a_full_cup_scores_reverts_restarts_and_unlocks_the_next_cup() {
    let dir = data_dir("cup");
    let mut app = app_in(&dir, None, Some(5));
    let baseline = count_entities::<()>(&mut app);
    start(&mut app, cup_request("rookie_cup"));
    wait_for_outcome(&mut app);
    assert_eq!(
        app.world().resource::<RaceConfig>().variant.track.as_str(),
        "sofa_circuit"
    );
    assert_eq!(outcome(&app).points[..3], [10, 8, 6]);
    assert_eq!(run(&app).series.points(&PLAYER), 10);

    // Restart from Results reverts the race; the replay finishes 2nd.
    app.world_mut().resource_mut::<FinishOrder>().0 = vec![1, 0, 2, 3, 4, 5, 6, 7];
    send(&mut app, GameCommand::Restart);
    step(&mut app, 2);
    assert_eq!(
        run(&app).series.points(&PLAYER),
        0,
        "restart reverts the race's points"
    );
    wait_for_outcome(&mut app);
    assert_eq!(run(&app).series.points(&PLAYER), 8);

    app.world_mut().resource_mut::<FinishOrder>().0.clear(); // the player wins the rest
    for (race, track) in [(1, "toy_town"), (2, "backyard_bash"), (3, "aisle_rush")] {
        send(&mut app, GameCommand::NextRace);
        wait_for_next_outcome(&mut app);
        assert_eq!(run(&app).series.race, race);
        assert_eq!(
            app.world().resource::<RaceConfig>().variant.track.as_str(),
            track
        );
    }
    assert_eq!(counters(&app).loads, 4, "one load per cup race");
    assert_eq!(run(&app).series.points(&PLAYER), 38);
    assert!(outcome(&app).cup_finished);
    assert_eq!(
        (run(&app).final_position, run(&app).trophy()),
        (Some(1), Some(Trophy::Gold))
    );
    assert_eq!(
        profile(&app).trophy(&"rookie_cup".into()),
        Some(Trophy::Gold)
    );
    let pro_cup = Content::Cup("pro_cup".into());
    assert!(outcome(&app).unlocked.contains(&pro_cup));
    assert!(
        outcome(&app)
            .unlocked
            .contains(&Content::Car("torque".into()))
    );
    assert!(
        seen(&app)
            .notices
            .contains(&GameNotice::Unlocked(pro_cup.clone()))
    );
    assert!(profile(&app).new_unlocks.contains(&pro_cup));

    send(&mut app, GameCommand::NextRace); // after the last race: Cup end in the menu
    wait_for_state(&mut app, AppState::Menu);
    assert_eq!(count_entities::<()>(&mut app), baseline);
    assert!(
        run(&app).finished(),
        "the finished cup stays for the Cup end screen"
    );
    send(&mut app, GameCommand::MarkSeen(pro_cup.clone()));
    step(&mut app, 2);
    assert!(!profile(&app).new_unlocks.contains(&pro_cup));

    start(&mut app, cup_request("pro_cup"));
    wait_for_state(&mut app, AppState::Loading);
    assert_eq!(
        run(&app).cup.as_str(),
        "pro_cup",
        "the unlocked cup starts a new run"
    );

    let reloaded = app_in(&dir, None, None);
    assert_eq!(
        profile(&reloaded).trophy(&"rookie_cup".into()),
        Some(Trophy::Gold),
        "saved"
    );
    assert!(!profile(&reloaded).new_unlocks.contains(&pro_cup));
}

#[test]
fn abandoned_cups_and_failed_next_races_leave_nothing_behind() {
    let mut app = app_in(&data_dir("cup-abandon"), None, Some(5));
    let baseline = count_entities::<()>(&mut app);

    start(&mut app, cup_request("rookie_cup"));
    wait_for_phase(&mut app, RacePhase::Countdown);
    send(&mut app, GameCommand::NextRace);
    step(&mut app, 3);
    assert_eq!(
        (app_state(&app), counters(&app).loads),
        (AppState::InRace, 1),
        "no result yet"
    );
    wait_for_outcome(&mut app);
    send(&mut app, GameCommand::QuitToMenu);
    wait_for_state(&mut app, AppState::Menu);
    assert!(
        !app.world().contains_resource::<CupRun>(),
        "quitting loses the cup"
    );
    assert_eq!(count_entities::<()>(&mut app), baseline);

    start(&mut app, cup_request("rookie_cup"));
    wait_for_outcome(&mut app);
    app.insert_resource(FailLoading);
    send(&mut app, GameCommand::NextRace);
    wait_for_state(&mut app, AppState::Unloading);
    wait_for_state(&mut app, AppState::Menu);
    assert!(!app.world().contains_resource::<CupRun>());
    assert!(
        seen(&app)
            .notices
            .iter()
            .any(|n| matches!(n, GameNotice::Error(_)))
    );
    assert_eq!(count_entities::<()>(&mut app), baseline);
}

fn settings_text(dir: &Path) -> String {
    fs::read_to_string(dir.join(rc_game::SETTINGS_FILE)).unwrap_or_default()
}

fn with_music(music: f32) -> GameCommand {
    let mut settings = Settings::default();
    settings.audio.music = music;
    GameCommand::ApplySettings(Box::new(settings))
}

#[test]
fn settings_profile_and_ghosts_survive_an_app_restart() {
    let dir = data_dir("round-trip");
    let mut app = app_in(&dir, None, Some(5));
    send(&mut app, with_music(0.25));
    step(&mut app, 2);
    assert!(
        settings_text(&dir).contains("music: 0.25"),
        "first write is immediate"
    );
    send(&mut app, with_music(0.5));
    step(&mut app, 2);
    assert!(
        settings_text(&dir).contains("music: 0.25"),
        "throttled to one write per second"
    );
    step_seconds(&mut app, 1.0);
    assert!(settings_text(&dir).contains("music: 0.5"));

    let tt = RaceRequest {
        mode: GameMode::TimeTrial,
        car: Some("sparky".into()),
        ..RaceRequest::quick("test_oval")
    };
    start(&mut app, tt.clone());
    wait_for_outcome(&mut app);
    let o = outcome(&app).clone();
    assert!(o.new_best_race && o.new_best_lap && o.ghost_saved);
    assert_eq!(
        o.medal,
        Some(Medal::Bronze),
        "3000 ticks vs a 45 s reference"
    );
    assert_eq!(o.next_medal.map(|n| n.medal), Some(Medal::Silver));
    assert!(dir.join("ghosts/test_oval.ghost").is_file());
    assert_eq!(
        app.world()
            .resource::<RaceConfig>()
            .ghost
            .as_ref()
            .map(|g| g.time_ticks),
        Some(3000)
    );
    send(&mut app, with_music(0.6));
    step(&mut app, 2);
    assert!(settings_text(&dir).contains("music: 0.6"));
    send(&mut app, with_music(0.75)); // within the second: only the exit flush writes it
    send(&mut app, GameCommand::QuitGame);
    step(&mut app, 1);
    assert_eq!(app.should_exit(), Some(AppExit::Success));
    assert!(
        settings_text(&dir).contains("music: 0.75"),
        "flushed on exit"
    );

    let mut app = app_in(&dir, None, Some(5));
    assert_eq!(app.world().resource::<Settings>().audio.music, 0.75);
    let record = profile(&app)
        .record(&TrackVariant::forward("test_oval"))
        .cloned()
        .unwrap_or_default();
    assert_eq!(
        record.best_race.map(|r| (r.ticks, r.car.0)),
        Some((3000, "sparky".into()))
    );
    assert_eq!(profile(&app).stats.races, 1);
    assert!(profile(&app).stats.time_played_secs > 0.0);
    start(&mut app, tt);
    wait_for_state(&mut app, AppState::Loading);
    let ghost = app.world().resource::<RaceConfig>().ghost.clone();
    assert_eq!(
        ghost.map(|g| (g.time_ticks, g.geometry_hash)),
        Some((3000, HASH)),
        "races its ghost"
    );
}

#[test]
fn corrupt_saves_are_backed_up_and_replaced_by_defaults() {
    let dir = data_dir("corrupt");
    fs::create_dir_all(&dir).expect("dir");
    fs::write(
        dir.join(rc_game::PROFILE_FILE),
        "(version: 1, data: (trophies: 12",
    )
    .expect("write");
    fs::write(dir.join(rc_game::SETTINGS_FILE), "(version: 99, data: ())").expect("write");
    let app = app_in(&dir, None, None);
    assert_eq!(app.world().resource::<Settings>(), &Settings::default());
    assert!(profile(&app).trophies.is_empty());
    let restored = seen(&app)
        .notices
        .iter()
        .filter(|n| {
            matches!(
                n,
                GameNotice::SaveRestored {
                    backup: Some(_),
                    ..
                }
            )
        })
        .count();
    assert_eq!(restored, 2);
    let backups = fs::read_dir(&dir)
        .expect("dir")
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| n.contains(".corrupt-"))
        .count();
    assert_eq!(backups, 2, "both corrupt files kept for inspection");
}

#[test]
fn a_changed_track_discards_records_and_the_ghost_with_a_notice() {
    let dir = data_dir("geometry");
    let mut app = app_in(&dir, None, Some(5));
    let tt = RaceRequest {
        mode: GameMode::TimeTrial,
        ..RaceRequest::quick("test_oval")
    };
    start(&mut app, tt.clone());
    wait_for_outcome(&mut app);
    send(&mut app, GameCommand::Restart); // Retry races the fresh ghost
    wait_for_phase(&mut app, RacePhase::Countdown);
    assert!(app.world().resource::<RaceConfig>().ghost.is_some());
    send(&mut app, GameCommand::QuitToMenu);
    wait_for_state(&mut app, AppState::Menu);

    app.insert_resource(Geometry(HASH + 1)); // the track was edited
    start(&mut app, tt);
    wait_for_phase(&mut app, RacePhase::Intro);
    let variant = TrackVariant::forward("test_oval");
    assert!(
        seen(&app)
            .notices
            .contains(&GameNotice::RecordsDiscarded(variant.clone()))
    );
    assert!(
        app.world().resource::<RaceConfig>().ghost.is_none(),
        "no ghost from the old track"
    );
    assert!(!dir.join("ghosts/test_oval.ghost").exists());
    let record = profile(&app).record(&variant).cloned().unwrap_or_default();
    assert!(record.best_race.is_none() && record.best_lap.is_none());
    assert_eq!(record.medal, Some(Medal::Bronze), "medals stay");
}

#[test]
fn golden_batteries_and_hints_are_saved_at_once_and_reset_clears_progress() {
    let dir = data_dir("battery");
    let mut app = app_in(&dir, None, None);
    start(&mut app, RaceRequest::quick("toy_town"));
    wait_for_phase(&mut app, RacePhase::Racing);
    assert!(app.world().resource::<RaceConfig>().golden_battery);
    let mut q = app
        .world_mut()
        .query_filtered::<(Entity, Has<LocalPlayer>), With<Racer>>();
    let racers: Vec<(Entity, bool)> = q.iter(app.world()).collect();
    let player = racers.iter().find(|r| r.1).map(|r| r.0);
    let rival = racers.iter().find(|r| !r.1).map(|r| r.0);
    let event = |racer, kind| RaceEvent { racer, kind };
    app.world_mut()
        .write_message(event(rival, RaceEventKind::GoldenBattery));
    step(&mut app, 2);
    assert!(
        profile(&app).golden_batteries.is_empty(),
        "only players collect it"
    );
    app.world_mut()
        .write_message(event(player, RaceEventKind::GoldenBattery));
    app.world_mut()
        .write_message(event(None, RaceEventKind::Hint(HintKind::DriftCharge)));
    step(&mut app, 2);
    assert!(profile(&app).has_golden_battery(&"toy_town".into()));
    assert!(
        seen(&app)
            .notices
            .contains(&GameNotice::GoldenBattery("toy_town".into()))
    );
    assert!(
        !app.world().resource::<RaceConfig>().golden_battery,
        "a restart won't respawn it"
    );
    assert_eq!(
        app.world().resource::<HintsSeen>().0,
        [HintKind::DriftCharge]
    );
    let saved = fs::read_to_string(dir.join(rc_game::PROFILE_FILE)).unwrap_or_default();
    assert!(
        saved.contains("toy_town") && saved.contains("DriftCharge"),
        "saved immediately"
    );

    send(&mut app, GameCommand::ResetProgress);
    step(&mut app, 2);
    assert!(
        profile(&app).has_golden_battery(&"toy_town".into()),
        "only from the menu"
    );
    send(&mut app, GameCommand::QuitToMenu);
    wait_for_state(&mut app, AppState::Menu);
    fs::create_dir_all(dir.join(rc_game::GHOSTS_DIR)).expect("dir");
    send(&mut app, GameCommand::ResetProgress);
    step(&mut app, 2);
    assert_eq!(profile(&app), &Profile::default());
    assert!(app.world().resource::<HintsSeen>().0.is_empty());
    assert!(!dir.join(rc_game::GHOSTS_DIR).exists());
    let reloaded = app_in(&dir, None, None);
    assert!(
        profile(&reloaded).golden_batteries.is_empty(),
        "the reset was saved"
    );
}
