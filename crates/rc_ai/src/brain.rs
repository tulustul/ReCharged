//! Per-racer state kept by rc_ai: the [`TrackView`] of every racer (track coordinates, velocity,
//! progress — what traffic decisions and the assist see) and the [`AiBrain`] of AI racers.

use bevy::prelude::*;
use rc_core::prelude::*;
use rc_track::Track;

use crate::data::{AiTuning, DifficultyProfile, Personality};
use crate::items::ItemBrain;
use crate::profile::{CarCaps, SpeedProfile};
use crate::recovery::Recovery;

/// A racer moving farther than this in one tick was teleported (u).
const TELEPORT_STEP_U: f32 = 4.0;
/// Perfect starts press the throttle this long before GO (s, GDD §3.2 window: 0.3 s) …
const PERFECT_PRESS_SECS: (f32, f32) = (0.06, 0.24);
/// … other starts react this long after GO (s).
const LATE_PRESS_SECS: (f32, f32) = (0.08, 0.35);
/// RNG keys per racer: `index * KEYS_PER_RACER + purpose`.
pub(crate) const KEYS_PER_RACER: u64 = 16;

/// Where a racer is on the track (kept for every racer, AI or human).
#[derive(Component, Debug, Clone, Copy, Default, PartialEq)]
pub struct TrackView {
    /// Centreline arc length (u).
    pub s: f32,
    /// Lateral offset (u, + = right).
    pub lateral: f32,
    /// Height above the surface (u).
    pub height: f32,
    /// World position.
    pub pos: Vec3,
    /// Velocity from the last tick's motion (u/s).
    pub vel: Vec3,
    /// Progress along the track during the last tick (u).
    pub progress: f32,
    /// Teleported (spawn, respawn) since the previous tick.
    pub teleported: bool,
    /// `s` is a valid projection hint.
    fresh: bool,
}

/// `SimSet::Drivers` (first): track coordinates of every racer.
pub(crate) fn observe_racers(
    track: Option<Res<Track>>,
    mut respawned: MessageReader<CarRespawned>,
    mut racers: Query<(Entity, &Transform, &mut TrackView)>,
    mut teleports: Local<Vec<Entity>>,
) {
    teleports.clear();
    teleports.extend(respawned.read().map(|m| m.car));
    let Some(track) = track else { return };
    for (entity, transform, mut view) in &mut racers {
        let pos = transform.translation;
        let continuous =
            view.fresh && pos.distance(view.pos) < TELEPORT_STEP_U && !teleports.contains(&entity);
        let tp = track.project(pos, continuous.then_some(view.s));
        *view = TrackView {
            s: tp.s,
            lateral: tp.lateral,
            height: tp.height,
            pos,
            vel: if continuous {
                (pos - view.pos) / SIM_DT
            } else {
                Vec3::ZERO
            },
            progress: if continuous {
                track.delta_s(view.s, tp.s)
            } else {
                0.0
            },
            teleported: !continuous,
            fresh: true,
        };
    }
}

/// Decorates every racer with a [`TrackView`].
pub(crate) fn add_track_view(add: On<Add<Racer>>, mut commands: Commands) {
    commands.entity(add.entity).insert(TrackView::default());
}

/// A driving mistake in progress (GDD §8.2).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mistake {
    /// Brakes late: the speed plan is exceeded by this share.
    LateBraking(f32),
    /// Runs wide: this many u toward the outside of the corner.
    WideLine(f32),
}

/// The mind of one AI driver.
#[derive(Component, Debug, Clone)]
pub struct AiBrain {
    /// What the car can do.
    pub caps: CarCaps,
    /// The car's speed plan.
    pub profile: SpeedProfile,
    /// Drift zones (start s, end s).
    pub drift_zones: Vec<(f32, f32)>,
    /// Rival aggression (catalog), 0–1.
    pub aggression: f32,
    /// Rival consistency (catalog), 0–1.
    pub consistency: f32,
    /// Item and boost traits.
    pub personality: Personality,
    /// Personal racing-line offset (u).
    pub line_offset: f32,
    /// Current deviation from the planned line for overtaking/avoiding (u).
    pub lane: f32,
    /// Cornering grip adaptation from measured slides (≤ 1).
    pub adapt: f32,
    /// Stuck detection.
    pub recovery: Recovery,
    /// Mistake in progress and its remaining seconds.
    pub mistake: Option<(Mistake, f32)>,
    /// Drift zone being drifted (index) and seconds into it; `None` = not drifting.
    pub drift: Option<(usize, f32)>,
    /// Drift zone decided against (index).
    pub skip_zone: Option<usize>,
    /// Holding the charge boost.
    pub boosting: bool,
    /// Catching the slide of a released drift (throttle eased).
    pub catching: bool,
    /// Countdown ticks seen and the tick at which the throttle goes down.
    pub countdown: (u32, u32),
    /// Item decisions.
    pub items: ItemBrain,
    /// Throttle asked for last tick (stuck detection).
    pub last_throttle: f32,
}

