//! Headless behaviour of the camera rig (ARCHITECTURE §5.8): exactly one UI host in every state,
//! one session-scoped game camera per local player (kept across restarts, re-bound to the new
//! racers), split viewports + `PlayerViewports`, following and camera-mode cycling.

use bevy::prelude::*;
use bevy::ui::IsDefaultUiCamera;
use bevy::window::{PrimaryWindow, WindowResolution};
use rc_camera::GameCameraPlugin;
use rc_core::prelude::*;
use rc_testkit::*;

fn spawn_racers(mut commands: Commands, config: Res<RaceConfig>) {
    for (index, setup) in config.racers.iter().enumerate() {
        let x = index as f32 * 3.0;
        let mut racer = commands.spawn((
            Racer { index, ..default() },
            Transform::from_xyz(x, 0.5, 0.0),
            DespawnOnExit(RaceAttempt),
        ));
        if let Controller::Human { slot, .. } = setup.controller {
            racer.insert(LocalPlayer(slot));
        }
    }
}

/// A 1600×900 primary window (no renderer: the rig only reads its size).
fn window(app: &mut App) {
    app.world_mut().spawn((
        Window {
            resolution: WindowResolution::new(1600, 900),
            ..default()
        },
        PrimaryWindow,
    ));
}

fn app() -> App {
    headless_app((GameCameraPlugin, |app: &mut App| {
        app.init_resource::<Settings>()
            .add_systems(OnEnter(RaceAttempt), spawn_racers);
        window(app);
    }))
}

fn humans(n: u8) -> RaceConfig {
    RaceConfig {
        racers: (0..n)
            .map(|slot| RacerSetup {
                controller: Controller::Human {
                    slot: PlayerSlot(slot),
                    autopilot: false,
                },
                ..default()
            })
            .chain(std::iter::once(RacerSetup {
                controller: Controller::Ai {
                    rival: "volt".into(),
                    difficulty: Difficulty::Pro,
                },
                ..default()
            }))
            .collect(),
        ..default()
    }
}

/// Entities carrying `F`.
fn entities<F: bevy::ecs::query::QueryFilter>(app: &mut App) -> Vec<Entity> {
    let mut q = app.world_mut().query_filtered::<Entity, F>();
    q.iter(app.world()).collect()
}

fn assert_one_host(app: &mut App, what: &str) -> Entity {
    let hosts = entities::<(With<UiHost>, With<IsDefaultUiCamera>)>(app);
    let marked = entities::<With<IsDefaultUiCamera>>(app);
    assert_eq!(hosts.len(), 1, "{what}: exactly one UI host");
    assert_eq!(
        marked, hosts,
        "{what}: the host is the only IsDefaultUiCamera"
    );
    hosts[0]
}

#[test]
fn exactly_one_ui_host_in_every_state() {
    let mut app = app();
    let menu = assert_one_host(&mut app, "menu");
    assert!(app.world().get::<MenuCamera>(menu).is_some());

    request(&mut app, AppState::Loading);
    app.world_mut().insert_resource(humans(1));
    app.update();
    let loading = assert_one_host(&mut app, "loading");
    assert!(
        app.world()
            .get::<rc_camera::LoadingCamera>(loading)
            .is_some()
    );
    assert!(entities::<With<MenuCamera>>(&mut app).is_empty());

    request(&mut app, AppState::InRace);
    app.update();
    let host = assert_one_host(&mut app, "1P race");
    assert!(
        app.world().get::<GameCamera>(host).is_some(),
        "1P: the game camera hosts the UI"
    );
    assert!(entities::<With<OverlayCamera>>(&mut app).is_empty());

    quit_to_menu(&mut app);
    assert!(entities::<With<GameCamera>>(&mut app).is_empty());
    assert_one_host(&mut app, "menu again");
}

#[test]
fn split_screen_gets_an_overlay_host_and_tiled_viewports() {
    let mut app = app();
    start_race(&mut app, humans(3)).expect("race starts");
    let host = assert_one_host(&mut app, "3P race");
    assert!(app.world().get::<OverlayCamera>(host).is_some());
    let overlay = app.world().get::<Camera>(host).expect("camera");
    assert!(
        overlay.viewport.is_none() && overlay.order > 3,
        "full window, on top"
    );
    let mut q = app.world_mut().query::<(&GameCamera, &Camera)>();
    let mut views: Vec<(u8, UVec2, UVec2)> = q
        .iter(app.world())
        .map(|(g, c)| {
            let v = c.viewport.as_ref().expect("split views have viewports");
            (g.slot.0, v.physical_position, v.physical_size)
        })
        .collect();
    views.sort_by_key(|v| v.0);
    assert_eq!(
        views,
        [
            (0, UVec2::new(0, 0), UVec2::new(800, 450)),
            (1, UVec2::new(800, 0), UVec2::new(800, 450)),
            (2, UVec2::new(0, 450), UVec2::new(800, 450)),
        ]
    );
    let viewports = app.world().resource::<PlayerViewports>().0.clone();
    assert_eq!(viewports.len(), 3);
    assert_eq!(
        viewports[2],
        (PlayerSlot(2), Rect::new(0.0, 450.0, 800.0, 900.0))
    );
}

