//! # rc_game — app flow (F1)
//!
//! - **Responsibilities:** boot (catalog, settings), the session lifecycle transitions of
//!   ARCHITECTURE §3.1 (incl. `Unloading` and restart via `RacePhase::Setup`), the loading gate
//!   (sole runtime writer of `LoadingTasks`), pause / resume, building `RaceConfig` from a
//!   [`RaceRequest`], `--quick-race`. Later (T07): modes, cups, results, progression, persistence.
//! - **Inputs:** [`GameCommand`] (from rc_ui and automation), `PauseRequest`, `LoadingDone` /
//!   `LoadingFailed`, `UserPaths`, `AutomationConfig`.
//! - **Outputs:** `AppState` / `RacePhase` (Setup) / `PauseState`, `Catalog`, `Settings`,
//!   `RaceConfig`, `LoadingTasks`.
//! - **Must never:** pause `Time<Virtual>` (bevy-react animates with it), spawn gameplay entities,
//!   or panic on bad data (log, then fall back to the menu).

mod flow;
mod phases;
mod race_setup;

use bevy::prelude::*;
use rc_core::prelude::*;

pub use race_setup::build_race_config;

/// Minimum time the loading screen stays up for humans (0 with automation or without a window).
pub const MIN_LOADING_SECS: f32 = 1.0;

/// A request to the app flow (producers rc_ui, automation, tests; consumer rc_game).
#[derive(Message, Debug, Clone, PartialEq)]
pub enum GameCommand {
    /// Menu → Loading with a `RaceConfig` built from the request.
    StartRace(RaceRequest),
    /// Same track again without reloading: `RacePhase::Setup` (+ `PauseState::Running`).
    Restart,
    /// Next race: Unloading → Loading (quick race: the same config again).
    NextRace,
    /// Leave the session: Unloading → Menu.
    QuitToMenu,
    /// Pause the running race.
    Pause,
    /// Resume (idempotent; React's "Resume").
    Resume,
    /// Exit the app.
    QuitGame,
}

/// What the player (or automation) asked to race; rc_game turns it into a `RaceConfig`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RaceRequest {
    /// Mode.
    pub mode: GameMode,
    /// Layout.
    pub variant: TrackVariant,
    /// Car of the local players (`None` = the first catalog car).
    pub car: Option<CarId>,
    /// Paint of the local players' car.
    pub paint: PaintId,
    /// Local players (0 is treated as 1).
    pub players: u8,
    /// AI opponents (`None` = fill the grid; always 0 in Time Trial).
    pub ai: Option<u8>,
    /// AI difficulty (`None` = `Settings.gameplay.ai_difficulty`).
    pub difficulty: Option<Difficulty>,
    /// Laps (`None` = the track's default; Time Trial: 3).
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
        Self {
            mode,
            variant: TrackVariant {
                track: args.track.clone(),
                reverse: args.reverse,
                mirror: args.mirror,
            },
            car: args.car.clone(),
            players: args.players.unwrap_or(1),
            ai: args.ai,
            laps: args.laps,
            weather: args.weather,
            time: args.time,
            seed: args.seed,
            ..default()
        }
    }
}

/// The app flow. Add after `CorePlugin`.
pub struct GameFlowPlugin;

impl Plugin for GameFlowPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<GameCommand>();
        flow::build(app);
        phases::build(app);
    }
}
