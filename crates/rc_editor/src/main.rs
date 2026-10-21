//! `rc_editor <track> [options]` (see [`USAGE`]): the track editor on
//! `assets/maps/<track>/`; `rc_editor new <id> --env <map>` creates the map first. It runs the whole game ([`recharged::game`]) plus
//! [`rc_editor::EditorPlugin`]; saves, settings and records go to `<user data>/editor`, apart
//! from the game's.

use std::path::{Path, PathBuf};

use bevy::prelude::*;
use rc_core::prelude::*;
use rc_editor::BakeQuality;
use rc_render::ENVIRONMENT_FILE;
use rc_render::EnvironmentDef;
use rc_track::{PropCatalog, RibbonStyle, TRACK_FILE, map_dir};

const USAGE: &str = "usage: rc_editor <track> [--assets-dir <dir>] [--user-data-dir <dir>]
                 [--screenshot <png> [--screenshot-at <s>]] [--exit-after <s>]
       rc_editor new <id> --env <map> [--name <name>] [--style <style>] [options]

  <track>                  track id: the map directory assets/maps/<track>/
  new <id>                 create a hidden track (an oval of <style>: ToyTrack, Asphalt,
                           CarpetRunner or Invisible; default ToyTrack) with a copy of <map>'s
                           environment (assets/maps/<map>/environment.ron), then edit it
  --assets-dir <dir>       assets root (default: ./assets, then next to the executable)
  --user-data-dir <dir>    saves, settings and logs (default: the game's, + /editor)
  --screenshot <png>       save the window to <png> (at --screenshot-at seconds, default 3)
  --exit-after <s>         quit after <s> seconds of wall time
  --window <W>x<H>         fixed window size (reproducible screenshots)
  --bake                   bake the irradiance volumes (every allowed time, normal and mirror)
                           into assets/maps/<track>/, then quit
  --bake-time <time>       bake one volume (morning, noon, sunset or night; normal layout), then quit
  --bake-quality <q>       low (default) or high: half the probe spacing, twice the rays (≈ 16×
                           the bake time)

  Keys: F5 test drive / back to editing · Ctrl+S save · Ctrl+Z / Ctrl+Y undo / redo
        Delete · Insert (after the selected point) · Ctrl+D duplicate · T top view · F focus
        P light probes on/off · G / R / C gizmo move / rotate / scale · L world / local axes
        Esc cancel a drag, else deselect
  Mouse: left click select · drag a gizmo handle (Ctrl: snap) · Ctrl+click the road (Track
        tab): new control point · right drag look · WASD move · Q/E down/up · Shift faster";

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
    /// `--bake` (`Some(None)`) or `--bake-time <t>` (`Some(Some(t))`).
    bake: Option<Option<TimeOfDay>>,
    bake_quality: BakeQuality,
    window: Option<(u32, u32)>,
}

fn parse(args: &[String]) -> Result<Args, String> {
    let (mut track, mut assets_dir, mut user_data_dir) = (None, None, None);
    let (mut screenshot, mut screenshot_at, mut exit_after) = (None, None, None);
    let (mut new, mut env, mut name, mut style) = (false, None, None, None);
    let (mut bake, mut bake_quality) = (None, BakeQuality::Low);
    let mut window = None;
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
            "--window" => {
                let v = value()?;
                let size = v
                    .split_once('x')
                    .and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)));
                window = Some(size.ok_or(format!("--window {v}: expected <W>x<H>"))?);
            }
            "--env" => env = Some(value()?.clone()),
            "--name" => name = Some(value()?.clone()),
            "--style" => style = Some(value()?.clone()),
            "--bake" => bake = Some(None),
            "--bake-time" => {
                let t = value()?;
                let time = match t.to_lowercase().as_str() {
                    "morning" => TimeOfDay::Morning,
                    "noon" => TimeOfDay::Noon,
                    "sunset" => TimeOfDay::Sunset,
                    "night" => TimeOfDay::Night,
                    _ => return Err(format!("--bake-time {t}: morning, noon, sunset or night")),
                };
                bake = Some(Some(time));
            }
            "--bake-quality" => {
                bake_quality = match value()?.as_str() {
                    "low" => BakeQuality::Low,
                    "high" => BakeQuality::High,
                    q => return Err(format!("--bake-quality {q}: low or high")),
                };
            }
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
        (true, None) => return Err("new needs --env <map>".into()),
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
        bake_quality,
        window,
    })
}

