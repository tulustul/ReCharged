//! # rc_input — devices, bindings and intents (L1)
//!
//! - **Responsibilities:** devices (`KeyboardA`, `KeyboardB`, gamepads) and bindings from
//!   `Settings.controls`; `LocalPlayers` (single player takes every device; Versus lobby
//!   join/leave; auto-assignment for multi-player sessions without a lobby; disconnect and
//!   takeover; last-used device for glyphs); PreUpdate intents (analog + latched edges, dead zone,
//!   sensitivity, auto-accelerate after GO, reset level) and `CarControl` in `SimSet::Drivers`
//!   (digital steering ramp per tick, each edge consumed by exactly one tick); menu navigation
//!   (`UiNav`, GDD §12.2); pause requests; camera cycling; rebind capture; rumble.
//! - **Inputs:** Bevy keyboard/gamepad state (PreUpdate only), `Settings`, `LobbyState`,
//!   `RaceConfig` (session humans), `AppState` / `RacePhase` / `PauseState`, `InputFocus`,
//!   `RebindRequest`, `CarImpact` / `HitEvent` / `CarLanded`, `VehicleTelemetry::boosting`.
//! - **Outputs:** `CarControl` of `LocalPlayer` racers without `AiDriver`, `LocalPlayers`, `UiNav`,
//!   `PauseRequest`, `CycleCamera`, `DeviceEvent`, `RebindResult`, gamepad rumble requests.
//! - **Must never:** read device state in `FixedUpdate` (only latched intents), latch driving
//!   edges while paused or in a menu, request a pause while paused, or write `Settings`.

mod bindings;
mod intents;
mod nav;
mod players;
mod rebind;
mod rumble;

use bevy::input::gamepad::GamepadRumbleRequest;
use bevy::prelude::*;
use rc_core::prelude::*;

/// Input devices → player intents and menu navigation.
pub struct GameInputPlugin;

impl Plugin for GameInputPlugin {
    fn build(&self, app: &mut App) {
        // Also registered by rc_camera (and Bevy's InputPlugin for rumble, absent in headless
        // apps); registering twice is a no-op.
        app.add_message::<CycleCamera>()
            .add_message::<GamepadRumbleRequest>()
            .init_resource::<players::Roster>()
            .init_resource::<LocalPlayers>()
            .init_resource::<intents::Intents>()
            .init_resource::<rebind::RebindCapture>()
            .add_systems(
                PreUpdate,
                (
                    players::track_devices,
                    rebind::capture,
                    nav::navigate,
                    intents::collect,
                    players::publish,
                    rumble::rumble,
                )
                    .chain()
                    .in_set(InputSet::Collect),
            )
            .add_systems(FixedUpdate, intents::drive_cars.in_set(SimSet::Drivers))
            .add_systems(OnEnter(PauseState::Running), intents::clear_edges)
            .add_systems(OnEnter(RacePhase::Setup), intents::reset)
            .add_systems(OnEnter(AppState::Loading), players::reconcile_session)
            .add_systems(OnEnter(PauseState::Paused), rumble::stop_all)
            .add_systems(OnEnter(AppState::Unloading), rumble::stop_all);
    }
}
