//! Persistence in `UserPaths::data_dir` (GDD §9, §15, §19; ARCHITECTURE §5.12).
//!
//! | File | Content | Written |
//! |---|---|---|
//! | `settings.ron` | `(version: 1, data: Settings)` | at most once per second after a change |
//! | `profile.ron` | `(version: 1, data: Profile)` | after results, Golden Battery, hints, resets, menu return |
//! | `ghosts/<variant>.ghost` | `(version: 1, data: GhostData)` | new Time Trial best |
//! | `logs/session-<unix ms>.log` | the session's log | by [`session_log_layer`]; last 3 kept |
//!
//! Writes are atomic (temp file + rename). A file that cannot be parsed is renamed to
//! `<name>.corrupt-<unix s>` and defaults are used. Versions: additions are serde defaults; a
//! version bump adds a `match` arm in [`parse_versioned`] with a pure, tested migration. Version 0
//! is the pre-envelope format (the bare struct, e.g. `Settings::to_ron`).

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use bevy::log::BoxedLayer;
use bevy::log::tracing_subscriber;
use bevy::prelude::*;
use rc_core::prelude::*;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::GameNotice;
use crate::profile::Profile;

/// Settings file name.
pub const SETTINGS_FILE: &str = "settings.ron";
/// Profile file name.
pub const PROFILE_FILE: &str = "profile.ron";
/// Ghost directory (`<variant>.ghost`).
pub const GHOSTS_DIR: &str = "ghosts";
/// Log directory.
pub const LOGS_DIR: &str = "logs";
/// Sessions whose logs are kept, including the current one (GDD §19).
pub const KEPT_LOGS: usize = 3;
/// Current version of every save file.
pub const SAVE_VERSION: u32 = 1;
/// Minimum seconds between two `settings.ron` writes (GDD §15).
pub const SETTINGS_WRITE_INTERVAL_SECS: f64 = 1.0;

#[derive(Serialize)]
struct EnvelopeOut<'a, T> {
    version: u32,
    data: &'a T,
}

#[derive(Deserialize)]
struct EnvelopeIn<T> {
    data: T,
}

#[derive(Deserialize)]
struct Header {
    /// Missing = 0, the bare pre-envelope format.
    #[serde(default)]
    version: u32,
}

/// Parses a save file of the current format, migrating older versions.
pub(crate) fn parse_versioned<T: DeserializeOwned>(text: &str) -> Result<T, String> {
    let header: Header = ron::from_str(text).map_err(|e| e.to_string())?;
    match header.version {
        // v0 → v1: the bare struct, before files had an envelope.
        0 => ron::from_str(text).map_err(|e| e.to_string()),
        SAVE_VERSION => ron::from_str::<EnvelopeIn<T>>(text)
            .map(|e| e.data)
            .map_err(|e| e.to_string()),
        v => Err(format!(
            "version {v} is newer than this game's {SAVE_VERSION}"
        )),
    }
}

/// The current format of `value`.
pub(crate) fn to_versioned<T: Serialize>(value: &T) -> String {
    let envelope = EnvelopeOut {
        version: SAVE_VERSION,
        data: value,
    };
    ron::ser::to_string_pretty(&envelope, ron::ser::PrettyConfig::default())
        .expect("save data contains only RON-serialisable values")
}

/// Outcome of reading a save file.
#[derive(Debug, PartialEq)]
pub(crate) enum Loaded<T> {
    /// No file yet.
    Missing,
    /// Parsed (and migrated).
    Ok(T),
    /// Unreadable or unparsable; moved to `backup` (if the rename worked).
    Corrupt {
        error: String,
        backup: Option<PathBuf>,
    },
}

/// Reads a save file; a corrupt one is moved aside so defaults can replace it.
pub(crate) fn load<T: DeserializeOwned>(path: &Path) -> Loaded<T> {
    let parsed = match fs::read_to_string(path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Loaded::Missing,
        Err(e) => Err(e.to_string()),
        Ok(text) => parse_versioned(&text),
    };
    match parsed {
        Ok(value) => Loaded::Ok(value),
        Err(error) => Loaded::Corrupt {
            error: format!("{}: {error}", path.display()),
            backup: back_up(path),
        },
    }
}

