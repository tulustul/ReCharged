//! The weather plan of a race: a pure function of `RaceConfig::weather`, the seed and the track's
//! catalog entry (GDD §10.2, ARCHITECTURE §5.4). Restarts replan with the same seed, so every
//! attempt of a session gets the same weather.

use std::f32::consts::TAU;

use bevy::math::ops;
use bevy::prelude::*;
use rc_core::prelude::*;

/// Share of the race distance covered by the leader when a Dynamic change starts: `[min, max)`.
pub(crate) const CHANGE_WINDOW: (f32, f32) = (0.3, 0.7);

// Keys of the plan's draws (stream `WeatherPlan`, tick 0).
const KEY_RANDOM: u64 = 0;
const KEY_CHANGE_AT: u64 = 1;
const KEY_CHANGE_TO: u64 = 2;
const KEY_WIND_DIR: u64 = 3;

/// The one change of Dynamic weather.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Change {
    /// Share of the race distance (leader) at which the blend starts.
    pub at: f32,
    /// The neighbouring preset blended to.
    pub to: WeatherKind,
}

/// What the weather of one race does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Plan {
    /// Preset at the start.
    pub start: WeatherKind,
    /// Dynamic weather's change (`None` = the start preset all race).
    pub change: Option<Change>,
    /// Time of day.
    pub time: TimeOfDay,
    /// Indoor track: outside weather only (no wetness, snow cover or wind).
    pub outside_only: bool,
    /// Horizontal unit vector the wind blows towards.
    pub wind_dir: Vec3,
    /// Race seed (lightning and gust draws).
    pub seed: u64,
}

/// Presets a Dynamic change may go to (GDD §10.2: Clear↔Cloudy↔Rain↔Storm, Cloudy↔Fog,
/// Cloudy↔Snow).
pub(crate) fn neighbours(kind: WeatherKind) -> &'static [WeatherKind] {
    use WeatherKind::*;
    match kind {
        Clear | Fog | Snow => &[Cloudy],
        Cloudy => &[Clear, Rain, Fog, Snow],
        Rain => &[Cloudy, Storm],
        Storm => &[Rain],
    }
}

