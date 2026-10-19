//! # rc_game — app flow, modes, progression and persistence (F1)
//!
//! - **Responsibilities:** boot (catalog, medal targets, settings, profile); the session
//!   lifecycle of ARCHITECTURE §3.1 (every exit through `Unloading`, restart via
//!   `RacePhase::Setup`); the loading gate (sole runtime writer of `LoadingTasks`); pause (commands,
//!   devices, window focus loss, controller disconnect); building `RaceConfig` per mode from a
//!   [`RaceRequest`] validated against the [`Profile`]'s unlocks; cup runs ([`CupRun`]), the Versus
//!   lobby and scoreboard ([`VersusSession`]); results processing into [`RaceOutcome`] (points,
//!   records, medals, trophies, Golden Batteries, stats, unlocks, hints); saves and session logs
//!   in `UserPaths::data_dir`; `--quick-race`, `--unlock-all`, `--soak`.
//! - **Inputs:** [`GameCommand`] (rc_ui, automation, tests), `PauseRequest`, `DeviceEvent`,
//!   `RebindResult`, `WindowFocused`, `LoadingDone` / `LoadingFailed`, `RaceResults`, `RaceEvent`
//!   (Golden Battery, hints), `ItemEvent` / `HitEvent` (stats), `TrackSummary` (geometry hash),
//!   `UserPaths`, `AutomationConfig`.
//! - **Outputs:** `AppState` / `RacePhase` (Setup) / `PauseState`, `Catalog`, `Settings`,
//!   `RaceConfig`, `LoadingTasks`, `LobbyState`, `HintsSeen`, `PlayerDropped`, [`Profile`],
//!   [`MedalTable`], [`CupRun`], [`VersusSession`], [`RaceOutcome`], [`GameNotice`]; files.
//! - **Must never:** pause `Time<Virtual>` (bevy-react animates with it), spawn gameplay entities,
//!   write the window (rc_render applies display settings), or panic on bad data or failed I/O
//!   (log, notify, fall back to the menu or to defaults).
//!
//! `RaceConfig` is replaced for every session and may also change between attempts of one
//! session (new Time Trial ghost, collected Golden Battery, dropped Versus player): readers take it
//! in `OnEnter(RacePhase::Setup)` / `OnEnter(RaceAttempt)` and never cache it across attempts.

mod commands;
mod flow;
mod medals;
mod prefs;
mod profile;
mod race_setup;
mod records;
mod results;
mod save;
mod series;
mod soak;

use bevy::prelude::*;
use rc_core::prelude::*;

pub use medals::{MEDALS_FILE, MedalTable, MedalTargets};
pub use profile::{Content, LayoutRecord, Medal, Profile, Stats, TimedRecord, Trophy};
pub use profile::{unlock_rule, variant_key};
pub use records::NextMedal;
pub use results::RaceOutcome;
pub use save::{
    GHOSTS_DIR, KEPT_LOGS, LOGS_DIR, PROFILE_FILE, SAVE_VERSION, SETTINGS_FILE, session_log_layer,
};
pub use series::{CupRun, POINTS, RacerKey, Series, Standing, VersusSession, points_for};

/// Minimum time the loading screen stays up for humans (0 with automation or without a window).
pub const MIN_LOADING_SECS: f32 = 1.0;

