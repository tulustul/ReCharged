//! `--headless-race` (ARCHITECTURE §8): no window, render or audio — `MinimalPlugins` + states +
//! the sim plugins + rc_game's flow, one fixed tick per update, as fast as the CPU allows, every
//! racer driven by the AI (humans on autopilot). Prints the results as JSON on stdout.
//! [`run_race`] is shared by `--calibrate-medals` and the determinism test.

use bevy::log::tracing_subscriber::{self, EnvFilter};
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::time::TimeUpdateStrategy;
use rc_core::prelude::*;
use serde_json::json;

/// A race without results after this much simulated time failed (the soft-lock guard ends every
/// race that has a finisher long before).
const TIMEOUT_SECS: f32 = 30.0 * 60.0;

/// A finished race: its setup and results.
#[derive(Debug, Clone)]
pub struct Finished {
    /// The session's setup (seed, layout, grid).
    pub config: RaceConfig,
    /// rc_race's results.
    pub results: RaceResults,
}

impl Finished {
    /// The JSON printed by `--headless-race` (identical for identical seeded runs).
    pub fn to_json(&self) -> serde_json::Value {
        let c = &self.config;
        json!({
            "variant": c.variant,
            "mode": c.mode,
            "laps": c.laps,
            "seed": c.seed,
            "geometry_hash": self.results.geometry_hash,
            "entries": self.results.entries,
        })
    }
}

/// The headless game for `config` (every human on autopilot).
pub fn app(mut config: AutomationConfig, paths: UserPaths) -> App {
    config.autopilot = true;
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, TransformPlugin, StatesPlugin))
        .insert_resource(TimeUpdateStrategy::FixedTimesteps(1))
        .insert_resource(paths)
        .insert_resource(config);
    recharged::add_sim_plugins(&mut app);
    app.add_plugins(rc_game::GameFlowPlugin);
    app
}

/// Steps a fresh headless `app` until its race has results.
pub fn run_race(app: &mut App) -> Result<Finished, String> {
    app.finish();
    app.cleanup();
    for _ in 0..secs_to_ticks(TIMEOUT_SECS) {
        app.update();
        if let Some(exit) = app.should_exit() {
            return Err(format!("the game stopped before the race ended ({exit:?})"));
        }
        let world = app.world();
        let finished = world
            .get_resource::<State<RacePhase>>()
            .is_some_and(|p| *p.get() == RacePhase::Finished);
        if let (true, Some(results), Some(config)) = (
            finished,
            world.get_resource::<RaceResults>(),
            world.get_resource::<RaceConfig>(),
        ) {
            return Ok(Finished {
                config: config.clone(),
                results: results.clone(),
            });
        }
    }
    Err(format!("no results after {TIMEOUT_SECS} s"))
}

/// `--headless-race`: one race, results as JSON on stdout.
pub fn race(config: AutomationConfig, paths: UserPaths) -> AppExit {
    init_logging();
    match run_race(&mut app(config, paths)) {
        Ok(done) => {
            println!("{}", done.to_json());
            AppExit::Success
        }
        Err(e) => {
            eprintln!("--headless-race: {e}");
            AppExit::error()
        }
    }
}

/// Logs to stderr (stdout carries the results); `RUST_LOG` overrides the `warn` default. Bevy's
/// `LogPlugin` is not used: calibration builds many apps in one process.
pub fn init_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .try_init();
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    /// A seeded quick race in a fresh data dir, as JSON.
    fn seeded_race(name: &str, args: QuickRaceArgs) -> String {
        init_logging();
        let data_dir =
            std::env::temp_dir().join(format!("recharged-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&data_dir);
        let paths = UserPaths {
            assets_dir: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets"),
            data_dir: data_dir.clone(),
        };
        let config = AutomationConfig {
            quick_race: Some(args),
            headless_race: true,
            ..default()
        };
        let done = run_race(&mut app(config, paths)).unwrap_or_else(|e| panic!("{e}"));
        let _ = std::fs::remove_dir_all(&data_dir);
        assert!(done.results.entries.iter().any(|e| e.finished), "{done:?}");
        done.to_json().to_string()
    }

    #[test]
    fn a_seeded_headless_race_repeats_exactly() {
        let args = || QuickRaceArgs {
            track: TrackId::new("test_oval"),
            ai: Some(3),
            laps: Some(1),
            seed: Some(7),
            ..default()
        };
        assert_eq!(
            seeded_race("determinism-a", args()),
            seeded_race("determinism-b", args())
        );
    }

    /// The acceptance race: `cargo test -p recharged -- --ignored full_field`.
    #[test]
    #[ignore = "long: a 3-lap race of 8 cars, twice"]
    fn a_full_field_on_toy_town_repeats_exactly() {
        let args = || QuickRaceArgs {
            track: TrackId::new("toy_town"),
            ai: Some(7),
            seed: Some(1),
            ..default()
        };
        assert_eq!(
            seeded_race("full-field-a", args()),
            seeded_race("full-field-b", args())
        );
    }
}
