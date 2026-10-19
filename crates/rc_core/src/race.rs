//! Race setup, the racer entity, race state and race messages (ARCHITECTURE §4.2–§4.6).

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::catalog::{CarId, CupId, Difficulty, PaintId, PlayerSlot, RivalId, TrackVariant};
use crate::interp::Interpolated;
use crate::items::{ItemSlot, StatusEffects};
use crate::vehicle::{CarControl, Charge, VehicleTelemetry};
use crate::weather::WeatherSetup;

/// Ghost recording rate in Hz (Time Trial).
pub const GHOST_HZ: u32 = 20;

/// Length of the 3-2-1 countdown in fixed ticks (3 s, GDD §3.2), counted by rc_race in
/// `SimSet::Race` (a pause freezes it). Countdown tick `k` (1-based, from entering
/// `RacePhase::Countdown`) shows 3, 2, 1 from ticks 1, 61, 121; the cars are held for all
/// `COUNTDOWN_TICKS` of them. After tick `COUNTDOWN_TICKS` rc_race requests `RacePhase::Racing`;
/// GO (`RaceEvent::Go`, perfect starts / stalls) is applied before the next tick, so the first
/// racing tick (race tick 1) is tick `COUNTDOWN_TICKS + 1` after entering Countdown. Bevy applies
/// state transitions once per frame: when a frame runs several fixed ticks, its ticks after
/// `COUNTDOWN_TICKS` stay in Countdown (cars still held) and GO moves accordingly. Perfect start =
/// throttle pressed in the last 0.3 s (18 ticks) before GO; held since more than 1 s before GO =
/// stall.
pub const COUNTDOWN_TICKS: u32 = 180;

/// Game mode of a session.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GameMode {
    /// A cup race.
    Championship {
        /// The cup.
        cup: CupId,
        /// Index of this race in the cup.
        race_index: usize,
    },
    /// Single race with free options.
    #[default]
    QuickRace,
    /// Solo vs ghost, no items, 3 laps.
    TimeTrial,
    /// 2–4 local players.
    Versus,
}

/// Who drives a racer.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Controller {
    /// A local player; `autopilot` adds an `AiDriver` (camera and HUD stay).
    Human {
        /// The player's slot.
        slot: PlayerSlot,
        /// Driven by the AI (`--autopilot`).
        autopilot: bool,
    },
    /// An AI rival.
    Ai {
        /// Rival personality.
        rival: RivalId,
        /// Difficulty.
        difficulty: Difficulty,
    },
}

impl Default for Controller {
    fn default() -> Self {
        Self::Human {
            slot: PlayerSlot(0),
            autopilot: false,
        }
    }
}

/// One grid entry of a race.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RacerSetup {
    /// Display name (P1–P4 in Versus, rival name for AI).
    pub name: String,
    /// Car.
    pub car: CarId,
    /// Paint.
    pub paint: PaintId,
    /// Racer colour (markers, minimap), sRGB 0–1.
    pub color: [f32; 3],
    /// Driver.
    pub controller: Controller,
}

/// One recorded ghost pose.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct GhostSample {
    /// Position.
    pub translation: Vec3,
    /// Orientation.
    pub rotation: Quat,
}

/// A Time Trial ghost: poses at [`GHOST_HZ`] (persisted by rc_game as `ghosts/<variant>.ghost`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GhostData {
    /// Car used.
    pub car: CarId,
    /// Paint used.
    pub paint: PaintId,
    /// Total race time in ticks.
    pub time_ticks: u32,
    /// Track geometry hash the ghost was recorded on.
    pub geometry_hash: u64,
    /// Poses, one every `SIM_HZ / GHOST_HZ` ticks from GO.
    pub samples: Vec<GhostSample>,
}

/// Setup of the current session. Written only by rc_game: before `Loading` (replaced for a next
/// race) and between attempts during `Setup` (new Time Trial ghost, collected Golden Battery,
/// dropped player → autopilot, Versus players re-sorted by the session points). Everyone else
/// reads it, once per attempt.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct RaceConfig {
    /// Mode.
    pub mode: GameMode,
    /// Layout.
    pub variant: TrackVariant,
    /// Laps.
    pub laps: u8,
    /// Racers in grid order (GDD §3.2: the rivals in front, the local players behind them).
    pub racers: Vec<RacerSetup>,
    /// Items enabled.
    pub items: bool,
    /// Weather and time of day.
    pub weather: WeatherSetup,
    /// AI catch-up enabled.
    pub ai_catch_up: bool,
    /// Seed of every keyed RNG draw in this session.
    pub seed: u64,
    /// Spawn the track's Golden Battery (forward layout, single-player, not yet collected).
    pub golden_battery: bool,
    /// Ghost to race against (Time Trial).
    pub ghost: Option<GhostData>,
}

/// Identity of a racer entity. Spawned by rc_race in `OnEnter(RaceAttempt)` with
/// `DespawnOnExit(RaceAttempt)`; the required components give every racer its shared state.
#[derive(Component, Debug, Clone, Default, PartialEq)]
#[require(
    Interpolated,
    CarControl,
    VehicleTelemetry,
    Charge,
    RaceProgress,
    ItemSlot,
    StatusEffects
)]
pub struct Racer {
    /// Grid index (also the usual RNG key).
    pub index: usize,
    /// Display name.
    pub name: String,
    /// Car.
    pub car: CarId,
    /// Paint.
    pub paint: PaintId,
    /// Racer colour, sRGB 0–1.
    pub color: [f32; 3],
}

