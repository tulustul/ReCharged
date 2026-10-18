//! The weather state machine (GDD §10.2): per-preset targets, the Dynamic blend, wetness and snow
//! cover, Storm gusts and lightning. Pure: [`WeatherSim::step`] runs once per fixed tick; every
//! random number comes from `rc_core::rng`, keyed by the race seed and the sim's own tick.

use std::f32::consts::PI;

use bevy::math::ops;
use bevy::prelude::*;
use rc_core::prelude::*;

use crate::plan::Plan;

/// Length of the Dynamic blend, s (GDD §10.2).
const BLEND_SECS: f32 = 20.0;
/// Dry → fully wet while rain falls, s (GDD §10.2: "~20 s").
const WET_SECS: f32 = 20.0;
/// Fully wet → dry once no rain falls, s.
const DRY_SECS: f32 = 60.0;
/// Bare → full snow cover while snow falls, s.
const SNOW_SECS: f32 = 20.0;
/// Full snow cover → bare once no snow falls, s.
const MELT_SECS: f32 = 90.0;
/// Time between lightning slots, s (min, max).
const STRIKE_GAP_SECS: (f32, f32) = (5.0, 14.0);
/// Flash intensity of a strike at full Storm strength (min, max), 0–1.
const STRIKE_INTENSITY: (f32, f32) = (0.5, 1.0);
/// A flash of brightness 1 fades out linearly in this time, s.
const FLASH_SECS: f32 = 0.3;
/// Calm between two gusts, s (min, max).
const GUST_GAP_SECS: (f32, f32) = (3.0, 8.0);
/// One gust's sin² rise and fall, s.
const GUST_SECS: f32 = 2.5;
/// A gust's peak as a share of the preset's `gust_ups` (min, max).
const GUST_STRENGTH: (f32, f32) = (0.5, 1.0);

// Keys of the per-slot draws (streams `Lightning` / `WindGust`, tick = the slot's tick).
const KEY_GAP: u64 = 0;
const KEY_STRENGTH: u64 = 1;

/// Steady-state targets of a preset.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Preset {
    precipitation: f32,
    cloudiness: f32,
    fog: f32,
    /// Mean wind speed, u/s.
    wind_ups: f32,
    /// Extra wind at the peak of a full-strength gust, u/s.
    gust_ups: f32,
    /// Lightning strength 0–1 (scales strike intensity; 0 = no strikes).
    lightning: f32,
}

/// The parameter table (rationale in `docs/design/rc_weather.md`).
fn preset(kind: WeatherKind) -> Preset {
    use WeatherKind::*;
    let (precipitation, cloudiness, fog, wind_ups, gust_ups, lightning) = match kind {
        Clear => (0.0, 0.1, 0.0, 2.0, 0.0, 0.0),
        Cloudy => (0.0, 0.8, 0.05, 5.0, 0.0, 0.0),
        Rain => (0.6, 0.9, 0.15, 6.0, 0.0, 0.0),
        Storm => (1.0, 1.0, 0.25, 14.0, 16.0, 1.0),
        Fog => (0.0, 0.5, 0.85, 1.0, 0.0, 0.0),
        Snow => (0.5, 0.85, 0.3, 3.0, 0.0, 0.0),
    };
    Preset {
        precipitation,
        cloudiness,
        fog,
        wind_ups,
        gust_ups,
        lightning,
    }
}

impl Preset {
    /// `self` blended towards `to` by `t` (exactly `to` at `t = 1`).
    fn mix(self, to: Self, t: f32) -> Self {
        let m = |a: f32, b: f32| FloatExt::lerp(a, b, t);
        Self {
            precipitation: m(self.precipitation, to.precipitation),
            cloudiness: m(self.cloudiness, to.cloudiness),
            fog: m(self.fog, to.fog),
            wind_ups: m(self.wind_ups, to.wind_ups),
            gust_ups: m(self.gust_ups, to.gust_ups),
            lightning: m(self.lightning, to.lightning),
        }
    }
}

/// The running weather of an attempt (replaced in `OnEnter(RacePhase::Setup)`).
#[derive(Resource, Debug, Clone)]
pub(crate) struct WeatherSim {
    plan: Plan,
    /// Fixed ticks since Setup.
    tick: u32,
    /// Tick the Dynamic change started.
    changed_at: Option<u32>,
    /// Tick of the next lightning slot.
    next_strike: u32,
    /// Tick the current (or last) gust started.
    gust_start: u32,
    /// Its peak as a share of the preset's (0 before the first gust).
    gust_strength: f32,
    /// Tick the next gust starts.
    next_gust: u32,
}

