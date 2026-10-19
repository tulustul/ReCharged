//! Headless behaviour of rc_input (ARCHITECTURE §3.2, §5.3): who gets `CarControl`, edges and
//! pausing, the steering ramp, menu navigation gating and repeat, the lobby, auto-assignment,
//! rebinding and a pad takeover. Input is injected through `ButtonInput<KeyCode>` and `Gamepad`.

use std::time::Duration;

use bevy::input::gamepad::Gamepad;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use rc_core::prelude::*;
use rc_input::GameInputPlugin;
use rc_testkit::*;

#[derive(Resource, Default)]
struct Seen {
    nav: Vec<UiNav>,
    pauses: Vec<PauseRequest>,
    devices: Vec<DeviceEvent>,
    rebinds: Vec<RebindResult>,
    cameras: Vec<CycleCamera>,
    /// Fixed ticks in which a local racer had `use_item` set.
    items: u32,
}

fn record(
    mut seen: ResMut<Seen>,
    mut nav: MessageReader<UiNav>,
    mut pauses: MessageReader<PauseRequest>,
    mut devices: MessageReader<DeviceEvent>,
    mut rebinds: MessageReader<RebindResult>,
    mut cameras: MessageReader<CycleCamera>,
) {
    seen.nav.extend(nav.read().copied());
    seen.pauses.extend(pauses.read().copied());
    seen.devices.extend(devices.read().copied());
    seen.rebinds.extend(rebinds.read().copied());
    seen.cameras.extend(cameras.read().copied());
}

fn count_items(mut seen: ResMut<Seen>, controls: Query<&CarControl, With<LocalPlayer>>) {
    seen.items += controls.iter().filter(|c| c.use_item).count() as u32;
}

/// rc_race's job in the game: one racer per grid entry.
fn spawn_racers(mut commands: Commands, config: Res<RaceConfig>) {
    for (index, setup) in config.racers.iter().enumerate() {
        let mut racer = commands.spawn((Racer { index, ..default() }, DespawnOnExit(RaceAttempt)));
        let ai = AiDriver {
            rival: "volt".into(),
            difficulty: Difficulty::Pro,
        };
        match setup.controller {
            Controller::Human { slot, autopilot } => {
                racer.insert(LocalPlayer(slot));
                if autopilot {
                    racer.insert(ai);
                }
            }
            Controller::Ai { .. } => {
                racer.insert(ai);
            }
        }
    }
}

fn app() -> App {
    headless_app((GameInputPlugin, |app: &mut App| {
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<Settings>()
            .init_resource::<Seen>()
            .add_systems(OnEnter(RaceAttempt), spawn_racers)
            .add_systems(FixedUpdate, count_items.in_set(SimSet::Effects))
            .add_systems(Last, record);
    }))
}

fn human(slot: u8) -> Controller {
    Controller::Human {
        slot: PlayerSlot(slot),
        autopilot: false,
    }
}

fn race(controllers: Vec<Controller>) -> RaceConfig {
    RaceConfig {
        racers: controllers
            .into_iter()
            .map(|controller| RacerSetup {
                controller,
                ..default()
            })
            .collect(),
        ..default()
    }
}

fn keys(app: &mut App) -> Mut<'_, ButtonInput<KeyCode>> {
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>()
}

/// One frame in which `key` goes down (it stays held until [`release`]).
fn press(app: &mut App, key: KeyCode) {
    keys(app).press(key);
    app.update();
    keys(app).clear();
}

fn release(app: &mut App, key: KeyCode) {
    keys(app).release(key);
    app.update();
    keys(app).clear();
}

fn seen(app: &App) -> &Seen {
    app.world().resource::<Seen>()
}

fn take(app: &mut App) -> Seen {
    std::mem::take(&mut *app.world_mut().resource_mut::<Seen>())
}

/// `(slot, has AiDriver, CarControl)` of every racer, in grid order.
fn controls(app: &mut App) -> Vec<(Option<u8>, bool, CarControl)> {
    let mut q = app
        .world_mut()
        .query::<(&Racer, Option<&LocalPlayer>, Has<AiDriver>, &CarControl)>();
    let mut v: Vec<_> = q
        .iter(app.world())
        .map(|(r, l, ai, c)| (r.index, l.map(|l| l.0.0), ai, *c))
        .collect();
    v.sort_by_key(|(index, ..)| *index);
    v.into_iter().map(|(_, l, ai, c)| (l, ai, c)).collect()
}

#[test]
fn car_control_reaches_only_local_humans_without_an_ai_driver() {
    let mut app = app();
    let ai = Controller::Ai {
        rival: "volt".into(),
        difficulty: Difficulty::Pro,
    };
    let autopilot = Controller::Human {
        slot: PlayerSlot(1),
        autopilot: true,
    };
    start_race(&mut app, race(vec![human(0), ai, autopilot])).expect("race starts");
    keys(&mut app).press(KeyCode::KeyW); // keyboard A = P1 (two humans: the keyboard is split)
    keys(&mut app).press(KeyCode::ArrowUp); // keyboard B = P2, but P2 is on autopilot
    app.update();
    let c = controls(&mut app);
    assert_eq!((c[0].0, c[0].2.throttle), (Some(0), 1.0), "P1 drives");
    assert_eq!(c[1].2, CarControl::default(), "AI racers are rc_ai's");
    assert!(
        c[2].1 && c[2].2 == CarControl::default(),
        "autopilot humans are rc_ai's"
    );
}

