//! Test arena for the car physics: flat ground with surface patches (ice, tile, grass, dirt,
//! carpet), a kicker jump, a ramp up to a table top, a long slope, a 0.5 u wall, speed bumps and
//! knockable boxes.
//!
//! `cargo run -p rc_vehicle --example arena -- --car <id>`
//!
//! Keys: W/↑ throttle · S/↓ brake/reverse · A/D/←/→ steer · Space drift · Left Shift boost ·
//! R flip upright · Backspace back to start · 1–8 switch car · G fill the battery · V camera.
//! Automation: `--auto` (scripted drive), `--exit-after <s>`, `--screenshot <path>`
//! `--screenshot-at <s>[,<s>…]` (sim seconds since the car spawned; numbered files for several).
//! `--user-data-dir` is accepted and ignored (the arena writes nothing).

use std::path::PathBuf;

use bevy::light::{CascadeShadowConfigBuilder, light_consts::lux};
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::world_serialization::WorldInstanceReady;
use bevy_rapier3d::prelude::*;
use rc_core::prelude::*;
use rc_vehicle::{PhysicsPlugin, VehicleLibrary, VehiclePlugin};

const CARS: [&str; 8] = [
    "sparky", "dumpster", "zip", "torque", "hornet", "phantom", "volt_gt", "bolt",
];
/// Digital steering ramp: 0 → full lock in this many seconds (GDD §5.1).
const STEER_RAMP_SECS: f32 = 0.12;
const START: Vec3 = Vec3::new(0.0, 0.05, 30.0);

#[derive(Resource, Clone, Default)]
struct Args {
    car: String,
    auto: bool,
    exit_after: Option<f32>,
    screenshot: Option<PathBuf>,
    shots_at: Vec<f32>,
}

fn parse_args() -> Args {
    let mut args = Args {
        car: "sparky".into(),
        ..default()
    };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let mut value = || it.next().unwrap_or_default();
        match flag.as_str() {
            "--car" => args.car = value(),
            "--auto" => args.auto = true,
            "--exit-after" => args.exit_after = value().parse().ok(),
            "--screenshot" => args.screenshot = Some(value().into()),
            "--screenshot-at" => {
                args.shots_at = value().split(',').filter_map(|s| s.parse().ok()).collect();
            }
            "--user-data-dir" => drop(value()),
            other => eprintln!("arena: ignoring unknown argument {other}"),
        }
    }
    if args.screenshot.is_some() && args.shots_at.is_empty() {
        args.shots_at.push(3.0);
    }
    args
}

/// The car the player drives.
#[derive(Component)]
struct Player;

/// The visual of a sim entity, posed from its `PoseHistory` every frame.
#[derive(Component)]
struct ProxyOf(Entity);

/// Wheel nodes of the car model (FL, FR, RL, RR) and their rest transforms.
#[derive(Component)]
struct WheelNodes([(Entity, Transform); 4]);

#[derive(Component)]
struct Hud;

#[derive(Component)]
struct ChaseCam {
    far: bool,
}

/// Fixed ticks since the player's car spawned (automation clock).
#[derive(Resource, Default)]
struct RunTicks(u32);

fn main() -> AppExit {
    let args = parse_args();
    let assets = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "ReCharged — vehicle arena".into(),
                        resolution: (1600, 900).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    file_path: assets.to_string_lossy().into_owned(),
                    ..default()
                }),
        )
        .insert_resource(UserPaths {
            data_dir: std::env::temp_dir(),
            assets_dir: assets,
        })
        .insert_resource(args)
        .insert_resource(RaceConfig::default())
        .insert_resource(ClearColor(Color::srgb(0.55, 0.7, 0.9)))
        .insert_resource(GlobalAmbientLight {
            color: Color::WHITE,
            brightness: 500.0,
            ..default()
        })
        .init_resource::<RunTicks>()
        .add_plugins((CorePlugin, PhysicsPlugin, VehiclePlugin))
        .add_systems(Startup, (build_arena, spawn_view))
        .add_systems(OnEnter(RaceAttempt), spawn_player)
        .add_systems(
            FixedUpdate,
            (count_ticks, auto_drive).in_set(SimSet::Drivers),
        )
        .add_systems(
            Update,
            (
                advance_states,
                keyboard_drive,
                keyboard_actions,
                pose_proxies,
                animate_wheels,
                follow_camera,
                update_hud,
                automation,
            )
                .chain(),
        )
        .add_observer(collect_wheels)
        .run()
}

