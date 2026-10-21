//! App-flow contracts: the loading gate, lobby state, seen hints, dropped Versus players and
//! automation flags (ARCHITECTURE §3.1, §4.6, §8). `GameCommand` (the flow's input), the profile
//! and the cup/Versus series live in rc_game.

use std::path::PathBuf;

use bevy::prelude::*;

use crate::catalog::{CarId, CupId, PlayerSlot, TrackId};
use crate::race::HintKind;
use crate::settings::GraphicsSettings;
use crate::weather::{TimeOfDay, WeatherChoice};

/// The loading gate. Loaders register a key at build time; rc_game is the only runtime writer: it
/// resets every key to pending in `OnEnter(AppState::Loading)`, completes keys from
/// [`LoadingDone`] messages and opens the gate when none is pending. A plugin that is not added
/// never blocks.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct LoadingTasks {
    registered: Vec<&'static str>,
    done: Vec<&'static str>,
}

impl LoadingTasks {
    /// Build-time registration from a plugin's `build()`: `LoadingTasks::register(app, "track")`.
    pub fn register(app: &mut App, key: &'static str) {
        let mut tasks = app.world_mut().get_resource_or_init::<LoadingTasks>();
        if !tasks.registered.contains(&key) {
            tasks.registered.push(key);
        }
    }

    /// Every registered key.
    pub fn registered(&self) -> &[&'static str] {
        &self.registered
    }

    /// Marks every key pending (rc_game, `OnEnter(Loading)`).
    pub fn reset(&mut self) {
        self.done.clear();
    }

    /// Marks `key` done; returns `false` for unregistered or already-done keys.
    pub fn complete(&mut self, key: &'static str) -> bool {
        if self.registered.contains(&key) && !self.done.contains(&key) {
            self.done.push(key);
            true
        } else {
            false
        }
    }

    /// Keys still loading.
    pub fn pending(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.registered
            .iter()
            .copied()
            .filter(|k| !self.done.contains(k))
    }

    /// No key is pending.
    pub fn is_done(&self) -> bool {
        self.pending().next().is_none()
    }

    /// Fraction of keys done, 0–1 (1 when nothing is registered).
    pub fn progress(&self) -> f32 {
        if self.registered.is_empty() {
            1.0
        } else {
            self.done.len() as f32 / self.registered.len() as f32
        }
    }
}

/// A loader finished its key (producer every loader; consumer rc_game).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadingDone(pub &'static str);

/// A loader failed (bad data file, missing asset): rc_game leaves the session through
/// `Unloading` back to the menu and shows the error.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct LoadingFailed {
    /// The loader's key.
    pub key: &'static str,
    /// Human-readable cause naming the file and field.
    pub error: String,
}

/// Versus lobby state. Written only by rc_game; read by rc_input (join/leave handling).
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct LobbyState {
    /// Devices may join and leave slots.
    pub open: bool,
    /// Number of joinable slots.
    pub max_slots: u8,
}

impl Default for LobbyState {
    fn default() -> Self {
        Self {
            open: false,
            max_slots: crate::MAX_LOCAL_PLAYERS as u8,
        }
    }
}

/// Onboarding hints the player has already seen (GDD §13: each is shown once). Written only by
/// rc_game (from the profile, and when it sees a `RaceEvent::Hint`); rc_race emits a hint only when
/// `Settings.gameplay.hints` is on and the kind is not listed here. Absent without rc_game.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct HintsSeen(pub Vec<HintKind>);

/// A Versus player whose controller disconnected was dropped: the AI drives their car from now on
/// (producer rc_game; consumer rc_race, which adds `AiDriver` to that slot's racer). Later attempts
/// of the session get `Controller::Human { autopilot: true }` for the slot from `RaceConfig`.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerDropped {
    /// The dropped player.
    pub slot: PlayerSlot,
}

/// Mode of an automated quick race (`--mode`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum QuickRaceMode {
    /// Quick Race.
    #[default]
    Quick,
    /// Time Trial (`tt`).
    TimeTrial,
    /// Versus (`versus`).
    Versus,
    /// The cup's race on this track (`cup:<id>`).
    Cup(CupId),
}

/// `--quick-race <track>` and its options: start a race skipping the menus.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct QuickRaceArgs {
    /// Track.
    pub track: TrackId,
    /// `--reverse`.
    pub reverse: bool,
    /// `--mirror`.
    pub mirror: bool,
    /// `--car <id>`.
    pub car: Option<CarId>,
    /// `--ai <n>`: AI opponents.
    pub ai: Option<u8>,
    /// `--laps <n>`.
    pub laps: Option<u8>,
    /// `--weather <w>`.
    pub weather: Option<WeatherChoice>,
    /// `--time <t>`.
    pub time: Option<TimeOfDay>,
    /// `--players <n>`: local players.
    pub players: Option<u8>,
    /// `--seed <n>`.
    pub seed: Option<u64>,
    /// `--mode tt|versus|cup:<id>`.
    pub mode: QuickRaceMode,
}

