//! `--bench <track>_<n>p` (ARCHITECTURE §9, DoD 7). The CLI turns the scenario into an autopilot
//! quick race with `n` players (seed 1, 1920×1080 physical px unless `--window`) and the binary
//! starts it with default settings, vsync off and continuous updates. After GO and a [`WARMUP_SECS`] warm-up, the wall-clock frame
//! times of [`MEASURE_SECS`] are measured and printed as JSON on stdout; the game then exits —
//! non-zero if the race never started, ended early or the UI failed.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use rc_core::prelude::*;
use rc_ui::UiStatus;
use serde_json::json;

/// After GO, frames are not measured for this long (s): pipeline compiles, first effects.
const WARMUP_SECS: f32 = 5.0;
/// Measured wall-clock time (s).
const MEASURE_SECS: f32 = 20.0;
/// The race must reach GO this long after startup (s).
const START_TIMEOUT_SECS: f32 = 180.0;

/// `settings` (the defaults, or `--graphics` over them) with vsync off: the same quality on every
/// machine whatever the profile's settings. Inserted before rc_game's boot, which then keeps it
/// (and never saves it).
pub fn settings(mut settings: Settings) -> Settings {
    settings.graphics.vsync = false;
    settings
}

/// Measures the race and exits (like every automated run, the window renders without vsync and
/// updates continuously even unfocused: `main.rs`).
pub fn plugin(app: &mut App) {
    app.add_systems(Last, measure.before(bevy::app::OnAppExitSystems));
}

#[derive(Default)]
struct Run {
    go: Option<f32>,
    frames: Vec<f32>,
    done: bool,
}

fn measure(
    cfg: Res<AutomationConfig>,
    time: Res<Time<Real>>,
    phase: Option<Res<State<RacePhase>>>,
    ui: Option<Res<UiStatus>>,
    window: Option<Single<&Window, With<PrimaryWindow>>>,
    mut run: Local<Run>,
    mut exit: MessageWriter<AppExit>,
) {
    let now = time.elapsed_secs();
    let racing = phase.is_some_and(|p| *p.get() == RacePhase::Racing);
    let mut fail = |why: &str| {
        error!("--bench: {why}");
        exit.write(AppExit::error());
    };
    if run.done {
        return;
    }
    let Some(go) = run.go else {
        if racing {
            info!("--bench: GO; measuring {MEASURE_SECS} s after {WARMUP_SECS} s");
            run.go = Some(now);
            run.frames.reserve(4096);
        } else if now > START_TIMEOUT_SECS {
            run.done = true;
            fail("the race did not start");
        }
        return;
    };
    if now - go < WARMUP_SECS {
        return;
    }
    if !racing {
        run.done = true;
        return fail("the race ended before the measurement did");
    }
    run.frames.push(time.delta_secs());
    if now - go < WARMUP_SECS + MEASURE_SECS {
        return;
    }
    run.done = true;
    if !ui.is_some_and(|ui| ui.ready && ui.errors == 0) {
        return fail("the UI is not healthy");
    }
    let size = window.map(|w| [w.width(), w.height()]);
    let report = report(cfg.bench.as_deref().unwrap_or_default(), size, &run.frames);
    println!("{report}");
    exit.write(AppExit::Success);
}

/// The JSON line: frame time percentiles (ms) and frames per second, rounded to 0.01.
fn report(scenario: &str, window: Option<[f32; 2]>, frames: &[f32]) -> serde_json::Value {
    let mut sorted: Vec<f64> = frames.iter().map(|&f| f64::from(f)).collect();
    sorted.sort_by(f64::total_cmp);
    let round = |x: f64| (x * 100.0).round() / 100.0;
    let total: f64 = sorted.iter().sum();
    let ms = |p: f64| round(sorted[((sorted.len() - 1) as f64 * p).round() as usize] * 1000.0);
    json!({
        "scenario": scenario,
        "window": window,
        "frames": sorted.len(),
        "secs": round(total),
        "fps": round(sorted.len() as f64 / total),
        "frame_ms": { "p50": ms(0.5), "p95": ms(0.95), "p99": ms(0.99), "max": ms(1.0) },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_percentiles_and_fps() {
        let frames: Vec<f32> = (1..=100).map(|i| i as f32 / 1000.0).collect();
        let r = report("toy_town_1p", Some([1920.0, 1080.0]), &frames);
        assert_eq!(r["frames"], 100);
        assert_eq!(r["frame_ms"]["p50"], 51.0);
        assert_eq!(r["frame_ms"]["p95"], 95.0);
        assert_eq!(r["frame_ms"]["p99"], 99.0);
        assert_eq!(r["frame_ms"]["max"], 100.0);
        assert_eq!(r["fps"], 19.8, "100 frames in 5.05 s");
        assert_eq!(r["secs"], 5.05);
        assert!(!settings(Settings::default()).graphics.vsync);
    }
}