fn material(materials: &mut Assets<StandardMaterial>, rgb: [f32; 3]) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color: Color::srgb(rgb[0], rgb[1], rgb[2]),
        perceptual_roughness: 0.85,
        ..default()
    })
}

fn track_groups() -> CollisionGroups {
    CollisionGroups::new(Group::from_bits_truncate(layers::TRACK), Group::ALL)
}

/// A static box of the arena with its visual.
fn solid(half: Vec3, at: Transform, surface: SurfaceKind, mesh: &mut Assets<Mesh>) -> impl Bundle {
    (
        Collider::cuboid(half.x, half.y, half.z),
        Surface(surface),
        TrackPiece {
            kind: TrackPieceKind::Ground,
        },
        track_groups(),
        Mesh3d(mesh.add(Cuboid::from_size(half * 2.0))),
        at,
    )
}

fn build_arena(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Floor: one collider, a checkerboard of 10 u tiles for a sense of speed.
    commands.spawn((
        Collider::cuboid(150.0, 0.5, 150.0),
        Transform::from_xyz(0.0, -0.5, 0.0),
        Surface(SurfaceKind::Asphalt),
        TrackPiece {
            kind: TrackPieceKind::Ground,
        },
        track_groups(),
    ));
    let tile = meshes.add(Plane3d::default().mesh().size(10.0, 10.0));
    let shades = [
        material(&mut materials, [0.30, 0.31, 0.33]),
        material(&mut materials, [0.36, 0.37, 0.39]),
    ];
    for x in -15..15 {
        for z in -15..15 {
            commands.spawn((
                Mesh3d(tile.clone()),
                MeshMaterial3d(shades[((x + z) & 1) as usize].clone()),
                Transform::from_xyz(x as f32 * 10.0 + 5.0, 0.0, z as f32 * 10.0 + 5.0),
            ));
        }
    }
    // Surface patches west of the start (each a thin slab just above the floor).
    let patches = [
        (SurfaceKind::Ice, [0.75, 0.9, 1.0]),
        (SurfaceKind::Tile, [0.92, 0.92, 0.88]),
        (SurfaceKind::Grass, [0.3, 0.6, 0.25]),
        (SurfaceKind::Dirt, [0.5, 0.36, 0.22]),
        (SurfaceKind::Carpet, [0.7, 0.18, 0.2]),
    ];
    for (i, (kind, rgb)) in patches.into_iter().enumerate() {
        let at = Transform::from_xyz(-50.0, -0.045, 60.0 - i as f32 * 26.0);
        commands.spawn((
            solid(Vec3::new(12.0, 0.05, 12.0), at, kind, &mut meshes),
            MeshMaterial3d(material(&mut materials, rgb)),
        ));
    }
    let wood = material(&mut materials, [0.72, 0.55, 0.33]);
    let concrete = material(&mut materials, [0.62, 0.62, 0.6]);
    // Kicker (14°) north of the start; ≈ 0.8 s of air at 26 u/s onto open floor.
    let kicker = ramp(
        Vec3::new(5.0, 0.25, 6.0),
        14f32.to_radians(),
        Vec3::new(0.0, 0.0, -20.0),
    );
    commands.spawn((
        solid(
            Vec3::new(5.0, 0.25, 6.0),
            kicker,
            SurfaceKind::Wood,
            &mut meshes,
        ),
        MeshMaterial3d(wood.clone()),
    ));
    // A 1.5 u table top west of the kicker with a ramp up to it (drive off the far edge).
    commands.spawn((
        solid(
            Vec3::new(6.0, 0.75, 8.0),
            Transform::from_xyz(-25.0, 0.75, -38.0),
            SurfaceKind::Wood,
            &mut meshes,
        ),
        MeshMaterial3d(wood.clone()),
    ));
    let table_ramp = ramp(
        Vec3::new(6.0, 0.2, 5.0),
        (1.5f32 / 10.0).asin(),
        Vec3::new(-25.0, 0.0, -20.1),
    );
    commands.spawn((
        solid(
            Vec3::new(6.0, 0.2, 5.0),
            table_ramp,
            SurfaceKind::Wood,
            &mut meshes,
        ),
        MeshMaterial3d(wood),
    ));
    // Long slope east, a 0.5 u thick wall and speed bumps.
    let slope = ramp(
        Vec3::new(8.0, 0.5, 22.0),
        11f32.to_radians(),
        Vec3::new(55.0, 0.0, 10.0),
    );
    commands.spawn((
        solid(
            Vec3::new(8.0, 0.5, 22.0),
            slope,
            SurfaceKind::Stone,
            &mut meshes,
        ),
        MeshMaterial3d(concrete.clone()),
    ));
    commands.spawn((
        solid(
            Vec3::new(18.0, 0.75, 0.25),
            Transform::from_xyz(30.0, 0.75, -40.0),
            SurfaceKind::Stone,
            &mut meshes,
        ),
        MeshMaterial3d(concrete.clone()),
    ));
    for k in 0..6 {
        let at = Transform::from_xyz(-20.0, 0.0, 10.0 - k as f32 * 4.0);
        commands.spawn((
            solid(
                Vec3::new(5.0, 0.08, 0.3),
                at,
                SurfaceKind::Asphalt,
                &mut meshes,
            ),
            MeshMaterial3d(concrete.clone()),
        ));
    }
    // Knockable cardboard boxes.
    let cardboard = material(&mut materials, [0.78, 0.62, 0.4]);
    let cube = meshes.add(Cuboid::from_length(1.2));
    for (i, (x, y, z)) in [
        (0, 0, 0),
        (1, 0, 0),
        (2, 0, 0),
        (0, 0, 1),
        (1, 0, 1),
        (0, 1, 0),
        (1, 1, 0),
    ]
    .into_iter()
    .enumerate()
    {
        let at = Vec3::new(
            20.0 + x as f32 * 1.3,
            0.6 + y as f32 * 1.25,
            30.0 - z as f32 * 1.3,
        );
        commands.spawn((
            RigidBody::Dynamic,
            Collider::cuboid(0.6, 0.6, 0.6),
            ColliderMassProperties::Mass(0.5 + 0.05 * i as f32),
            CollisionGroups::new(Group::from_bits_truncate(layers::PROP), Group::ALL),
            Surface(SurfaceKind::Cardboard),
            Mesh3d(cube.clone()),
            MeshMaterial3d(cardboard.clone()),
            Transform::from_translation(at),
        ));
    }
}

