//! # rc_render — graphics (P)
//!
//! - **Responsibilities:** visual proxies (`VisualOf` / `Visual`) posed from `PoseHistory` in
//!   `InterpolateSet` ([`proxy`]); per-camera quality & post-processing, display settings
//!   ([`quality`]); environments — lighting, sky, time of day, colour grading, headlights
//!   ([`lighting`], [`sky`]); weather visuals ([`weather_fx`]); the [`MaterialLibrary`] of
//!   surface materials and car paints ([`materials`]); the menu showroom ([`showroom`]). Track,
//!   prop and car visuals and captures are T13b's content modules.
//! - **Inputs:** `Interpolated` + `PoseHistory`, `Settings.graphics` / `.accessibility`,
//!   `RaceConfig`, `Catalog`, `WeatherState`, `Track` (bounds), `MenuBackdrop`, camera markers
//!   (`GameCamera`, `MenuCamera`), `LocalPlayer` racers, `UserPaths`.
//! - **Outputs:** visual entities only (proxies, lights, sky, showroom), camera rendering
//!   components, the window's mode/size/vsync, [`MaterialLibrary`], [`ActiveEnvironment`],
//!   [`PipelineQueue`], `ShowroomReady`, `LoadingDone("visuals")` / `LoadingFailed`.
//! - **Must never:** write simulation state or rapier bodies, touch the split-screen
//!   `OverlayCamera`, or load assets per frame.

mod capture;
mod cars;
mod content;
mod lighting;
mod materials;
mod preprocess_fix;
mod props;
mod proxy;
mod quality;
mod showroom;
mod sky;
mod track_visuals;
mod weather_fx;

use bevy::prelude::*;
use rc_core::prelude::*;

pub use cars::{CarModel, CarModelReady};
pub use content::ContentVisualsPlugin;
pub use lighting::{
    ActiveEnvironment, ENVIRONMENTS_DIR, EnvironmentDef, LUMEN_SCALE, LampGrid, PipelineQueue,
    RoomDef, SHOWROOM_ENV, TimeLook, WindowDef, WindowWall, lamp_lumens,
};
pub use materials::{
    CarPaint, MaterialDef, MaterialLibrary, SurfaceExtension, SurfaceMaterial, TEXTURE_SETS,
    builtin_materials, default_surface, paint_material, with_world_uvs,
};
pub use quality::{AaMode, ShadowTier, ViewQuality, shadow_tier, view_quality};

/// Loading-gate key of this crate.
pub const LOADING_KEY: &str = "visuals";

/// Rendering of the simulation and the menus.
pub struct GraphicsPlugin;

impl Plugin for GraphicsPlugin {
    fn build(&self, app: &mut App) {
        LoadingTasks::register(app, LOADING_KEY);
        proxy::build(app);
        quality::build(app);
        materials::build(app);
        lighting::build(app);
        sky::build(app);
        weather_fx::build(app);
        showroom::build(app);
        preprocess_fix::build(app);
        app.add_plugins(content::ContentVisualsPlugin);
    }
}
