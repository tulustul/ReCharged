//! System sets (ARCHITECTURE §3.2). [`crate::CorePlugin`] configures and chains them once; crates
//! only place their systems in them.
//!
//! ```text
//! PreUpdate    InputSet::Collect
//! FixedUpdate  SimSet::{Drivers → Assist → Effects → Items → Vehicle → [rapier] → PostPhysics → Race → Weather}
//!              (every SimSet runs only while PauseState::Running, i.e. only in a running race)
//! FixedLast    CoreSet::Snapshot
//! Update       FlowSet → InterpolateSet → CameraSet → VisualSet → AudioSet → UiSet
//! ```
//! Rapier's `PhysicsSet`s sit between `SimSet::Vehicle` and `SimSet::PostPhysics`; that edge is
//! configured where rapier is added (`rc_vehicle::PhysicsPlugin`), because rc_core does not depend
//! on rapier.

use bevy::input::InputSystems;
use bevy::prelude::*;

use crate::state::PauseState;

/// `PreUpdate`: device input → latched intents, menu navigation, pause requests (rc_input).
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputSet {
    /// Reads Bevy's input state (after `InputSystems`) and produces intents/`UiNav`.
    Collect,
}

/// `FixedUpdate` simulation stages, chained in declaration order and gated on
/// `PauseState::Running`.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SimSet {
    /// rc_input writes `CarControl` of local human racers; rc_ai of AI racers.
    Drivers,
    /// rc_ai steering assist: may change only `CarControl::steer` of assisted local racers.
    Assist,
    /// rc_items: `ApplyEffect` → `StatusEffects`, effect timers, shield pops.
    Effects,
    /// rc_items: roulette, item use, projectiles, hazards, pickups.
    Items,
    /// rc_vehicle: suspension, tyres, drive, boost/charge, `RespawnCar` execution. Before rapier.
    Vehicle,
    /// After rapier's writeback: telemetry, impacts, landings (vehicle); hit detection (items).
    PostPhysics,
    /// rc_race: progress, laps, standings, reset/respawn decisions, finish.
    Race,
    /// rc_weather: weather plan and state.
    Weather,
}

/// `FixedLast`: rc_core's own generic systems.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CoreSet {
    /// `PoseHistory` snapshot of every `Interpolated` entity (see [`crate::interp`]).
    Snapshot,
}

/// `Update` 1/6 — rc_game: transitions, loading gate, results, persistence.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FlowSet;

/// `Update` 2/6 — rc_render: poses visual proxies from `PoseHistory` and the overstep fraction.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InterpolateSet;

/// `Update` 3/6 — rc_camera.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CameraSet;

/// `Update` 4/6 — rc_render / rc_vfx gameplay visuals.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VisualSet;

/// `Update` 5/6 — rc_audio.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AudioSet;

/// `Update` 6/6 — rc_ui: forwards events to React, HUD pushes, shared values.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UiSet;

/// All simulation sets in tick order.
pub const SIM_SETS: [SimSet; 8] = [
    SimSet::Drivers,
    SimSet::Assist,
    SimSet::Effects,
    SimSet::Items,
    SimSet::Vehicle,
    SimSet::PostPhysics,
    SimSet::Race,
    SimSet::Weather,
];

/// Configures and chains every set (called by [`crate::CorePlugin`]).
pub(crate) fn configure_sets(app: &mut App) {
    app.configure_sets(PreUpdate, InputSet::Collect.after(InputSystems))
        .configure_sets(
            FixedUpdate,
            (
                SimSet::Drivers,
                SimSet::Assist,
                SimSet::Effects,
                SimSet::Items,
                SimSet::Vehicle,
                SimSet::PostPhysics,
                SimSet::Race,
                SimSet::Weather,
            )
                .chain(),
        )
        .configure_sets(FixedLast, CoreSet::Snapshot)
        .configure_sets(
            Update,
            (
                FlowSet,
                InterpolateSet,
                CameraSet,
                VisualSet,
                AudioSet,
                UiSet,
            )
                .chain(),
        );
    // Pause gate: a missing PauseState (outside a race) also means "not running".
    for set in SIM_SETS {
        app.configure_sets(FixedUpdate, set.run_if(in_state(PauseState::Running)));
    }
}
