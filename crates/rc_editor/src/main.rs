//! `rc_editor <track> [options]` (see [`USAGE`]): the track editor on
//! `assets/data/tracks/<track>.ron`. It runs the whole game ([`recharged::game`]) plus
//! [`rc_editor::EditorPlugin`]; saves, settings and records go to `<user data>/editor`, apart
//! from the game's.

use std::path::PathBuf;

use bevy::prelude::*;
use rc_core::prelude::*;
use rc_track::TrackDef;

const USAGE: &str = "usage: rc_editor <track> [--assets-dir <dir>] [--user-data-dir <dir>]
                 [--screenshot <png> [--screenshot-at <s>]] [--exit-after <s>]

  <track>                  track id: assets/data/tracks/<track>.ron
  --assets-dir <dir>       assets root (default: ./assets, then next to the executable)
  --user-data-dir <dir>    saves, settings and logs (default: the game's, + /editor)
  --screenshot <png>       save the window to <png> (at --screenshot-at seconds, default 3)
  --exit-after <s>         quit after <s> seconds of wall time

  F5 test drive / back to editing · Ctrl+S save · right mouse look · WASD move · Q/E down/up";

struct Args {
    track: String,
    assets_dir: Option<PathBuf>,
    user_data_dir: Option<PathBuf>,
    screenshot: Option<ScreenshotPlan>,
    exit_after: Option<f32>,
}

fn parse(args: &[String]) -> Result<Args, String> {
    let (mut track, mut assets_dir, mut user_data_dir) = (None, None, None);
    let (mut screenshot, mut screenshot_at, mut exit_after) = (None, None, None);
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        let mut value = || it.next().ok_or(format!("{arg} needs a value"));
        let secs = |v: &String| v.parse::<f32>().map_err(|e| format!("{arg} {v}: {e}"));
        match arg.as_str() {
            "--assets-dir" => assets_dir = Some(PathBuf::from(value()?)),
            "--user-data-dir" => user_data_dir = Some(PathBuf::from(value()?)),
            "--screenshot" => screenshot = Some(PathBuf::from(value()?)),
            "--screenshot-at" => screenshot_at = Some(secs(value()?)?),
            "--exit-after" => exit_after = Some(secs(value()?)?),
            a if !a.starts_with('-') && track.is_none() => track = Some(a.to_owned()),
            a => return Err(format!("unexpected argument {a}")),
        }
    }
    Ok(Args {
        track: track.ok_or("no track given")?,
        assets_dir,
        user_data_dir,
        screenshot: screenshot.map(|path| ScreenshotPlan {
            path,
            at: screenshot_at,
            every: None,
        }),
        exit_after,
    })
}

fn main() -> AppExit {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("{USAGE}");
        return AppExit::Success;
    }
    let args = match parse(&args) {
        Ok(args) => args,
        Err(e) => {
            eprintln!("error: {e}\n\n{USAGE}");
            return AppExit::from_code(2);
        }
    };
    let mut paths = UserPaths::resolve(args.assets_dir.as_deref(), args.user_data_dir.as_deref());
    if args.user_data_dir.is_none() {
        paths.data_dir = paths.data_dir.join("editor");
    }
    let def = match TrackDef::load(&paths.assets_dir, &args.track) {
        Ok(def) => def,
        Err(e) => {
            eprintln!("error: {e}");
            return AppExit::from_code(2);
        }
    };
    let bundle = match recharged::checked_ui_bundle() {
        Ok(bundle) => bundle,
        Err(e) => {
            eprintln!("{e}");
            return AppExit::error();
        }
    };
    let config = AutomationConfig {
        unlock_all: true,
        assets_dir: args.assets_dir,
        user_data_dir: args.user_data_dir,
        screenshot: args.screenshot,
        exit_after: args.exit_after,
        ..default()
    };
    let mut app = recharged::game(config, paths, bundle);
    app.add_plugins(rc_editor::EditorPlugin {
        track: TrackId::new(args.track),
        def,
    });
    app.run()
}