/// A box tilted by `angle` about X (rising toward −Z for positive angles) whose low top edge
/// sits on the floor at `foot`.
fn ramp(half: Vec3, angle: f32, foot: Vec3) -> Transform {
    let rot = Quat::from_rotation_x(angle);
    let low_edge = rot * Vec3::new(0.0, half.y, half.z * angle.signum());
    Transform::from_translation(foot - low_edge).with_rotation(rot)
}

fn spawn_view(mut commands: Commands, mut next: ResMut<NextState<AppState>>) {
    commands.spawn((
        DirectionalLight {
            illuminance: lux::AMBIENT_DAYLIGHT,
            shadow_maps_enabled: true,
            ..default()
        },
        CascadeShadowConfigBuilder {
            first_cascade_far_bound: 15.0,
            maximum_distance: 120.0,
            ..default()
        }
        .build(),
        Transform::from_xyz(30.0, 60.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 4.0, START.z + 8.0).looking_at(START, Vec3::Y),
        ChaseCam { far: false },
    ));
    commands.spawn((
        Hud,
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(17.0),
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            padding: UiRect::all(px(10)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.45)),
    ));
    next.set(AppState::Loading);
}

/// Loading → InRace once the vehicle files are loaded, then straight to Racing.
fn advance_states(
    app_state: Res<State<AppState>>,
    phase: Option<Res<State<RacePhase>>>,
    library: Option<Res<VehicleLibrary>>,
    mut failed: MessageReader<LoadingFailed>,
    mut next_app: ResMut<NextState<AppState>>,
    mut next_phase: ResMut<NextState<RacePhase>>,
    mut exit: MessageWriter<AppExit>,
) {
    if let Some(f) = failed.read().last() {
        error!("arena: loading failed: {}", f.error);
        exit.write(AppExit::error());
    }
    match (app_state.get(), phase.map(|p| *p.get())) {
        (AppState::Loading, _) if library.is_some() => next_app.set(AppState::InRace),
        (AppState::InRace, Some(RacePhase::Setup)) => next_phase.set(RacePhase::Racing),
        _ => {}
    }
}