fn back_up(path: &Path) -> Option<PathBuf> {
    let name = path.file_name()?.to_string_lossy();
    let backup = path.with_file_name(format!("{name}.corrupt-{}", unix_time().as_secs()));
    fs::rename(path, &backup).ok().map(|()| backup)
}

/// Writes `text` to `path` atomically (temp file in the same directory, then rename).
pub(crate) fn write_atomic(path: &Path, text: &str) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    let mut file = fs::File::create(&tmp)?;
    file.write_all(text.as_bytes())?;
    file.sync_all()?;
    fs::rename(&tmp, path)
}

/// Writes a save file in the current format.
pub(crate) fn save<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    write_atomic(path, &to_versioned(value)).map_err(|e| format!("{}: {e}", path.display()))
}

/// The settings in `path`, sanitized, without ever changing the file (unlike the game's load,
/// which renames a corrupt file aside): for tools that borrow the player's settings (the track
/// editor). Missing → defaults; unreadable or unparsable → defaults and the error.
pub fn read_settings(path: &Path) -> (Settings, Option<String>) {
    let (mut settings, error) = match fs::read_to_string(path) {
        Ok(text) => match parse_versioned(&text) {
            Ok(settings) => (settings, None),
            Err(e) => (
                Settings::default(),
                Some(format!("{}: {e}", path.display())),
            ),
        },
        Err(e) if e.kind() == io::ErrorKind::NotFound => (Settings::default(), None),
        Err(e) => (
            Settings::default(),
            Some(format!("{}: {e}", path.display())),
        ),
    };
    crate::prefs::sanitize(&mut settings);
    (settings, error)
}

/// `ghosts/<key>.ghost` in `data_dir`.
pub(crate) fn ghost_path(data_dir: &Path, key: &str) -> PathBuf {
    data_dir.join(GHOSTS_DIR).join(format!("{key}.ghost"))
}

/// The saved ghost of a layout; a broken ghost file is deleted (it can only be re-earned).
pub(crate) fn load_ghost(data_dir: &Path, key: &str) -> Option<GhostData> {
    let path = ghost_path(data_dir, key);
    match load(&path) {
        Loaded::Ok(ghost) => Some(ghost),
        Loaded::Missing => None,
        Loaded::Corrupt { error, backup } => {
            warn!("ghost discarded: {error}");
            if let Some(backup) = backup {
                let _ = fs::remove_file(backup);
            }
            None
        }
    }
}

/// Whether a throttled settings write may happen now (≤ 1 write per second).
pub(crate) fn settings_write_due(dirty: bool, last_write: Option<f64>, now: f64) -> bool {
    dirty && last_write.is_none_or(|t| now - t >= SETTINGS_WRITE_INTERVAL_SECS)
}

pub(crate) fn unix_time() -> std::time::Duration {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
}

/// A `LogPlugin::custom_layer` that also writes the session's log to
/// `<data_dir>/logs/session-<unix ms>.log`, keeping the last [`KEPT_LOGS`] sessions. Needs
/// `UserPaths` inserted before `DefaultPlugins`; without it (or on an I/O error) no file is written.
pub fn session_log_layer(app: &mut App) -> Option<BoxedLayer> {
    let dir = app
        .world()
        .get_resource::<UserPaths>()?
        .data_dir
        .join(LOGS_DIR);
    match open_session_log(&dir) {
        Ok(file) => Some(Box::new(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_writer(Mutex::new(file)),
        )),
        Err(e) => {
            eprintln!("session log in {}: {e}", dir.display());
            None
        }
    }
}

fn open_session_log(dir: &Path) -> io::Result<fs::File> {
    fs::create_dir_all(dir)?;
    let names: Vec<String> = fs::read_dir(dir)?
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .collect();
    for old in logs_to_delete(&names, KEPT_LOGS - 1) {
        fs::remove_file(dir.join(old))?;
    }
    fs::File::create(dir.join(format!("session-{}.log", unix_time().as_millis())))
}

/// Pending writes. Mutations of `Settings` / `Profile` mark them dirty; `write_saves` writes.
#[derive(Resource, Debug, Default)]
pub(crate) struct SaveState {
    /// `Settings` changed since the last write.
    pub settings_dirty: bool,
    /// Real time of the last settings write.
    settings_written: Option<f64>,
    /// `Profile` changed: written at the end of this frame.
    pub profile_dirty: bool,
    /// `Stats::time_played_secs` when the game started.
    played_base: f64,
}