/// Ticks until the next slot of `stream`, drawn at `tick`.
fn gap(seed: u64, tick: u32, stream: RngStream, (min, max): (f32, f32)) -> u32 {
    secs_to_ticks(rng::range(seed, u64::from(tick), stream, KEY_GAP, min, max)).max(1)
}

/// Strength of the slot of `stream` at `tick`, in `[min, max)`.
fn strength(seed: u64, tick: u32, stream: RngStream, (min, max): (f32, f32)) -> f32 {
    rng::range(seed, u64::from(tick), stream, KEY_STRENGTH, min, max)
}

impl WeatherSim {
    pub(crate) fn new(plan: Plan) -> Self {
        Self {
            next_strike: gap(plan.seed, 0, RngStream::Lightning, STRIKE_GAP_SECS),
            next_gust: gap(plan.seed, 0, RngStream::WindGust, GUST_GAP_SECS),
            plan,
            tick: 0,
            changed_at: None,
            gust_start: 0,
            gust_strength: 0.0,
        }
    }

    /// The start preset, settled: outdoors the ground is already wet (or white) when it has been
    /// raining (or snowing) before the race.
    pub(crate) fn initial_state(&self) -> WeatherState {
        let mut state = WeatherState {
            time: self.plan.time,
            outside_only: self.plan.outside_only,
            ..default()
        };
        let row = self.apply_targets(&mut state);
        if !self.plan.outside_only {
            let (rain, snow) = falling(&state);
            state.wetness = f32::from(rain);
            state.snow_cover = f32::from(snow);
            state.wind = self.plan.wind_dir * row.wind_ups;
        }
        state
    }

    /// Advances one fixed tick; `progress` is the leader's share of the race distance.
    pub(crate) fn step(
        &mut self,
        state: &mut WeatherState,
        progress: f32,
        mut emit: impl FnMut(WeatherEvent),
    ) {
        self.tick += 1;
        if let Some(change) = self.plan.change
            && self.changed_at.is_none()
            && progress >= change.at
        {
            info!(
                "weather: {:?} -> {:?} at {:.0} % of the race",
                self.plan.start,
                change.to,
                progress * 100.0
            );
            self.changed_at = Some(self.tick);
            emit(WeatherEvent::Changed(change.to));
        }
        let row = self.apply_targets(state);
        if !self.plan.outside_only {
            let (rain, snow) = falling(state);
            state.wetness = approach(state.wetness, rain, WET_SECS, DRY_SECS);
            state.snow_cover = approach(state.snow_cover, snow, SNOW_SECS, MELT_SECS);
            state.wind = self.plan.wind_dir * (row.wind_ups + row.gust_ups * self.gust());
        }
        state.lightning = state.lightning.move_towards(0.0, SIM_DT / FLASH_SECS);
        if self.tick >= self.next_strike {
            let (seed, tick) = (self.plan.seed, self.tick);
            self.next_strike = tick + gap(seed, tick, RngStream::Lightning, STRIKE_GAP_SECS);
            if row.lightning > 0.0 {
                let intensity =
                    row.lightning * strength(seed, tick, RngStream::Lightning, STRIKE_INTENSITY);
                state.lightning = state.lightning.max(intensity);
                emit(WeatherEvent::Lightning { intensity });
            }
        }
    }

    /// Writes the blended preset (kind, snowing, precipitation, cloudiness, fog) and returns it.
    fn apply_targets(&self, state: &mut WeatherState) -> Preset {
        let from = self.plan.start;
        let (to, t) = match (self.plan.change, self.changed_at) {
            (Some(change), Some(at)) => {
                let t = ticks_to_secs(self.tick - at) / BLEND_SECS;
                (change.to, t.smoothstep(0.0, 1.0))
            }
            _ => (from, 0.0),
        };
        let row = preset(from).mix(preset(to), t);
        state.kind = to;
        state.snowing = from == WeatherKind::Snow || to == WeatherKind::Snow;
        state.precipitation = row.precipitation;
        state.cloudiness = row.cloudiness;
        state.fog = row.fog;
        row
    }

