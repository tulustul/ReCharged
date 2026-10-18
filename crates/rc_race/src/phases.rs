//! Attempt reset and the phase driver (GDD §3.2): Setup → Intro (first attempt of a session,
//! 3 s, Confirm skips) → Countdown (3-2-1, 1 s each) → GO → Racing. Racing → Finished is the
//! referee's (`finish.rs`). Phases count fixed ticks in `SimSet::Race`, so pausing freezes them.
//!
//! Start timing (GDD §3.2): throttle pressed ≤ 0.3 s before GO → perfect start (1 s boost);
//! held since more than 1 s before GO → 0.6 s stall. AI racers follow the same rule (rc_ai times
//! their throttle).

use bevy::prelude::*;
use rc_core::prelude::*;

use crate::ghost::GhostRecorder;
use crate::hints::HintsShown;

/// Intro fly-by length (ticks, 3 s).
const INTRO_TICKS: u32 = 180;
/// The countdown starts at this number (`COUNTDOWN_TICKS` lives in rc_core: rc_ai times its
/// starts against it).
const COUNT_FROM: u8 = 3;
/// Each countdown number shows this long (ticks, 1 s).
const COUNT_STEP_TICKS: u32 = COUNTDOWN_TICKS / COUNT_FROM as u32;
/// Pressed at most this long before GO: perfect start (ticks, 0.3 s).
const PERFECT_WINDOW_TICKS: u32 = 18;
/// Held since more than this long before GO: stall (ticks, 1.0 s).
const STALL_EARLY_TICKS: u32 = 60;
/// Throttle above this counts as pressed.
const THROTTLE_PRESSED: f32 = 0.5;
/// Perfect start boost (GDD §5.5).
const PERFECT_START: Effect = Effect::Boost {
    top_speed: 1.30,
    force: 1.50,
    secs: 1.0,
    freeze_charge: false,
};
/// Wheel-spin stall: no drive for 0.6 s.
const STALL: Effect = Effect::PowerLoss {
    mul: 0.0,
    secs: 0.6,
};

/// Countdown throttle timing of a racer: the countdown tick its current press began.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Launch(Option<u32>);

/// How a racer left the grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Start {
    Perfect,
    Stall,
    Normal,
}

/// The start of a racer whose throttle press began at countdown tick `pressed_at` (still held)
/// when GO came after `go_at` countdown ticks.
pub(crate) fn start_timing(pressed_at: Option<u32>, go_at: u32) -> Start {
    match pressed_at.map(|p| go_at.saturating_sub(p)) {
        Some(early) if early <= PERFECT_WINDOW_TICKS => Start::Perfect,
        Some(early) if early > STALL_EARLY_TICKS => Start::Stall,
        _ => Start::Normal,
    }
}

/// The phase after `phase` once it has run `ticks` ticks (this one included).
pub(crate) fn next_phase(
    phase: RacePhase,
    ticks: u32,
    first_attempt: bool,
    skip_intro: bool,
) -> Option<RacePhase> {
    match phase {
        RacePhase::Setup if first_attempt => Some(RacePhase::Intro),
        RacePhase::Setup => Some(RacePhase::Countdown),
        RacePhase::Intro if skip_intro || ticks >= INTRO_TICKS => Some(RacePhase::Countdown),
        RacePhase::Countdown if ticks >= COUNTDOWN_TICKS => Some(RacePhase::Racing),
        _ => None,
    }
}

/// The countdown number showing in countdown tick `ticks` (1-based), and whether it just
/// appeared.
pub(crate) fn countdown_number(ticks: u32) -> (u8, bool) {
    if !(1..=COUNTDOWN_TICKS).contains(&ticks) {
        return (0, false);
    }
    let t = ticks - 1;
    (
        COUNT_FROM - (t / COUNT_STEP_TICKS) as u8,
        t.is_multiple_of(COUNT_STEP_TICKS),
    )
}

/// `OnEnter(RacePhase::Setup)`: fresh attempt resources (the attempt counter goes on).
pub(crate) fn reset_attempt(mut commands: Commands, old: Option<Res<RaceStatus>>) {
    let attempt = old.map_or(0, |s| s.attempt) + 1;
    commands.insert_resource(RaceStatus {
        attempt,
        ..default()
    });
    commands.insert_resource(Standings::default());
    commands.insert_resource(GhostRecorder::default());
    commands.remove_resource::<RaceResults>();
    commands.init_resource::<HintsShown>(); // session-long
}

