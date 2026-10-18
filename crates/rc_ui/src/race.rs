//! `race.*` channels: start, restart, resume, quit.

use bevy::prelude::*;
use bevy_react::prelude::*;
use rc_core::prelude::*;
use rc_game::{GameCommand, RaceRequest};

/// React → Bevy message `race.start`: start a race from the menu.
#[react_message(name = "race.start")]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct StartRace {
    /// Track id (`catalog.ron`).
    pub track: String,
    /// Reverse layout.
    #[ts(optional)]
    pub reverse: Option<bool>,
    /// Mirrored.
    #[ts(optional)]
    pub mirror: Option<bool>,
    /// Car id (default: the first car).
    #[ts(optional)]
    pub car: Option<String>,
    /// AI opponents (default: fill the grid).
    #[ts(optional)]
    pub ai: Option<u8>,
    /// Laps (default: the track's).
    #[ts(optional)]
    pub laps: Option<u8>,
}

/// React → Bevy message `race.restart`: restart the current race (no reload).
#[react_message(name = "race.restart")]
pub struct RestartRace;

/// React → Bevy message `race.resume`: close the pause menu (idempotent).
#[react_message(name = "race.resume")]
pub struct ResumeRace;

/// React → Bevy message `race.quit`: leave the race for the menu.
#[react_message(name = "race.quit")]
pub struct QuitRace;

pub(crate) fn register(app: &mut App) {
    app.add_react_handler(on_start)
        .add_react_handler(|_: On<RestartRace>, mut out: MessageWriter<GameCommand>| {
            out.write(GameCommand::Restart);
        })
        .add_react_handler(|_: On<ResumeRace>, mut out: MessageWriter<GameCommand>| {
            out.write(GameCommand::Resume);
        })
        .add_react_handler(|_: On<QuitRace>, mut out: MessageWriter<GameCommand>| {
            out.write(GameCommand::QuitToMenu);
        });
}

fn on_start(start: On<StartRace>, mut out: MessageWriter<GameCommand>) {
    let start = start.event();
    out.write(GameCommand::StartRace(RaceRequest {
        variant: TrackVariant {
            track: TrackId::new(start.track.as_str()),
            reverse: start.reverse.unwrap_or(false),
            mirror: start.mirror.unwrap_or(false),
        },
        car: start.car.as_deref().map(CarId::from),
        ai: start.ai,
        laps: start.laps,
        ..RaceRequest::quick(start.track.as_str())
    }));
}