#[test]
fn two_players_follow_the_split_setting_and_window_resizes() {
    let mut app = app();
    start_race(&mut app, humans(2)).expect("race starts");
    let sizes = |app: &mut App| {
        let mut q = app.world_mut().query::<(&GameCamera, &Camera)>();
        let mut v: Vec<(u8, UVec2)> = q
            .iter(app.world())
            .map(|(g, c)| {
                (
                    g.slot.0,
                    c.viewport.as_ref().map_or(UVec2::ZERO, |v| v.physical_size),
                )
            })
            .collect();
        v.sort_by_key(|v| v.0);
        v
    };
    assert_eq!(
        sizes(&mut app),
        [(0, UVec2::new(1600, 450)), (1, UVec2::new(1600, 450))]
    );
    app.world_mut().resource_mut::<Settings>().gameplay.split = SplitOrientation::Vertical;
    app.update();
    assert_eq!(
        sizes(&mut app),
        [(0, UVec2::new(800, 900)), (1, UVec2::new(800, 900))]
    );
    let mut window = app
        .world_mut()
        .query_filtered::<&mut Window, With<PrimaryWindow>>();
    window
        .single_mut(app.world_mut())
        .expect("window")
        .resolution
        .set(1280.0, 720.0);
    app.update();
    assert_eq!(
        sizes(&mut app),
        [(0, UVec2::new(640, 720)), (1, UVec2::new(640, 720))]
    );
}

#[test]
fn game_cameras_survive_a_restart_and_follow_the_new_racers() {
    let mut app = app();
    start_race(&mut app, humans(2)).expect("race starts");
    step(&mut app, 30);
    let before = entities::<With<GameCamera>>(&mut app);
    let host = assert_one_host(&mut app, "2P");
    let p2_camera = |app: &mut App| {
        let mut q = app.world_mut().query::<(&GameCamera, &Transform)>();
        q.iter(app.world())
            .find(|(g, _)| g.slot == PlayerSlot(1))
            .map(|(_, t)| *t)
            .expect("P2 camera")
    };
    let pose = p2_camera(&mut app);
    // P2's racer stands at x = 3: its chase camera sits behind it (+Z) and looks at it.
    assert!(
        (pose.translation.x - 3.0).abs() < 0.1,
        "{}",
        pose.translation
    );
    assert!(
        pose.translation.z > 3.0 && pose.translation.y > 1.0,
        "{}",
        pose.translation
    );
    assert!(pose.forward().z < -0.8, "looks forward along −Z");

    restart(&mut app);
    step(&mut app, 2);
    assert_eq!(
        entities::<With<GameCamera>>(&mut app),
        before,
        "a restart keeps the cameras"
    );
    assert_eq!(
        assert_one_host(&mut app, "after restart"),
        host,
        "and the UI host"
    );
    assert!((p2_camera(&mut app).translation.x - 3.0).abs() < 0.1);
}

#[test]
fn the_camera_button_cycles_chase_far_and_hood() {
    let mut app = app();
    start_race(&mut app, humans(1)).expect("race starts");
    step(&mut app, 30);
    let distance = |app: &mut App| {
        let mut q = app
            .world_mut()
            .query_filtered::<&Transform, With<GameCamera>>();
        let eye = q.single(app.world()).expect("one camera").translation;
        eye.distance(Vec3::new(0.0, 0.5, 0.0))
    };
    let chase = distance(&mut app);
    let cycle = |app: &mut App| {
        app.world_mut().write_message(CycleCamera {
            slot: PlayerSlot(0),
        });
        step(app, 30);
        distance(app)
    };
    let far = cycle(&mut app);
    let hood = cycle(&mut app);
    let back = cycle(&mut app);
    assert!(far > chase + 2.0, "far chase {far} vs chase {chase}");
    assert!(hood < 1.0, "hood camera on the car: {hood}");
    assert!(
        (back - chase).abs() < 0.2,
        "back to chase: {back} vs {chase}"
    );
}
