//! Placeholder race phases: Setup → Intro (first attempt only) → Countdown → Racing on timers.
// ponytail: placeholder until T06 — rc_race owns the phases (and counts fixed ticks); delete this
// module when rc_race drives them.

use bevy::prelude::*;
use rc_core::prelude::*;

/// Intro fly-by length in seconds.
const INTRO_SECS: f32 = 3.0;
/// Countdown length in seconds.
const COUNTDOWN_SECS: f32 = 3.0;

/// Per-session phase clock.
#[derive(Resource, Debug, Default)]
struct PhaseClock {
    /// Attempts started in this session (Intro only before the first).
    attempts: u32,
    /// Seconds spent running in the current phase.
    elapsed: f32,
}

pub(crate) fn build(app: &mut App) {
    app.add_systems(OnEnter(AppState::Loading), |mut c: Commands| {
        c.insert_resource(PhaseClock::default());
    })
    .add_systems(OnEnter(RaceAttempt), |clock: Option<ResMut<PhaseClock>>| {
        if let Some(mut clock) = clock {
            clock.attempts += 1;
        }
    })
    .add_systems(OnEnter(AppState::Unloading), |mut c: Commands| {
        c.remove_resource::<PhaseClock>();
    })
    .add_systems(
        Update,
        drive_phases
            .in_set(crate::Step::Phases)
            .run_if(in_state(PauseState::Running).and_then(resource_exists::<PhaseClock>)),
    );
}

fn drive_phases(
    phase: Res<State<RacePhase>>,
    time: Res<Time>,
    mut clock: ResMut<PhaseClock>,
    mut next: ResMut<NextState<RacePhase>>,
) {
    if phase.is_changed() {
        clock.elapsed = 0.0;
    } else {
        clock.elapsed += time.delta_secs();
    }
    let to = match phase.get() {
        RacePhase::Setup if clock.attempts == 0 => Some(RacePhase::Intro),
        RacePhase::Setup => Some(RacePhase::Countdown),
        RacePhase::Intro if clock.elapsed >= INTRO_SECS => Some(RacePhase::Countdown),
        RacePhase::Countdown if clock.elapsed >= COUNTDOWN_SECS => Some(RacePhase::Racing),
        _ => None,
    };
    if let Some(to) = to {
        next.set_if_different(to);
    }
}
