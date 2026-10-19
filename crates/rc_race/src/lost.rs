//! The "lost" warning (GDD §7.3): a racer outside the legal corridor, or moving without getting
//! anywhere along the track (driving sideways or circling on an open floor), is told so and the
//! HUD points to `RaceProgress::rejoin`. Driving backwards is the wrong-way warning's job, so
//! `progress.rs` keeps the two apart. Pure rule with hysteresis, one step per racing tick.

/// Outside the corridor this long turns the warning on (ticks, 0.5 s)…
const OUTSIDE_TICKS: u32 = 30;
/// …and so does moving faster than this (u/s)…
const MOVING_UPS: f32 = 5.0;
/// …with less than this share of the motion going along the track…
const STALLED_SHARE: f32 = 0.3;
/// …for this long (ticks, 2 s).
const STALLED_TICKS: u32 = 120;
/// Back inside the corridor with at least this share of the motion along the track (or slower
/// than [`MOVING_UPS`])…
const FOUND_SHARE: f32 = 0.6;
/// …for this long turns it off (ticks, 0.5 s).
const FOUND_TICKS: u32 = 30;
/// The HUD points at the centreline this far ahead of the car's projection (u).
pub(crate) const REJOIN_AHEAD_U: f32 = 10.0;

/// The lost warning's state.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct Lost {
    on: bool,
    outside: u32,
    stalled: u32,
    found: u32,
}

impl Lost {
    /// One tick: the car is `outside` the legal corridor and moves at `speed` (u/s), of which
    /// `track_speed` along the track (negative = backwards). Returns whether it is lost.
    pub(crate) fn step(&mut self, outside: bool, speed: f32, track_speed: f32) -> bool {
        let moving = speed > MOVING_UPS;
        self.outside = if outside { self.outside + 1 } else { 0 };
        let stalled = moving && track_speed < STALLED_SHARE * speed;
        self.stalled = if stalled { self.stalled + 1 } else { 0 };
        if self.on {
            let found = !outside && !(moving && track_speed < FOUND_SHARE * speed);
            self.found = if found { self.found + 1 } else { 0 };
            if self.found >= FOUND_TICKS {
                *self = Self::default();
            }
        } else if self.outside >= OUTSIDE_TICKS || self.stalled >= STALLED_TICKS {
            (self.on, self.found) = (true, 0);
        }
        self.on
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leaving_the_corridor_warns_after_half_a_second() {
        let mut lost = Lost::default();
        assert!(
            (0..OUTSIDE_TICKS - 1).all(|_| !lost.step(true, 20.0, 20.0)),
            "a quick excursion stays quiet"
        );
        assert!(lost.step(true, 20.0, 20.0), "on after 0.5 s outside");
        assert!(lost.step(false, 20.0, 20.0), "on until found for a while");
        assert!((0..FOUND_TICKS - 2).all(|_| lost.step(false, 20.0, 20.0)));
        assert!(
            !lost.step(false, 20.0, 20.0),
            "off after 0.5 s back on track"
        );
    }

    #[test]
    fn driving_sideways_on_an_open_floor_warns_after_two_seconds() {
        let mut lost = Lost::default();
        assert!(
            (0..STALLED_TICKS - 1).all(|_| !lost.step(false, 20.0, 3.0)),
            "inside the corridor, getting nowhere"
        );
        assert!(lost.step(false, 20.0, 3.0));
        // Turning back onto the track: progress below the exit share keeps it on.
        assert!((0..200).all(|_| lost.step(false, 20.0, 10.0)));
        assert!((0..FOUND_TICKS - 1).all(|_| lost.step(false, 20.0, 18.0)));
        assert!(!lost.step(false, 20.0, 18.0), "found again");
    }

    #[test]
    fn slow_cars_and_corners_never_count_as_lost() {
        let mut lost = Lost::default();
        assert!(
            (0..600).all(|_| !lost.step(false, 3.0, 0.0)),
            "creeping or stopped"
        );
        assert!(
            (0..600).all(|_| !lost.step(false, 20.0, 14.0)),
            "a drift through a hairpin still makes progress"
        );
    }
}