/// `--screenshot <path> [--screenshot-at <s>] [--screenshot-every <s>]`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ScreenshotPlan {
    /// Output path (`--screenshot-every` appends `_<n>` before the extension).
    pub path: PathBuf,
    /// Wall-clock seconds after start of the (first) capture.
    pub at: Option<f32>,
    /// Repeat every N wall-clock seconds.
    pub every: Option<f32>,
}

/// Command-line automation & test hooks (ARCHITECTURE §8). Written only by the binary.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct AutomationConfig {
    /// `--user-data-dir <path>` as given (resolved into `UserPaths`).
    pub user_data_dir: Option<PathBuf>,
    /// `--assets-dir <path>` as given (resolved into `UserPaths`).
    pub assets_dir: Option<PathBuf>,
    /// `--unlock-all`: everything unlocked, in memory only.
    pub unlock_all: bool,
    /// `--quick-race …`.
    pub quick_race: Option<QuickRaceArgs>,
    /// `--autopilot`: local players driven by the AI.
    pub autopilot: bool,
    /// `--input-script <file>`: timed `UiNav` + driving intents.
    pub input_script: Option<PathBuf>,
    /// `--exit-after <secs>`: quit after N seconds of wall time.
    pub exit_after: Option<f32>,
    /// `--screenshot …`.
    pub screenshot: Option<ScreenshotPlan>,
    /// `--headless-race`: run the race sim as fast as possible, print result JSON.
    pub headless_race: bool,
    /// `--soak <minutes>`: chain random races, assert counts return to baseline.
    pub soak_minutes: Option<f32>,
    /// `--soak-seed <n>`: seed of the soak's random choices, to replay a failed soak (default: from
    /// the clock; the soak logs the seed it uses).
    pub soak_seed: Option<u64>,
    /// `--bench <scenario>`: frame-time percentiles as JSON.
    pub bench: Option<String>,
    /// `--calibrate-medals`.
    pub calibrate_medals: bool,
    /// `--capture-previews`.
    pub capture_previews: bool,
    /// `--track-tour <track>`.
    pub track_tour: Option<TrackId>,
    /// `--export-bindings <path>`: write the TS bindings and exit.
    pub export_bindings: Option<PathBuf>,
    /// `--window <W>x<H>`: initial window size in logical px; overrides the window size from
    /// `Settings` for this run (screenshots at fixed resolutions).
    pub window: Option<(u32, u32)>,
    /// `--camera <x,y,z,tx,ty,tz>`: `--screenshot` is taken by rc_render's capture camera (1280×720,
    /// no HUD) standing at the eye and looking at the target, instead of from the window.
    pub camera: Option<(Vec3, Vec3)>,
    /// `--graphics <RON>`: these graphics settings over the defaults for this run (never saved).
    pub graphics: Option<GraphicsSettings>,
}

impl AutomationConfig {
    /// Any flag that drives the game without a human is set: loading screens are not held for
    /// their minimum display time and failures should end the run with a non-zero exit code.
    pub fn is_automated(&self) -> bool {
        self.quick_race.is_some()
            || self.autopilot
            || self.input_script.is_some()
            || self.exit_after.is_some()
            || self.screenshot.is_some()
            || self.headless_race
            || self.soak_minutes.is_some()
            || self.bench.is_some()
            || self.calibrate_medals
            || self.capture_previews
            || self.track_tour.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loading_gate_tracks_registered_keys() {
        let mut app = App::new();
        LoadingTasks::register(&mut app, "track");
        LoadingTasks::register(&mut app, "audio");
        LoadingTasks::register(&mut app, "track");
        let mut tasks = app.world_mut().resource_mut::<LoadingTasks>();
        assert_eq!(tasks.registered(), ["track", "audio"]);
        tasks.reset();
        assert!(!tasks.is_done());
        assert!(tasks.complete("track"));
        assert!(!tasks.complete("track"), "second completion is ignored");
        assert!(!tasks.complete("unknown"));
        assert_eq!(tasks.pending().collect::<Vec<_>>(), ["audio"]);
        assert_eq!(tasks.progress(), 0.5);
        assert!(tasks.complete("audio"));
        assert!(tasks.is_done());
        tasks.reset();
        assert_eq!(tasks.progress(), 0.0);
    }

    #[test]
    fn empty_gate_is_open() {
        let tasks = LoadingTasks::default();
        assert!(tasks.is_done());
        assert_eq!(tasks.progress(), 1.0);
    }
}
