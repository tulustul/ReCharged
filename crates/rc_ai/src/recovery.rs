//! Stuck detection and recovery (pure state machine, GDD §8.1 "reverses out when stuck against a
//! wall; uses reset if still stuck; never blocks the race for more than a few seconds").

use crate::data::Recovery as Tuning;

/// A reset tap (flip in place) is held this long (s); rc_race respawns from a 0.4 s hold.
const TAP_SECS: f32 = 0.15;
/// After a manoeuvre the detectors wait this long before judging again (s).
const GRACE_SECS: f32 = 1.0;
/// Below this speed (u/s) a car on its roof counts as resting there.
const FLIPPED_MAX_UPS: f32 = 2.0;
/// Boxed in right behind a stopped car with the way round open, the car backs off this long (s)
/// to get room to pull out — once until it makes progress again (wedged at a wall it would back
/// off forever); not a stuck try (the next one still reverses before it resets).
const BACK_OFF_SECS: f32 = 0.6;
/// Less progress than this (u) over the long window (s) means the car is trapped (under a ramp,
/// wedged between props): hold reset whatever the reverse tries say.
const TRAPPED_PROGRESS_U: f32 = 12.0;
const TRAPPED_WINDOW_SECS: f32 = 7.0;

/// What the car sees this tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sense {
    /// |forward speed| (u/s).
    pub speed: f32,
    /// The AI asked for throttle last tick.
    pub throttle: bool,
    /// Progress along the track this tick (u, + = forward).
    pub progress: f32,
    /// Outside the legal corridor or fallen below the track.
    pub off_course: bool,
    /// On its side or roof.
    pub flipped: bool,
    /// No drive power (EMP, Storm Strike, a stalled start): it cannot move, so it is not stuck.
    pub powerless: bool,
    /// Too close behind a stopped car to pull out round it, with room behind: back off first.
    pub boxed: bool,
}

/// What recovery wants the car to do.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    /// Normal driving.
    Drive,
    /// Reverse out (the caller steers the nose back toward the track).
    Reverse,
    /// Hold the reset button.
    HoldReset,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum State {
    Driving,
    Reversing(f32),
    /// Reversing to make room behind a stopped car (not a stuck try).
    BackingOff(f32),
    Resetting(f32),
}

/// Recovery state of one AI car.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Recovery {
    state: State,
    slow: f32,
    off: f32,
    flipped: f32,
    window: f32,
    window_progress: f32,
    grace: f32,
    tries: u32,
    /// Backed off from a stopped car since the last progress window with progress.
    backed_off: bool,
    long_window: f32,
    long_progress: f32,
}

impl Default for Recovery {
    fn default() -> Self {
        Self {
            state: State::Driving,
            slow: 0.0,
            off: 0.0,
            flipped: 0.0,
            window: 0.0,
            window_progress: 0.0,
            grace: GRACE_SECS,
            tries: 0,
            backed_off: false,
            long_window: 0.0,
            long_progress: 0.0,
        }
    }
}

