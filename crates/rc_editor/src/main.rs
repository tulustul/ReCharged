//! `rc_editor <track> [options]` (see [`USAGE`]): the track editor on
//! `assets/data/tracks/<track>.ron`; `rc_editor new <id> --env <environment>` creates the track
//! first. It runs the whole game ([`recharged::game`]) plus
//! [`rc_editor::EditorPlugin`]; saves, settings and records go to `<user data>/editor`, apart
//! from the game's.

use std::path::{Path, PathBuf};

use bevy::prelude::*;
use rc_core::prelude::*;
use rc_render::EnvironmentDef;
use rc_track::{PropCatalog, RibbonStyle, TRACKS_DIR};

const USAGE: &str = "usage: rc_editor <track> [--assets-dir <dir>] [--user-data-dir <dir>]
                 [--screenshot <png> [--screenshot-at <s>]] [--exit-after <s>]
       rc_editor new <id> --env <environment> [--name <name>] [--style <style>] [options]

  <track>                  track id: assets/data/tracks/<track>.ron
  new <id>                 create a hidden track (an oval of <style>: ToyTrack, Asphalt,
                           CarpetRunner or Invisible; default ToyTrack) in <environment>
                           (assets/data/environments/<environment>.ron), then edit it
  --assets-dir <dir>       assets root (default: ./assets, then next to the executable)
  --user-data-dir <dir>    saves, settings and logs (default: the game's, + /editor)
  --screenshot <png>       save the window to <png> (at --screenshot-at seconds, default 3)
  --exit-after <s>         quit after <s> seconds of wall time
  --bake                   bake the irradiance volumes (every allowed time, normal and mirror)
                           into assets/data/lighting/<track>/, then quit

  Keys: F5 test drive / back to editing · Ctrl+S save · Ctrl+Z / Ctrl+Y undo / redo
        Delete · Insert (after the selected point) · Ctrl+D duplicate · T top view · F focus
  Mouse: left click select · left drag move (Alt: height, Ctrl: snap) · right drag look
        WASD move · Q/E down/up · Shift faster";

/// `rc_editor new …`.
struct NewTrack {
    environment: String,
    name: Option<String>,
    style: Option<String>,
}

struct Args {
    track: String,
    new: Option<NewTrack>,
    assets_dir: Option<PathBuf>,
    user_data_dir: Option<PathBuf>,
    screenshot: Option<ScreenshotPlan>,
    exit_after: Option<f32>,
    bake: bool,
}

fn parse(args: &[String]) -> Result<Args, String> {
    let (mut track, mut assets_dir, mut user_data_dir) = (None, None, None);
    let (mut screenshot, mut screenshot_at, mut exit_after) = (None, None, None);
    let (mut new, mut env, mut name, mut style) = (false, None, None, None);
    let mut bake = false;
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
            "--env" => env = Some(value()?.clone()),
            "--name" => name = Some(value()?.clone()),
            "--style" => style = Some(value()?.clone()),
            "--bake" => bake = true,
            "new" if track.is_none() && !new => new = true,
            a if !a.starts_with('-') && track.is_none() => track = Some(a.to_owned()),
            a => return Err(format!("unexpected argument {a}")),
        }
    }
    let new = match (new, env) {
        (true, Some(environment)) => Some(NewTrack {
            environment,
            name,
            style,
        }),
        (true, None) => return Err("new needs --env <environment>".into()),
        (false, _) if name.is_some() || style.is_some() => {
            return Err("--name and --style go with new".into());
        }
        (false, _) => None,
    };
    Ok(Args {
        track: track.ok_or("no track given")?,
        new,
        assets_dir,
        user_data_dir,
        screenshot: screenshot.map(|path| ScreenshotPlan {
            path,
            at: screenshot_at,
            every: None,
        }),
        exit_after,
        bake,
    })
}

/// Writes a new track's layout and catalog entry (refusing to replace an existing track).
fn create(assets_dir: &Path, id: &str, new: &NewTrack) -> Result<(), String> {
    let valid = id
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
    if id.is_empty() || !valid {
        return Err(format!(
            "track id \"{id}\": use lowercase letters, digits and _"
        ));
    }
    let path = assets_dir.join(TRACKS_DIR).join(format!("{id}.ron"));
    if path.exists() {
        return Err(format!(
            "{} exists: edit it with rc_editor {id}",
            path.display()
        ));
    }
    let indoor = EnvironmentDef::load(assets_dir, &new.environment)?
        .room
        .is_some();
    let style = match &new.style {
        Some(s) => rc_editor::fields::parse::<RibbonStyle>(s)?,
        None => RibbonStyle::ToyTrack,
    };
    let track = TrackId::new(id);
    let name = new.name.clone().unwrap_or_else(|| id.replace('_', " "));
    let doc = rc_editor::doc::new_track(&track, &name, &new.environment, indoor, style);
    rc_editor::doc::validate(&doc, &PropCatalog::load(assets_dir)?, &track)?;
    let catalog_path = assets_dir.join(CATALOG_FILE);
    let catalog = std::fs::read_to_string(&catalog_path)
        .map_err(|e| format!("{}: {e}", catalog_path.display()))?;
    let catalog = rc_editor::catalog::with_entry(&catalog, &doc.info)?;
    let header = format!("// Written by the track editor: cargo run -p rc_editor -- {id}\n");
    std::fs::write(&path, format!("{header}{}", doc.def.to_ron()?))
        .map_err(|e| format!("{}: {e}", path.display()))?;
    std::fs::write(&catalog_path, catalog)
        .map_err(|e| format!("{}: {e}", catalog_path.display()))?;
    println!(
        "created {} and its (hidden) entry in {CATALOG_FILE}",
        path.display()
    );
    Ok(())
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
    if let Some(new) = &args.new
        && let Err(e) = create(&paths.assets_dir, &args.track, new)
    {
        eprintln!("error: {e}");
        return AppExit::from_code(2);
    }
    let editor = match rc_editor::EditorPlugin::load(&paths.assets_dir, &args.track) {
        Ok(editor) => editor,
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
    // The light baker reads meshes, materials and textures on the CPU.
    rc_render::keep_cpu_copies();
    let mut app = recharged::game(config, paths, bundle);
    if args.bake {
        rc_editor::bake_on_start(&mut app);
    }
    app.add_plugins((editor, rc_editor::EditorViewPlugin));
    app.run()
}