    /// The current gust as a share of the preset's peak (0–1); starts gusts on schedule.
    fn gust(&mut self) -> f32 {
        let (seed, tick) = (self.plan.seed, self.tick);
        if tick >= self.next_gust {
            self.gust_start = tick;
            self.gust_strength = strength(seed, tick, RngStream::WindGust, GUST_STRENGTH);
            self.next_gust = tick
                + secs_to_ticks(GUST_SECS)
                + gap(seed, tick, RngStream::WindGust, GUST_GAP_SECS);
        }
        let t = ticks_to_secs(tick - self.gust_start) / GUST_SECS;
        if t >= 1.0 {
            return 0.0;
        }
        let rise = ops::sin(PI * t);
        self.gust_strength * rise * rise
    }
}

/// (rain falls, snow falls).
fn falling(state: &WeatherState) -> (bool, bool) {
    let falling = state.precipitation > 0.0;
    (falling && !state.snowing, falling && state.snowing)
}

/// Linear ramp of a 0–1 level towards 1 while `on` (full in `rise_secs`), else towards 0
/// (`fall_secs`).
fn approach(level: f32, on: bool, rise_secs: f32, fall_secs: f32) -> f32 {
    let secs = if on { rise_secs } else { fall_secs };
    level.move_towards(f32::from(on), SIM_DT / secs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::Change;
    use WeatherKind::*;

    const TICK: f32 = SIM_DT;

    fn sim(start: WeatherKind, change: Option<(f32, WeatherKind)>, indoor: bool) -> WeatherSim {
        WeatherSim::new(Plan {
            start,
            change: change.map(|(at, to)| Change { at, to }),
            time: TimeOfDay::Noon,
            outside_only: indoor,
            wind_dir: Vec3::X,
            seed: 11,
        })
    }

    /// Runs `secs` at a constant `progress`; returns the events.
    fn run(
        sim: &mut WeatherSim,
        state: &mut WeatherState,
        secs: f32,
        progress: f32,
    ) -> Vec<WeatherEvent> {
        let mut events = Vec::new();
        for _ in 0..secs_to_ticks(secs) {
            sim.step(state, progress, |e| events.push(e));
        }
        events
    }

    fn changes(events: &[WeatherEvent]) -> usize {
        events
            .iter()
            .filter(|e| matches!(e, WeatherEvent::Changed(_)))
            .count()
    }

    #[test]
    fn fixed_weather_starts_settled_and_holds() {
        for kind in WeatherKind::ALL {
            let mut sim = sim(kind, None, false);
            let start = sim.initial_state();
            assert_eq!(start.kind, kind);
            assert_eq!(
                start.wetness,
                f32::from(matches!(kind, Rain | Storm)),
                "{kind:?}"
            );
            assert_eq!(start.snow_cover, f32::from(kind == Snow), "{kind:?}");
            assert_eq!(start.snowing, kind == Snow);
            let mut state = start.clone();
            let events = run(&mut sim, &mut state, 60.0, 0.5);
            assert_eq!(changes(&events), 0, "{kind:?}");
            if kind != Storm {
                assert!(events.is_empty(), "{kind:?}: {events:?}");
                assert_eq!(state, start, "{kind:?} holds still");
            }
        }
    }

    #[test]
    fn dynamic_change_fires_once_at_its_distance_and_blends_for_20_s() {
        let mut sim = sim(Cloudy, Some((0.5, Rain)), false);
        let mut state = sim.initial_state();
        assert!(run(&mut sim, &mut state, 10.0, 0.49).is_empty());
        assert_eq!((state.kind, state.precipitation), (Cloudy, 0.0));

        let events = run(&mut sim, &mut state, TICK, 0.5);
        assert_eq!(events, [WeatherEvent::Changed(Rain)]);
        assert_eq!(state.kind, Rain, "kind is the target while blending");

        let mut last = state.clone();
        for _ in 0..secs_to_ticks(BLEND_SECS) {
            assert_eq!(changes(&run(&mut sim, &mut state, TICK, 0.9)), 0);
            for (now, before) in [
                (state.precipitation, last.precipitation),
                (state.cloudiness, last.cloudiness),
                (state.fog, last.fog),
            ] {
                let delta = now - before;
                assert!((0.0..1e-3).contains(&delta), "smooth, monotonic: {delta}");
            }
            last = state.clone();
        }
        assert_eq!(state.precipitation, preset(Rain).precipitation);
        assert_eq!(
            changes(&run(&mut sim, &mut state, 60.0, 1.0)),
            0,
            "only once"
        );
        assert_eq!(
            state,
            self::sim(Rain, None, false).initial_state(),
            "settles as Rain"
        );
    }

    #[test]
    fn ground_wets_or_whitens_in_20_s_and_recovers_slower() {
        type Level = fn(&WeatherState) -> f32;
        let cases: [(WeatherKind, Level, Level, f32, f32); 2] = [
            (Rain, |s| s.wetness, |s| s.snow_cover, WET_SECS, DRY_SECS),
            (Snow, |s| s.snow_cover, |s| s.wetness, SNOW_SECS, MELT_SECS),
        ];
        for (kind, level, other, rise, fall) in cases {
            // The change tick itself has no precipitation yet.
            let mut sim = sim(Cloudy, Some((0.0, kind)), false);
            let mut state = sim.initial_state();
            run(&mut sim, &mut state, TICK + rise / 2.0, 0.0);
            assert!((level(&state) - 0.5).abs() < 0.01, "{kind:?}: {state:?}");
            run(&mut sim, &mut state, rise / 2.0 + 2.0 * TICK, 0.0);
            assert_eq!((level(&state), other(&state)), (1.0, 0.0), "{kind:?}");

            let mut sim = self::sim(kind, Some((0.0, Cloudy)), false);
            let mut state = sim.initial_state();
            run(&mut sim, &mut state, BLEND_SECS, 0.0);
            assert!(
                state.precipitation > 0.0,
                "still falling at the end of the blend"
            );
            assert_eq!((level(&state), state.kind), (1.0, Cloudy), "{kind:?}");
            assert_eq!(state.snowing, kind == Snow, "fading snow is still snow");
            run(&mut sim, &mut state, fall / 2.0, 0.0);
            assert!((level(&state) - 0.5).abs() < 0.01, "{kind:?}: {state:?}");
            run(&mut sim, &mut state, fall / 2.0 + 2.0 * TICK, 0.0);
            assert_eq!((level(&state), other(&state)), (0.0, 0.0), "{kind:?}");
        }
    }

    #[test]
    fn indoor_weather_never_wets_the_track() {
        let cases = [
            (Rain, None),
            (Storm, None),
            (Snow, None),
            (Cloudy, Some((0.0, Rain))),
            (Cloudy, Some((0.0, Snow))),
            (Rain, Some((0.0, Storm))),
        ];
        for (start, change) in cases {
            let mut sim = sim(start, change, true);
            let mut state = sim.initial_state();
            for _ in 0..secs_to_ticks(40.0) {
                assert!(state.outside_only);
                let dry = (state.wetness, state.snow_cover, state.wind);
                assert_eq!(dry, (0.0, 0.0, Vec3::ZERO), "{start:?} → {change:?}");
                sim.step(&mut state, 0.5, |_| {});
            }
            assert!(state.precipitation > 0.0, "the outside weather still shows");
        }
    }

    #[test]
    fn storms_strike_lightning_and_gust() {
        let mut sim = sim(Storm, None, false);
        let mut state = sim.initial_state();
        let storm = preset(Storm);
        let (mut strikes, mut flash, mut wind) = (Vec::new(), Vec::new(), Vec::new());
        for tick in 0..secs_to_ticks(120.0) {
            sim.step(&mut state, 0.0, |e| {
                if let WeatherEvent::Lightning { intensity } = e {
                    strikes.push((tick as usize, intensity));
                }
            });
            flash.push(state.lightning);
            wind.push(state.wind);
        }
        assert!(
            (8..=25).contains(&strikes.len()),
            "{} strikes in 120 s",
            strikes.len()
        );
        let (min_gap, max_gap) = STRIKE_GAP_SECS;
        for pair in strikes.windows(2) {
            let gap = ticks_to_secs((pair[1].0 - pair[0].0) as u32);
            assert!(gap >= min_gap - TICK && gap <= max_gap + TICK, "gap {gap}");
        }
        for &(tick, intensity) in &strikes {
            assert!((0.5..1.0).contains(&intensity), "{intensity}");
            assert_eq!(flash[tick], intensity, "the flash starts at full intensity");
            let later = flash.get(tick + secs_to_ticks(FLASH_SECS) as usize);
            assert!(later.is_none_or(|&f| f == 0.0), "and fades out");
        }
        let speeds: Vec<f32> = wind.iter().map(|w| w.length()).collect();
        assert!(
            wind.iter().all(|w| w.y == 0.0 && w.z == 0.0),
            "gusts keep the direction"
        );
        assert!(speeds.iter().all(|&s| s >= storm.wind_ups - 1e-4));
        let peak = speeds.iter().copied().fold(0.0, f32::max);
        assert!(
            peak >= storm.wind_ups + 0.5 * storm.gust_ups - 0.1,
            "gusty: {peak}"
        );
        assert!(peak <= storm.wind_ups + storm.gust_ups + 1e-3);
    }
}
