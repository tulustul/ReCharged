//! Lane choice (pure): overtaking slower cars and steering around hazards and stopped cars.
//! Everything is in track coordinates — `ahead` along the track, `lateral` across it (+ = right).

/// Something on or near the AI's line ahead.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Obstacle {
    /// Distance ahead along the track (u).
    pub ahead: f32,
    /// Lateral position of its centre (u).
    pub lateral: f32,
    /// Lateral half extent including the clearance to keep (u).
    pub half_width: f32,
    /// Its speed along the track (u/s; 0 for hazards).
    pub speed: f32,
    /// A car (followed when it cannot be passed) rather than a hazard.
    pub car: bool,
    /// A car standing still or crawling ([`crate::TrackView::stopped`]): steered around like a
    /// hazard, never followed.
    pub stopped: bool,
}

/// Outcome of [`choose_lane`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LaneChoice {
    /// Lateral position to drive at near the obstacle (u).
    pub lateral: f32,
    /// The car to follow because it cannot be passed (its speed and distance set the follow
    /// brake).
    pub follow: Option<Obstacle>,
    /// The lateral steers around a hazard or a stopped car (dodge at once, not as a gradual lane
    /// change).
    pub dodge: bool,
}

/// Where to drive given the planned `lateral`, the lateral the car is heading for (`current`:
/// last tick's target), its half width `my_half`, its speed and `obstacles` ahead; `bounds` =
/// (min, max) legal laterals. A blocking obstacle is passed on the side the car is already
/// heading for when there is room, else on the other side; a moving car that cannot be passed is
/// followed, a hazard or a stopped car is squeezed past on the roomier side (nobody queues
/// behind a car that missed GO). A side is free when no hazard anywhere ahead and no car near the
/// blocker is in the way (passing a car must not lead onto a Spring Pad the car hid) — except the
/// cars still behind a stopped blocker: they are queued behind it or passing it too, and the
/// rear-end limit keeps the car behind them in the lane it picks.
pub fn choose_lane(
    planned: f32,
    current: f32,
    my_half: f32,
    speed: f32,
    bounds: (f32, f32),
    obstacles: &[Obstacle],
) -> LaneChoice {
    let relevant = |o: &&Obstacle| o.ahead > 0.0 && (!o.car || o.stopped || speed > o.speed - 0.5);
    let blocks = |lat: f32, o: &Obstacle| (lat - o.lateral).abs() < o.half_width + my_half;
    let Some(blocker) = obstacles
        .iter()
        .filter(relevant)
        .filter(|o| blocks(planned, o))
        .min_by(|a, b| a.ahead.total_cmp(&b.ahead))
    else {
        return LaneChoice {
            lateral: planned,
            follow: None,
            dodge: false,
        };
    };
    let reach = blocker.half_width + my_half;
    let (left, right) = (blocker.lateral - reach, blocker.lateral + reach);
    let vetoes = |o: &&Obstacle| {
        !o.car
            || ((o.ahead - blocker.ahead).abs() < 2.0 * reach.max(2.0)
                && !(blocker.stopped && o.ahead < blocker.ahead))
    };
    let free = |lat: f32| {
        lat >= bounds.0
            && lat <= bounds.1
            && !obstacles
                .iter()
                .filter(relevant)
                .filter(vetoes)
                .any(|o| blocks(lat, o))
    };
    let sides = match current <= blocker.lateral {
        true => [left, right],
        false => [right, left],
    };
    let dodge = !blocker.car || blocker.stopped;
    match sides.into_iter().find(|&lat| free(lat)) {
        Some(lat) => LaneChoice {
            lateral: lat,
            follow: None,
            dodge,
        },
        None if !dodge => LaneChoice {
            lateral: planned,
            follow: Some(*blocker),
            dodge,
        },
        // No room around a hazard or a stopped car: squeeze past on the roomier side.
        None => LaneChoice {
            lateral: if bounds.1 - blocker.lateral > blocker.lateral - bounds.0 {
                right.min(bounds.1)
            } else {
                left.max(bounds.0)
            },
            follow: None,
            dodge,
        },
    }
}