/// Creates the brain of every AI racer that has none (spawn, autopilot, dropped player) and
/// rebuilds the plans when the tuning changes (hot reload).
pub(crate) fn ensure_brains(
    mut commands: Commands,
    track: Option<Res<Track>>,
    tuning: Option<Res<AiTuning>>,
    catalog: Option<Res<Catalog>>,
    config: Option<Res<RaceConfig>>,
    new: Query<(Entity, &Racer, &AiDriver, Has<LocalPlayer>), Without<AiBrain>>,
    mut existing: Query<(&AiDriver, &mut AiBrain)>,
) {
    let (Some(track), Some(tuning)) = (track, tuning) else {
        return;
    };
    let seed = config.map_or(0, |c| c.seed);
    for (entity, racer, driver, human) in &new {
        let brain = AiBrain::new(
            &track,
            &tuning,
            catalog.as_deref(),
            racer.index,
            &racer.car,
            driver,
            seed,
        );
        let mut e = commands.entity(entity);
        e.insert(brain);
        if !human {
            e.insert(CatchUp::default());
        }
    }
    if tuning.is_changed() && !tuning.is_added() {
        for (driver, mut brain) in &mut existing {
            brain.retune(&track, &tuning, driver.difficulty);
        }
    }
}

/// Neutral catalog personality for unknown rivals (the autopilot).
const NEUTRAL_AGGRESSION: f32 = 0.5;
const NEUTRAL_CONSISTENCY: f32 = 0.8;
/// Stars of a car missing from the catalog: the 3★ reference car.
const REFERENCE_STARS: CarStars = CarStars {
    speed: 3,
    accel: 3,
    handling: 3,
    weight: 3,
};

impl AiBrain {
    /// A new driver for racer `index` of `car` driven by `driver`.
    pub fn new(
        track: &Track,
        tuning: &AiTuning,
        catalog: Option<&Catalog>,
        index: usize,
        car: &CarId,
        driver: &AiDriver,
        seed: u64,
    ) -> Self {
        let stars = catalog
            .and_then(|c| c.car(car))
            .map_or(REFERENCE_STARS, |c| c.stars);
        let rival = catalog.and_then(|c| c.rival(&driver.rival));
        let skill = tuning.difficulty.get(driver.difficulty);
        let caps = CarCaps::from_stars(stars, &tuning.driving.lateral_accel_by_handling);
        let key = index as u64 * KEYS_PER_RACER;
        let spread = rng::range(seed, 0, RngStream::AiLineOffset, key, -1.0, 1.0);
        let mut brain = Self {
            caps,
            profile: SpeedProfile::build(
                track,
                &caps,
                skill.corner_grip,
                tuning.driving.brake_decel,
            ),
            drift_zones: Vec::new(),
            aggression: rival.map_or(NEUTRAL_AGGRESSION, |r| r.aggression),
            consistency: rival.map_or(NEUTRAL_CONSISTENCY, |r| r.consistency),
            personality: tuning.personality(driver.rival.as_str()),
            line_offset: spread * skill.line_spread_u,
            lane: 0.0,
            adapt: 1.0,
            recovery: Recovery::default(),
            mistake: None,
            drift: None,
            skip_zone: None,
            boosting: false,
            catching: false,
            countdown: (0, start_press_tick(seed, key, skill.perfect_start)),
            items: ItemBrain::default(),
            last_throttle: 0.0,
        };
        brain.drift_zones = drift_zones(&brain, tuning, skill);
        brain
    }