/// Plans a race. `track: None` (not in the catalog) counts as outdoor with every preset allowed.
///
/// Random picks one of the track's allowed presets; Dynamic changes once to a neighbour the track
/// allows (no allowed neighbour → no change). rc_game only builds Dynamic for outdoor tracks; on
/// an indoor one it would change the outside weather only.
pub(crate) fn plan(setup: WeatherSetup, seed: u64, track: Option<&TrackInfo>) -> Plan {
    let allowed = track.map_or(&WeatherKind::ALL[..], |t| &t.allowed_weather[..]);
    let pick = |key, n| rng::index(seed, 0, RngStream::WeatherPlan, key, n);
    let (start, change) = match setup.weather {
        WeatherChoice::Fixed(kind) => (kind, None),
        WeatherChoice::Random => {
            let kind = allowed.get(pick(KEY_RANDOM, allowed.len()));
            (kind.copied().unwrap_or_default(), None)
        }
        WeatherChoice::Dynamic { start } => {
            let options: Vec<WeatherKind> = neighbours(start)
                .iter()
                .copied()
                .filter(|k| allowed.contains(k))
                .collect();
            let (lo, hi) = CHANGE_WINDOW;
            let change = (!options.is_empty()).then(|| Change {
                at: rng::range(seed, 0, RngStream::WeatherPlan, KEY_CHANGE_AT, lo, hi),
                to: options[pick(KEY_CHANGE_TO, options.len())],
            });
            (start, change)
        }
    };
    let yaw = rng::range(seed, 0, RngStream::WeatherPlan, KEY_WIND_DIR, 0.0, TAU);
    let (sin, cos) = ops::sin_cos(yaw);
    Plan {
        start,
        change,
        time: setup.time,
        outside_only: track.is_some_and(|t| t.indoor),
        wind_dir: Vec3::new(cos, 0.0, sin),
        seed,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use WeatherKind::*;

    const SEEDS: u64 = 300;

    fn track(indoor: bool, allowed: &[WeatherKind]) -> TrackInfo {
        TrackInfo {
            indoor,
            allowed_weather: allowed.to_vec(),
            ..default()
        }
    }

    fn setup(weather: WeatherChoice) -> WeatherSetup {
        WeatherSetup {
            weather,
            time: TimeOfDay::Night,
        }
    }

    #[test]
    fn plan_depends_only_on_the_seed() {
        let outdoor = track(false, &WeatherKind::ALL);
        for choice in [
            WeatherChoice::Random,
            WeatherChoice::Dynamic { start: Cloudy },
        ] {
            let plans: Vec<Plan> = (0..SEEDS)
                .map(|seed| plan(setup(choice), seed, Some(&outdoor)))
                .collect();
            for (seed, p) in (0..SEEDS).zip(&plans) {
                assert_eq!(plan(setup(choice), seed, Some(&outdoor)), *p);
                assert_eq!((p.time, p.seed), (TimeOfDay::Night, seed));
                assert!((p.wind_dir.length() - 1.0).abs() < 1e-5 && p.wind_dir.y == 0.0);
            }
            assert!(plans.iter().any(|p| p != &plans[0]), "seeds vary the plan");
        }
        let fixed = plan(setup(WeatherChoice::Fixed(Fog)), 9, Some(&outdoor));
        assert_eq!((fixed.start, fixed.change), (Fog, None));
    }

    #[test]
    fn random_picks_every_allowed_preset_and_nothing_else() {
        for t in [
            track(true, &[Clear, Cloudy, Rain, Storm]),
            track(true, &[Clear]),
            track(false, &WeatherKind::ALL),
        ] {
            let picked: HashSet<WeatherKind> = (0..SEEDS)
                .map(|seed| plan(setup(WeatherChoice::Random), seed, Some(&t)))
                .inspect(|p| assert_eq!(p.change, None))
                .map(|p| p.start)
                .collect();
            let allowed: HashSet<WeatherKind> = t.allowed_weather.iter().copied().collect();
            assert_eq!(picked, allowed);
        }
        let empty = track(false, &[]);
        assert_eq!(
            plan(setup(WeatherChoice::Random), 1, Some(&empty)).start,
            Clear
        );
    }

    #[test]
    fn neighbour_graph_is_the_gdd_one() {
        let edges = [
            (Clear, Cloudy),
            (Cloudy, Rain),
            (Rain, Storm),
            (Cloudy, Fog),
            (Cloudy, Snow),
        ];
        for a in WeatherKind::ALL {
            for b in WeatherKind::ALL {
                let edge = edges.contains(&(a, b)) || edges.contains(&(b, a));
                assert_eq!(neighbours(a).contains(&b), edge, "{a:?} – {b:?}");
            }
        }
    }

    #[test]
    fn dynamic_changes_to_a_neighbour_inside_the_window() {
        let outdoor = track(false, &WeatherKind::ALL);
        for start in WeatherKind::ALL {
            let changes: Vec<Change> = (0..SEEDS)
                .map(|seed| {
                    plan(
                        setup(WeatherChoice::Dynamic { start }),
                        seed,
                        Some(&outdoor),
                    )
                })
                .map(|p| p.change.expect("outdoor Dynamic plans a change"))
                .collect();
            let targets: HashSet<WeatherKind> = changes.iter().map(|c| c.to).collect();
            assert_eq!(
                targets,
                neighbours(start).iter().copied().collect(),
                "{start:?}"
            );
            let (lo, hi) = changes.iter().fold((1.0f32, 0.0f32), |(lo, hi), c| {
                assert!((CHANGE_WINDOW.0..CHANGE_WINDOW.1).contains(&c.at), "{c:?}");
                (lo.min(c.at), hi.max(c.at))
            });
            assert!(lo < 0.32 && hi > 0.68, "window used: {lo}..{hi}");
        }
    }

    #[test]
    fn dynamic_only_goes_where_the_track_allows() {
        let windows_only = track(true, &[Clear, Cloudy, Rain, Storm]);
        for seed in 0..SEEDS {
            let p = plan(
                setup(WeatherChoice::Dynamic { start: Cloudy }),
                seed,
                Some(&windows_only),
            );
            assert!(
                matches!(p.change.map(|c| c.to), Some(Clear | Rain)),
                "{p:?}"
            );
        }
        let clear_only = track(true, &[Clear]);
        let p = plan(
            setup(WeatherChoice::Dynamic { start: Clear }),
            4,
            Some(&clear_only),
        );
        assert_eq!(p.change, None, "no allowed neighbour: no change");
    }

    #[test]
    fn indoor_tracks_get_outside_only_weather() {
        let choice = setup(WeatherChoice::Fixed(Rain));
        assert!(plan(choice, 0, Some(&track(true, &[Rain]))).outside_only);
        assert!(!plan(choice, 0, Some(&track(false, &[Rain]))).outside_only);
        assert!(
            !plan(choice, 0, None).outside_only,
            "unknown track: outdoor"
        );
    }
}
