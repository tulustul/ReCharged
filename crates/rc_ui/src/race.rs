//! `race.*` channels (start, restart, next race, rematch, next track, resume, quit, the session
//! snapshot) and `loading.progress`. Results, standings and the scoreboard: [`crate::results`].

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_react::prelude::*;
use rc_core::prelude::*;
use rc_game::{CupRun, GameCommand, PlayerCar, RaceOutcome, RaceRequest, VersusSession};

use crate::content::{UiDifficulty, UiTimeOfDay, UiWeatherChoice};
use crate::results::{UiCupStandings, UiResults, UiVersusScore};

/// A game mode.
#[derive(serde::Serialize, serde::Deserialize, ts_rs::TS, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(tag = "kind", rename_all = "camelCase")]
pub enum UiMode {
    /// Quick Race.
    QuickRace,
    /// Time Trial.
    TimeTrial,
    /// Versus.
    Versus,
    /// A cup race.
    Championship {
        /// Cup id.
        cup: String,
        /// Race index in the cup, 0-based.
        race: usize,
    },
}

impl From<&GameMode> for UiMode {
    fn from(mode: &GameMode) -> Self {
        match mode {
            GameMode::QuickRace => Self::QuickRace,
            GameMode::TimeTrial => Self::TimeTrial,
            GameMode::Versus => Self::Versus,
            GameMode::Championship { cup, race_index } => Self::Championship {
                cup: cup.0.clone(),
                race: *race_index,
            },
        }
    }
}

impl From<&UiMode> for GameMode {
    fn from(mode: &UiMode) -> Self {
        match mode {
            UiMode::QuickRace => Self::QuickRace,
            UiMode::TimeTrial => Self::TimeTrial,
            UiMode::Versus => Self::Versus,
            UiMode::Championship { cup, race } => Self::Championship {
                cup: CupId::new(cup.as_str()),
                race_index: *race,
            },
        }
    }
}

/// A local player's car choice.
#[derive(serde::Deserialize, ts_rs::TS, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct UiPlayerChoice {
    /// Slot 0–3.
    pub slot: u8,
    /// Car id.
    pub car: String,
    /// Paint id.
    pub paint: u8,
}

/// React → Bevy message `race.start`: start a race from the menu (rc_game validates it).
#[react_message(name = "race.start")]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct StartRace {
    /// Mode.
    pub mode: UiMode,
    /// Track id.
    pub track: String,
    /// Reverse layout.
    pub reverse: bool,
    /// Mirrored.
    pub mirror: bool,
    /// Local players in slot order (one outside Versus).
    pub players: Vec<UiPlayerChoice>,
    /// AI opponents (default: fill the grid).
    #[ts(optional)]
    pub ai: Option<u8>,
    /// AI difficulty (default: the setting).
    #[ts(optional)]
    pub difficulty: Option<UiDifficulty>,
    /// Laps (default: the track's).
    #[ts(optional)]
    pub laps: Option<u8>,
    /// Weather (default: the track's first preset).
    #[ts(optional)]
    pub weather: Option<UiWeatherChoice>,
    /// Time of day.
    #[ts(optional)]
    pub time: Option<UiTimeOfDay>,
    /// Items on/off.
    #[ts(optional)]
    pub items: Option<bool>,
}

impl From<&StartRace> for RaceRequest {
    fn from(start: &StartRace) -> Self {
        let first = start.players.first();
        let versus = start.mode == UiMode::Versus;
        Self {
            mode: (&start.mode).into(),
            variant: TrackVariant {
                track: TrackId::new(start.track.as_str()),
                reverse: start.reverse,
                mirror: start.mirror,
            },
            car: first.map(|p| CarId::new(p.car.as_str())),
            paint: PaintId(first.map_or(0, |p| p.paint)),
            players: start.players.len().clamp(1, MAX_LOCAL_PLAYERS) as u8,
            // Versus: every joined player with their own car (rc_game validates slots and cars).
            humans: if versus {
                start
                    .players
                    .iter()
                    .map(|p| PlayerCar {
                        slot: PlayerSlot(p.slot),
                        car: CarId::new(p.car.as_str()),
                        paint: PaintId(p.paint),
                    })
                    .collect()
            } else {
                Vec::new()
            },
            ai: start.ai,
            difficulty: start.difficulty.map(Into::into),
            laps: start.laps,
            weather: start.weather.map(Into::into),
            time: start.time.map(Into::into),
            items: start.items,
            seed: None,
        }
    }
}

