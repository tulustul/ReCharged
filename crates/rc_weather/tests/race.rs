//! Headless: Dynamic weather in a 3-lap race (the testkit's flow plus a stand-in for rc_race's
//! progress) changes exactly once, inside 30–70 % of the race distance; a restart replays the same
//! weather; `WeatherState` is removed while unloading and the menu gets the default back.

use bevy::prelude::*;
use rc_core::prelude::*;
use rc_testkit::*;
use rc_weather::WeatherPlugin;

const LAP_U: f32 = 300.0;
const LAPS: u8 = 3;
/// Leader speed, u/s: the race lasts `LAPS × LAP_U / LEADER_UPS` = 20 s.
const LEADER_UPS: f32 = 45.0;
/// The race and a second (the settled end of the blend is covered by unit tests).
const RUN_SECS: f32 = 21.0;

/// `Changed` events seen: (preset, the leader's share of the race distance at that tick).
#[derive(Resource, Default)]
struct Changes(Vec<(WeatherKind, f32)>);

fn race_length() -> f32 {
    LAP_U * f32::from(LAPS)
}

fn race_stand_in(app: &mut App) {
    app.insert_resource(TrackSummary {
        length: LAP_U,
        ..default()
    })
    .init_resource::<Changes>()
    .add_systems(OnEnter(RaceAttempt), spawn_racers)
    .add_systems(FixedUpdate, drive.in_set(SimSet::Race))
    .add_systems(Update, record_changes);
}

fn spawn_racers(mut commands: Commands) {
    for index in 0..2 {
        commands.spawn((Racer { index, ..default() }, DespawnOnExit(RaceAttempt)));
    }
}

/// Racer 0 leads at `LEADER_UPS`, racer 1 follows at half speed; both stop at the finish.
fn drive(mut racers: Query<(&Racer, &mut RaceProgress)>) {
    for (racer, mut progress) in &mut racers {
        let speed = LEADER_UPS / (1 + racer.index) as f32;
        progress.total_distance = (progress.total_distance + speed * SIM_DT).min(race_length());
    }
}

fn record_changes(
    mut events: MessageReader<WeatherEvent>,
    racers: Query<&RaceProgress>,
    mut changes: ResMut<Changes>,
) {
    let leader = racers.iter().map(|p| p.total_distance).fold(0.0, f32::max);
    for event in events.read() {
        if let WeatherEvent::Changed(kind) = *event {
            changes.0.push((kind, leader / race_length()));
        }
    }
}

fn weather(app: &App) -> WeatherState {
    app.world().resource::<WeatherState>().clone()
}

fn changes(app: &App) -> Vec<(WeatherKind, f32)> {
    app.world().resource::<Changes>().0.clone()
}

#[test]
fn dynamic_weather_changes_exactly_once_per_three_lap_race() {
    let mut app = headless_app((WeatherPlugin, race_stand_in));
    app.insert_resource(Catalog::load(&assets_dir()).expect("catalog loads"));
    assert_eq!(weather(&app), WeatherState::default(), "menu: Clear, Noon");

    let config = RaceConfig {
        variant: TrackVariant::forward("backyard_bash"),
        laps: LAPS,
        weather: WeatherSetup {
            weather: WeatherChoice::Dynamic {
                start: WeatherKind::Cloudy,
            },
            time: TimeOfDay::Sunset,
        },
        seed: 7,
        ..default()
    };
    start_race(&mut app, config).expect("loads");
    let start = weather(&app);
    assert_eq!(
        (start.kind, start.time, start.outside_only),
        (WeatherKind::Cloudy, TimeOfDay::Sunset, false)
    );

    step_seconds(&mut app, RUN_SECS);
    let first = changes(&app);
    assert_eq!(first.len(), 1, "exactly one change: {first:?}");
    let (kind, at) = first[0];
    use WeatherKind::*;
    assert!(
        [Clear, Rain, Fog, Snow].contains(&kind),
        "{kind:?} neighbours Cloudy"
    );
    assert!((0.3..0.701).contains(&at), "changed at {at} of the race");
    let end = weather(&app);
    assert_eq!(end.kind, kind);
    assert_ne!(end, start, "the blend moved the weather");

    restart(&mut app);
    assert_eq!(weather(&app), start, "restart resets to the start weather");
    step_seconds(&mut app, RUN_SECS);
    assert_eq!(changes(&app), [first[0]; 2], "same seed, same change");
    assert_eq!(weather(&app), end);

    request(&mut app, AppState::Unloading);
    app.update();
    assert!(app.world().get_resource::<WeatherState>().is_none());
    request(&mut app, AppState::Menu);
    app.update();
    assert_eq!(weather(&app), WeatherState::default(), "menu default again");
}