/// Narrows the legal lateral range `bounds` so the car never squeezes into cars alongside
/// (`alongside` = their laterals): it keeps `gap` (centre to centre) from each, on the side
/// it is on. When a car is pinned on both sides it holds its lateral.
pub fn side_bounds(current: f32, bounds: (f32, f32), alongside: &[f32], gap: f32) -> (f32, f32) {
    let (mut lo, mut hi) = bounds;
    for &lat in alongside {
        if lat >= current {
            hi = hi.min(lat - gap);
        } else {
            lo = lo.max(lat + gap);
        }
    }
    if lo > hi {
        let hold = current.clamp(bounds.0, bounds.1);
        (hold, hold)
    } else {
        (lo, hi)
    }
}

/// Less than this far ahead (u, a car length) a car is beside this one, not in front of it.
const BESIDE_U: f32 = 2.0;
/// Within this distance ahead (u, the drivers' alongside window) a slower car limits the speed
/// only when the bodies would touch: two cars side by side must not hold each other up.
const ALONGSIDE_AHEAD_U: f32 = 3.0;

/// Speed limit (u/s) that keeps the car from running into slower moving cars ahead that overlap
/// its current lateral `current` (`half_overlap` = centre distance across the track below which
/// the bodies come too close; for a car alongside, `touch`, where they would touch): close in to
/// `gap` behind them at `rate` (u/s per u of extra gap). Stopped cars are not followed:
/// [`stop_short_limit`].
pub fn rear_end_limit(
    current: f32,
    speed: f32,
    obstacles: &[Obstacle],
    (half_overlap, touch): (f32, f32),
    gap: f32,
    rate: f32,
) -> Option<f32> {
    obstacles
        .iter()
        .filter(|o| o.car && !o.stopped && o.ahead > 0.0 && o.speed < speed)
        .filter(|o| {
            let reach = if o.ahead < ALONGSIDE_AHEAD_U {
                touch
            } else {
                half_overlap
            };
            (o.lateral - current).abs() < reach
        })
        .map(|o| o.speed + (o.ahead - gap) * rate)
        .min_by(f32::total_cmp)
}

/// How a car meets a stopped car it is steering around (u, u/s, u/s²).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StopShort {
    /// Centre distance across the track below which the bodies touch.
    pub touch: f32,
    /// While the way round is closed the car waits this far back (centre to centre): room to
    /// pull out at full lock once it opens.
    pub wait: f32,
    /// It never comes closer than this (centre to centre; it never pushes the car).
    pub contact: f32,
    /// Speed it creeps at while it turns out from a standstill (u/s).
    pub creep: f32,
    /// Braking the limit counts on (u/s²).
    pub decel: f32,
}