#[test]
fn digital_steering_ramps_to_full_lock_in_eight_ticks() {
    let mut app = app();
    start_race(&mut app, race(vec![human(0)])).expect("race starts");
    keys(&mut app).press(KeyCode::KeyD);
    let mut steer = Vec::new();
    for _ in 0..9 {
        app.update();
        steer.push(controls(&mut app)[0].2.steer);
    }
    assert!(steer[0] > 0.1 && steer[0] < 0.2, "{steer:?}");
    assert!(steer.windows(2).all(|w| w[1] >= w[0]));
    assert!(steer[6] < 1.0 && steer[7] == 1.0, "{steer:?}");
}

#[test]
fn an_item_press_fires_in_exactly_one_tick_with_zero_or_two_ticks_per_frame() {
    let mut app = app();
    start_race(&mut app, race(vec![human(0)])).expect("race starts");
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
    press(&mut app, KeyCode::KeyE); // latched in a frame without a fixed tick
    assert_eq!(seen(&app).items, 0);
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(2));
    app.update();
    assert_eq!(seen(&app).items, 1, "the first of two ticks consumes it");
    app.update();
    assert_eq!(seen(&app).items, 1, "held: no new edge");
}

#[test]
fn confirm_on_the_pause_menu_never_fires_an_item_after_resuming() {
    let mut app = app();
    start_race(&mut app, race(vec![human(0)])).expect("race starts");
    pause(&mut app);
    take(&mut app);
    press(&mut app, KeyCode::Enter); // keyboard B's Use item and the menu Confirm, still held
    assert!(
        seen(&app)
            .nav
            .iter()
            .any(|n| n.action == NavAction::Confirm)
    );
    resume(&mut app);
    step(&mut app, 10);
    assert_eq!(
        seen(&app).items,
        0,
        "pause → Confirm → resume → no use_item"
    );
    release(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Enter);
    assert_eq!(seen(&app).items, 1, "a press while running does fire");
}

#[test]
fn a_latched_edge_does_not_survive_a_pause() {
    let mut app = app();
    start_race(&mut app, race(vec![human(0)])).expect("race starts");
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
    press(&mut app, KeyCode::KeyE); // latched, no tick yet
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
    pause(&mut app);
    resume(&mut app);
    step(&mut app, 5);
    assert_eq!(seen(&app).items, 0, "entering Running clears latched edges");
}

#[test]
fn menu_navigation_is_tagged_repeats_and_stays_quiet_while_racing() {
    let mut app = app();
    press(&mut app, KeyCode::ArrowDown);
    let first = UiNav {
        action: NavAction::Down,
        slot: Some(PlayerSlot(0)),
        device: DeviceKind::KeyboardB,
        repeat: false,
    };
    assert_eq!(take(&mut app).nav, [first]);
    step(&mut app, 25); // held 0.43 s: one repeat (0.35 s), the next is due at 0.45 s
    assert_eq!(
        take(&mut app).nav,
        [UiNav {
            repeat: true,
            ..first
        }]
    );
    release(&mut app, KeyCode::ArrowDown);

    start_race(&mut app, race(vec![human(0)])).expect("race starts");
    take(&mut app);
    press(&mut app, KeyCode::ArrowDown);
    press(&mut app, KeyCode::Space);
    assert!(
        take(&mut app).nav.is_empty(),
        "racing: D-pad steering must not spam the UI"
    );
    pause(&mut app);
    release(&mut app, KeyCode::ArrowDown);
    press(&mut app, KeyCode::ArrowDown);
    assert_eq!(take(&mut app).nav.len(), 1, "the pause menu navigates");
}

#[test]
fn pause_is_requested_only_while_running_and_camera_presses_cycle() {
    let mut app = app();
    start_race(&mut app, race(vec![human(0)])).expect("race starts");
    take(&mut app);
    press(&mut app, KeyCode::Escape);
    press(&mut app, KeyCode::KeyC);
    let s = take(&mut app);
    assert_eq!(
        s.pauses,
        [PauseRequest {
            slot: Some(PlayerSlot(0))
        }]
    );
    assert!(
        s.nav.is_empty(),
        "Esc pauses in a running race, it does not navigate"
    );
    assert_eq!(
        s.cameras,
        [CycleCamera {
            slot: PlayerSlot(0)
        }]
    );
    pause(&mut app);
    release(&mut app, KeyCode::Escape);
    press(&mut app, KeyCode::Escape);
    let s = take(&mut app);
    assert!(s.pauses.is_empty(), "never toggles while paused");
    assert_eq!(
        s.nav.iter().map(|n| n.action).collect::<Vec<_>>(),
        [NavAction::Back]
    );
}