impl SaveState {
    pub(crate) fn new(played_base: f64) -> Self {
        Self {
            played_base,
            ..default()
        }
    }

    /// Play time counts from zero again (progress reset at real time `now`).
    pub(crate) fn reset_play_time(&mut self, now: f64) {
        self.played_base = -now;
    }
}

pub(crate) fn build(app: &mut App) {
    app.add_systems(Update, write_saves.in_set(crate::Step::Saves))
        .add_systems(
            Last,
            flush_on_exit
                .in_set(bevy::app::OnAppExitSystems)
                .run_if(on_message::<AppExit>),
        );
}

/// Writes the profile when dirty and the settings when due (≤ 1/s). Failures are reported once
/// and retried with the next change.
fn write_saves(
    state: Option<ResMut<SaveState>>,
    (settings, profile): (Option<Res<Settings>>, Option<ResMut<Profile>>),
    paths: Option<Res<UserPaths>>,
    time: Res<Time<Real>>,
    mut notices: MessageWriter<GameNotice>,
) {
    let (Some(mut state), Some(paths)) = (state, paths) else {
        return;
    };
    let now = time.elapsed_secs_f64();
    if state.profile_dirty
        && let Some(mut profile) = profile
    {
        state.profile_dirty = false;
        profile.stats.time_played_secs = state.played_base + now;
        report(
            save(&paths.data_dir.join(PROFILE_FILE), &*profile),
            &mut notices,
        );
    }
    if settings_write_due(state.settings_dirty, state.settings_written, now)
        && let Some(settings) = settings
    {
        state.settings_dirty = false;
        state.settings_written = Some(now);
        report(
            save(&paths.data_dir.join(SETTINGS_FILE), &*settings),
            &mut notices,
        );
    }
}

/// The app is exiting: write what is still pending, ignoring the throttle.
fn flush_on_exit(
    state: Option<ResMut<SaveState>>,
    (settings, profile): (Option<Res<Settings>>, Option<Res<Profile>>),
    paths: Option<Res<UserPaths>>,
    time: Res<Time<Real>>,
) {
    let (Some(mut state), Some(paths)) = (state, paths) else {
        return;
    };
    if let Some(mut profile) = profile.map(|p| p.clone()) {
        profile.stats.time_played_secs = state.played_base + time.elapsed_secs_f64();
        let _ = save(&paths.data_dir.join(PROFILE_FILE), &profile).map_err(|e| error!("{e}"));
    }
    if state.settings_dirty
        && let Some(settings) = settings
    {
        state.settings_dirty = false;
        let _ = save(&paths.data_dir.join(SETTINGS_FILE), &*settings).map_err(|e| error!("{e}"));
    }
}

fn report(result: Result<(), String>, notices: &mut MessageWriter<GameNotice>) {
    if let Err(e) = result {
        error!("save failed: {e}");
        notices.write(GameNotice::SaveFailed(e));
    }
}

