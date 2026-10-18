//! Session commands (ARCHITECTURE §5.12, GDD §3.3): race starts per mode, restart, next race,
//! rematch, next track, quit, dropping a Versus player, the lobby, `--quick-race`.

use std::time::{SystemTime, UNIX_EPOCH};

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use rc_core::prelude::*;

use crate::flow::{AfterUnload, exits_on_error};
use crate::profile::{Profile, variant_key};
use crate::race_setup::{SetupContext, build_race_config, validate};
use crate::save::load_ghost;
use crate::series::{CupRun, Series, VersusSession};
use crate::{GameCommand, GameNotice, RaceRequest};

/// The request of the running series: `NextRace` / `NextTrack` rebuild from it.
#[derive(Resource, Debug, Clone)]
pub(crate) struct ActiveRequest(pub RaceRequest);

/// What building a `RaceConfig` reads.
#[derive(SystemParam)]
pub(crate) struct Setup<'w> {
    catalog: Option<Res<'w, Catalog>>,
    settings: Option<Res<'w, Settings>>,
    profile: Option<Res<'w, Profile>>,
    automation: Option<Res<'w, AutomationConfig>>,
    paths: Option<Res<'w, UserPaths>>,
}

impl Setup<'_> {
    /// Validates `request` and builds its `RaceConfig` (with the layout's ghost in Time Trial).
    fn config(&self, request: &RaceRequest, dropped: &[PlayerSlot]) -> Result<RaceConfig, String> {
        let (Some(catalog), Some(settings), Some(profile)) = (
            self.catalog.as_deref(),
            self.settings.as_deref(),
            self.profile.as_deref(),
        ) else {
            return Err("the game has not finished booting".into());
        };
        validate(request, catalog, profile)?;
        let ghost = match (&request.mode, self.paths.as_deref()) {
            (GameMode::TimeTrial, Some(paths)) => {
                load_ghost(&paths.data_dir, &variant_key(&request.variant))
            }
            _ => None,
        };
        let ctx = SetupContext {
            catalog,
            settings,
            profile,
            autopilot: self.automation.as_ref().is_some_and(|a| a.autopilot),
            dropped,
            ghost,
        };
        build_race_config(request, &ctx)
    }

    fn reject(&self, what: &str, error: String, out: &mut Outbox) {
        error!("{what}: {error}");
        out.notices.write(GameNotice::Error(error));
        if exits_on_error(self.automation.as_deref()) {
            out.exit.write(AppExit::error());
        }
    }
}

/// Where the session commands send the flow.
#[derive(SystemParam)]
pub(crate) struct Next<'w> {
    app: ResMut<'w, NextState<AppState>>,
    phase: ResMut<'w, NextState<RacePhase>>,
    pause: ResMut<'w, NextState<PauseState>>,
    after: ResMut<'w, AfterUnload>,
}

/// Messages the commands send.
#[derive(SystemParam)]
pub(crate) struct Outbox<'w> {
    notices: MessageWriter<'w, GameNotice>,
    exit: MessageWriter<'w, AppExit>,
    dropped: MessageWriter<'w, PlayerDropped>,
}

/// `--quick-race`: start the race once, skipping the menu. `--mode cup:<id>` starts the cup's
/// race on that track.
pub(crate) fn start_quick_race(
    automation: Option<Res<AutomationConfig>>,
    catalog: Option<Res<Catalog>>,
    mut started: Local<bool>,
    mut out: MessageWriter<GameCommand>,
    mut notices: MessageWriter<GameNotice>,
) {
    let Some(args) = automation.as_ref().and_then(|a| a.quick_race.as_ref()) else {
        return;
    };
    if std::mem::replace(&mut *started, true) {
        return;
    }
    info!("--quick-race {}", args.track);
    let mut request = RaceRequest::from(args);
    if let GameMode::Championship { cup, race_index } = &mut request.mode {
        let found = catalog
            .as_deref()
            .and_then(|c| c.cup(cup))
            .and_then(|c| cup_race_index(c, &request.variant));
        match found {
            Some(index) => *race_index = index,
            None => {
                let error = format!("--mode cup:{cup}: no race on {}", args.track);
                error!("{error}");
                notices.write(GameNotice::Error(error));
                return;
            }
        }
    }
    out.write(GameCommand::StartRace(request));
}

/// The cup race on `variant` (exact layout first, then any layout of its track).
pub(crate) fn cup_race_index(cup: &CupInfo, variant: &TrackVariant) -> Option<usize> {
    let races = || cup.races.iter();
    races()
        .position(|r| r.variant == *variant)
        .or_else(|| races().position(|r| r.variant.track == variant.track))
}