/// Writes a new map — its layout, a copy of the `--env` map's environment — and its catalog entry
/// (refusing to replace an existing map).
fn create(assets_dir: &Path, id: &str, new: &NewTrack) -> Result<(), String> {
    let valid = id
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
    if id.is_empty() || !valid {
        return Err(format!(
            "track id \"{id}\": use lowercase letters, digits and _"
        ));
    }
    let dir = map_dir(assets_dir, id);
    let path = dir.join(TRACK_FILE);
    if path.exists() {
        return Err(format!(
            "{} exists: edit it with rc_editor {id}",
            path.display()
        ));
    }
    let template = map_dir(assets_dir, &new.environment).join(ENVIRONMENT_FILE);
    let env = EnvironmentDef::load_file(&template)?;
    let style = match &new.style {
        Some(s) => rc_editor::fields::parse::<RibbonStyle>(s)?,
        None => RibbonStyle::ToyTrack,
    };
    let track = TrackId::new(id);
    let name = new.name.clone().unwrap_or_else(|| id.replace('_', " "));
    let kind = catalog_kind(assets_dir, &new.environment)?;
    let doc = rc_editor::doc::new_track(&track, &name, &kind, env, style);
    rc_editor::doc::validate(&doc, &PropCatalog::load(assets_dir)?, &track)?;
    let catalog_path = assets_dir.join(CATALOG_FILE);
    let catalog = std::fs::read_to_string(&catalog_path)
        .map_err(|e| format!("{}: {e}", catalog_path.display()))?;
    let catalog = rc_editor::catalog::with_entry(&catalog, &doc.info)?;
    let header = format!("// Written by the track editor: cargo run -p rc_editor -- {id}\n");
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    std::fs::copy(&template, dir.join(ENVIRONMENT_FILE))
        .map_err(|e| format!("{}: {e}", template.display()))?;
    std::fs::write(&path, format!("{header}{}", doc.def.to_ron()?))
        .map_err(|e| format!("{}: {e}", path.display()))?;
    std::fs::write(&catalog_path, catalog)
        .map_err(|e| format!("{}: {e}", catalog_path.display()))?;
    println!(
        "created {} and its (hidden) entry in {CATALOG_FILE}",
        dir.display()
    );
    Ok(())
}

/// The environment kind (`TrackInfo.environment`) of catalog map `map`.
fn catalog_kind(assets_dir: &Path, map: &str) -> Result<String, String> {
    let catalog = Catalog::load(assets_dir).map_err(|e| e.to_string())?;
    let info = catalog.track(&TrackId::new(map));
    info.map(|t| t.environment.clone())
        .ok_or_else(|| format!("--env {map}: not a map in {CATALOG_FILE}"))
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
    // The player's settings, read-only: the editor's own saves stay in `editor/`.
    let (settings, settings_error) =
        rc_game::read_settings(&paths.data_dir.join(rc_game::SETTINGS_FILE));
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
        window: args.window,
        ..default()
    };
    // The light baker reads meshes, materials and textures on the CPU.
    rc_render::keep_cpu_copies();
    let mut app = recharged::game(config, paths, bundle);
    if let Some(e) = settings_error {
        warn!("settings: {e}; using defaults");
    }
    app.insert_resource(settings); // boot_saves keeps a `Settings` that is already there
    if let Some(time) = args.bake {
        rc_editor::bake_on_start(&mut app, time, args.bake_quality);
    }
    app.add_plugins((editor, rc_editor::EditorViewPlugin));
    app.run()
}