/// A layout.
#[derive(serde::Deserialize, ts_rs::TS, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct UiTrackVariant {
    /// Track id.
    pub track: String,
    /// Reverse layout.
    pub reverse: bool,
    /// Mirrored.
    pub mirror: bool,
}

/// React → Bevy message `race.restart`: same race again, no reload (Pause/Results "Restart",
/// Time Trial "Retry").
#[react_message(name = "race.restart")]
pub struct RestartRace;

/// React → Bevy message `race.next`: the next cup race.
#[react_message(name = "race.next")]
pub struct NextRace;

/// React → Bevy message `race.rematch`: Versus, same track again (the scoreboard keeps counting).
#[react_message(name = "race.rematch")]
pub struct Rematch;

/// React → Bevy message `race.nextTrack`: Versus, P1 picked the next track.
#[react_message(name = "race.nextTrack")]
pub struct NextTrack(pub UiTrackVariant);

/// React → Bevy message `race.resume`: close the pause menu (idempotent).
#[react_message(name = "race.resume")]
pub struct ResumeRace;

/// React → Bevy message `race.quit`: leave the race for the menu.
#[react_message(name = "race.quit")]
pub struct QuitRace;

/// A racer of the session.
#[derive(serde::Serialize, ts_rs::TS, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct UiRacer {
    /// Display name.
    pub name: String,
    /// Car id.
    pub car: String,
    /// Local player slot (`null` = AI).
    pub slot: Option<u8>,
    /// Racer colour, sRGB 0–1.
    pub color: [f32; 3],
}

/// The session's setup (loading screen, pause, results).
#[derive(serde::Serialize, ts_rs::TS, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct UiRaceSetup {
    /// Mode.
    pub mode: UiMode,
    /// Track id.
    pub track: String,
    /// Reverse layout.
    pub reverse: bool,
    /// Mirrored.
    pub mirror: bool,
    /// Laps.
    pub laps: u8,
    /// Weather.
    pub weather: UiWeatherChoice,
    /// Time of day.
    pub time: UiTimeOfDay,
    /// Items enabled.
    pub items: bool,
    /// Racers in grid order.
    pub racers: Vec<UiRacer>,
}

impl From<&RaceConfig> for UiRaceSetup {
    fn from(config: &RaceConfig) -> Self {
        Self {
            mode: (&config.mode).into(),
            track: config.variant.track.0.clone(),
            reverse: config.variant.reverse,
            mirror: config.variant.mirror,
            laps: config.laps,
            weather: config.weather.weather.into(),
            time: config.weather.time.into(),
            items: config.items,
            racers: config
                .racers
                .iter()
                .map(|r| UiRacer {
                    name: r.name.clone(),
                    car: r.car.0.clone(),
                    slot: match r.controller {
                        Controller::Human { slot, .. } => Some(slot.0),
                        Controller::Ai { .. } => None,
                    },
                    color: r.color,
                })
                .collect(),
        }
    }
}

/// Bevy → React event `loading.progress`: fraction of loaders done.
#[react_event(name = "loading.progress")]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoadingProgress {
    /// 0–1.
    pub progress: f32,
}

/// Response of `race.snapshot`: the session as far as it exists.
#[derive(serde::Serialize, ts_rs::TS, Debug, Clone, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct UiRaceSession {
    /// Setup (`null` outside a session).
    pub setup: Option<UiRaceSetup>,
    /// Results of the finished race.
    pub results: Option<UiResults>,
    /// Cup standings (cup sessions).
    pub cup: Option<UiCupStandings>,
    /// Scoreboard (Versus sessions).
    pub versus: Option<UiVersusScore>,
}

