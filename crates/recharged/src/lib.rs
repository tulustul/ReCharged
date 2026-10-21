//! # recharged — the game's app assembly (BIN library)
//!
//! [`game`] wires every plugin into the windowed app; the `recharged` binary (`main.rs`) and the
//! track editor (`rc_editor`) both start from it. Owns the wall-clock automation hooks
//! ([`automation`]), `--bench` ([`bench`]) and the human-run quit guard ([`quit`]).
//!
//! Must never: contain game logic (it only wires plugins and flags together).

mod automation;
mod bench;
mod quit;

use std::path::PathBuf;

use bevy::prelude::*;
use bevy::render::settings::{WgpuFeatures, WgpuSettings};
use bevy::window::{PresentMode, WindowResolution};
use bevy::winit::{UpdateMode, WinitSettings};
use bevy_react::prelude::*;
use rc_core::prelude::*;

/// The React bundle (`app.js`; `vendor.js` must sit next to it), or why it cannot be used.
pub fn checked_ui_bundle() -> Result<PathBuf, String> {
    let bundle = ui_bundle();
    for file in [bundle.clone(), bundle.with_file_name("vendor.js")] {
        if !file.is_file() {
            return Err(format!(
                "UI bundle missing: {} (build it: npm --prefix ui run build)",
                file.display()
            ));
        }
    }
    Ok(bundle)
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

/// The windowed game: every plugin, the React UI from `bundle`, the automation hooks of `config`.
pub fn game(config: AutomationConfig, paths: UserPaths, bundle: PathBuf) -> App {
    let mut app = App::new();
    // Systems that fail log the error instead of panicking the game.
    app.set_error_handler(bevy::ecs::error::error);
    // Before DefaultPlugins: rc_game's session-log layer reads the user-data dir from it.
    app.insert_resource(paths.clone());
    let automated = config.is_automated();
    let (w, h) = config.window.unwrap_or((1600, 900));
    let mut resolution = WindowResolution::new(w, h);
    if config.bench.is_some() {
        // The budget's 1080p means physical pixels, whatever the desktop's DPI scale.
        resolution = resolution.with_scale_factor_override(1.0);
    }
    // A script's virtual pads are unknown to gilrs, which warns about every rumble for them.
    let filter = match config.input_script {
        Some(_) => format!("{},bevy_gilrs::rumble=error", bevy::log::DEFAULT_FILTER),
        None => bevy::log::DEFAULT_FILTER.to_owned(),
    };
    app.add_plugins(
        DefaultPlugins
            .set(bevy::log::LogPlugin {
                custom_layer: rc_game::session_log_layer,
                filter,
                ..default()
            })
            // Automated runs exit within seconds; exiting while pipelines still compile on
            // Bevy's async threads drops the GPU device on a worker thread and crashes the NVIDIA
            // driver (exit 139). Synchronous compilation removes that window; human runs defer
            // their exit until the pipeline queue is idle instead ([`quit`]).
            .set(bevy::render::RenderPlugin {
                synchronous_pipeline_compilation: automated,
                // Bevy 0.20's binding arrays (bindless `StandardMaterial`, environment-map arrays)
                // hang Intel GPUs on Mesa at the first 3D frame (DeviceLost seconds later,
                // docs/upstream-notes.md U7). Without them Bevy binds per material and per view.
                render_creation: WgpuSettings {
                    disabled_features: Some(
                        WgpuFeatures::TEXTURE_BINDING_ARRAY | WgpuFeatures::BUFFER_BINDING_ARRAY,
                    ),
                    ..default()
                }
                .into(),
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "ReCharged".into(),
                    resolution,
                    // A fixed-size window makes tiling window managers float it at the requested
                    // size, so `--window` screenshots really have that resolution.
                    resizable: config.window.is_none(),
                    // Automated runs must not depend on the monitor: a DPMS-blanked screen stalls
                    // vsync'd presentation to ~1 fps (rc_render keeps it off in automated runs).
                    present_mode: if automated {
                        PresentMode::AutoNoVsync
                    } else {
                        PresentMode::AutoVsync
                    },
                    ..default()
                }),
                // Human runs: `quit` closes the window once no pipeline is compiling.
                close_when_requested: automated,
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
                // The devtools keep their layout in the user-data dir, not in the cwd; F12 / F11
                // belong to the profiler (rc_ui), so they toggle on F10 and pause on F9.
                .devtools(bevy_react::DevtoolsConfig {
                    toggle_key: KeyCode::F10,
                    pause_key: KeyCode::F9,
                    settings_path: Some(paths.data_dir.join("devtools.json")),
                    ..default()
                }),
        )),
    )
    .insert_resource(config);
    if automated {
        // Bevy throttles an unfocused window to 60 Hz; automated windows rarely have the focus.
        app.insert_resource(WinitSettings {
            focused_mode: UpdateMode::Continuous,
            unfocused_mode: UpdateMode::Continuous,
        });
    } else {
        app.add_plugins(quit::plugin);
    }
    add_sim_plugins(&mut app);
    app.add_plugins((
        rc_input::GameInputPlugin,
        rc_camera::GameCameraPlugin,
        rc_render::GraphicsPlugin,
        rc_vfx::VfxPlugin,
        rc_audio::GameAudioPlugin,
        rc_game::GameFlowPlugin,
        rc_ui::GameUiPlugin,
        automation::AutomationPlugin,
    ));
    if app.world().resource::<AutomationConfig>().bench.is_some() {
        app.insert_resource(bench::settings())
            .add_plugins(bench::plugin);
    }
    #[cfg(feature = "debug-physics")]
    app.add_plugins(bevy_rapier3d::prelude::RapierDebugRenderPlugin::default());
    app
}

/// Core contracts and every simulation plugin (no devices): shared by the game and the headless
/// runs.
pub fn add_sim_plugins(app: &mut App) {
    app.add_plugins((
        CorePlugin,
        rc_vehicle::PhysicsPlugin,
        rc_track::TrackPlugin,
        rc_vehicle::VehiclePlugin,
        rc_weather::WeatherPlugin,
        rc_race::RacePlugin,
        rc_ai::AiPlugin,
        rc_items::ItemsPlugin,
    ));
}