fn spawn_car(commands: &mut Commands, server: &AssetServer, car: &str, at: Transform) {
    let sim = commands
        .spawn((
            Player,
            Racer {
                name: "Player".into(),
                car: CarId::new(car),
                ..default()
            },
            at,
            DespawnOnExit(RaceAttempt),
        ))
        .id();
    let scene = server.load(GltfAssetLabel::Scene(0).from_asset(format!("models/cars/{car}.glb")));
    commands.spawn((ProxyOf(sim), WorldAssetRoot(scene), at));
}

fn spawn_player(
    mut commands: Commands,
    server: Res<AssetServer>,
    args: Res<Args>,
    mut ticks: ResMut<RunTicks>,
) {
    let car = if CARS.contains(&args.car.as_str()) {
        args.car.as_str()
    } else {
        warn!("arena: unknown car {:?}, using sparky", args.car);
        "sparky"
    };
    spawn_car(
        &mut commands,
        &server,
        car,
        Transform::from_translation(START),
    );
    ticks.0 = 0;
}

fn collect_wheels(
    ready: On<WorldInstanceReady>,
    proxies: Query<(), With<ProxyOf>>,
    children: Query<&Children>,
    nodes: Query<(&Name, &Transform)>,
    mut commands: Commands,
) {
    if proxies.get(ready.entity).is_err() {
        return;
    }
    let names = ["wheel_fl", "wheel_fr", "wheel_rl", "wheel_rr"];
    let mut found = [None; 4];
    for e in children.iter_descendants(ready.entity) {
        if let Ok((name, t)) = nodes.get(e)
            && let Some(i) = names.iter().position(|n| *n == name.as_str())
        {
            found[i] = Some((e, *t));
        }
    }
    if let [Some(a), Some(b), Some(c), Some(d)] = found {
        commands
            .entity(ready.entity)
            .insert(WheelNodes([a, b, c, d]));
    }
}

fn count_ticks(mut ticks: ResMut<RunTicks>) {
    ticks.0 += 1;
}

/// `--auto`: a scripted run over the kicker, a drift and a boost (deterministic per tick).
fn auto_drive(
    args: Res<Args>,
    ticks: Res<RunTicks>,
    mut cars: Query<&mut CarControl, With<Player>>,
) {
    if !args.auto {
        return;
    }
    let t = ticks.0 as f32 / SIM_HZ as f32;
    for mut c in &mut cars {
        *c = CarControl {
            throttle: 1.0,
            steer: match t {
                t if (5.2..6.6).contains(&t) => 1.0,
                t if (6.6..7.2).contains(&t) => 0.3,
                _ => 0.0,
            },
            drift: (5.4..7.0).contains(&t),
            boost: (7.6..8.6).contains(&t),
            ..default()
        };
    }
}

