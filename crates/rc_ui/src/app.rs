//! `app.*` channels: app state for React stores, build info, quitting.

use bevy::prelude::*;
use bevy_react::prelude::*;
use rc_core::prelude::*;
use rc_game::GameCommand;

/// [`AppState`] as React sees it.
#[derive(serde::Serialize, ts_rs::TS, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum UiAppState {
    /// Booting.
    Boot,
    /// Menus.
    Menu,
    /// Loading a race.
    Loading,
    /// In a race.
    InRace,
    /// Leaving a race (one frame).
    Unloading,
}

/// [`RacePhase`] as React sees it.
#[derive(serde::Serialize, ts_rs::TS, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum UiRacePhase {
    /// Resetting the attempt.
    Setup,
    /// Fly-by.
    Intro,
    /// 3-2-1-GO.
    Countdown,
    /// Racing.
    Racing,
    /// Finished.
    Finished,
}

/// Bevy → React event `app.state`: sent whenever the status changes; also the `app.snapshot`
/// response (events are not buffered, so stores request a snapshot on mount).
#[react_event(name = "app.state")]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppStatus {
    /// App state.
    pub state: UiAppState,
    /// Race phase while in a race.
    pub phase: Option<UiRacePhase>,
    /// The race is paused.
    pub paused: bool,
    /// The 3D menu showroom renders the backdrop car (`ShowroomReady`); without it the menus draw
    /// their own background and 2D car art.
    pub showroom: bool,
}

/// React → Bevy request `app.snapshot`: the current [`AppStatus`].
#[react_request(name = "app.snapshot", response = AppStatus)]
pub struct AppSnapshot;

/// Static facts about the build (title screen).
#[derive(serde::Serialize, ts_rs::TS, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct AppInfo {
    /// Game version, e.g. `"0.1.0"`.
    pub version: String,
}

/// React → Bevy request `app.info`: the [`AppInfo`].
#[react_request(name = "app.info", response = AppInfo)]
pub struct GetAppInfo;

/// React → Bevy message `app.quit`: exit the game (React confirms first).
#[react_message(name = "app.quit")]
pub struct AppQuit;

pub(crate) fn register(app: &mut App) {
    app.add_react_event::<AppStatus>()
        .add_react_request_handler(on_snapshot)
        .add_react_request_handler(|request: On<Request<GetAppInfo>>| {
            request.respond(AppInfo {
                version: env!("CARGO_PKG_VERSION").into(),
            });
        })
        .add_react_handler(|_: On<AppQuit>, mut out: MessageWriter<GameCommand>| {
            out.write(GameCommand::QuitGame);
        });
}

fn status(
    state: &State<AppState>,
    phase: Option<&State<RacePhase>>,
    pause: Option<&State<PauseState>>,
    showroom: bool,
) -> AppStatus {
    AppStatus {
        state: match state.get() {
            AppState::Boot => UiAppState::Boot,
            AppState::Menu => UiAppState::Menu,
            AppState::Loading => UiAppState::Loading,
            AppState::InRace => UiAppState::InRace,
            AppState::Unloading => UiAppState::Unloading,
        },
        phase: phase.map(|p| match p.get() {
            RacePhase::Setup => UiRacePhase::Setup,
            RacePhase::Intro => UiRacePhase::Intro,
            RacePhase::Countdown => UiRacePhase::Countdown,
            RacePhase::Racing => UiRacePhase::Racing,
            RacePhase::Finished => UiRacePhase::Finished,
        }),
        paused: pause.is_some_and(|p| *p.get() == PauseState::Paused),
        showroom,
    }
}

/// Sends `app.state` when the status differs from the last one sent.
pub(crate) fn push_state(
    state: Res<State<AppState>>,
    phase: Option<Res<State<RacePhase>>>,
    pause: Option<Res<State<PauseState>>>,
    showroom: Option<Res<ShowroomReady>>,
    mut sent: Local<Option<AppStatus>>,
    events: ReactEvents,
) {
    let now = status(
        &state,
        phase.as_deref(),
        pause.as_deref(),
        showroom.is_some(),
    );
    if *sent != Some(now) {
        events.send(&now);
        *sent = Some(now);
    }
}

fn on_snapshot(
    request: On<Request<AppSnapshot>>,
    state: Option<Res<State<AppState>>>,
    phase: Option<Res<State<RacePhase>>>,
    pause: Option<Res<State<PauseState>>>,
    showroom: Option<Res<ShowroomReady>>,
) {
    match state {
        Some(state) => request.respond(status(
            &state,
            phase.as_deref(),
            pause.as_deref(),
            showroom.is_some(),
        )),
        None => request.respond_err("app state not initialised"),
    }
}
