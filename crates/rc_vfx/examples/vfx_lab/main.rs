//! Effects lab: a lit test ground where scripted scenes trigger every effect group, for visual
//! checks of effects a quick race does not produce on demand. Scenes spawn stand-in entities
//! (`Racer`, `Pickup`, `Projectile`, `Hazard`, …) and write the messages rc_items, rc_race and
//! rc_weather would, exactly as rc_vfx reads them in the game.
//!
//! `cargo run -p rc_vfx --example vfx_lab -- --scene
//! <shapes|drive|sparks|items|blasts|events|weather> [--chase] [--wet] [--snow] [--split]
//! [--night] [--calm] [--slow <speed>] [--screenshot <png>
//! --screenshot-at <s>[,<s>…]] [--exit-after <s>]` — `--slow 0.25` runs at quarter speed, so
//! short flashes can be caught; screenshot times stay in wall-clock seconds.

mod scenes;

use std::path::PathBuf;

use bevy::camera::{Exposure, Viewport};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::light::light_consts::lux;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::window::{PresentMode, PrimaryWindow};
use rc_core::prelude::*;
use rc_vfx::{CarShape, CarShapes, VfxPlugin};

/// Command line of the lab.
#[derive(Resource, Clone, Default)]
pub(crate) struct Args {
    pub(crate) scene: String,
    pub(crate) chase: bool,
    pub(crate) snow: bool,
    pub(crate) split: bool,
    pub(crate) night: bool,
    pub(crate) calm: bool,
    pub(crate) wet: bool,
    slow: Option<f32>,
    screenshot: Option<PathBuf>,
    shots_at: Vec<f32>,
    exit_after: Option<f32>,
}

fn parse_args() -> Args {
    let mut args = Args {
        scene: "shapes".into(),
        ..default()
    };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let mut value = || it.next().unwrap_or_default();
        match flag.as_str() {
            "--scene" => args.scene = value(),
            "--chase" => args.chase = true,
            "--snow" => args.snow = true,
            "--split" => args.split = true,
            "--night" => args.night = true,
            "--calm" => args.calm = true,
            "--wet" => args.wet = true,
            "--slow" => args.slow = value().parse().ok(),
            "--screenshot" => args.screenshot = Some(value().into()),
            "--screenshot-at" => {
                args.shots_at = value().split(',').filter_map(|s| s.parse().ok()).collect();
            }
            "--exit-after" => args.exit_after = value().parse().ok(),
            other => eprintln!("vfx_lab: ignoring unknown argument {other}"),
        }
    }
    if args.screenshot.is_some() && args.shots_at.is_empty() {
        args.shots_at.push(2.0);
    }
    args
}

fn main() -> AppExit {
    let args = parse_args();
    let assets = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    let mut settings = Settings::default();
    settings.accessibility.reduce_flashing = args.calm;
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "ReCharged — effects lab".into(),
                        resolution: (1280, 720).into(),
                        // A fixed-size window floats in tiling window managers (i3), so
                        // screenshots keep their size.
                        resizable: false,
                        // Frame-time numbers must not wait on vsync (or on a hidden window).
                        present_mode: PresentMode::AutoNoVsync,
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
        .insert_resource(settings)
        .insert_resource(ClearColor(if args.night {
            Color::srgb(0.01, 0.015, 0.03)
        } else {
            Color::srgb(0.5, 0.62, 0.78)
        }))
        .insert_resource(GlobalAmbientLight {
            color: Color::srgb(0.75, 0.82, 1.0),
            brightness: if args.night { 30.0 } else { 2500.0 },
            ..default()
        })
        .insert_resource(args)
        .add_plugins((CorePlugin, VfxPlugin))
        .add_systems(
            Startup,
            (enter_race, build_ground, spawn_views, load_shapes),
        )
        .add_systems(Update, (split_viewports, automation))
        .add_plugins(scenes::plugin)
        .run()
}

fn enter_race(
    args: Res<Args>,
    mut time: ResMut<Time<Virtual>>,
    mut next: ResMut<NextState<AppState>>,
    mut phase: ResMut<NextState<RacePhase>>,
) {
    next.set(AppState::InRace);
    phase.set(RacePhase::Racing);
    if let Some(speed) = args.slow {
        time.set_relative_speed(speed);
    }
}

/// Every car model the lab uses.
pub(crate) const CARS: [&str; 8] = [
    "sparky", "dumpster", "zip", "torque", "hornet", "phantom", "volt_gt", "bolt",
];

fn load_shapes(paths: Res<UserPaths>, mut shapes: ResMut<CarShapes>) {
    for car in CARS {
        let path = paths
            .assets_dir
            .join(format!("models/cars/{car}.wheels.ron"));
        let text = std::fs::read_to_string(&path).expect("wheel file");
        let shape = CarShape::parse(&text).expect("wheel file parses");
        shapes.0.insert(CarId::new(car), shape);
    }
}