impl Recovery {
    /// After a respawn: start over.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Advances by `dt` with what the car senses; returns what to do this tick.
    pub fn step(&mut self, t: &Tuning, sense: Sense, dt: f32) -> Action {
        if sense.powerless {
            // Progress cannot be judged while the car has no drive: the windows start over.
            (self.window, self.window_progress) = (0.0, 0.0);
            (self.long_window, self.long_progress) = (0.0, 0.0);
        }
        self.long_window += dt;
        self.long_progress += sense.progress;
        if self.long_window >= TRAPPED_WINDOW_SECS {
            let trapped = self.long_progress < TRAPPED_PROGRESS_U;
            self.long_window = 0.0;
            self.long_progress = 0.0;
            if trapped && !matches!(self.state, State::Resetting(_)) {
                self.state = State::Resetting(t.reset_hold_secs);
                return Action::HoldReset;
            }
        }
        match self.state {
            State::Reversing(left) => {
                self.state = match left - dt {
                    l if l > 0.0 => State::Reversing(l),
                    _ => self.back_to_driving(),
                };
                return Action::Reverse;
            }
            State::BackingOff(left) => {
                self.state = match left - dt {
                    l if l > 0.0 => State::BackingOff(l),
                    _ => self.back_to_driving(),
                };
                return Action::Reverse;
            }
            State::Resetting(left) => {
                self.state = match left - dt {
                    l if l > 0.0 => State::Resetting(l),
                    _ => self.back_to_driving(),
                };
                return Action::HoldReset;
            }
            State::Driving => {}
        }
        if sense.boxed && !sense.powerless && !self.backed_off {
            self.backed_off = true;
            self.state = State::BackingOff(BACK_OFF_SECS);
            return Action::Reverse;
        }
        self.grace = (self.grace - dt).max(0.0);
        self.window += dt;
        self.window_progress += sense.progress;
        let stalled = self.grace <= 0.0
            && self.window >= t.progress_secs
            && self.window_progress < t.progress_u;
        if self.window >= t.progress_secs {
            if self.window_progress >= t.progress_u {
                self.tries = 0;
                self.backed_off = false;
            }
            self.window = 0.0;
            self.window_progress = 0.0;
        }
        // Nothing is judged during the grace period after a start or a manoeuvre.
        let judge = self.grace <= 0.0;
        let count =
            |timer: &mut f32, on: bool| *timer = if on && judge { *timer + dt } else { 0.0 };
        count(
            &mut self.slow,
            sense.throttle && !sense.powerless && sense.speed < t.stuck_ups,
        );
        count(&mut self.off, sense.off_course);
        count(
            &mut self.flipped,
            sense.flipped && sense.speed < FLIPPED_MAX_UPS,
        );
        if !judge {
            return Action::Drive;
        }
        if self.flipped >= t.flipped_secs {
            self.state = State::Resetting(TAP_SECS);
            return Action::HoldReset;
        }
        if self.off >= t.off_course_secs {
            self.state = State::Resetting(t.reset_hold_secs);
            return Action::HoldReset;
        }
        if self.slow >= t.stuck_secs || stalled {
            self.tries += 1;
            self.state = match self.tries > t.reset_after_tries {
                true => State::Resetting(t.reset_hold_secs),
                false => State::Reversing(t.reverse_secs),
            };
            return self.step(t, sense, 0.0);
        }
        Action::Drive
    }