fn keyboard_drive(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    args: Res<Args>,
    mut cars: Query<&mut CarControl, With<Player>>,
) {
    let Ok(mut c) = cars.single_mut() else {
        return;
    };
    if args.auto {
        return;
    }
    let any = |k: &[KeyCode]| keys.any_pressed(k.iter().copied());
    let target = f32::from(any(&[KeyCode::KeyD, KeyCode::ArrowRight]))
        - f32::from(any(&[KeyCode::KeyA, KeyCode::ArrowLeft]));
    let step = time.delta_secs() / STEER_RAMP_SECS;
    c.steer += (target - c.steer).clamp(-step, step);
    c.throttle = f32::from(any(&[KeyCode::KeyW, KeyCode::ArrowUp]));
    c.brake = f32::from(any(&[KeyCode::KeyS, KeyCode::ArrowDown]));
    c.drift = any(&[KeyCode::Space]);
    c.boost = any(&[KeyCode::ShiftLeft]);
}

/// R flip upright · Backspace back to start · G fill the battery · 1–8 switch car.
fn keyboard_actions(
    keys: Res<ButtonInput<KeyCode>>,
    server: Res<AssetServer>,
    mut commands: Commands,
    cars: Query<(Entity, &Transform, &Racer), With<Player>>,
    proxies: Query<(Entity, &ProxyOf)>,
    mut respawn: MessageWriter<RespawnCar>,
    mut grant: MessageWriter<GrantCharge>,
) {
    let Ok((car, t, racer)) = cars.single() else {
        return;
    };
    if keys.just_pressed(KeyCode::KeyR) {
        let yaw = t.rotation.to_euler(EulerRot::YXZ).0;
        let upright = Transform::from_translation(t.translation + Vec3::Y * 0.6)
            .with_rotation(Quat::from_rotation_y(yaw));
        respawn.write(RespawnCar {
            car,
            transform: upright,
            kind: RespawnKind::FlipInPlace,
        });
    }
    if keys.just_pressed(KeyCode::Backspace) {
        let start = Transform::from_translation(START);
        respawn.write(RespawnCar {
            car,
            transform: start,
            kind: RespawnKind::ToTrack,
        });
    }
    if keys.just_pressed(KeyCode::KeyG) {
        grant.write(GrantCharge { car, amount: 1.0 });
    }
    let digits = [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
    ];
    if let Some(i) = digits.iter().position(|k| keys.just_pressed(*k))
        && racer.car.as_str() != CARS[i]
    {
        commands.entity(car).despawn();
        for (proxy, of) in &proxies {
            if of.0 == car {
                commands.entity(proxy).despawn();
            }
        }
        let at =
            Transform::from_translation(t.translation + Vec3::Y * 0.3).with_rotation(t.rotation);
        spawn_car(&mut commands, &server, CARS[i], at);
    }
}

fn pose_proxies(
    fixed: Res<Time<Fixed>>,
    sims: Query<&PoseHistory>,
    mut proxies: Query<(&ProxyOf, &mut Transform)>,
) {
    let alpha = fixed.overstep_fraction();
    for (of, mut t) in &mut proxies {
        if let Ok(history) = sims.get(of.0) {
            *t = history.interpolate(alpha);
        }
    }
}

fn animate_wheels(
    proxies: Query<(&ProxyOf, &WheelNodes)>,
    tel: Query<&VehicleTelemetry>,
    mut nodes: Query<&mut Transform, Without<ProxyOf>>,
) {
    for (of, wheels) in &proxies {
        let Ok(tel) = tel.get(of.0) else { continue };
        for ((node, rest), w) in wheels.0.iter().zip(&tel.wheels) {
            if let Ok(mut t) = nodes.get_mut(*node) {
                t.translation = rest.translation + Vec3::Y * w.offset;
                t.rotation = Quat::from_rotation_y(-w.steer)
                    * Quat::from_rotation_x(-w.spin)
                    * rest.rotation;
            }
        }
    }
}

