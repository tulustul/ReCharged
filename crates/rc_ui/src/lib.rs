//! # rc_ui — the bevy-react bridge (F2)
//!
//! - **Responsibilities:** every typed React channel, one module per namespace (`app`, `ui`,
//!   `input`, `menu`, `profile`, `records`, `settings`, `lobby`, `race`, `results`, `hud`; shared
//!   types in `content`), mapping React messages to `GameCommand`s and rc_core messages and
//!   pushing state to React;
//!   [`register_bindings`], shared by the app and the TypeScript exporter ([`export_bindings`]);
//!   the UI fonts ([`with_fonts`]); `UiScale` (menus are authored at [`DESIGN_SIZE`] UI px and
//!   scaled to fit the window); the `chargeSwap` morph filter; the `ui.ready` watchdog and the
//!   `RECHARGED_UI_SCRIPT` automation hook ([`debug`]).
//! - **Inputs:** React messages/requests, `AppState` / `RacePhase` / `PauseState`, `UiNav`,
//!   `DeviceEvent`, `RebindResult`, `LoadingTasks`, `Catalog`, `Settings`, `LocalPlayers`,
//!   `LobbyState`, `RaceConfig`, `RaceResults`, the race state (`RaceStatus`, `Standings`, racer
//!   components, item entities, `RaceEvent`, `HitEvent`, `TrackSummary`, `PlayerViewports`); rc_game's `Profile`, `CupRun`, `VersusSession`,
//!   `RaceOutcome`, `GameNotice`; `AutomationConfig`, `UserPaths`.
//! - **Outputs:** React events, `GameCommand`s, `UiSound`, `MusicStinger`, `RebindRequest`,
//!   `MenuBackdrop` (sole writer), `UiScale` (sole writer), [`UiStatus`], the HUD's shared values.
//! - **Must never:** send events per frame (state changes and ≤ 10 Hz batches only), leave a
//!   request unanswered, hold gameplay logic, or write `Settings` (rc_game applies them).
//!
//! UI px = window logical px / `UiScale`: convert window-space rects (e.g. `PlayerViewports`)
//! before sending them to React.
//!
//! Regenerate `ui/src/bevy.ts` after changing any channel:
//! `cargo run -p rc_ui --example export_bindings -- ui/src/bevy.ts`.

use std::path::Path;

use bevy::asset::embedded_asset;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_react::prelude::*;
use rc_core::prelude::*;

/// Declares a camelCase TypeScript mirror of an rc_core enum (same variant names) with `From`
/// both ways; an exhaustive match keeps the two in sync at compile time.
macro_rules! mirror_enum {
    ($(#[$meta:meta])* $name:ident = $core:ident { $($variant:ident),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(serde::Serialize, serde::Deserialize, ts_rs::TS)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[serde(rename_all = "camelCase")]
        #[ts(rename_all = "camelCase")]
        #[allow(missing_docs)] // the variants mirror the documented rc_core enum
        pub enum $name { $($variant),+ }

        impl From<$core> for $name {
            fn from(value: $core) -> Self {
                match value { $($core::$variant => Self::$variant),+ }
            }
        }

        impl From<$name> for $core {
            fn from(value: $name) -> Self {
                match value { $($name::$variant => $core::$variant),+ }
            }
        }
    };
}

pub mod app;
pub mod content;
pub mod debug;
pub mod filters;
pub mod hud;
pub mod input;
pub mod lobby;
pub mod menu;
pub mod profile;
pub mod race;
pub mod records;
pub mod results;
pub mod settings;
pub mod ui;

pub use ui::{READY_TIMEOUT_SECS, UiStatus};

/// Size the menus are authored at, in UI px; `UiScale` fits it into the window.
pub const DESIGN_SIZE: Vec2 = Vec2::new(1920.0, 1080.0);

/// Body font (variable weight): every `<text>` without a `fontFamily`.
pub const BODY_FONT: &str = "fonts/Saira-Variable.ttf";
/// Display font (variable weight, italic) for titles and buttons: `fontFamily: "Display"`.
pub const DISPLAY_FONT: &str = "fonts/Saira-Italic-Variable.ttf";

/// The React bridge. Add after `ReactPlugins` and `GameFlowPlugin`.
pub struct GameUiPlugin;

impl Plugin for GameUiPlugin {
    fn build(&self, app: &mut App) {
        register_bindings(app);
        // The exporter has no asset server; only the running app embeds the shader.
        embedded_asset!(app, "shaders/charge_swap.wesl");
        app.init_resource::<UiStatus>()
            .init_resource::<MenuBackdrop>()
            .add_systems(
                Update,
                (
                    fit_ui_scale,
                    app::push_state,
                    ui::forward_nav,
                    ui::forward_notices,
                    ui::watchdog,
                    input::forward_devices,
                    input::forward_rebinds,
                    input::push_bindings,
                    lobby::push_slots,
                    profile::push_profile,
                    race::push_loading,
                    results::push_results,
                    results::push_series,
                )
                    .in_set(UiSet),
            );
        debug::build(app);
        hud::build(app);
    }
}

/// Registers the UI fonts on the React plugin:
/// `ReactPlugins.set(rc_ui::with_fonts(ReactUiPlugin::new(bundle)))`.
pub fn with_fonts(plugin: ReactUiPlugin) -> ReactUiPlugin {
    plugin.default_font(BODY_FONT).font("Display", DISPLAY_FONT)
}

/// Registers every channel; the app and [`export_bindings`] call exactly this.
pub fn register_bindings(app: &mut App) {
    app::register(app);
    debug::register(app);
    filters::register(app);
    hud::register(app);
    input::register(app);
    lobby::register(app);
    menu::register(app);
    profile::register(app);
    race::register(app);
    records::register(app);
    results::register(app);
    settings::register(app);
    ui::register(app);
}

/// Writes the typed TypeScript client (bevy-react's element typings + our channels) to `path`.
/// No window and no JS runtime are created.
pub fn export_bindings(path: impl AsRef<Path>) -> std::io::Result<()> {
    let mut app = App::new();
    ReactPlugins::register_bindings(&mut app);
    register_bindings(&mut app);
    app.export_react_typescript(path)
}

/// `UiScale` that fits [`DESIGN_SIZE`] into a window of `width × height` logical px (the
/// design canvas grows along the longer side on other aspect ratios).
pub fn ui_scale_for(width: f32, height: f32) -> f32 {
    if width <= 0.0 || height <= 0.0 {
        return 1.0;
    }
    (width / DESIGN_SIZE.x).min(height / DESIGN_SIZE.y)
}

fn fit_ui_scale(
    windows: Query<&Window, (With<PrimaryWindow>, Changed<Window>)>,
    scale: Option<ResMut<UiScale>>,
) {
    let (Ok(window), Some(mut scale)) = (windows.single(), scale) else {
        return;
    };
    let fit = ui_scale_for(window.width(), window.height());
    if (scale.0 - fit).abs() > 1e-4 {
        scale.0 = fit;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_scale_fits_the_design_canvas() {
        assert_eq!(ui_scale_for(1920.0, 1080.0), 1.0);
        assert!((ui_scale_for(1280.0, 720.0) - 2.0 / 3.0).abs() < 1e-6);
        // 16:10 and ultrawide: limited by the shorter side, the canvas grows along the other.
        assert!((ui_scale_for(1280.0, 800.0) - 2.0 / 3.0).abs() < 1e-6);
        assert_eq!(ui_scale_for(2560.0, 1080.0), 1.0);
        assert_eq!(ui_scale_for(0.0, 720.0), 1.0, "minimised window");
    }
}
