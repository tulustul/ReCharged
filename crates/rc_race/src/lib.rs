//! # rc_race — race rules, the referee (L2)
//!
//! - **Responsibilities:** attempt entities in `OnEnter(RaceAttempt)` (racers on the track's grid
//!   slots, `GoldenBattery`, the Time Trial `GhostCar`); the phases Setup → Intro → Countdown →
//!   Racing → Finished (in fixed ticks; perfect start / stall); progress with continuity hints,
//!   ordered checkpoint gates, laps in ticks, standings, wrong way, the soft-lock guard and
//!   projected times; the **sole decider of resets and respawns** (`RespawnCar` + ghosting);
//!   the Golden Battery pickup; ghost recording (20 Hz) and playback; onboarding hints;
//!   `RaceResults` when the race ends. Rules and events: `docs/design/rc_race.md`.
//! - **Inputs:** `RaceConfig` (re-read every attempt), `Track` (rc_track), `TrackSummary`,
//!   `CarControl` (`throttle`, `reset_held`), `VehicleTelemetry`, `Charge`, `ItemSlot`, racer
//!   `Transform`s, `Teleported`, `UiNav` (Confirm skips the intro), `PlayerDropped`, `Settings`,
//!   `HintsSeen`, the session lifecycle states.
//! - **Outputs:** racer entities (`Racer`, `LocalPlayer`, `AiDriver`), `RaceProgress`,
//!   `RaceStatus`, `Standings`, `RaceResults`, `RacePhase` (Intro onwards), `RaceEvent`,
//!   `RespawnCar`, `ApplyEffect` (ghosting, perfect start, stall), `Teleported` (spawns), the
//!   ghost car's `Transform`.
//! - **Must never:** move cars (it requests `RespawnCar`), read input devices, spawn attempt
//!   entities in `OnEnter(RacePhase::Setup)`, or keep race state past `OnEnter(Unloading)`.

mod finish;
mod ghost;
mod hints;
mod lost;
mod phases;
mod progress;
mod recovery;
mod spawn;
mod standings;

use bevy::prelude::*;
use rc_core::prelude::*;

/// Race rules and racer spawning. Needs `rc_track::TrackPlugin` (the `Track`).
pub struct RacePlugin;

impl Plugin for RacePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(RacePhase::Setup), phases::reset_attempt)
            .add_systems(OnEnter(RaceAttempt), spawn::spawn_attempt)
            .add_systems(OnEnter(RacePhase::Racing), phases::go)
            .add_systems(OnEnter(AppState::Unloading), remove_session)
            .add_systems(
                FixedUpdate,
                (
                    phases::drive_phases,
                    phases::track_launches.run_if(in_state(RacePhase::Countdown)),
                    (progress::update_progress, standings::update_standings)
                        .chain()
                        .run_if(live),
                    (
                        recovery::decide_respawns,
                        spawn::collect_battery,
                        hints::emit_hints,
                        ghost::record,
                        ghost::play,
                        finish::referee,
                    )
                        .chain()
                        .run_if(racing),
                )
                    .chain()
                    .run_if(resource_exists::<RaceStatus>)
                    .in_set(SimSet::Race),
            )
            .add_systems(Update, spawn::drop_players.in_set(FlowSet));
    }
}

/// Racers exist and the race is not decided: Intro, Countdown or Racing.
fn live(phase: Option<Res<State<RacePhase>>>, results: Option<Res<RaceResults>>) -> bool {
    use RacePhase::*;
    results.is_none() && phase.is_some_and(|p| matches!(p.get(), Intro | Countdown | Racing))
}

/// The race is on and not decided.
fn racing(phase: Option<Res<State<RacePhase>>>, results: Option<Res<RaceResults>>) -> bool {
    results.is_none() && phase.is_some_and(|p| *p.get() == RacePhase::Racing)
}

/// `OnEnter(AppState::Unloading)`: nothing of the session survives it.
fn remove_session(mut commands: Commands) {
    commands.remove_resource::<RaceStatus>();
    commands.remove_resource::<Standings>();
    commands.remove_resource::<RaceResults>();
    commands.remove_resource::<ghost::GhostRecorder>();
    commands.remove_resource::<hints::HintsShown>();
}