/// Session logs to delete so that the newest `keep` remain (other files are ignored).
fn logs_to_delete(names: &[String], keep: usize) -> Vec<String> {
    let stamp = |n: &str| {
        n.strip_prefix("session-")?
            .strip_suffix(".log")?
            .parse::<u128>()
            .ok()
    };
    let mut logs: Vec<(u128, &String)> =
        names.iter().filter_map(|n| Some((stamp(n)?, n))).collect();
    logs.sort_unstable_by(|a, b| b.cmp(a));
    logs.into_iter()
        .skip(keep)
        .map(|(_, n)| n.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::{Content, Profile, Trophy};

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("rc_game-save-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn reading_settings_never_touches_the_file() {
        let dir = temp_dir("read-settings");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(SETTINGS_FILE);
        assert_eq!(read_settings(&path), (Settings::default(), None));
        let mut wide = Settings::default();
        wide.graphics.fov_deg = 90.0;
        save(&path, &wide).unwrap();
        assert_eq!(read_settings(&path).0.graphics.fov_deg, 90.0);
        fs::write(&path, "garbage").unwrap();
        let (settings, error) = read_settings(&path);
        assert_eq!(settings, Settings::default());
        assert!(error.is_some());
        assert_eq!(fs::read_to_string(&path).unwrap(), "garbage");
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1, "no backup made");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn saves_round_trip_in_the_current_format() {
        let mut profile = Profile::default();
        profile.trophies.insert("rookie_cup".into(), Trophy::Gold);
        profile.unlocked.push(Content::Car("torque".into()));
        profile.stats.distance = 1234.5;
        let text = to_versioned(&profile);
        assert!(text.contains("version: 1"), "{text}");
        assert_eq!(parse_versioned::<Profile>(&text), Ok(profile));

        let mut settings = Settings::default();
        settings.audio.music = 0.3;
        settings
            .controls
            .keyboard_a
            .boost
            .push(Binding::Key(KeyCode::KeyB));
        assert_eq!(
            parse_versioned::<Settings>(&to_versioned(&settings)),
            Ok(settings)
        );
    }

    #[test]
    fn version_0_bare_files_migrate() {
        let mut settings = Settings::default();
        settings.gameplay.speed_unit = SpeedUnit::Mph;
        settings.controls.gamepad.drift.clear();
        let bare = settings.to_ron(); // what `Settings::to_ron` wrote before envelopes
        assert_eq!(parse_versioned::<Settings>(&bare), Ok(settings));
        let profile: Profile = parse_versioned(r#"(golden_batteries: ["toy_town"])"#).expect("v0");
        assert_eq!(profile.golden_batteries, ["toy_town".into()]);
    }

    #[test]
    fn newer_or_broken_files_are_rejected() {
        assert!(parse_versioned::<Profile>("(version: 99, data: ())").is_err());
        assert!(parse_versioned::<Profile>("(version: 1, data: (stats: oops))").is_err());
        assert!(parse_versioned::<Profile>("").is_err());
        assert!(parse_versioned::<Settings>("not ron at all {").is_err());
    }

    #[test]
    fn corrupt_files_are_backed_up_and_missing_files_reported() {
        let dir = temp_dir("corrupt");
        let path = dir.join(PROFILE_FILE);
        assert_eq!(load::<Profile>(&path), Loaded::Missing);
        write_atomic(&path, "(version: 1, data: (trophies: {\"x\": Platinum}))").expect("write");
        let Loaded::Corrupt {
            backup: Some(backup),
            ..
        } = load::<Profile>(&path)
        else {
            panic!("expected a corrupt file with a backup");
        };
        assert!(!path.exists() && backup.exists());
        assert!(backup.to_string_lossy().contains("profile.ron.corrupt-"));

        save(&path, &Profile::default()).expect("save");
        assert_eq!(load::<Profile>(&path), Loaded::Ok(Profile::default()));
        assert!(!dir.join("profile.tmp").exists(), "temp file renamed");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn ghosts_load_and_broken_ones_vanish() {
        let dir = temp_dir("ghosts");
        let ghost = GhostData {
            car: "zip".into(),
            time_ticks: 900,
            geometry_hash: 5,
            samples: vec![GhostSample::default(); 3],
            ..default()
        };
        save(&ghost_path(&dir, "toy_town_rev"), &ghost).expect("save ghost");
        assert_eq!(load_ghost(&dir, "toy_town_rev"), Some(ghost));
        assert_eq!(load_ghost(&dir, "toy_town"), None);
        write_atomic(&ghost_path(&dir, "sofa_circuit"), "garbage").expect("write");
        assert_eq!(load_ghost(&dir, "sofa_circuit"), None);
        let left = fs::read_dir(dir.join(GHOSTS_DIR)).expect("dir").count();
        assert_eq!(left, 1, "only the good ghost remains");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn settings_writes_are_throttled_to_one_per_second() {
        assert!(!settings_write_due(false, None, 10.0), "nothing to write");
        assert!(
            settings_write_due(true, None, 0.0),
            "first write is immediate"
        );
        assert!(!settings_write_due(true, Some(10.0), 10.5));
        assert!(settings_write_due(true, Some(10.0), 11.0));
    }

    #[test]
    fn only_the_newest_session_logs_are_kept() {
        let names: Vec<String> = [
            "session-30.log",
            "session-100.log",
            "session-9.log",
            "notes.txt",
        ]
        .map(String::from)
        .into();
        assert_eq!(logs_to_delete(&names, 2), ["session-9.log"]);
        assert_eq!(logs_to_delete(&names, 0).len(), 3);
        assert!(logs_to_delete(&names, 5).is_empty());
    }
}
