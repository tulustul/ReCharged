//! # rc_input — devices, bindings and intents (L1)
//!
//! - **Responsibilities:** devices (`KeyboardA`, `KeyboardB`, gamepads) and bindings from
//!   `Settings.controls`; `LocalPlayers` (single player takes every device; lobby join/leave);
//!   PreUpdate intents (analog + latched edges, digital steering ramp, dead zone, sensitivity,
//!   auto-accelerate, reset level); writing `CarControl` in `SimSet::Drivers`; menu navigation
//!   (`UiNav`); pause requests; rebind capture; rumble; `--input-script`.
//! - **Inputs:** Bevy keyboard/gamepad state (PreUpdate only), `Settings`, `LobbyState`,
//!   `PauseState`, `RacePhase`, `RebindRequest`, `CarImpact` / `HitEvent` / `CarLanded`,
//!   `AutomationConfig`.
//! - **Outputs:** `CarControl` of `LocalPlayer` racers without `AiDriver`, `LocalPlayers`, `UiNav`,
//!   `PauseRequest`, `DeviceEvent`, `RebindResult`, gamepad rumble.
//! - **Must never:** read device state in `FixedUpdate` (only latched intents), latch driving
//!   edges while paused or in a menu, toggle pause while paused, or write `Settings`.
//!
//! Status: skeleton (T01); implemented by T04 (`docs/tasks/T04-input-camera.md`).

use bevy::prelude::*;

/// Input devices → player intents and menu navigation.
pub struct GameInputPlugin;

impl Plugin for GameInputPlugin {
    fn build(&self, _app: &mut App) {}
}