pub(crate) fn start_race(
    mut requests: MessageReader<GameCommand>,
    state: Res<State<AppState>>,
    setup: Setup,
    (versus, mut lobby): (Option<Res<VersusSession>>, ResMut<LobbyState>),
    mut next: ResMut<NextState<AppState>>,
    mut commands: Commands,
    mut out: Outbox,
) {
    let mut started = false;
    for command in requests.read() {
        let GameCommand::StartRace(request) = command else {
            continue;
        };
        if *state.get() != AppState::Menu || started {
            warn!("StartRace ignored in {:?}", state.get());
            continue;
        }
        let mut request = request.clone();
        request.seed = request.seed.or_else(|| Some(clock_seed()));
        match setup.config(&request, &[]) {
            Ok(config) => {
                info!(
                    "starting {:?} on {:?} ({} racers, {} laps, seed {})",
                    config.mode,
                    config.variant,
                    config.racers.len(),
                    config.laps,
                    config.seed
                );
                begin_series(&mut commands, &request, &config, versus.as_deref());
                lobby.set_if_neq(LobbyState {
                    open: false,
                    ..*lobby
                });
                commands.insert_resource(config);
                commands.insert_resource(ActiveRequest(request));
                next.set(AppState::Loading);
                started = true;
            }
            Err(e) => setup.reject("cannot start race", e, &mut out),
        }
    }
}

/// A cup starts a new run; Versus continues the lobby's session (or starts one); other modes end
/// both.
fn begin_series(
    commands: &mut Commands,
    request: &RaceRequest,
    config: &RaceConfig,
    versus: Option<&VersusSession>,
) {
    match &request.mode {
        GameMode::Championship { cup, race_index } => {
            commands.insert_resource(CupRun {
                cup: cup.clone(),
                series: Series {
                    race: *race_index,
                    results: vec![Vec::new(); *race_index],
                },
                racers: config.racers.clone(),
                final_position: None,
            });
            commands.remove_resource::<VersusSession>();
        }
        GameMode::Versus => {
            let mut session = versus.cloned().unwrap_or_default();
            session.series.race = session.series.races_played();
            session.dropped.clear(); // the lobby's players are back
            commands.insert_resource(session);
            commands.remove_resource::<CupRun>();
        }
        GameMode::QuickRace | GameMode::TimeTrial => {
            commands.remove_resource::<CupRun>();
            commands.remove_resource::<VersusSession>();
        }
    }
}

/// Restart, rematch, next race, next track, quit and drop player.
pub(crate) fn session_commands(
    mut requests: MessageReader<GameCommand>,
    state: Res<State<AppState>>,
    setup: Setup,
    (mut cup, mut versus): (Option<ResMut<CupRun>>, Option<ResMut<VersusSession>>),
    (mut config, mut active): (Option<ResMut<RaceConfig>>, Option<ResMut<ActiveRequest>>),
    mut next: Next,
    mut out: Outbox,
) {
    let state = *state.get();
    for command in requests.read() {
        let in_race = state == AppState::InRace;
        match command {
            GameCommand::Restart | GameCommand::Rematch if in_race => {
                let rematch = *command == GameCommand::Rematch;
                match versus.as_deref_mut() {
                    Some(session) if rematch && session.series.race_done() => {
                        session.series.race += 1;
                    }
                    Some(session) => session.series.revert(),
                    None => {}
                }
                if let Some(run) = cup.as_deref_mut() {
                    run.series.revert();
                    run.final_position = None;
                }
                info!("{command:?}");
                next.phase.set(RacePhase::Setup);
                next.pause.set(PauseState::Running);
            }
            GameCommand::NextRace | GameCommand::NextTrack(_) if in_race => {
                let (Some(config), Some(active)) = (config.as_deref(), active.as_deref_mut())
                else {
                    continue;
                };
                let session = (cup.as_deref_mut(), versus.as_deref_mut());
                match next_session(command, config, active, session, &setup) {
                    Ok(Some(after)) => {
                        *next.after = after;
                        next.app.set(AppState::Unloading);
                    }
                    Ok(None) => {}
                    Err(e) => setup.reject("cannot continue", e, &mut out),
                }
            }
            GameCommand::QuitToMenu if matches!(state, AppState::Loading | AppState::InRace) => {
                *next.after = AfterUnload::Menu;
                next.app.set(AppState::Unloading);
            }
            GameCommand::DropPlayer(slot) if in_race => {
                let (Some(config), Some(session)) = (config.as_deref_mut(), versus.as_deref_mut())
                else {
                    warn!("DropPlayer: not a Versus race");
                    continue;
                };
                for racer in &mut config.racers {
                    if let Controller::Human { slot: s, autopilot } = &mut racer.controller
                        && s == slot
                    {
                        *autopilot = true;
                    }
                }
                if !session.dropped.contains(slot) {
                    session.dropped.push(*slot);
                }
                info!("P{} dropped: the AI takes over", slot.0 + 1);
                out.dropped.write(PlayerDropped { slot: *slot });
            }
            GameCommand::Restart
            | GameCommand::Rematch
            | GameCommand::NextRace
            | GameCommand::NextTrack(_)
            | GameCommand::QuitToMenu
            | GameCommand::DropPlayer(_) => warn!("{command:?} ignored in {state:?}"),
            _ => {}
        }
    }
}

