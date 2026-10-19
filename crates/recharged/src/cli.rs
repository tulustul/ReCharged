//! Command line → [`AutomationConfig`] (ARCHITECTURE §8), std only.

use std::path::PathBuf;
use std::str::FromStr;

use rc_core::prelude::*;

/// Printed for `--help` and after a usage error.
pub const USAGE: &str = "\
Usage: recharged [flags]
  --user-data-dir <path>   saves, settings and logs here (agents: target/agent-data/<task>)
  --assets-dir <path>      assets root (default: ./assets, then next to the executable)
  --unlock-all             everything unlocked, in memory only
  --quick-race <track>     skip the menus; options:
      [--reverse] [--mirror] [--car <id>] [--ai <n>] [--laps <n>] [--weather <w>] [--time <t>]
      [--players <n>] [--seed <n>] [--mode tt|versus|cup:<id>]
      w: clear|cloudy|rain|storm|fog|snow|random|dynamic[:<start>]   t: morning|noon|sunset|night
  --autopilot              local players driven by the AI
  --input-script <file>    play scripted keyboard/gamepad input (format: docs/design/automation.md)
  --exit-after <secs>      quit after N seconds of wall time (non-zero exit if the UI failed or
                           the input script did not finish)
  --screenshot <path> [--screenshot-at <s>] [--screenshot-every <s>]   capture the window (at 3 s by default)
  --headless-race          with --quick-race: no window/render/audio, AI drives everyone; prints
                           the results as JSON
  --soak <minutes>         chain random races; assert counts return to baseline
  --soak-seed <n>          seed of the soak's random race sequence (default: from the clock)
  --bench <track>_<n>p     autopilot race with n players (+ race options): frame times as JSON
                           (implies --unlock-all)
  --calibrate-medals       headless Time Trials on every layout -> assets/data/medals.ron
  --capture-previews       render track previews and car thumbnails
  --track-tour <track>     screenshots along the racing line (implies --quick-race <track>)
  --export-bindings <path> write the TypeScript bindings and exit
  --window <W>x<H>         initial window size (overrides the settings' window size; --bench: 1920x1080)
  -h, --help               this text";

/// Why the command line was rejected.
#[derive(Debug, PartialEq, Eq)]
pub enum CliError {
    /// `--help` / `-h`.
    Help,
    /// A usage error (exit code 2).
    Invalid(String),
}

fn invalid(message: impl Into<String>) -> CliError {
    CliError::Invalid(message.into())
}

/// Options that only make sense with `--quick-race` / `--screenshot`.
const RACE_OPTIONS: [&str; 10] = [
    "--reverse",
    "--mirror",
    "--car",
    "--ai",
    "--laps",
    "--weather",
    "--time",
    "--players",
    "--seed",
    "--mode",
];
const SHOT_OPTIONS: [&str; 2] = ["--screenshot-at", "--screenshot-every"];
/// Flags that need the window; headless runs reject them.
const WINDOWED: [&str; 8] = [
    "--input-script",
    "--exit-after",
    "--screenshot",
    "--bench",
    "--soak",
    "--capture-previews",
    "--track-tour",
    "--window",
];
/// `--bench` races this seed unless `--seed` says otherwise (the same field every run).
const BENCH_SEED: u64 = 1;
/// `--bench` window size unless `--window` says otherwise (the budget's resolution, §9).
const BENCH_WINDOW: (u32, u32) = (1920, 1080);

/// Parses the arguments after the program name. Accepts `--flag value` and `--flag=value`.
pub fn parse<S: AsRef<str>>(args: &[S]) -> Result<AutomationConfig, CliError> {
    let mut cfg = AutomationConfig::default();
    let mut race = QuickRaceArgs::default();
    let mut shot = ScreenshotPlan::default();
    let (mut quick_track, mut shot_path) = (None, None);
    let mut seen: Vec<&str> = Vec::new();

    let mut it = args.iter().map(AsRef::as_ref);
    while let Some(arg) = it.next() {
        let (flag, inline) = match arg.split_once('=') {
            Some((f, v)) if f.starts_with("--") => (f, Some(v)),
            _ => (arg, None),
        };
        if inline.is_some() && !takes_value(flag) {
            return Err(invalid(format!("{flag} takes no value")));
        }
        let mut value = || -> Result<String, CliError> {
            inline
                .map(str::to_owned)
                .or_else(|| it.next().map(str::to_owned))
                .ok_or_else(|| invalid(format!("{flag} needs a value")))
        };
        match flag {
            "-h" | "--help" => return Err(CliError::Help),
            "--user-data-dir" => cfg.user_data_dir = Some(value()?.into()),
            "--assets-dir" => cfg.assets_dir = Some(value()?.into()),
            "--unlock-all" => cfg.unlock_all = true,
            "--quick-race" => quick_track = Some(TrackId::new(value()?)),
            "--reverse" => race.reverse = true,
            "--mirror" => race.mirror = true,
            "--car" => race.car = Some(CarId::new(value()?)),
            "--ai" => race.ai = Some(number(flag, &value()?)?),
            "--laps" => race.laps = Some(number(flag, &value()?)?),
            "--weather" => race.weather = Some(weather(&value()?)?),
            "--time" => race.time = Some(time_of_day(&value()?)?),
            "--players" => race.players = Some(players(&value()?)?),
            "--seed" => race.seed = Some(number(flag, &value()?)?),
            "--mode" => race.mode = mode(&value()?)?,
            "--autopilot" => cfg.autopilot = true,
            "--input-script" => cfg.input_script = Some(value()?.into()),
            "--exit-after" => cfg.exit_after = Some(seconds(flag, &value()?)?),
            "--screenshot" => shot_path = Some(PathBuf::from(value()?)),
            "--screenshot-at" => shot.at = Some(seconds(flag, &value()?)?),
            "--screenshot-every" => shot.every = Some(seconds(flag, &value()?)?),
            "--headless-race" => cfg.headless_race = true,
            "--soak" => cfg.soak_minutes = Some(seconds(flag, &value()?)?),
            "--soak-seed" => cfg.soak_seed = Some(number(flag, &value()?)?),
            "--bench" => cfg.bench = Some(value()?),
            "--calibrate-medals" => cfg.calibrate_medals = true,
            "--capture-previews" => cfg.capture_previews = true,
            "--track-tour" => cfg.track_tour = Some(TrackId::new(value()?)),
            "--export-bindings" => cfg.export_bindings = Some(value()?.into()),
            "--window" => cfg.window = Some(window_size(&value()?)?),
            _ => return Err(invalid(format!("unknown flag `{arg}`"))),
        }
        seen.push(flag);
    }

    let orphans = |options: &[&str]| -> Vec<&str> {
        seen.iter()
            .copied()
            .filter(|f| options.contains(f))
            .collect()
    };
    let first_of = |options: &[&str]| seen.iter().copied().find(|f| options.contains(f));
    if cfg.headless_race || cfg.calibrate_medals {
        let run = if cfg.calibrate_medals {
            "--calibrate-medals"
        } else {
            "--headless-race"
        };
        let race_flags = ["--quick-race", "--headless-race"];
        let calibrate_conflict = cfg
            .calibrate_medals
            .then(|| first_of(&race_flags))
            .flatten();
        if let Some(other) = first_of(&WINDOWED).or(calibrate_conflict) {
            return Err(invalid(format!("{run} cannot be combined with {other}")));
        }
        if cfg.headless_race && quick_track.is_none() {
            return Err(invalid("--headless-race needs --quick-race <track>"));
        }
    }
    if let Some(name) = &cfg.bench {
        if let Some(other) = first_of(&["--quick-race", "--players"]) {
            return Err(invalid(format!(
                "--bench sets the race itself; drop {other}"
            )));
        }
        let (track, players) = bench_scenario(name)?;
        quick_track = Some(track);
        race.players = Some(players);
        race.seed = race.seed.or(Some(BENCH_SEED));
        cfg.window = cfg.window.or(Some(BENCH_WINDOW));
        cfg.autopilot = true;
        // The bench ignores the profile like it ignores the settings: any track, locked or not.
        cfg.unlock_all = true;
    }
    // `--track-tour <t>` without `--quick-race` tours that track.
    let quick_track = quick_track.or_else(|| cfg.track_tour.clone());
    match quick_track {
        Some(track) => cfg.quick_race = Some(QuickRaceArgs { track, ..race }),
        None if !orphans(&RACE_OPTIONS).is_empty() => {
            return Err(invalid(format!(
                "{} requires --quick-race",
                orphans(&RACE_OPTIONS).join(", ")
            )));
        }
        None => {}
    }
    match shot_path {
        Some(path) => cfg.screenshot = Some(ScreenshotPlan { path, ..shot }),
        None if !orphans(&SHOT_OPTIONS).is_empty() => {
            return Err(invalid(format!(
                "{} requires --screenshot",
                orphans(&SHOT_OPTIONS).join(", ")
            )));
        }
        None => {}
    }
    Ok(cfg)
}

/// `toy_town_2p` → (`toy_town`, 2).
fn bench_scenario(text: &str) -> Result<(TrackId, u8), CliError> {
    let parsed = text
        .rsplit_once('_')
        .and_then(|(track, n)| Some((track, n.strip_suffix('p')?.parse::<u8>().ok()?)))
        .filter(|&(track, n)| !track.is_empty() && (1..=MAX_LOCAL_PLAYERS as u8).contains(&n));
    let (track, players) = parsed.ok_or_else(|| {
        invalid(format!(
            "--bench expects <track>_<n>p with n 1-{MAX_LOCAL_PLAYERS} (e.g. toy_town_2p), got `{text}`"
        ))
    })?;
    Ok((TrackId::new(track), players))
}

/// `1280x720` → `(1280, 720)`.
fn window_size(text: &str) -> Result<(u32, u32), CliError> {
    let parsed = text
        .split_once(['x', 'X'])
        .and_then(|(w, h)| Some((w.parse::<u32>().ok()?, h.parse::<u32>().ok()?)))
        .filter(|&(w, h)| (320..=7680).contains(&w) && (240..=4320).contains(&h));
    parsed.ok_or_else(|| {
        invalid(format!(
            "--window expects <W>x<H> (e.g. 1280x720), got {text:?}"
        ))
    })
}

fn takes_value(flag: &str) -> bool {
    !matches!(
        flag,
        "--unlock-all"
            | "--reverse"
            | "--mirror"
            | "--autopilot"
            | "--headless-race"
            | "--calibrate-medals"
            | "--capture-previews"
    )
}

fn players(text: &str) -> Result<u8, CliError> {
    match text.parse::<u8>() {
        Ok(n) if (1..=MAX_LOCAL_PLAYERS as u8).contains(&n) => Ok(n),
        _ => Err(invalid(format!(
            "--players must be 1-{MAX_LOCAL_PLAYERS}, got `{text}`"
        ))),
    }
}

fn number<T: FromStr>(flag: &str, text: &str) -> Result<T, CliError> {
    text.parse()
        .map_err(|_| invalid(format!("{flag}: `{text}` is not a valid number")))
}

fn seconds(flag: &str, text: &str) -> Result<f32, CliError> {
    match text.parse::<f32>() {
        Ok(s) if s.is_finite() && s >= 0.0 => Ok(s),
        _ => Err(invalid(format!(
            "{flag}: `{text}` is not a duration in seconds"
        ))),
    }
}

fn weather_kind(text: &str) -> Option<WeatherKind> {
    WeatherKind::ALL
        .into_iter()
        .find(|k| format!("{k:?}").eq_ignore_ascii_case(text))
}

fn weather(text: &str) -> Result<WeatherChoice, CliError> {
    let bad = || invalid(format!("--weather: unknown weather `{text}`"));
    match text.split_once(':') {
        Some(("dynamic", start)) => Ok(WeatherChoice::Dynamic {
            start: weather_kind(start).ok_or_else(bad)?,
        }),
        None if text == "dynamic" => Ok(WeatherChoice::Dynamic {
            start: WeatherKind::Clear,
        }),
        None if text == "random" => Ok(WeatherChoice::Random),
        None => weather_kind(text).map(WeatherChoice::Fixed).ok_or_else(bad),
        Some(_) => Err(bad()),
    }
}

fn time_of_day(text: &str) -> Result<TimeOfDay, CliError> {
    TimeOfDay::ALL
        .into_iter()
        .find(|t| format!("{t:?}").eq_ignore_ascii_case(text))
        .ok_or_else(|| invalid(format!("--time: unknown time of day `{text}`")))
}

fn mode(text: &str) -> Result<QuickRaceMode, CliError> {
    match text {
        "quick" => Ok(QuickRaceMode::Quick),
        "tt" => Ok(QuickRaceMode::TimeTrial),
        "versus" => Ok(QuickRaceMode::Versus),
        _ => match text.strip_prefix("cup:") {
            Some(cup) if !cup.is_empty() => Ok(QuickRaceMode::Cup(CupId::new(cup))),
            _ => Err(invalid(format!(
                "--mode: expected tt, versus or cup:<id>, got `{text}`"
            ))),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(args: &[&str]) -> AutomationConfig {
        parse(args).unwrap_or_else(|e| panic!("{args:?}: {e:?}"))
    }

    fn err(args: &[&str]) -> String {
        match parse(args) {
            Err(CliError::Invalid(m)) => m,
            other => panic!("{args:?} should be invalid, got {other:?}"),
        }
    }

    #[test]
    fn parses_the_acceptance_command_lines() {
        let cfg = ok(&[
            "--user-data-dir",
            "target/agent-data/T01",
            "--exit-after",
            "12",
            "--screenshot-at",
            "8",
            "--screenshot",
            "/tmp/t01.png",
        ]);
        assert_eq!(cfg.user_data_dir, Some("target/agent-data/T01".into()));
        assert_eq!(cfg.exit_after, Some(12.0));
        assert_eq!(
            cfg.screenshot,
            Some(ScreenshotPlan {
                path: "/tmp/t01.png".into(),
                at: Some(8.0),
                every: None
            })
        );
        assert!(cfg.is_automated());

        let cfg = ok(&[
            "--quick-race",
            "test_oval",
            "--reverse",
            "--car=zip",
            "--ai",
            "3",
            "--laps",
            "2",
            "--weather",
            "dynamic:fog",
            "--time",
            "Night",
            "--seed",
            "42",
            "--mode",
            "cup:pro_cup",
        ]);
        let race = cfg.quick_race.expect("quick race");
        assert_eq!(race.track, TrackId::new("test_oval"));
        assert!(race.reverse && !race.mirror);
        assert_eq!(
            (race.car, race.ai, race.laps, race.seed),
            (Some(CarId::new("zip")), Some(3), Some(2), Some(42))
        );
        assert_eq!(
            race.weather,
            Some(WeatherChoice::Dynamic {
                start: WeatherKind::Fog
            })
        );
        assert_eq!(race.time, Some(TimeOfDay::Night));
        assert_eq!(race.mode, QuickRaceMode::Cup(CupId::new("pro_cup")));
    }

    #[test]
    fn parses_every_other_flag() {
        let cfg = ok(&[
            "--assets-dir",
            "a",
            "--unlock-all",
            "--autopilot",
            "--input-script",
            "s.script",
            "--soak",
            "30",
            "--soak-seed",
            "42",
            "--capture-previews",
            "--track-tour",
            "toy_town",
            "--export-bindings",
            "ui/src/bevy.ts",
        ]);
        assert_eq!(cfg.assets_dir, Some("a".into()));
        assert!(cfg.unlock_all && cfg.autopilot && cfg.capture_previews);
        assert_eq!(cfg.soak_minutes, Some(30.0));
        assert_eq!(cfg.track_tour, Some(TrackId::new("toy_town")));
        assert_eq!(cfg.export_bindings, Some("ui/src/bevy.ts".into()));
        assert_eq!(cfg.input_script, Some("s.script".into()));
        assert_eq!(cfg.soak_seed, Some(42));
        assert_eq!(ok(&["--window", "1280x720"]).window, Some((1280, 720)));
        assert!(err(&["--window", "big"]).contains("--window"));
        assert!(ok(&[]) == AutomationConfig::default() && !ok(&[]).is_automated());
    }

    #[test]
    fn headless_runs_need_a_race_and_no_window() {
        let cfg = ok(&["--headless-race", "--quick-race", "toy_town", "--ai", "7"]);
        assert!(cfg.headless_race && cfg.quick_race.is_some());
        assert!(err(&["--headless-race"]).contains("needs --quick-race"));
        let windowed = err(&[
            "--headless-race",
            "--quick-race",
            "x",
            "--screenshot",
            "s.png",
        ]);
        assert!(windowed.contains("cannot be combined with --screenshot"));
        assert!(ok(&["--calibrate-medals", "--unlock-all"]).calibrate_medals);
        let race = err(&["--calibrate-medals", "--quick-race", "toy_town"]);
        assert!(race.contains("--calibrate-medals cannot be combined with --quick-race"));
        assert!(err(&["--calibrate-medals", "--exit-after", "9"]).contains("--exit-after"));
    }

    #[test]
    fn bench_scenarios_set_up_an_autopilot_race() {
        let cfg = ok(&["--bench", "toy_town_4p", "--weather", "rain"]);
        assert!(cfg.autopilot && cfg.is_automated());
        assert!(cfg.unlock_all, "locked tracks bench too");
        assert_eq!(cfg.bench.as_deref(), Some("toy_town_4p"));
        let race = cfg.quick_race.expect("bench race");
        assert_eq!((race.track.as_str(), race.players), ("toy_town", Some(4)));
        assert_eq!(
            (race.seed, race.weather),
            (
                Some(BENCH_SEED),
                Some(WeatherChoice::Fixed(WeatherKind::Rain))
            )
        );
        let race = ok(&["--bench", "maple_street_1p", "--seed", "5"])
            .quick_race
            .expect("race");
        assert_eq!(
            (race.track.as_str(), race.players, race.seed),
            ("maple_street", Some(1), Some(5))
        );
        for bad in ["toy_town", "toy_town_5p", "_2p", "toy_town_xp"] {
            assert!(err(&["--bench", bad]).contains("<track>_<n>p"), "{bad}");
        }
        assert!(err(&["--bench", "toy_town_1p", "--players", "2"]).contains("drop --players"));
        assert!(
            err(&["--bench", "toy_town_1p", "--quick-race", "x"]).contains("drop --quick-race")
        );
    }

    #[test]
    fn rejects_bad_command_lines() {
        assert!(err(&["--frobnicate"]).contains("unknown flag"));
        assert!(err(&["--exit-after"]).contains("needs a value"));
        assert!(err(&["--exit-after", "soon"]).contains("duration"));
        assert!(err(&["--laps", "3"]).contains("requires --quick-race"));
        assert!(err(&["--screenshot-at", "2"]).contains("requires --screenshot"));
        assert!(err(&["--quick-race", "x", "--weather", "hail"]).contains("unknown weather"));
        assert!(err(&["--quick-race", "x", "--mode", "cup:"]).contains("--mode"));
        assert!(err(&["--quick-race", "x", "--players", "5"]).contains("--players"));
        assert!(err(&["--unlock-all=yes"]).contains("takes no value"));
        assert_eq!(parse(&["-h"]), Err(CliError::Help));
    }
}