    /// Rebuilds what depends on the tuning (hot reload).
    pub fn retune(&mut self, track: &Track, tuning: &AiTuning, difficulty: Difficulty) {
        let skill = tuning.difficulty.get(difficulty);
        self.profile = SpeedProfile::build(
            track,
            &self.caps,
            skill.corner_grip,
            tuning.driving.brake_decel,
        );
        self.drift_zones = drift_zones(self, tuning, skill);
    }
}

/// Countdown tick (1-based, GO after `COUNTDOWN_TICKS`) at which the AI with RNG key `key`
/// presses the throttle: inside the perfect-start window with chance `perfect`, else a short
/// reaction after GO — never early enough to stall (GDD §3.2).
pub fn start_press_tick(seed: u64, key: u64, perfect: f32) -> u32 {
    let draw = |k: u64, (lo, hi): (f32, f32)| {
        secs_to_ticks(rng::range(
            seed,
            0,
            RngStream::AiPerfectStart,
            key + k,
            lo,
            hi,
        ))
    };
    match rng::chance(seed, 0, RngStream::AiPerfectStart, key, perfect) {
        true => COUNTDOWN_TICKS - draw(1, PERFECT_PRESS_SECS),
        false => COUNTDOWN_TICKS + draw(1, LATE_PRESS_SECS),
    }
}

/// Corners worth a drift: tighter than the drift radius, fast enough to charge, long enough.
fn drift_zones(brain: &AiBrain, tuning: &AiTuning, skill: &DifficultyProfile) -> Vec<(f32, f32)> {
    if skill.drift <= 0.0 {
        return Vec::new();
    }
    let c = &tuning.charge;
    let p = &brain.profile;
    let ds = p.spacing();
    let n = (p.length() / ds).round() as usize;
    let min_speed = c.drift_min_speed * brain.caps.top_speed;
    let tight = |i: usize| {
        let s = i as f32 * ds;
        p.kappa_at(s).abs() > 1.0 / c.drift_max_radius_u && p.corner_at(s) >= min_speed
    };
    // Start the scan on a sample outside any zone so no zone straddles the wrap.
    let Some(first) = (0..n).find(|&i| !tight(i)) else {
        return Vec::new();
    };
    let mut zones = Vec::new();
    let mut run: Option<(usize, f32)> = None;
    for k in 1..=n {
        let i = (first + k) % n;
        let s = i as f32 * ds;
        match (tight(i), run) {
            (true, None) => run = Some((k, ds / p.corner_at(s).min(brain.caps.top_speed))),
            (true, Some((start, secs))) => {
                run = Some((start, secs + ds / p.corner_at(s).min(brain.caps.top_speed)))
            }
            (false, Some((start, secs))) => {
                if secs >= c.drift_min_secs {
                    let from = ((first + start) % n) as f32 * ds;
                    // End early so the catch happens inside the corner.
                    let exit = c.drift_exit_secs * p.corner_at(s).min(brain.caps.top_speed);
                    zones.push((from, s - exit));
                }
                run = None;
            }
            (false, None) => {}
        }
    }
    zones
}

#[cfg(test)]
mod tests {
    use super::*;

    /// rc_race: a press at most this many ticks before GO is perfect, more than the second stalls.
    const PERFECT_WINDOW: u32 = 18;
    const STALL_EARLY: u32 = 60;

    #[test]
    fn start_presses_hit_the_window_at_the_difficulty_rate() {
        for p in [0.05, 0.2, 0.4, 0.6] {
            let n = 4000;
            let mut perfect = 0;
            for seed in 0..n {
                let tick = start_press_tick(seed, 3 * KEYS_PER_RACER, p);
                let early = COUNTDOWN_TICKS.saturating_sub(tick);
                assert!(
                    early <= STALL_EARLY,
                    "never a stall: pressed {early} ticks early"
                );
                if tick <= COUNTDOWN_TICKS {
                    assert!(
                        early <= PERFECT_WINDOW,
                        "a planned perfect start is perfect"
                    );
                    perfect += 1;
                }
            }
            let share = perfect as f32 / n as f32;
            assert!((share - p).abs() < 0.025, "perfect share {share} vs {p}");
        }
    }
}