/// A request to the app flow (producers rc_ui, automation, tests; consumer rc_game). Commands
/// that do not fit the current state are logged and ignored; rejected race starts also send a
/// [`GameNotice::Error`].
#[derive(Message, Debug, Clone, PartialEq)]
pub enum GameCommand {
    /// Menu → Loading with a `RaceConfig` built from the request (validated against unlocks).
    StartRace(RaceRequest),
    /// The same race again without reloading: `RacePhase::Setup` (+ `PauseState::Running`).
    /// Reverts the race's cup/Versus points. Also Time Trial "Retry".
    Restart,
    /// Unloading → Loading: the next cup race (once the current one has a result; after the
    /// last one: back to the menu, the finished [`CupRun`] stays for the Cup end screen), or the
    /// same setup again in other modes.
    NextRace,
    /// Versus "Rematch": the same track as a new race of the session, without reloading. Other
    /// modes: same as [`GameCommand::Restart`].
    Rematch,
    /// The same options on another layout (Unloading → Loading). Not in cups.
    NextTrack(TrackVariant),
    /// Quick Race / Time Trial "Change track", Versus "Next track" (P1 picks): another layout with
    /// the race options picked on Track select; the players and their cars stay (Unloading →
    /// Loading). Not in cups.
    ChangeTrack(Box<TrackPick>),
    /// Leave the session: Unloading → Menu. An unfinished cup is lost (React confirms).
    QuitToMenu,
    /// Pause the race (Intro, Countdown or Racing).
    Pause,
    /// Resume (idempotent; React's "Resume").
    Resume,
    /// Open the Versus lobby: devices may join slots (`LobbyState::open`). Menu only.
    OpenLobby,
    /// Close the lobby and end the Versus session.
    CloseLobby,
    /// Versus: the AI drives this player's car for the rest of the session ("Drop player" after
    /// a controller disconnect). Send `Resume` to continue the race.
    DropPlayer(PlayerSlot),
    /// Replace the settings (values are clamped to their ranges); saved at most once per second.
    ApplySettings(Box<Settings>),
    /// Restore the default bindings of one scheme.
    ResetBindings(ControlScheme),
    /// Erase all progress, records and ghosts (menu only; React confirms twice).
    ResetProgress,
    /// A NEW unlock was viewed: clear its badge (`Profile::new_unlocks`).
    MarkSeen(Content),
    /// Exit the app (pending saves are flushed).
    QuitGame,
}

/// Car and paint of one local player.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlayerCar {
    /// The player.
    pub slot: PlayerSlot,
    /// Car.
    pub car: CarId,
    /// Paint (must be a base paint or of an unlocked paint set).
    pub paint: PaintId,
}

/// What the player (or automation) asked to race; rc_game turns it into a `RaceConfig`.
///
/// Championship: `mode: Championship { cup, race_index: 0 }` plus the player's car and paint —
/// layout, weather, laps, field and difficulty come from the cup.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RaceRequest {
    /// Mode.
    pub mode: GameMode,
    /// Layout (ignored in cups).
    pub variant: TrackVariant,
    /// Car of the local players (`None` = the first catalog car) unless `humans` is set.
    pub car: Option<CarId>,
    /// Paint of the local players' car unless `humans` is set.
    pub paint: PaintId,
    /// Local players (0 is treated as 1) unless `humans` is set.
    pub players: u8,
    /// Versus: each joined player with their own car and paint, in grid order. Empty = `players`
    /// players in slots 0.. driving `car` / `paint`.
    pub humans: Vec<PlayerCar>,
    /// AI opponents (`None` = fill the grid; always 0 in Time Trial, 7 in cups).
    pub ai: Option<u8>,
    /// AI difficulty (`None` = `Settings.gameplay.ai_difficulty`; cups use theirs).
    pub difficulty: Option<Difficulty>,
    /// Laps (`None` = the track's default; cups: theirs; Time Trial: 3).
    pub laps: Option<u8>,
    /// Weather (`None` or not allowed on the track = its first allowed preset).
    pub weather: Option<WeatherChoice>,
    /// Time of day (`None` or not allowed = Noon, else the first allowed).
    pub time: Option<TimeOfDay>,
    /// Items (`None` = on, except Time Trial).
    pub items: Option<bool>,
    /// RNG seed (`None` = from the clock).
    pub seed: Option<u64>,
}

/// A layout and race options picked after a race ([`GameCommand::ChangeTrack`]); `None` keeps
/// the session's value (rc_game validates and clamps them like a [`RaceRequest`]).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TrackPick {
    /// Layout.
    pub variant: TrackVariant,
    /// Laps.
    pub laps: Option<u8>,
    /// AI opponents.
    pub ai: Option<u8>,
    /// AI difficulty.
    pub difficulty: Option<Difficulty>,
    /// Items.
    pub items: Option<bool>,
    /// Weather.
    pub weather: Option<WeatherChoice>,
    /// Time of day.
    pub time: Option<TimeOfDay>,
}