/// A human (or autopilot) racer of a local player slot.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalPlayer(pub PlayerSlot);

/// An AI-driven racer (also on autopilot humans).
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct AiDriver {
    /// Personality.
    pub rival: RivalId,
    /// Difficulty.
    pub difficulty: Difficulty,
}

/// Standing of a racer in the race. Written only by rc_race.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct RaceProgress {
    /// Current lap, 1-based (0 before the first crossing of the start line).
    pub lap: u8,
    /// Index of the next checkpoint to pass.
    pub next_checkpoint: usize,
    /// Distance along the current lap in u.
    pub distance: f32,
    /// Race distance in u (completed laps × length + `distance`); ranks unfinished racers.
    pub total_distance: f32,
    /// Position, 1-based.
    pub rank: u8,
    /// Crossed the line on the final lap.
    pub finished: bool,
    /// Completed lap times in ticks.
    pub lap_times: Vec<u32>,
    /// Best lap in ticks.
    pub best_lap: Option<u32>,
    /// Driving the wrong way (> 5 u/s for 2 s).
    pub wrong_way: bool,
    /// Last checkpoint validated in order (respawn points are never ahead of it).
    pub last_valid_checkpoint: usize,
    /// Seconds off the drivable area.
    pub off_track_time: f32,
}

/// Global state of the current attempt. Written only by rc_race; reset in `OnEnter(Setup)`.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct RaceStatus {
    /// Ticks spent in the current phase.
    pub phase_ticks: u32,
    /// Ticks since GO (the race clock).
    pub race_ticks: u32,
    /// Countdown number showing (3, 2, 1; 0 = none).
    pub countdown: u8,
    /// Laps of this race.
    pub total_laps: u8,
    /// Lap of the leader.
    pub leader_lap: u8,
    /// Soft-lock guard: seconds until the race ends, if running.
    pub ends_in: Option<f32>,
    /// Attempt number in this session (1 = first; restarts count up).
    pub attempt: u32,
}

/// Racer entities ordered by rank. Written only by rc_race.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct Standings(pub Vec<Entity>);

/// One line of the results table.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RaceResultEntry {
    /// Grid index of the racer.
    pub index: usize,
    /// Display name.
    pub name: String,
    /// Car.
    pub car: CarId,
    /// Local player slot, if human.
    pub slot: Option<PlayerSlot>,
    /// Final position, 1-based.
    pub rank: u8,
    /// Finished normally (false = classified by progress at the soft-lock guard).
    pub finished: bool,
    /// Total time in ticks (projected when not finished).
    pub time_ticks: u32,
    /// Best lap in ticks.
    pub best_lap_ticks: Option<u32>,
}

/// Results of the finished race. Written by rc_race when the race ends; read by game and ui.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct RaceResults {
    /// Entries ordered by rank.
    pub entries: Vec<RaceResultEntry>,
    /// The human's ghost (Time Trial).
    pub ghost: Option<GhostData>,
    /// Geometry hash of the track the race ran on.
    pub geometry_hash: u64,
}

/// Onboarding hints (GDD §13), filtered by rc_game's `hints_seen`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HintKind {
    /// Drifting charges the battery.
    DriftCharge,
    /// Boost is ready.
    BoostReady,
    /// An item is ready.
    ItemReady,
    /// Stuck for 3 s: hold Reset.
    HoldReset,
    /// Driving the wrong way.
    WrongWay,
}

/// What happened in the race.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RaceEventKind {
    /// Countdown number shown (3, 2, 1).
    CountdownTick(u8),
    /// GO.
    Go,
    /// Throttle timed within 0.3 s before GO.
    PerfectStart,
    /// Throttle held too early: wheel-spin stall.
    Stall,
    /// Time Trial split vs ghost/best at a checkpoint, seconds (+ = slower).
    CheckpointSplit(f32),
    /// A lap was completed.
    LapCompleted {
        /// The completed lap, 1-based.
        lap: u8,
        /// Lap time in ticks.
        time: u32,
        /// Personal best of this race.
        best: bool,
    },
    /// The final lap started.
    FinalLap,
    /// The racer finished.
    Finished {
        /// Position.
        rank: u8,
        /// Total time in ticks.
        time: u32,
    },
    /// Passed a checkpoint's `s` outside its gate: the racer's distance stays frozen there until
    /// it drives back through the gate (once per missed pass).
    MissedCheckpoint,
    /// Wrong-way warning on/off.
    WrongWay(bool),
    /// The Golden Battery was collected.
    GoldenBattery,
    /// Soft-lock guard countdown (last 10 s).
    RaceEndsIn(u8),
    /// Show an onboarding hint.
    Hint(HintKind),
}

/// A race happening (producer race; consumers ui, audio, game, vfx).
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct RaceEvent {
    /// The racer it concerns (`None` = global, e.g. `Go`).
    pub racer: Option<Entity>,
    /// What happened.
    pub kind: RaceEventKind,
}

/// The track's collectible (attempt-scoped, spawned by rc_race when `RaceConfig::golden_battery`).
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GoldenBattery;

/// The Time Trial ghost car (attempt-scoped, spawned by rc_race).
#[derive(Component, Debug, Clone, Default, PartialEq, Eq)]
#[require(Interpolated)]
pub struct GhostCar {
    /// Car model.
    pub car: CarId,
    /// Paint.
    pub paint: PaintId,
}