#[test]
fn the_lobby_joins_devices_into_free_slots_and_back_leaves() {
    let mut app = app();
    app.world_mut().insert_resource(LobbyState {
        open: true,
        max_slots: 4,
    });
    app.update();
    assert!(app.world().resource::<LocalPlayers>().slots.is_empty());
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Space);
    let s = take(&mut app);
    assert_eq!(
        s.devices,
        [
            DeviceEvent::Joined {
                slot: PlayerSlot(0),
                device: InputDevice::KeyboardB
            },
            DeviceEvent::Joined {
                slot: PlayerSlot(1),
                device: InputDevice::KeyboardA
            },
        ]
    );
    assert!(s.nav.is_empty(), "joining presses do not navigate");
    let players = app.world().resource::<LocalPlayers>().clone();
    assert_eq!(players.slots[1].devices, [InputDevice::KeyboardA]);
    press(&mut app, KeyCode::ArrowDown);
    press(&mut app, KeyCode::Escape);
    let s = take(&mut app);
    assert_eq!(
        s.nav[0].slot,
        Some(PlayerSlot(0)),
        "scheme keys belong to their player"
    );
    assert_eq!(
        (s.nav[1].action, s.nav[1].slot),
        (NavAction::Back, None),
        "Esc is nobody's"
    );
    press(&mut app, KeyCode::Backspace);
    assert_eq!(
        take(&mut app).devices,
        [DeviceEvent::Left {
            slot: PlayerSlot(0)
        }]
    );
    app.world_mut().resource_mut::<LobbyState>().open = false;
    app.update();
    let players = &app.world().resource::<LocalPlayers>().slots;
    assert_eq!(players.len(), 1, "the remaining player stays for the race");
    assert_eq!(players[0].slot, PlayerSlot(1));
}

#[test]
fn a_two_player_session_without_a_lobby_splits_the_keyboard() {
    let mut app = app();
    start_race(&mut app, race(vec![human(0), human(1)])).expect("race starts");
    let players = app.world().resource::<LocalPlayers>().clone();
    assert_eq!(players.slots.len(), 2);
    assert_eq!(players.slots[0].devices, [InputDevice::KeyboardA]);
    assert_eq!(players.slots[1].devices, [InputDevice::KeyboardB]);
    keys(&mut app).press(KeyCode::ArrowUp);
    app.update();
    let c = controls(&mut app);
    assert_eq!((c[0].2.throttle, c[1].2.throttle), (0.0, 1.0));
    quit_to_menu(&mut app);
    start_race(&mut app, race(vec![human(0)])).expect("race starts");
    assert_eq!(
        app.world().resource::<LocalPlayers>().slots[0]
            .devices
            .len(),
        2,
        "1P: all devices"
    );
}

#[test]
fn rebinding_captures_the_next_key_and_reports_conflicts() {
    let mut app = app();
    let request = RebindRequest {
        scheme: ControlScheme::KeyboardA,
        action: DriveAction::Boost,
    };
    let capture = |app: &mut App, key| {
        app.world_mut().write_message(request);
        app.update();
        press(app, key);
        release(app, key);
        take(app)
    };
    let s = capture(&mut app, KeyCode::KeyK);
    assert_eq!(
        s.rebinds,
        [RebindResult {
            scheme: ControlScheme::KeyboardA,
            action: DriveAction::Boost,
            binding: Some(Binding::Key(KeyCode::KeyK)),
            conflict: None,
            previous: Some(Binding::Key(KeyCode::ShiftLeft)),
        }]
    );
    let s = capture(&mut app, KeyCode::ArrowUp);
    assert_eq!(
        s.rebinds[0].conflict,
        Some((ControlScheme::KeyboardB, DriveAction::Throttle))
    );
    assert!(s.nav.is_empty(), "the captured key does not navigate");
    let s = capture(&mut app, KeyCode::Escape);
    assert_eq!(s.rebinds[0].binding, None, "Esc cancels");
    assert!(s.nav.is_empty());
}

#[test]
fn a_disconnected_pad_is_reported_and_another_device_takes_over() {
    let mut app = app();
    let pad = app.world_mut().spawn(Gamepad::default()).id();
    app.update();
    start_race(&mut app, race(vec![human(0), human(1)])).expect("race starts");
    let players = app.world().resource::<LocalPlayers>().clone();
    assert_eq!(
        players.slots[0].devices,
        [InputDevice::Gamepad(pad)],
        "pads first"
    );
    take(&mut app);
    app.world_mut().entity_mut(pad).remove::<Gamepad>();
    app.update();
    assert_eq!(
        take(&mut app).devices,
        [DeviceEvent::Disconnected {
            slot: PlayerSlot(0)
        }]
    );
    pause(&mut app); // rc_game pauses and shows the dialog
    press(&mut app, KeyCode::Enter); // keyboard B is not playing
    let s = take(&mut app);
    assert_eq!(
        s.devices,
        [DeviceEvent::Joined {
            slot: PlayerSlot(0),
            device: InputDevice::KeyboardB
        }]
    );
    assert!(
        s.nav.is_empty(),
        "the takeover press does not click the dialog"
    );
}
