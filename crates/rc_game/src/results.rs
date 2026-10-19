//! Progression systems: results → points, records, medals, trophies, stats and unlocks
//! ([`RaceOutcome`]); Golden Batteries; hints; live stats; stale records; profile commands.

use std::fs;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use rc_core::prelude::*;

use crate::medals::MedalTable;
use crate::profile::{Content, Medal, Profile, variant_key};
use crate::records::{NextMedal, apply_results, discard_stale, single_player};
use crate::save::{self, GHOSTS_DIR, SaveState, ghost_path};
use crate::series::{CupRun, RacerKey, VersusSession, points_for};
use crate::{GameCommand, GameNotice, Step};

/// The post-race summary for the Results screen. Inserted once the race's `RaceResults` are
/// processed (in `RacePhase::Finished`); removed on restart and when the session ends.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct RaceOutcome {
    /// Points this race gave each line of `RaceResults::entries` (same order): every racer in a
    /// cup, the players in Versus; `None` for lines that score nothing (AI in Versus, every line
    /// in Quick Race and Time Trial).
    pub points: Vec<Option<u32>>,
    /// New best lap on this layout (single-player).
    pub new_best_lap: bool,
    /// New best race time on this layout (Time Trial).
    pub new_best_race: bool,
    /// Medal of this run (Time Trial).
    pub medal: Option<Medal>,
    /// The next better medal and how far this run was from it (Time Trial).
    pub next_medal: Option<NextMedal>,
    /// This run is now the layout's ghost (Time Trial).
    pub ghost_saved: bool,
    /// Older records of this layout were discarded because the track changed.
    pub records_discarded: bool,
    /// Content this result unlocked.
    pub unlocked: Vec<Content>,
    /// It was the cup's last race: `CupRun::final_position` and `CupRun::trophy` are set.
    pub cup_finished: bool,
}

/// Profile access shared by the progression systems.
#[derive(SystemParam)]
struct Progress<'w> {
    profile: Option<ResMut<'w, Profile>>,
    save: Option<ResMut<'w, SaveState>>,
    catalog: Option<Res<'w, Catalog>>,
    paths: Option<Res<'w, UserPaths>>,
    notices: MessageWriter<'w, GameNotice>,
}

impl Progress<'_> {
    fn dirty(&mut self) {
        if let Some(save) = self.save.as_deref_mut() {
            save.profile_dirty = true;
        }
    }

    /// Earns and announces newly satisfied unlocks.
    fn unlocks(&mut self) -> Vec<Content> {
        let (Some(profile), Some(catalog)) = (self.profile.as_deref_mut(), self.catalog.as_deref())
        else {
            return Vec::new();
        };
        let fresh = profile.refresh_unlocks(catalog);
        for content in &fresh {
            info!("unlocked {content:?}");
            self.notices.write(GameNotice::Unlocked(content.clone()));
        }
        if !fresh.is_empty() {
            self.dirty();
        }
        fresh
    }

    fn delete_ghost(&self, key: &str) {
        if let Some(paths) = self.paths.as_deref() {
            let _ = fs::remove_file(ghost_path(&paths.data_dir, key));
        }
    }
}

pub(crate) fn build(app: &mut App) {
    let unprocessed = resource_exists::<RaceResults>.and_then(not(resource_exists::<RaceOutcome>));
    app.add_systems(
        Update,
        (
            process_results.run_if(in_state(RacePhase::Finished).and_then(unprocessed)),
            race_events,
            live_stats,
            profile_commands,
        )
            .chain()
            .in_set(Step::Progress),
    )
    .add_systems(OnEnter(AppState::InRace), check_geometry)
    .add_systems(OnEnter(RacePhase::Setup), clear_outcome)
    .add_systems(OnEnter(AppState::Unloading), clear_outcome)
    .add_systems(OnEnter(AppState::Menu), menu_return);
}

