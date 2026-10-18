//! # rc_ui — the bevy-react bridge (F2)
//!
//! - **Responsibilities:** every typed React channel, one module per namespace (`app`, `race`,
//!   `ui`; later `menu`, `hud`, `lobby`, `settings`, `input`, `profile`), mapping React messages to
//!   `GameCommand`s and rc_core messages and pushing state to React; [`register_bindings`], shared
//!   by the app and the TypeScript exporter ([`export_bindings`]); the `ui.ready` watchdog.
//! - **Inputs:** React messages/requests, `AppState` / `RacePhase` / `PauseState`,
//!   `AutomationConfig`.
//! - **Outputs:** React events (`app.state`), `GameCommand`s, [`UiStatus`].
//! - **Must never:** send events per frame (state changes and ≤ 10 Hz batches only), leave a
//!   request unanswered, or hold gameplay logic.
//!
//! Regenerate `ui/src/bevy.ts` after changing any channel:
//! `cargo run -p rc_ui --example export_bindings -- ui/src/bevy.ts`.

pub mod app;
pub mod race;
pub mod ui;

use std::path::Path;

use bevy::prelude::*;
use bevy_react::prelude::*;
use rc_core::prelude::*;

pub use ui::{READY_TIMEOUT_SECS, UiStatus};

/// The React bridge. Add after `ReactPlugins` and `GameFlowPlugin`.
pub struct GameUiPlugin;

impl Plugin for GameUiPlugin {
    fn build(&self, app: &mut App) {
        register_bindings(app);
        app.init_resource::<UiStatus>()
            .add_systems(Update, (app::push_state, ui::watchdog).in_set(UiSet));
    }
}

/// Registers every channel; the app and [`export_bindings`] call exactly this.
pub fn register_bindings(app: &mut App) {
    app::register(app);
    race::register(app);
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
