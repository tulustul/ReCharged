//! # rc_render — graphics (P)
//!
//! - **Responsibilities:** visual proxies (`VisualOf` / `Visual`) posed from `PoseHistory` in
//!   `InterpolateSet`; quality & post-processing per camera; lighting, sky and time of day;
//!   materials; track, prop and car visuals; the menu showroom; captures (previews, thumbnails,
//!   track tour).
//! - **Inputs:** `Interpolated` + `PoseHistory`, `Settings.graphics`, `WeatherState` /
//!   `WeatherEvent`, track geometry (rc_track), `VehicleTelemetry`, `StatusEffects`,
//!   `MenuBackdrop`, camera markers.
//! - **Outputs:** visual entities only, camera rendering components, `LoadingDone("visuals")`.
//! - **Must never:** write simulation state or rapier bodies.
//!
//! Status: skeleton (T01); implemented by T13.

use bevy::prelude::*;
use rc_core::prelude::*;

/// Loading-gate key of this crate.
pub const LOADING_KEY: &str = "visuals";

/// Rendering of the simulation and the menus.
pub struct GraphicsPlugin;

impl Plugin for GraphicsPlugin {
    fn build(&self, app: &mut App) {
        LoadingTasks::register(app, LOADING_KEY);
        app.add_systems(OnEnter(AppState::Loading), report_loaded);
    }
}

// ponytail: stub — reports "loaded" at once so the gate opens; T13 loads the visuals first.
fn report_loaded(mut done: MessageWriter<LoadingDone>) {
    done.write(LoadingDone(LOADING_KEY));
}