/// Points into the cup run / Versus session, then records, medals, trophy, stats, unlocks.
fn process_results(
    results: Res<RaceResults>,
    config: Option<ResMut<RaceConfig>>,
    medals: Option<Res<MedalTable>>,
    (mut cup, mut versus): (Option<ResMut<CupRun>>, Option<ResMut<VersusSession>>),
    players: Query<(&LocalPlayer, &RaceProgress)>,
    mut progress: Progress,
    mut commands: Commands,
) {
    let Some(mut config) = config else {
        return;
    };
    let key_of = |e: &RaceResultEntry| {
        config
            .racers
            .get(e.index)
            .map(|r| RacerKey::of(&r.controller))
    };
    let mut outcome = RaceOutcome::default();
    let scored = |e: &RaceResultEntry| match config.mode {
        GameMode::Championship { .. } => true,
        GameMode::Versus => e.slot.is_some(),
        _ => false,
    };
    outcome.points = results
        .entries
        .iter()
        .map(|e| scored(e).then(|| points_for(e.rank)))
        .collect();
    let positions = || {
        let scored_entries = results.entries.iter().filter(|e| scored(e));
        scored_entries
            .filter_map(|e| Some((key_of(e)?, e.rank)))
            .collect::<Vec<_>>()
    };
    if let (GameMode::Championship { .. }, Some(run)) = (&config.mode, cup.as_deref_mut()) {
        run.series.record(positions());
        let total = progress
            .catalog
            .as_deref()
            .and_then(|c| c.cup(&run.cup))
            .map_or(0, |c| c.races.len());
        if run.series.race + 1 >= total {
            run.final_position = run.series.position(&RacerKey::Player(PlayerSlot(0)));
            outcome.cup_finished = true;
            info!(
                "cup {} finished: position {:?}",
                run.cup, run.final_position
            );
        }
    }
    if let (GameMode::Versus, Some(session)) = (&config.mode, versus.as_deref_mut()) {
        session.series.record(positions());
    }

    let distance = players
        .iter()
        .find(|(p, _)| p.0 == PlayerSlot(0))
        .map_or(0.0, |(_, progress)| progress.total_distance);
    let targets = medals.and_then(|m| m.targets(&config.variant));
    let key = variant_key(&config.variant);
    if let Some(profile) = progress.profile.as_deref_mut() {
        let update = apply_results(profile, &config, &results, targets, distance);
        if let Some(run) = cup.as_deref()
            && let Some(trophy) = run.trophy()
        {
            let best = profile.trophies.entry(run.cup.clone()).or_insert(trophy);
            *best = (*best).max(trophy); // the best trophy per cup is kept
        }
        outcome.new_best_lap = update.new_best_lap;
        outcome.new_best_race = update.new_best_race;
        outcome.medal = update.medal;
        outcome.next_medal = update.next_medal;
        outcome.records_discarded = update.discarded;
        if let (true, Some(ghost)) = (update.new_best_race, results.ghost.as_ref()) {
            if let Some(paths) = progress.paths.as_deref() {
                match save::save(&ghost_path(&paths.data_dir, &key), ghost) {
                    Ok(()) => outcome.ghost_saved = true,
                    Err(e) => {
                        error!("ghost: {e}");
                        progress.notices.write(GameNotice::SaveFailed(e));
                    }
                }
            }
            config.ghost = Some(ghost.clone()); // a retry races the new best
        }
    }
    if outcome.records_discarded {
        if !outcome.ghost_saved {
            progress.delete_ghost(&key);
        }
        progress
            .notices
            .write(GameNotice::RecordsDiscarded(config.variant.clone()));
    }
    outcome.unlocked = progress.unlocks();
    progress.dirty();
    commands.insert_resource(outcome);
}

/// Golden Battery pickups (saved at once) and shown hints.
fn race_events(
    mut events: MessageReader<RaceEvent>,
    players: Query<(), With<LocalPlayer>>,
    config: Option<ResMut<RaceConfig>>,
    hints: Option<ResMut<HintsSeen>>,
    mut progress: Progress,
) {
    let (Some(mut config), Some(mut hints)) = (config, hints) else {
        return;
    };
    for event in events.read() {
        let Some(profile) = progress.profile.as_deref_mut() else {
            return;
        };
        match event.kind {
            RaceEventKind::GoldenBattery => {
                let track = config.variant.track.clone();
                let human = event.racer.is_some_and(|e| players.contains(e));
                if human && single_player(&config) && !profile.has_golden_battery(&track) {
                    info!("Golden Battery of {track} collected");
                    profile.golden_batteries.push(track.clone());
                    config.golden_battery = false; // a restart does not bring it back
                    progress.notices.write(GameNotice::GoldenBattery(track));
                    progress.dirty();
                }
            }
            RaceEventKind::Hint(kind) if !profile.hints_seen.contains(&kind) => {
                profile.hints_seen.push(kind);
                hints.0.push(kind);
                progress.dirty();
            }
            _ => {}
        }
    }
}