fn follow_camera(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    cars: Query<&ProxyOf>,
    targets: Query<&Transform, Without<ChaseCam>>,
    mut cam: Single<(&mut Transform, &mut ChaseCam)>,
) {
    let (ref mut cam_t, ref mut chase) = *cam;
    if keys.just_pressed(KeyCode::KeyV) {
        chase.far = !chase.far;
    }
    let Some(target) = cars.iter().next().and_then(|of| targets.get(of.0).ok()) else {
        return;
    };
    let fwd = (target.rotation * Vec3::NEG_Z)
        .with_y(0.0)
        .normalize_or(Vec3::NEG_Z);
    let (back, up) = if chase.far { (11.0, 4.5) } else { (6.5, 2.6) };
    let goal = target.translation - fwd * back + Vec3::Y * up;
    let k = 1.0 - (-6.0 * time.delta_secs()).exp();
    cam_t.translation = cam_t.translation.lerp(goal, k);
    cam_t.look_at(target.translation + Vec3::Y * 0.7, Vec3::Y);
}

fn update_hud(
    cars: Query<(&Racer, &VehicleTelemetry, &Charge, &CarControl), With<Player>>,
    mut hud: Single<&mut Text, With<Hud>>,
) {
    let Ok((racer, t, charge, c)) = cars.single() else {
        hud.0 = "loading...".into();
        return;
    };
    let bar = |v: f32| {
        let n = (v.clamp(0.0, 1.0) * 20.0).round() as usize;
        format!("[{}{}]", "#".repeat(n), ".".repeat(20 - n))
    };
    hud.0 = format!(
        "{}  [1-8 car | W/S | A/D | Space drift | Shift boost | R flip | Bksp start | G charge | V cam]\n\
         speed {:5.1} km/h  fwd {:5.1} u/s   rpm {}\n\
         charge {} {:3.0} %  +{:4.1} %/s  {}\n\
         wheels {}  air {:4.2} s  slip {:+5.1} deg  {}{}\n\
         surface {:?}  up {:+.2}  steer {:+.2}  last landing {}",
        racer.car,
        t.speed * SPEED_DISPLAY_KMH,
        t.forward_speed,
        bar(t.rpm),
        bar(charge.0),
        charge.0 * 100.0,
        t.charge_rate * 100.0,
        if t.boosting { "BOOST" } else { "" },
        t.grounded_wheels,
        t.airborne_time,
        t.slip_angle.to_degrees(),
        if t.drifting { "DRIFT " } else { "" },
        if c.drift { "(handbrake)" } else { "" },
        t.surface,
        t.up_dot,
        c.steer,
        if t.clean_landing { "clean!" } else { "-" },
    );
}

fn automation(
    args: Res<Args>,
    ticks: Res<RunTicks>,
    real: Res<Time<bevy::time::Real>>,
    window: Single<&Window>,
    mut taken: Local<usize>,
    mut commands: Commands,
    mut exit: MessageWriter<AppExit>,
) {
    if let Some(path) = &args.screenshot
        && let Some(at) = args.shots_at.get(*taken)
        && ticks.0 as f32 / SIM_HZ as f32 >= *at
    {
        let path = match args.shots_at.len() {
            1 => path.clone(),
            _ => path.with_file_name(format!(
                "{}_{}.png",
                path.file_stem()
                    .map_or("shot".into(), |s| s.to_string_lossy()),
                *taken + 1
            )),
        };
        info!(
            "arena: screenshot {} ({:?})",
            path.display(),
            window.physical_size()
        );
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
        *taken += 1;
    }
    if args.exit_after.is_some_and(|s| real.elapsed_secs() >= s) {
        exit.write(AppExit::Success);
    }
}