/// Speed limit (u/s) in front of a stopped car whose body overlaps the car's current lateral:
/// braking no earlier than it must, the car stops `wait` behind it — unless its heading (`slope`
/// = lateral u per u along the track) already carries it clear, when it may close in to
/// `contact`, or it steers for a lateral clear of it (`aim`), when it creeps on at `creep` (no
/// closer than `contact`) while it turns out. Once beside it the car drives on.
pub fn stop_short_limit(
    current: f32,
    slope: f32,
    aim: f32,
    obstacles: &[Obstacle],
    k: &StopShort,
) -> Option<f32> {
    obstacles
        .iter()
        .filter(|o| o.stopped && o.ahead > BESIDE_U && (o.lateral - current).abs() < k.touch)
        .map(|o| {
            let stop = |gap: f32| (2.0 * k.decel * (o.ahead - gap).max(0.0)).sqrt();
            let clears = (current + slope * (o.ahead - k.contact) - o.lateral).abs() >= k.touch;
            let open = (aim - o.lateral).abs() >= k.touch && o.ahead > k.contact;
            match (clears, open) {
                (true, _) => stop(k.contact),
                (false, true) => stop(k.wait).max(k.creep),
                (false, false) => stop(k.wait),
            }
        })
        .min_by(f32::total_cmp)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HALF: f32 = 0.6;
    const BOUNDS: (f32, f32) = (-4.0, 4.0);

    fn car(ahead: f32, lateral: f32, speed: f32) -> Obstacle {
        Obstacle {
            ahead,
            lateral,
            half_width: 0.6 + 0.8,
            speed,
            car: true,
            stopped: false,
        }
    }

    #[test]
    fn clear_road_keeps_the_plan() {
        let c = choose_lane(1.0, 1.0, HALF, 20.0, BOUNDS, &[]);
        assert_eq!(
            c,
            LaneChoice {
                lateral: 1.0,
                follow: None,
                dodge: false
            }
        );
        // A car to the side or behind does not matter.
        let side = [car(8.0, -3.0, 15.0), car(-5.0, 1.0, 15.0)];
        assert_eq!(
            choose_lane(1.0, 1.0, HALF, 20.0, BOUNDS, &side).lateral,
            1.0
        );
    }

    #[test]
    fn faster_cars_ahead_are_not_passed() {
        let c = choose_lane(0.0, 0.0, HALF, 20.0, BOUNDS, &[car(6.0, 0.0, 25.0)]);
        assert_eq!(c.lateral, 0.0);
        assert_eq!(c.follow, None);
    }

    #[test]
    fn slower_car_is_passed_on_the_side_we_are_on() {
        let o = [car(6.0, 0.0, 12.0)];
        let left = choose_lane(0.0, -0.3, HALF, 20.0, BOUNDS, &o);
        assert!((left.lateral - -2.0).abs() < 1e-5, "{left:?}");
        let right = choose_lane(0.0, 0.3, HALF, 20.0, BOUNDS, &o);
        assert!((right.lateral - 2.0).abs() < 1e-5, "{right:?}");
    }

    #[test]
    fn the_track_edge_forces_the_other_side() {
        let o = [car(6.0, -3.0, 12.0)];
        let c = choose_lane(-3.0, -3.5, HALF, 20.0, BOUNDS, &o);
        assert!((c.lateral - -1.0).abs() < 1e-5, "{c:?}");
    }

    #[test]
    fn a_hazard_beyond_the_car_decides_the_passing_side() {
        let pad = Obstacle {
            ahead: 14.0,
            lateral: -2.0,
            half_width: 1.6 + 0.6,
            speed: 0.0,
            car: false,
            stopped: false,
        };
        // On the car's left already, but a Spring Pad lies on the left lane farther on.
        let o = [car(6.0, 0.0, 12.0), pad];
        let c = choose_lane(0.0, -0.3, HALF, 20.0, BOUNDS, &o);
        assert!((c.lateral - 2.0).abs() < 1e-5, "passes on the right: {c:?}");
    }

    #[test]
    fn a_blocked_road_means_following() {
        let o = [car(6.0, -2.0, 12.0), car(6.5, 1.8, 13.0)];
        let c = choose_lane(-2.0, -2.0, HALF, 20.0, (-3.0, 3.0), &o);
        assert_eq!(c.follow, Some(o[0]), "the slower car ahead");
        assert_eq!(c.lateral, -2.0);
    }

    #[test]
    fn a_stopped_car_is_steered_around_never_followed() {
        let parked = Obstacle {
            stopped: true,
            ..car(4.0, -2.0, 0.0)
        };
        let bounds = (-3.0, 4.0); // no room on its left
        // A car passing it on the right, still behind it, does not close that side.
        let passing = car(1.5, 0.6, 3.0);
        let c = choose_lane(-2.0, -2.0, HALF, 5.0, bounds, &[parked, passing]);
        assert!(c.dodge && c.follow.is_none(), "{c:?}");
        assert!(
            (c.lateral - 0.0).abs() < 1e-5,
            "past it on the right: {c:?}"
        );
        // A car beside it closes the right side too: squeeze past on the roomier side, never stop.
        let beside = car(4.5, 0.5, 3.0);
        let c = choose_lane(-2.0, -2.0, HALF, 5.0, bounds, &[parked, beside]);
        assert!(c.dodge && c.follow.is_none(), "{c:?}");
        assert!((c.lateral - 0.0).abs() < 1e-5, "{c:?}");
        // The same car moving on: followed when it cannot be passed.
        let moving = car(4.0, -2.0, 12.0);
        let c = choose_lane(-2.0, -2.0, HALF, 20.0, bounds, &[moving, beside]);
        assert_eq!(c.follow, Some(moving));
    }

    #[test]
    fn cars_alongside_narrow_the_range() {
        let b = side_bounds(0.0, BOUNDS, &[1.5, -2.0], 1.4);
        assert!(
            (b.0 - -0.6).abs() < 1e-5 && (b.1 - 0.1).abs() < 1e-5,
            "{b:?}"
        );
        assert_eq!(side_bounds(0.0, BOUNDS, &[], 1.4), BOUNDS);
        // Pinned on both sides: hold the current lateral.
        assert_eq!(side_bounds(0.3, BOUNDS, &[1.0, -0.5], 1.4), (0.3, 0.3));
    }

    #[test]
    fn slower_cars_in_the_way_limit_speed() {
        let o = [car(8.0, 0.4, 15.0), car(6.0, 3.0, 10.0)];
        let fit = (1.6, 1.2);
        let v = rear_end_limit(0.0, 25.0, &o, fit, 4.0, 1.0).expect("the car at 0.4 overlaps");
        assert!((v - 19.0).abs() < 1e-5, "{v}");
        assert_eq!(
            rear_end_limit(0.0, 12.0, &o, fit, 4.0, 1.0),
            None,
            "not catching up"
        );
        assert_eq!(
            rear_end_limit(-2.0, 25.0, &o, fit, 4.0, 1.0),
            None,
            "clear lateral"
        );
        // Side by side (up to 3 u ahead, 1.5 u across): the bodies clear each other, no brake.
        for ahead in [0.4, 2.5] {
            let beside = [car(ahead, 1.5, 0.0)];
            assert_eq!(rear_end_limit(0.0, 1.0, &beside, fit, 3.0, 1.0), None);
        }
    }

    #[test]
    fn a_stopped_car_ahead_is_braked_for_as_late_as_the_brakes_allow() {
        let parked = [Obstacle {
            stopped: true,
            ..car(21.0, 0.5, 0.0)
        }];
        let fit = (1.6, 1.2);
        assert_eq!(
            rear_end_limit(0.0, 25.0, &parked, fit, 4.0, 1.0),
            None,
            "not followed"
        );
        let k = StopShort {
            touch: 1.2,
            wait: 5.0,
            contact: 2.5,
            creep: 2.0,
            decel: 28.0,
        };
        let stop = |gap: f32| (2.0f32 * 28.0 * (21.0 - gap)).sqrt();
        let v = |current, slope, aim| stop_short_limit(current, slope, aim, &parked, &k);
        assert_eq!(
            v(0.0, 0.0, 0.0),
            Some(stop(5.0)),
            "the way round closed: wait 5 u back"
        );
        assert_eq!(
            v(0.0, 0.05, 3.0),
            Some(stop(5.0)),
            "its heading does not clear it yet"
        );
        assert_eq!(v(0.0, 0.2, 3.0), Some(stop(2.5)), "heading out: close in");
        assert_eq!(v(1.8, 0.0, 3.0), None, "the bodies clear it");
        let near = |ahead| {
            let o = [Obstacle { ahead, ..parked[0] }];
            stop_short_limit(0.0, 0.0, 3.0, &o, &k)
        };
        assert_eq!(
            near(4.0),
            Some(2.0),
            "turning out from a standstill: creep on"
        );
        assert_eq!(near(2.4), Some(0.0), "never pushes it");
        assert_eq!(near(1.0), None, "beside it: drive on");
    }

    #[test]
    fn hazards_are_avoided_even_when_slow() {
        let oil = Obstacle {
            ahead: 10.0,
            lateral: 0.5,
            half_width: 1.25 + 0.9,
            speed: 0.0,
            car: false,
            stopped: false,
        };
        let c = choose_lane(0.0, 0.0, HALF, 5.0, BOUNDS, &[oil]);
        assert!(c.lateral < -2.0 && c.follow.is_none() && c.dodge, "{c:?}");
        // No room on either side: squeeze past on the roomier side, never stop.
        let wide = Obstacle {
            half_width: 4.0,
            ..oil
        };
        let c = choose_lane(0.0, 0.0, HALF, 5.0, BOUNDS, &[wide]);
        assert_eq!(c.follow, None);
        assert!(c.lateral <= -3.9 || c.lateral >= 3.9, "{c:?}");
    }
}