/// Items used and hits landed by the profile owner (saved with the next save).
fn live_stats(
    mut items: MessageReader<ItemEvent>,
    mut hits: MessageReader<HitEvent>,
    players: Query<&LocalPlayer>,
    profile: Option<ResMut<Profile>>,
) {
    let owner = |e: Entity| players.get(e).is_ok_and(|p| p.0 == PlayerSlot(0));
    let used = items
        .read()
        .filter(|e| matches!(e.kind, ItemEventKind::Used(_)) && owner(e.racer))
        .count() as u32;
    let landed = hits
        .read()
        .filter(|h| !h.blocked && h.attacker.is_some_and(|a| a != h.victim && owner(a)))
        .count() as u32;
    if let Some(mut profile) = profile.filter(|_| used + landed > 0) {
        profile.stats.items_used += used;
        profile.stats.hits_landed += landed;
    }
}

/// Once the track is built: records and the ghost from other geometry are discarded (GDD §9).
fn check_geometry(
    summary: Option<Res<TrackSummary>>,
    config: Option<ResMut<RaceConfig>>,
    mut progress: Progress,
) {
    let (Some(summary), Some(mut config)) = (summary, config) else {
        return;
    };
    let hash = summary.geometry_hash;
    let key = variant_key(&config.variant);
    let stale_ghost = config
        .ghost
        .as_ref()
        .is_some_and(|g| g.geometry_hash != hash);
    if stale_ghost {
        config.ghost = None;
    }
    let discarded = progress
        .profile
        .as_deref_mut()
        .is_some_and(|p| discard_stale(p, &key, hash));
    if discarded || stale_ghost {
        warn!("records of {key} were set on other track geometry: discarded");
        progress.delete_ghost(&key);
        progress
            .notices
            .write(GameNotice::RecordsDiscarded(config.variant.clone()));
        progress.dirty();
    }
}

fn clear_outcome(mut commands: Commands) {
    commands.remove_resource::<RaceOutcome>();
}

/// Back in the menu: announce unlocks earned outside a result (e.g. a Golden Battery, then quit)
/// and save stats and play time.
fn menu_return(mut progress: Progress) {
    progress.unlocks();
    progress.dirty();
}

/// `ResetProgress` (menu only) and `MarkSeen`.
fn profile_commands(
    mut requests: MessageReader<GameCommand>,
    state: Res<State<AppState>>,
    hints: Option<ResMut<HintsSeen>>,
    time: Res<Time<Real>>,
    mut progress: Progress,
) {
    let mut hints = hints;
    for command in requests.read() {
        let Some(profile) = progress.profile.as_deref_mut() else {
            return;
        };
        match command {
            GameCommand::ResetProgress if *state.get() == AppState::Menu => {
                info!("progress reset");
                *profile = Profile {
                    unlock_all: profile.unlock_all,
                    ..default()
                };
                if let Some(hints) = hints.as_deref_mut() {
                    hints.0.clear();
                }
                if let Some(save) = progress.save.as_deref_mut() {
                    save.reset_play_time(time.elapsed_secs_f64());
                }
                if let Some(paths) = progress.paths.as_deref() {
                    let _ = fs::remove_dir_all(paths.data_dir.join(GHOSTS_DIR));
                }
                progress.dirty();
            }
            GameCommand::ResetProgress => warn!("ResetProgress ignored in {:?}", state.get()),
            GameCommand::MarkSeen(content) if profile.new_unlocks.contains(content) => {
                profile.new_unlocks.retain(|c| c != content);
                progress.dirty();
            }
            _ => {}
        }
    }
}