/// React → Bevy request `race.snapshot`: the [`UiRaceSession`].
#[react_request(name = "race.snapshot", response = UiRaceSession)]
pub struct RaceSnapshot;

/// The snapshot inputs (one param keeps the observer within Bevy's param limit).
#[derive(SystemParam)]
struct Session<'w> {
    config: Option<Res<'w, RaceConfig>>,
    results: Option<Res<'w, RaceResults>>,
    outcome: Option<Res<'w, RaceOutcome>>,
    cup: Option<Res<'w, CupRun>>,
    versus: Option<Res<'w, VersusSession>>,
}

pub(crate) fn register(app: &mut App) {
    app.add_react_event::<LoadingProgress>()
        .add_react_request_handler(|request: On<Request<RaceSnapshot>>, s: Session| {
            request.respond(UiRaceSession {
                setup: s.config.map(|c| UiRaceSetup::from(&*c)),
                results: s.results.map(|r| UiResults::new(&r, s.outcome.as_deref())),
                cup: s.cup.map(|c| UiCupStandings::from(&*c)),
                versus: s.versus.map(|v| UiVersusScore::from(&*v)),
            });
        })
        .add_react_handler(
            |start: On<StartRace>, mut out: MessageWriter<GameCommand>| {
                out.write(GameCommand::StartRace(RaceRequest::from(start.event())));
            },
        )
        .add_react_handler(|_: On<RestartRace>, mut out: MessageWriter<GameCommand>| {
            out.write(GameCommand::Restart);
        })
        .add_react_handler(|_: On<NextRace>, mut out: MessageWriter<GameCommand>| {
            out.write(GameCommand::NextRace);
        })
        .add_react_handler(|_: On<Rematch>, mut out: MessageWriter<GameCommand>| {
            out.write(GameCommand::Rematch);
        })
        .add_react_handler(|next: On<NextTrack>, mut out: MessageWriter<GameCommand>| {
            let v = &next.event().0;
            out.write(GameCommand::NextTrack(TrackVariant {
                track: TrackId::new(v.track.as_str()),
                reverse: v.reverse,
                mirror: v.mirror,
            }));
        })
        .add_react_handler(|_: On<ResumeRace>, mut out: MessageWriter<GameCommand>| {
            out.write(GameCommand::Resume);
        })
        .add_react_handler(|_: On<QuitRace>, mut out: MessageWriter<GameCommand>| {
            out.write(GameCommand::QuitToMenu);
        });
}

/// Sends `loading.progress` while loading, when a loader reports.
pub(crate) fn push_loading(
    state: Res<State<AppState>>,
    tasks: Res<LoadingTasks>,
    events: ReactEvents,
) {
    if *state.get() == AppState::Loading && tasks.is_changed() {
        events.send(&LoadingProgress {
            progress: tasks.progress(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versus_start_maps_to_a_race_request() {
        let start = StartRace {
            mode: UiMode::Championship {
                cup: "pro_cup".into(),
                race: 0,
            },
            track: "toy_town".into(),
            reverse: true,
            mirror: false,
            players: vec![
                UiPlayerChoice {
                    slot: 0,
                    car: "zip".into(),
                    paint: 2,
                },
                UiPlayerChoice {
                    slot: 1,
                    car: "sparky".into(),
                    paint: 0,
                },
            ],
            ai: Some(3),
            difficulty: Some(UiDifficulty::Elite),
            laps: None,
            weather: Some(UiWeatherChoice::Random),
            time: None,
            items: Some(false),
        };
        let request = RaceRequest::from(&start);
        assert_eq!(
            request.mode,
            GameMode::Championship {
                cup: CupId::new("pro_cup"),
                race_index: 0
            }
        );
        assert!(request.variant.reverse && !request.variant.mirror);
        assert_eq!(
            (request.car.as_ref().map(CarId::as_str), request.paint),
            (Some("zip"), PaintId(2))
        );
        assert_eq!(request.players, 2);
        assert_eq!(request.difficulty, Some(Difficulty::Elite));
        assert_eq!(request.weather, Some(WeatherChoice::Random));
        assert_eq!(request.items, Some(false));
    }
}
