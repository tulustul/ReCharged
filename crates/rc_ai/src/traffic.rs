//! Lane choice (pure): overtaking slower cars and steering around hazards. Everything is in
//! track coordinates — `ahead` along the track, `lateral` across it (+ = right).

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
}

/// Outcome of [`choose_lane`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LaneChoice {
    /// Lateral position to drive at near the obstacle (u).
    pub lateral: f32,
    /// Speed to hold behind a car that cannot be passed (u/s).
    pub follow_speed: Option<f32>,
    /// The lateral steers around a hazard (dodge at once, not as a gradual lane change).
    pub dodge: bool,
}

/// Where to drive given the planned `lateral`, the car's current lateral `current`, its half
/// width `my_half`, its speed and `obstacles` ahead; `bounds` = (min, max) legal laterals.
/// A blocking obstacle is passed on the side the car already is on when there is room, else on
/// the other side; a car that cannot be passed is followed.
pub fn choose_lane(
    planned: f32,
    current: f32,
    my_half: f32,
    speed: f32,
    bounds: (f32, f32),
    obstacles: &[Obstacle],
) -> LaneChoice {
    let relevant = |o: &&Obstacle| o.ahead > 0.0 && (!o.car || speed > o.speed - 0.5);
    let blocks = |lat: f32, o: &Obstacle| (lat - o.lateral).abs() < o.half_width + my_half;
    let Some(blocker) = obstacles
        .iter()
        .filter(relevant)
        .filter(|o| blocks(planned, o))
        .min_by(|a, b| a.ahead.total_cmp(&b.ahead))
    else {
        return LaneChoice {
            lateral: planned,
            follow_speed: None,
            dodge: false,
        };
    };
    let reach = blocker.half_width + my_half;
    let (left, right) = (blocker.lateral - reach, blocker.lateral + reach);
    let free = |lat: f32| {
        lat >= bounds.0
            && lat <= bounds.1
            && !obstacles
                .iter()
                .filter(relevant)
                .filter(|o| (o.ahead - blocker.ahead).abs() < 2.0 * reach.max(2.0))
                .any(|o| blocks(lat, o))
    };
    let sides = match current <= blocker.lateral {
        true => [left, right],
        false => [right, left],
    };
    let dodge = !blocker.car;
    match sides.into_iter().find(|&lat| free(lat)) {
        Some(lat) => LaneChoice {
            lateral: lat,
            follow_speed: None,
            dodge,
        },
        None if blocker.car => LaneChoice {
            lateral: planned,
            follow_speed: Some(blocker.speed),
            dodge,
        },
        // No room around a hazard: squeeze past on the roomier side.
        None => LaneChoice {
            lateral: if bounds.1 - blocker.lateral > blocker.lateral - bounds.0 {
                right.min(bounds.1)
            } else {
                left.max(bounds.0)
            },
            follow_speed: None,
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

/// Speed limit (u/s) that keeps the car from running into slower cars ahead that overlap its
/// current lateral `current` (`half_overlap` = centre distance below which bodies touch):
/// close in to `gap` behind them at `rate` (u/s per u of extra gap).
pub fn rear_end_limit(
    current: f32,
    speed: f32,
    obstacles: &[Obstacle],
    half_overlap: f32,
    gap: f32,
    rate: f32,
) -> Option<f32> {
    obstacles
        .iter()
        .filter(|o| o.car && o.ahead > 0.0 && o.speed < speed)
        .filter(|o| (o.lateral - current).abs() < half_overlap)
        .map(|o| o.speed + (o.ahead - gap) * rate)
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
        }
    }

    #[test]
    fn clear_road_keeps_the_plan() {
        let c = choose_lane(1.0, 1.0, HALF, 20.0, BOUNDS, &[]);
        assert_eq!(
            c,
            LaneChoice {
                lateral: 1.0,
                follow_speed: None,
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
        assert_eq!(c.follow_speed, None);
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
    fn a_blocked_road_means_following() {
        let o = [car(6.0, -2.0, 12.0), car(6.5, 1.8, 13.0)];
        let c = choose_lane(-2.0, -2.0, HALF, 20.0, (-3.0, 3.0), &o);
        assert_eq!(c.follow_speed, Some(12.0));
        assert_eq!(c.lateral, -2.0);
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
        let v = rear_end_limit(0.0, 25.0, &o, 1.6, 4.0, 1.0).expect("the car at 0.4 overlaps");
        assert!((v - 19.0).abs() < 1e-5, "{v}");
        assert_eq!(
            rear_end_limit(0.0, 12.0, &o, 1.6, 4.0, 1.0),
            None,
            "not catching up"
        );
        assert_eq!(
            rear_end_limit(-2.0, 25.0, &o, 1.6, 4.0, 1.0),
            None,
            "clear lateral"
        );
    }

    #[test]
    fn hazards_are_avoided_even_when_slow() {
        let oil = Obstacle {
            ahead: 10.0,
            lateral: 0.5,
            half_width: 1.25 + 0.9,
            speed: 0.0,
            car: false,
        };
        let c = choose_lane(0.0, 0.0, HALF, 5.0, BOUNDS, &[oil]);
        assert!(
            c.lateral < -2.0 && c.follow_speed.is_none() && c.dodge,
            "{c:?}"
        );
        // No room on either side: squeeze past on the roomier side, never stop.
        let wide = Obstacle {
            half_width: 4.0,
            ..oil
        };
        let c = choose_lane(0.0, 0.0, HALF, 5.0, BOUNDS, &[wide]);
        assert_eq!(c.follow_speed, None);
        assert!(c.lateral <= -3.9 || c.lateral >= 3.9, "{c:?}");
    }
}