/// Surface patches of the test ground: (centre x/z, radius, surface, colour).
pub(crate) const PATCHES: [(Vec2, f32, SurfaceKind, [f32; 3]); 4] = [
    (
        Vec2::new(-9.0, -8.0),
        7.0,
        SurfaceKind::Dirt,
        [0.36, 0.26, 0.17],
    ),
    (
        Vec2::new(9.0, -8.0),
        7.0,
        SurfaceKind::Grass,
        [0.2, 0.36, 0.12],
    ),
    (
        Vec2::new(9.0, 8.0),
        7.0,
        SurfaceKind::Water,
        [0.08, 0.12, 0.16],
    ),
    (
        Vec2::new(-9.0, 8.0),
        7.0,
        SurfaceKind::Asphalt,
        [0.24, 0.24, 0.26],
    ),
];

fn build_ground(
    args: Res<Args>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(400.0, 400.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.5, 0.47, 0.42),
            perceptual_roughness: 0.6,
            ..default()
        })),
    ));
    if args.scene == "drive" {
        for (centre, radius, surface, c) in PATCHES {
            commands.spawn((
                Mesh3d(meshes.add(Circle::new(radius).mesh().resolution(64))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(c[0], c[1], c[2]),
                    perceptual_roughness: if surface == SurfaceKind::Water {
                        0.05
                    } else {
                        0.9
                    },
                    ..default()
                })),
                Transform::from_xyz(centre.x, 0.005, centre.y)
                    .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
            ));
        }
    }
    // A wall to crash into (events scene).
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(12.0, 1.2, 0.5))),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.2, 0.15))),
        Transform::from_xyz(0.0, 0.6, -12.0),
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: if args.night {
                40.0
            } else {
                lux::FULL_DAYLIGHT * 5.0
            },
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(30.0, 50.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// The lab's views: one game camera, or two in a top/bottom split.
fn spawn_views(args: Res<Args>, mut commands: Commands) {
    let views = if args.split { 2 } else { 1 };
    for slot in 0..views {
        commands.spawn((
            Camera3d::default(),
            Camera {
                order: slot as isize,
                ..default()
            },
            Tonemapping::TonyMcMapface,
            Bloom {
                intensity: 0.12,
                ..Bloom::NATURAL
            },
            Exposure {
                ev100: if args.night { 5.0 } else { 14.7 },
            },
            if args.scene == "drive" {
                // Overview of the four surface patches.
                Transform::from_xyz(0.0, 13.0, 26.0).looking_at(Vec3::new(0.0, 0.0, 1.0), Vec3::Y)
            } else {
                Transform::from_xyz(0.0, 6.0, 16.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y)
            },
            GameCamera {
                slot: PlayerSlot(slot),
            },
        ));
    }
}

/// Top/bottom halves for `--split`.
fn split_viewports(
    window: Single<&Window, With<PrimaryWindow>>,
    mut cams: Query<(&GameCamera, &mut Camera)>,
) {
    let n = cams.iter().count() as u32;
    if n < 2 {
        return;
    }
    let size = window.physical_size();
    let cell = UVec2::new(size.x, size.y / n);
    for (cam, mut camera) in &mut cams {
        let pos = UVec2::new(0, u32::from(cam.slot.0) * cell.y);
        if camera
            .viewport
            .as_ref()
            .is_none_or(|v| v.physical_position != pos || v.physical_size != cell)
        {
            camera.viewport = Some(Viewport {
                physical_position: pos,
                physical_size: cell,
                ..default()
            });
        }
    }
}

fn automation(
    args: Res<Args>,
    time: Res<Time<Real>>,
    fx: Res<rc_vfx::Fx>,
    mut taken: Local<usize>,
    mut frames: Local<(u32, f32, usize)>,
    mut commands: Commands,
    mut exit: MessageWriter<AppExit>,
) {
    let now = time.elapsed_secs();
    // Frame statistics after a 2 s warm-up: frames, seconds, peak live particles.
    if now > 2.0 {
        frames.0 += 1;
        frames.1 += time.delta_secs();
        frames.2 = frames.2.max(fx.pool.count());
    }
    if let Some(path) = &args.screenshot
        && let Some(&at) = args.shots_at.get(*taken)
        && now >= at
    {
        let path = if args.shots_at.len() > 1 {
            path.with_file_name(format!(
                "{}_{}.png",
                path.file_stem().unwrap_or_default().to_string_lossy(),
                *taken
            ))
        } else {
            path.clone()
        };
        info!("screenshot -> {}", path.display());
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
        *taken += 1;
    }
    if args.exit_after.is_some_and(|limit| now >= limit) {
        let (n, secs, peak) = *frames;
        info!(
            "lab: {n} frames in {secs:.1} s = {:.1} fps ({:.2} ms), peak {peak} particles",
            n as f32 / secs.max(1e-3),
            secs * 1000.0 / n.max(1) as f32
        );
        exit.write(AppExit::Success);
    }
}