impl TrackPick {
    /// `request` on this layout with these options.
    pub(crate) fn apply_to(&self, request: &mut RaceRequest) {
        request.variant = self.variant.clone();
        request.laps = self.laps.or(request.laps);
        request.ai = self.ai.or(request.ai);
        request.difficulty = self.difficulty.or(request.difficulty);
        request.items = self.items.or(request.items);
        request.weather = self.weather.or(request.weather);
        request.time = self.time.or(request.time);
    }
}

impl RaceRequest {
    /// A default quick race on the forward layout of `track`.
    pub fn quick(track: impl Into<TrackId>) -> Self {
        Self {
            variant: TrackVariant::forward(track),
            players: 1,
            ..default()
        }
    }
}

impl From<&QuickRaceArgs> for RaceRequest {
    /// `--quick-race`; a cup's race index is resolved from the track when the race starts.
    fn from(args: &QuickRaceArgs) -> Self {
        let mode = match &args.mode {
            QuickRaceMode::Quick => GameMode::QuickRace,
            QuickRaceMode::TimeTrial => GameMode::TimeTrial,
            QuickRaceMode::Versus => GameMode::Versus,
            QuickRaceMode::Cup(cup) => GameMode::Championship {
                cup: cup.clone(),
                race_index: 0,
            },
        };
        let default_players = if mode == GameMode::Versus { 2 } else { 1 };
        Self {
            mode,
            variant: TrackVariant {
                track: args.track.clone(),
                reverse: args.reverse,
                mirror: args.mirror,
            },
            car: args.car.clone(),
            players: args.players.unwrap_or(default_players),
            ai: args.ai,
            laps: args.laps,
            weather: args.weather,
            time: args.time,
            seed: args.seed,
            ..default()
        }
    }
}

/// Something to show the player (producer rc_game; consumer rc_ui → toasts and dialogs).
/// Boot-time notices (a restored corrupt save) are written in the first frame, before React is
/// ready: rc_ui holds them until `ui.ready`.
#[derive(Message, Debug, Clone, PartialEq)]
pub enum GameNotice {
    /// Content was unlocked (toast; also in `Profile::new_unlocks` for the NEW badge and in
    /// `RaceOutcome::unlocked` when it came from a race).
    Unlocked(Content),
    /// The track's Golden Battery was collected and the profile saved (toast).
    GoldenBattery(TrackId),
    /// The layout's records and ghost were set on another version of the track and were
    /// discarded (notice).
    RecordsDiscarded(TrackVariant),
    /// A player's controller disconnected and the race is paused (dialog: press a button on any
    /// controller to take over, or `GameCommand::DropPlayer` in Versus).
    ControllerDisconnected(PlayerSlot),
    /// A race could not start or load, or data is broken; the game is in the menu (error dialog).
    Error(String),
    /// A save failed; it is retried with the next save (error toast).
    SaveFailed(String),
    /// A corrupt save was moved aside and defaults are used (toast).
    SaveRestored {
        /// The corrupt file.
        file: String,
        /// Where it was moved (`None` if moving failed).
        backup: Option<String>,
    },
}

/// Order of rc_game's systems inside `FlowSet`.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Step {
    /// Device and window signals, automation drivers.
    Signals,
    /// Results, Golden Battery, hints, stats (before commands: a restart in the same frame
    /// must revert the race it follows).
    Progress,
    /// `GameCommand`s and lifecycle transitions.
    Transitions,
    /// Writing saves.
    Saves,
}

/// The app flow. Add after `CorePlugin`.
pub struct GameFlowPlugin;

impl Plugin for GameFlowPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<GameCommand>()
            .add_message::<GameNotice>()
            .add_message::<PlayerDropped>()
            // Headless apps have no WindowPlugin; focus loss simply never happens there.
            .add_message::<bevy::window::WindowFocused>()
            .init_resource::<LobbyState>()
            .configure_sets(
                Update,
                (
                    Step::Signals,
                    Step::Progress,
                    Step::Transitions,
                    Step::Saves,
                )
                    .chain()
                    .in_set(FlowSet),
            );
        flow::build(app);
        prefs::build(app);
        results::build(app);
        save::build(app);
        soak::build(app);
    }
}
