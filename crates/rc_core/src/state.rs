//! App states and the session lifecycle (ARCHITECTURE §3.1).
//!
//! ```text
//! Boot → Menu → Loading → InRace ─(restart: RacePhase::Setup)─┐
//!                  │         │  ▲───────────────────────────────┘
//!                  └─────────┴─→ Unloading → Menu | Loading
//! ```
//! - [`RaceSession`] exists while `Loading | InRace`: scope for session entities
//!   (`DespawnOnExit(RaceSession)`).
//! - [`RaceAttempt`] exists while `InRace` and the phase is not `Setup`: scope for attempt entities,
//!   which are spawned in `OnEnter(RaceAttempt)` (never in `OnEnter(RacePhase::Setup)`).
//! - Every exit from a session passes through the one-frame [`AppState::Unloading`].

use bevy::prelude::*;

/// Top-level application state. Written only by rc_game.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    /// Loading the catalog, settings and profile.
    #[default]
    Boot,
    /// Front-end menus (React screens over the showroom).
    Menu,
    /// A race session is being built; the loading gate opens when every registered loader is done.
    Loading,
    /// The race session is running (see [`RacePhase`]).
    InRace,
    /// One frame outside any session; crates remove their session resources in `OnEnter(Unloading)`.
    Unloading,
}

/// Computed state present while `AppState` is `Loading` or `InRace`.
///
/// Scope for session entities (track, colliders, static visuals, game cameras).
/// `ALLOW_SAME_STATE_TRANSITIONS = false`, otherwise `Loading → InRace` would fire an exit/enter
/// pair and despawn the whole session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RaceSession;

impl ComputedStates for RaceSession {
    type SourceStates = AppState;
    const ALLOW_SAME_STATE_TRANSITIONS: bool = false;

    fn compute(state: AppState) -> Option<Self> {
        matches!(state, AppState::Loading | AppState::InRace).then_some(Self)
    }
}

/// Phase of the current race attempt; sub-state of [`AppState::InRace`].
///
/// rc_game sets `Setup` (start and restart); the phase driver (rc_race) advances the rest.
#[derive(SubStates, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[source(AppState = AppState::InRace)]
pub enum RacePhase {
    /// Attempt resources are reset (`OnEnter(Setup)`); no attempt entities exist.
    #[default]
    Setup,
    /// Camera fly-by and track banner (first attempt of a session only).
    Intro,
    /// 3-2-1-GO; cars are held.
    Countdown,
    /// The race is on.
    Racing,
    /// Every human has finished (or the soft-lock guard expired).
    Finished,
}

/// Computed state present while `InRace` and the phase is not [`RacePhase::Setup`].
///
/// Scope for attempt entities (racers, pickups, projectiles, hazards, ghost car, golden battery).
/// Restart = back to `Setup`, which exits this state and despawns the attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RaceAttempt;

impl ComputedStates for RaceAttempt {
    type SourceStates = RacePhase;
    const ALLOW_SAME_STATE_TRANSITIONS: bool = false;

    fn compute(phase: RacePhase) -> Option<Self> {
        (phase != RacePhase::Setup).then_some(Self)
    }
}

/// Pause state of a race; sub-state of [`AppState::InRace`]. Written only by rc_game.
///
/// Pausing gates every `SimSet` (see [`crate::schedule`]); `Time<Virtual>` keeps running so the UI
/// keeps animating.
#[derive(SubStates, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[source(AppState = AppState::InRace)]
pub enum PauseState {
    /// The simulation ticks.
    #[default]
    Running,
    /// The simulation is frozen; the pause menu is open.
    Paused,
}

/// Registers every state. Requires `StatesPlugin` (part of `DefaultPlugins`).
pub(crate) fn register_states(app: &mut App) {
    app.init_state::<AppState>()
        .add_computed_state::<RaceSession>()
        .add_sub_state::<RacePhase>()
        .add_sub_state::<PauseState>()
        .add_computed_state::<RaceAttempt>();
}
