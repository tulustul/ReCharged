//! # recharged — the game binary (BIN)
//!
//! Assembles every plugin, parses the command line ([`cli`]) into `AutomationConfig`, resolves
//! `UserPaths`, roots the `AssetPlugin` at the assets dir, loads the React bundle and owns the
//! wall-clock automation hooks ([`automation`]). `--export-bindings <path>` writes the TypeScript
//! client and exits without opening a window.
//!
//! Must never: contain game logic (it only wires plugins and flags together).

mod automation;
mod cli;

use std::path::PathBuf;

use bevy::prelude::*;
use bevy::window::WindowResolution;
use bevy_react::prelude::*;
use rc_core::prelude::*;

fn main() -> AppExit {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let config = match cli::parse(&args) {
        Ok(config) => config,
        Err(cli::CliError::Help) => {
            println!("{}", cli::USAGE);
            return AppExit::Success;
        }
        Err(cli::CliError::Invalid(message)) => {
            eprintln!("error: {message}\n\n{}", cli::USAGE);
            return AppExit::from_code(2);
        }
    };
    if let Some(path) = &config.export_bindings {
        return match rc_ui::export_bindings(path) {
            Ok(()) => {
                println!("wrote React bindings to {}", path.display());
                AppExit::Success
            }
            Err(e) => {
                eprintln!("cannot write {}: {e}", path.display());
                AppExit::error()
            }
        };
    }
    let bundle = ui_bundle();
    for file in [bundle.clone(), bundle.with_file_name("vendor.js")] {
        if !file.is_file() {
            eprintln!(
                "UI bundle missing: {} (build it: npm --prefix ui run build)",
                file.display()
            );
            return AppExit::error();
        }
    }
    let paths = UserPaths::resolve(
        config.assets_dir.as_deref(),
        config.user_data_dir.as_deref(),
    );
    game(config, paths, bundle).run()
}

/// Dev builds load the workspace bundle (cwd-independent); release builds ship `ui/app.js` +
/// `ui/vendor.js` next to the executable.
fn ui_bundle() -> PathBuf {
    if cfg!(debug_assertions) {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../ui/dist/app.js")
    } else {
        std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|dir| dir.join("ui/app.js")))
            .unwrap_or_else(|| PathBuf::from("ui/app.js"))
    }
}

fn game(config: AutomationConfig, paths: UserPaths, bundle: PathBuf) -> App {
    let mut app = App::new();
    // Systems that fail log the error instead of panicking the game.
    app.set_error_handler(bevy::ecs::error::error);
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "ReCharged".into(),
                    resolution: WindowResolution::new(1600, 900),
                    ..default()
                }),
                ..default()
            })
            .set(AssetPlugin {
                file_path: paths.assets_dir.to_string_lossy().into_owned(),
                ..default()
            }),
    )
    .add_plugins(
        ReactPlugins.set(rc_ui::with_fonts(
            ReactUiPlugin::new(bundle)
                .hot_reload(cfg!(debug_assertions))
                // The F12 devtools keep their layout in the user-data dir, not in the cwd.
                .devtools(bevy_react::DevtoolsConfig {
                    settings_path: Some(paths.data_dir.join("devtools.json")),
                    ..default()
                }),
        )),
    )
    .insert_resource(config)
    .insert_resource(paths)
    .add_plugins((
        CorePlugin,
        rc_vehicle::PhysicsPlugin,
        rc_track::TrackPlugin,
        rc_vehicle::VehiclePlugin,
        rc_input::GameInputPlugin,
        rc_weather::WeatherPlugin,
        rc_race::RacePlugin,
        rc_ai::AiPlugin,
        rc_items::ItemsPlugin,
    ))
    .add_plugins((
        rc_camera::GameCameraPlugin,
        rc_render::GraphicsPlugin,
        rc_vfx::VfxPlugin,
        rc_audio::GameAudioPlugin,
        rc_game::GameFlowPlugin,
        rc_ui::GameUiPlugin,
        automation::AutomationPlugin,
    ));
    #[cfg(feature = "debug-physics")]
    app.add_plugins(bevy_rapier3d::prelude::RapierDebugRenderPlugin::default());
    app
}