    fn back_to_driving(&mut self) -> State {
        self.slow = 0.0;
        self.off = 0.0;
        self.flipped = 0.0;
        self.window = 0.0;
        self.window_progress = 0.0;
        self.grace = GRACE_SECS;
        State::Driving
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    fn tuning() -> Tuning {
        Tuning {
            stuck_ups: 1.5,
            stuck_secs: 0.8,
            progress_u: 3.0,
            progress_secs: 2.5,
            reverse_secs: 1.2,
            reset_after_tries: 2,
            reset_hold_secs: 0.6,
            off_course_secs: 1.5,
            flipped_secs: 0.5,
        }
    }

    fn driving() -> Sense {
        Sense {
            speed: 20.0,
            throttle: true,
            progress: 20.0 * DT,
            off_course: false,
            flipped: false,
            powerless: false,
            boxed: false,
        }
    }

    fn stuck() -> Sense {
        Sense {
            speed: 0.2,
            progress: 0.0,
            ..driving()
        }
    }

    fn run(r: &mut Recovery, sense: Sense, secs: f32) -> Vec<Action> {
        (0..(secs / DT).round() as usize)
            .map(|_| r.step(&tuning(), sense, DT))
            .collect()
    }

    #[test]
    fn normal_driving_never_recovers() {
        let mut r = Recovery::default();
        assert!(
            run(&mut r, driving(), 20.0)
                .iter()
                .all(|a| *a == Action::Drive)
        );
    }

    #[test]
    fn stuck_against_a_wall_reverses_then_resets() {
        let mut r = Recovery::default();
        let actions = run(&mut r, stuck(), 12.0);
        let first_reverse = actions.iter().position(|a| *a == Action::Reverse);
        let first_reset = actions.iter().position(|a| *a == Action::HoldReset);
        let (rev, reset) = (
            first_reverse.expect("reverses"),
            first_reset.expect("resets"),
        );
        // Grace (1 s) + stuck (0.8 s) before the first reverse.
        assert!(
            (rev as f32 * DT - 1.8).abs() < 0.05,
            "reverse at {}",
            rev as f32 * DT
        );
        assert!(reset > rev, "reverses before resetting");
        assert!(reset as f32 * DT < 9.0, "reset within a few seconds");
        let held = actions[reset..]
            .iter()
            .take_while(|a| **a == Action::HoldReset)
            .count();
        assert!(held as f32 * DT > 0.4, "held long enough for a respawn");
    }

    #[test]
    fn a_car_without_drive_power_is_not_stuck() {
        let mut r = Recovery::default();
        let powerless = Sense {
            powerless: true,
            ..stuck()
        };
        // An EMP (2 s) or Storm Strike stopped it in a slow corner: it waits, then drives on.
        assert!(
            run(&mut r, powerless, 3.0)
                .iter()
                .all(|a| *a == Action::Drive)
        );
        let actions = run(&mut r, stuck(), 3.0);
        let first = actions
            .iter()
            .position(|a| *a != Action::Drive)
            .expect("stuck once the power is back");
        assert!(
            (first as f32 * DT - 0.8).abs() < 0.05,
            "judged from the moment the power returned: {}",
            first as f32 * DT
        );
    }

    #[test]
    fn a_car_boxed_in_behind_a_stopped_car_backs_off_at_once_once() {
        let mut r = Recovery::default();
        let boxed = Sense {
            boxed: true,
            ..stuck()
        };
        assert!(
            run(&mut r, boxed, 0.5)
                .iter()
                .all(|a| *a == Action::Reverse),
            "no grace"
        );
        assert_eq!(r.tries, 0, "making room is not a stuck try");
        let after = run(&mut r, driving(), 1.0);
        assert_eq!(after[..5], [Action::Reverse; 5], "the back-off runs out");
        assert_eq!(after.last(), Some(&Action::Drive));
        // Boxed again without progress (wedged at a wall): no endless backing off — the stuck
        // detectors take over (reverse, then reset).
        let mut r = Recovery::default();
        let actions = run(&mut r, boxed, 8.0);
        let backing = actions
            .iter()
            .take_while(|a| **a == Action::Reverse)
            .count();
        assert!(
            (36..=37).contains(&backing),
            "one 0.6 s back-off: {backing} ticks"
        );
        assert!(
            actions.contains(&Action::HoldReset),
            "then reverse and reset"
        );
    }

    #[test]
    fn progress_clears_the_tries() {
        let mut r = Recovery::default();
        run(&mut r, stuck(), 2.0);
        assert_eq!(r.tries, 1);
        run(&mut r, driving(), 6.0);
        assert_eq!(r.tries, 0, "a good progress window forgives earlier tries");
    }

    #[test]
    fn upside_down_taps_reset() {
        let mut r = Recovery::default();
        let flipped = Sense {
            flipped: true,
            speed: 0.5,
            throttle: false,
            ..driving()
        };
        let actions = run(&mut r, flipped, 2.0);
        let held = actions.iter().filter(|a| **a == Action::HoldReset).count();
        assert!(
            held > 0 && (held as f32 * DT) < 0.4,
            "a tap, not a respawn hold"
        );
    }

    #[test]
    fn off_course_holds_reset() {
        let mut r = Recovery::default();
        let off = Sense {
            off_course: true,
            ..driving()
        };
        let actions = run(&mut r, off, 3.0);
        let first = actions
            .iter()
            .position(|a| *a == Action::HoldReset)
            .expect("resets");
        assert!(
            (first as f32 * DT - 2.5).abs() < 0.05,
            "grace 1 s + 1.5 s off course"
        );
    }

    #[test]
    fn a_trapped_car_resets_even_when_it_wiggles() {
        let mut r = Recovery::default();
        // Back and forth: every progress window sees some forward motion, the long one not.
        let mut resets = 0;
        for i in 0..(10.0 / DT) as usize {
            let forward = (i / 60) % 2 == 0;
            let sense = Sense {
                speed: 3.0,
                throttle: true,
                progress: if forward { 3.0 * DT } else { -2.5 * DT },
                ..driving()
            };
            if r.step(&tuning(), sense, DT) == Action::HoldReset {
                resets += 1;
            }
        }
        assert!(resets > 0, "trapped: reset");
    }

    #[test]
    fn respawn_starts_over() {
        let mut r = Recovery::default();
        run(&mut r, stuck(), 2.0);
        r.reset();
        assert_eq!(r, Recovery::default());
    }
}