/// `SimSet::Race`: phase clock, intro skip, countdown, race clock.
pub(crate) fn drive_phases(
    phase: Res<State<RacePhase>>,
    mut status: ResMut<RaceStatus>,
    results: Option<Res<RaceResults>>,
    mut nav: MessageReader<UiNav>,
    mut next: ResMut<NextState<RacePhase>>,
    mut events: MessageWriter<RaceEvent>,
) {
    status.phase_ticks = if phase.is_changed() {
        1
    } else {
        status.phase_ticks + 1
    };
    let skip = nav.read().any(|n| n.action == NavAction::Confirm);
    let (n, fresh) = match phase.get() {
        RacePhase::Countdown => countdown_number(status.phase_ticks),
        _ => (0, false),
    };
    status.countdown = n;
    if fresh {
        events.write(RaceEvent {
            racer: None,
            kind: RaceEventKind::CountdownTick(n),
        });
    }
    if *phase.get() == RacePhase::Racing && results.is_none() {
        status.race_ticks += 1;
    }
    let first = status.attempt <= 1;
    if let Some(to) = next_phase(*phase.get(), status.phase_ticks, first, skip) {
        next.set_if_different(to);
    }
}

/// `SimSet::Race` (Countdown, after the driver): when each racer's throttle press began.
pub(crate) fn track_launches(
    status: Res<RaceStatus>,
    mut racers: Query<(&CarControl, &mut Launch)>,
) {
    for (control, mut launch) in &mut racers {
        let pressed = control.throttle > THROTTLE_PRESSED;
        let since = match (pressed, launch.0) {
            (true, None) => Some(status.phase_ticks),
            (true, since) => since,
            (false, _) => None,
        };
        launch.set_if_neq(Launch(since));
    }
}

/// `OnEnter(RacePhase::Racing)`: GO, perfect starts and stalls.
pub(crate) fn go(
    status: Option<Res<RaceStatus>>,
    racers: Query<(Entity, &Launch)>,
    mut effects: MessageWriter<ApplyEffect>,
    mut events: MessageWriter<RaceEvent>,
) {
    events.write(RaceEvent {
        racer: None,
        kind: RaceEventKind::Go,
    });
    let go_at = status.map_or(0, |s| s.phase_ticks);
    for (car, launch) in &racers {
        let (effect, kind) = match start_timing(launch.0, go_at) {
            Start::Perfect => (PERFECT_START, RaceEventKind::PerfectStart),
            Start::Stall => (STALL, RaceEventKind::Stall),
            Start::Normal => continue,
        };
        effects.write(ApplyEffect {
            car,
            effect,
            source: None,
        });
        events.write(RaceEvent {
            racer: Some(car),
            kind,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_windows_follow_the_gdd() {
        let go = COUNTDOWN_TICKS;
        assert_eq!(start_timing(None, go), Start::Normal, "no throttle");
        assert_eq!(start_timing(Some(go), go), Start::Perfect, "last tick");
        assert_eq!(
            start_timing(Some(go - 18), go),
            Start::Perfect,
            "0.3 s before"
        );
        assert_eq!(start_timing(Some(go - 19), go), Start::Normal);
        assert_eq!(
            start_timing(Some(go - 60), go),
            Start::Normal,
            "1.0 s before"
        );
        assert_eq!(
            start_timing(Some(go - 61), go),
            Start::Stall,
            "held too early"
        );
        assert_eq!(
            start_timing(Some(1), go),
            Start::Stall,
            "held all countdown"
        );
    }

    #[test]
    fn phases_run_intro_only_on_the_first_attempt() {
        use RacePhase::*;
        assert_eq!(next_phase(Setup, 1, true, false), Some(Intro));
        assert_eq!(next_phase(Setup, 1, false, false), Some(Countdown));
        assert_eq!(next_phase(Intro, INTRO_TICKS - 1, true, false), None);
        assert_eq!(next_phase(Intro, INTRO_TICKS, true, false), Some(Countdown));
        assert_eq!(
            next_phase(Intro, 5, true, true),
            Some(Countdown),
            "Confirm skips"
        );
        assert_eq!(next_phase(Countdown, COUNTDOWN_TICKS - 1, true, true), None);
        assert_eq!(
            next_phase(Countdown, COUNTDOWN_TICKS, true, false),
            Some(Racing)
        );
        assert_eq!(next_phase(Racing, 99_999, true, true), None);
    }

    #[test]
    fn countdown_shows_three_two_one() {
        let shown: Vec<(u32, u8)> = (1..=COUNTDOWN_TICKS)
            .filter_map(|t| match countdown_number(t) {
                (n, true) => Some((t, n)),
                _ => None,
            })
            .collect();
        assert_eq!(shown, [(1, 3), (61, 2), (121, 1)]);
        assert_eq!(countdown_number(60), (3, false));
        assert_eq!(countdown_number(COUNTDOWN_TICKS), (1, false));
        assert_eq!(countdown_number(COUNTDOWN_TICKS + 1), (0, false), "GO");
        assert_eq!(countdown_number(0), (0, false));
    }
}
