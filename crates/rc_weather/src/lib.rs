//! # rc_weather — weather & time of day (L1)
//!
//! - **Responsibilities:** the weather plan from `RaceConfig::weather` and the track's indoor flag
//!   (indoor → outside-only weather), seeded Random/Dynamic presets (keyed RNG), the Dynamic
//!   change at 30–70 % of the leader's race distance with a 20 s blend, wetness and snow
//!   accumulation/drying, Storm gusts and lightning. Design note: `docs/design/rc_weather.md`.
//! - **Inputs:** `RaceConfig` (weather, seed, laps), `Catalog` (indoor flag, allowed presets),
//!   `TrackSummary::length`, `RaceProgress::total_distance` of every racer.
//! - **Outputs:** `WeatherState` (sole writer), `WeatherEvent`.
//! - **Must never:** render or play sounds, or use randomness other than `rc_core::rng`.
//!
//! Lifecycle: `WeatherState` holds the default (Clear, Noon) in menus; `OnEnter(RacePhase::Setup)`
//! plans the attempt (same seed → same weather on every restart) and resets the state;
//! `SimSet::Weather` advances it once per fixed tick; `OnEnter(AppState::Unloading)` removes it and
//! the default returns when `Unloading` ends.

mod plan;
mod sim;

use bevy::prelude::*;
use rc_core::prelude::*;

use sim::WeatherSim;

/// Weather simulation.
pub struct WeatherPlugin;

impl Plugin for WeatherPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WeatherState>()
            .add_systems(OnEnter(RacePhase::Setup), reset)
            .add_systems(FixedUpdate, tick.in_set(SimSet::Weather))
            .add_systems(OnEnter(AppState::Unloading), remove)
            .add_systems(OnExit(AppState::Unloading), |mut commands: Commands| {
                commands.init_resource::<WeatherState>();
            });
    }
}

/// Plans the attempt's weather and resets `WeatherState` to its settled start.
fn reset(mut commands: Commands, config: Option<Res<RaceConfig>>, catalog: Option<Res<Catalog>>) {
    let config = config.as_deref();
    let track = config
        .zip(catalog.as_deref())
        .and_then(|(config, catalog)| catalog.track(&config.variant.track));
    let (setup, seed) = config.map_or((WeatherSetup::default(), 0), |c| (c.weather, c.seed));
    let plan = plan::plan(setup, seed, track);
    info!("weather plan: {plan:?}");
    let sim = WeatherSim::new(plan);
    commands.insert_resource(sim.initial_state());
    commands.insert_resource(sim);
}

/// One weather tick, driven by the leader's share of the race distance (`TrackSummary::length` ×
/// laps; without a track summary the Dynamic change never starts).
fn tick(
    sim: Option<ResMut<WeatherSim>>,
    state: Option<ResMut<WeatherState>>,
    config: Option<Res<RaceConfig>>,
    track: Option<Res<TrackSummary>>,
    racers: Query<&RaceProgress>,
    mut events: MessageWriter<WeatherEvent>,
) {
    let (Some(mut sim), Some(mut state)) = (sim, state) else {
        return;
    };
    let race_length = track.map_or(0.0, |t| t.length) * config.map_or(0.0, |c| f32::from(c.laps));
    let leader = racers.iter().map(|p| p.total_distance).fold(0.0, f32::max);
    let progress = if race_length > 0.0 {
        leader / race_length
    } else {
        0.0
    };
    let mut next = state.clone();
    sim.step(&mut next, progress, |event| {
        events.write(event);
    });
    // Readers can use change detection: settled weather does not mark the resource changed.
    state.set_if_neq(next);
}

/// Session cleanup (the menu default comes back on leaving `Unloading`).
fn remove(mut commands: Commands) {
    commands.remove_resource::<WeatherState>();
    commands.remove_resource::<WeatherSim>();
}