/// Where `NextRace` / `NextTrack` lead: the next session's setup, the menu (cup over), or nowhere
/// (`Ok(None)`: not allowed now).
fn next_session(
    command: &GameCommand,
    config: &RaceConfig,
    active: &mut ActiveRequest,
    (cup, versus): (Option<&mut CupRun>, Option<&mut VersusSession>),
    setup: &Setup,
) -> Result<Option<AfterUnload>, String> {
    let mut request = active.0.clone();
    request.seed = Some(rng::draw(config.seed, 1, RngStream::RaceSetup, u64::MAX));
    match (command, cup) {
        (GameCommand::NextTrack(_), Some(_)) => {
            warn!("NextTrack is not available in a cup");
            return Ok(None);
        }
        (_, Some(run)) => {
            let total = setup
                .catalog
                .as_deref()
                .and_then(|c| c.cup(&run.cup))
                .map_or(0, |c| c.races.len());
            if !run.series.race_done() {
                warn!("NextRace: the cup race has no result yet");
                return Ok(None);
            }
            if run.series.race + 1 >= total {
                info!("cup finished");
                return Ok(Some(AfterUnload::Menu));
            }
            request.mode = GameMode::Championship {
                cup: run.cup.clone(),
                race_index: run.series.race + 1,
            };
            let next = setup.config(&request, &[])?;
            run.series.race += 1;
            active.0 = request;
            return Ok(Some(AfterUnload::Reload(Box::new(next))));
        }
        (GameCommand::NextTrack(variant), None) => request.variant = variant.clone(),
        (_, None) => {}
    }
    let dropped = versus.as_ref().map_or(&[][..], |s| &s.dropped[..]);
    let next = if *command == GameCommand::NextRace && request.variant == config.variant {
        let mut same = config.clone(); // the same setup again
        same.seed = request.seed.unwrap_or(config.seed);
        same
    } else {
        setup.config(&request, dropped)?
    };
    if let Some(session) = versus
        && session.series.race_done()
    {
        session.series.race += 1;
    }
    active.0 = request;
    Ok(Some(AfterUnload::Reload(Box::new(next))))
}

/// Opening and closing the Versus lobby.
pub(crate) fn lobby_commands(
    mut requests: MessageReader<GameCommand>,
    state: Res<State<AppState>>,
    mut lobby: ResMut<LobbyState>,
    mut commands: Commands,
) {
    for command in requests.read() {
        match command {
            GameCommand::OpenLobby if *state.get() == AppState::Menu => {
                lobby.set_if_neq(LobbyState {
                    open: true,
                    ..*lobby
                });
            }
            GameCommand::OpenLobby => warn!("OpenLobby ignored in {:?}", state.get()),
            GameCommand::CloseLobby => {
                lobby.set_if_neq(LobbyState {
                    open: false,
                    ..*lobby
                });
                commands.remove_resource::<VersusSession>();
            }
            _ => {}
        }
    }
}

fn clock_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::tests::catalog;

    #[test]
    fn cup_races_are_found_by_layout_then_by_track() {
        let c = catalog();
        let pro = c.cup(&"pro_cup".into()).expect("pro cup");
        let reverse = TrackVariant {
            reverse: true,
            ..TrackVariant::forward("sofa_circuit")
        };
        assert_eq!(cup_race_index(pro, &reverse), Some(3));
        assert_eq!(
            cup_race_index(pro, &TrackVariant::forward("sofa_circuit")),
            Some(3)
        );
        assert_eq!(
            cup_race_index(pro, &TrackVariant::forward("wrench_works")),
            Some(1)
        );
        assert_eq!(
            cup_race_index(pro, &TrackVariant::forward("toy_town")),
            None
        );
    }
}
