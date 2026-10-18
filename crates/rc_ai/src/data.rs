//! `assets/data/ai.ron` ([`AiTuning`]): difficulty profiles, personalities and driving knobs.
//! Loaded on `OnEnter(AppState::Loading)` (`LoadingDone("ai")`, or `LoadingFailed` naming the
//! file and field), removed on `OnEnter(AppState::Unloading)`, hot-reloaded in debug builds.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use bevy::prelude::*;
use rc_core::prelude::*;
use serde::Deserialize;

/// Loading-gate key of this crate.
pub const LOADING_KEY: &str = "ai";

/// The tuning file relative to the assets root.
pub const AI_FILE: &str = "data/ai.ron";

/// The whole tuning file. Every struct rejects unknown fields (typos fail at load).
#[derive(Resource, Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiTuning {
    /// Per difficulty (GDD §8.2).
    pub difficulty: Difficulties,
    /// Personality of unknown rival ids (the autopilot).
    pub neutral: Personality,
    /// Personality by rival id (catalog ids).
    pub personalities: BTreeMap<String, Personality>,
    /// Steering and speed planning.
    pub driving: Driving,
    /// Overtaking and hazard avoidance.
    pub traffic: Traffic,
    /// Drift for charge, boost on straights.
    pub charge: ChargeUse,
    /// Stuck detection and recovery.
    pub recovery: Recovery,
    /// Catch-up (GDD §8.1).
    pub catch_up: CatchUpTuning,
    /// Item use conditions (GDD §8.1).
    pub items: ItemTuning,
    /// Steering assist (GDD §15).
    pub assist: AssistTuning,
}

/// The four difficulty profiles.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Difficulties {
    /// Rookie.
    pub rookie: DifficultyProfile,
    /// Pro.
    pub pro: DifficultyProfile,
    /// Elite.
    pub elite: DifficultyProfile,
    /// Elite+ (Grand Volt Cup).
    pub elite_plus: DifficultyProfile,
}

impl Difficulties {
    /// The profile of `difficulty`.
    pub fn get(&self, difficulty: Difficulty) -> &DifficultyProfile {
        match difficulty {
            Difficulty::Rookie => &self.rookie,
            Difficulty::Pro => &self.pro,
            Difficulty::Elite => &self.elite,
            Difficulty::ElitePlus => &self.elite_plus,
        }
    }
}

/// How one difficulty drives.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DifficultyProfile {
    /// Multiplies the car's own speed profile.
    pub speed_scale: f32,
    /// Share of the car's cornering grip the speed profile uses.
    pub corner_grip: f32,
    /// Mistakes per minute at consistency 0.5.
    pub mistakes_per_min: f32,
    /// Item reaction delay range (s).
    pub reaction_secs: (f32, f32),
    /// Perfect-start chance.
    pub perfect_start: f32,
    /// Share of drift zones drifted.
    pub drift: f32,
    /// Personal racing-line offset range (± u).
    pub line_spread_u: f32,
    /// Extra item hoarding added to the personality's.
    pub hoard: f32,
}

/// Rival traits not in the catalog (aggression and consistency are).
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Personality {
    /// Tendency to keep items for the best moment, 0–1.
    pub hoard: f32,
    /// Eagerness to spend charge, 0–1.
    pub boost: f32,
}

/// Steering and speed planning.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Driving {
    /// Pure-pursuit lookahead time (s).
    pub lookahead_secs: f32,
    /// Shortest lookahead (u).
    pub lookahead_min_u: f32,
    /// Longest lookahead (u).
    pub lookahead_max_u: f32,
    /// Cornering acceleration by Handling stars 1–5 (u/s², five values).
    pub lateral_accel_by_handling: Vec<f32>,
    /// Braking the speed plan assumes (u/s²).
    pub brake_decel: f32,
    /// The AI line stays this far inside the drivable edge (u).
    pub edge_margin_u: f32,
    /// Lateral speed of lane changes (u/s).
    pub lane_change_ups: f32,
    /// Speed scale after finishing.
    pub finished_speed_scale: f32,
}

/// Overtaking and hazard avoidance.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Traffic {
    /// Obstacles this far ahead along the track are considered (u).
    pub look_ahead_u: f32,
    /// Lateral gap kept between car bodies when passing (u).
    pub side_gap_u: f32,
    /// Clearance kept around hazards (u).
    pub hazard_clearance_u: f32,
    /// Extra clearance around a car carrying a Goo Bomb (u).
    pub goo_clearance_u: f32,
}

/// Drift for charge, boost on straights.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChargeUse {
    /// Drift corners tighter than this radius (u).
    pub drift_max_radius_u: f32,
    /// … lasting at least this long at the planned speed (s).
    pub drift_min_secs: f32,
    /// … above this share of the top speed.
    pub drift_min_speed: f32,
    /// Drifts end this long before the corner does (s).
    pub drift_exit_secs: f32,
    /// Counter-steer beyond this ends a drift (it turns too tight).
    pub drift_counter_steer: f32,
    /// Boost when the plan stays at top speed this long ahead (s, × (1.5 − boost)).
    pub boost_straight_secs: f32,
}

/// Stuck detection and recovery.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recovery {
    /// "Stuck" below this speed with the throttle down (u/s) …
    pub stuck_ups: f32,
    /// … for this long (s).
    pub stuck_secs: f32,
    /// Or less progress than this (u) …
    pub progress_u: f32,
    /// … in this long (s).
    pub progress_secs: f32,
    /// Reversing time (s).
    pub reverse_secs: f32,
    /// Reverse tries before holding reset.
    pub reset_after_tries: u32,
    /// How long reset is held (s; rc_race respawns after 0.4 s).
    pub reset_hold_secs: f32,
    /// Off the course this long → reset (s).
    pub off_course_secs: f32,
    /// Upside down and slow this long → reset tap (s).
    pub flipped_secs: f32,
}

/// Catch-up (GDD §8.1).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatchUpTuning {
    /// Largest scaling (± share of top speed).
    pub max: f32,
    /// No catch-up within this race distance of the leading human (u).
    pub dead_zone_u: f32,
    /// Full catch-up from this race distance (u).
    pub full_at_u: f32,
    /// Largest change per second.
    pub rate_per_sec: f32,
}

/// Item use conditions (GDD §8.1).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemTuning {
    /// Rocket target cone half-angle (degrees).
    pub rocket_cone_deg: f32,
    /// Rocket range ahead (u).
    pub rocket_range_u: f32,
    /// Rocket range behind (u).
    pub rocket_back_range_u: f32,
    /// Leech Coil range (u).
    pub leech_range_u: f32,
    /// Shock Orb range along the track (u).
    pub orb_range_u: f32,
    /// EMP radius used for the decision (u).
    pub emp_radius_u: f32,
    /// Oil / Spring Pad: a car this close behind (u).
    pub drop_behind_u: f32,
    /// Goo Bomb: use / seek cars this close (u).
    pub goo_seek_u: f32,
    /// Turbo Cell: straight this long ahead (s).
    pub turbo_straight_secs: f32,
    /// A full hoarder keeps a defensive item this long without a threat (s).
    pub hoard_secs: f32,
}

/// Steering assist (GDD §15).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssistTuning {
    /// Blend share toward the racing-line steer.
    pub strength: f32,
    /// Blend share after a respawn or crash.
    pub recovery_strength: f32,
    /// Duration of the recovery pull (s).
    pub recovery_secs: f32,
}

impl AiTuning {
    /// Parses and validates `text` (read from `path`); errors name the file and field.
    pub fn parse(text: &str, path: &Path) -> Result<Self, String> {
        let tuning: Self = ron::from_str(text).map_err(|e| format!("{}:{e}", path.display()))?;
        let problems = tuning.problems();
        match problems.is_empty() {
            true => Ok(tuning),
            false => Err(format!("{}: {}", path.display(), problems.join("; "))),
        }
    }

    /// Reads `data/ai.ron` under `assets_dir`.
    pub fn load(assets_dir: &Path) -> Result<Self, String> {
        let path = assets_dir.join(AI_FILE);
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        Self::parse(&text, &path)
    }

    /// The personality of `rival` (neutral when unknown).
    pub fn personality(&self, rival: &str) -> Personality {
        self.personalities
            .get(rival)
            .copied()
            .unwrap_or(self.neutral)
    }

    /// Problems with the values, each naming the field (empty = valid).
    pub fn problems(&self) -> Vec<String> {
        let mut out = Vec::new();
        let mut check = |name: String, ok: bool| {
            if !ok {
                out.push(format!("`{name}` is out of range"));
            }
        };
        let unit = |v: f32| (0.0..=1.0).contains(&v);
        let positive = |v: f32| v.is_finite() && v > 0.0;
        let d = &self.difficulty;
        for (name, p) in [
            ("rookie", &d.rookie),
            ("pro", &d.pro),
            ("elite", &d.elite),
            ("elite_plus", &d.elite_plus),
        ] {
            let f = |field: &str| format!("difficulty.{name}.{field}");
            check(f("speed_scale"), (0.5..=1.2).contains(&p.speed_scale));
            check(f("corner_grip"), (0.5..=1.1).contains(&p.corner_grip));
            check(
                f("mistakes_per_min"),
                (0.0..=30.0).contains(&p.mistakes_per_min),
            );
            let (lo, hi) = p.reaction_secs;
            check(f("reaction_secs"), 0.0 <= lo && lo <= hi && hi <= 5.0);
            check(f("perfect_start"), unit(p.perfect_start));
            check(f("drift"), unit(p.drift));
            check(f("line_spread_u"), (0.0..=4.0).contains(&p.line_spread_u));
            check(f("hoard"), unit(p.hoard));
        }
        let personalities = std::iter::once(("neutral".to_owned(), &self.neutral)).chain(
            self.personalities
                .iter()
                .map(|(id, p)| (format!("personalities.{id}"), p)),
        );
        for (name, p) in personalities {
            check(format!("{name}.hoard"), unit(p.hoard));
            check(format!("{name}.boost"), unit(p.boost));
        }
        let v = &self.driving;
        check("driving.lookahead_secs".into(), positive(v.lookahead_secs));
        check(
            "driving.lookahead_max_u".into(),
            positive(v.lookahead_min_u) && v.lookahead_min_u <= v.lookahead_max_u,
        );
        check(
            "driving.lateral_accel_by_handling".into(),
            v.lateral_accel_by_handling.len() == 5
                && v.lateral_accel_by_handling.iter().all(|&a| positive(a)),
        );
        check("driving.brake_decel".into(), positive(v.brake_decel));
        check(
            "driving.edge_margin_u".into(),
            (0.6..=4.0).contains(&v.edge_margin_u),
        );
        check(
            "driving.lane_change_ups".into(),
            positive(v.lane_change_ups),
        );
        check(
            "driving.finished_speed_scale".into(),
            (0.2..=1.2).contains(&v.finished_speed_scale),
        );
        let t = &self.traffic;
        for (name, x) in [
            ("look_ahead_u", t.look_ahead_u),
            ("side_gap_u", t.side_gap_u),
            ("hazard_clearance_u", t.hazard_clearance_u),
            ("goo_clearance_u", t.goo_clearance_u),
        ] {
            check(format!("traffic.{name}"), positive(x));
        }
        let c = &self.charge;
        check(
            "charge.drift_max_radius_u".into(),
            positive(c.drift_max_radius_u),
        );
        check("charge.drift_min_secs".into(), positive(c.drift_min_secs));
        check("charge.drift_min_speed".into(), unit(c.drift_min_speed));
        check("charge.drift_exit_secs".into(), c.drift_exit_secs >= 0.0);
        check(
            "charge.drift_counter_steer".into(),
            unit(c.drift_counter_steer),
        );
        check(
            "charge.boost_straight_secs".into(),
            positive(c.boost_straight_secs),
        );
        let r = &self.recovery;
        for (name, x) in [
            ("stuck_ups", r.stuck_ups),
            ("stuck_secs", r.stuck_secs),
            ("progress_u", r.progress_u),
            ("progress_secs", r.progress_secs),
            ("reverse_secs", r.reverse_secs),
            ("reset_hold_secs", r.reset_hold_secs),
            ("off_course_secs", r.off_course_secs),
            ("flipped_secs", r.flipped_secs),
        ] {
            check(format!("recovery.{name}"), positive(x));
        }
        check("recovery.reset_hold_secs".into(), r.reset_hold_secs > 0.4);
        let k = &self.catch_up;
        check("catch_up.max".into(), (0.0..=0.06).contains(&k.max));
        check(
            "catch_up.full_at_u".into(),
            k.dead_zone_u >= 0.0 && k.full_at_u > k.dead_zone_u,
        );
        check("catch_up.rate_per_sec".into(), positive(k.rate_per_sec));
        let i = &self.items;
        check(
            "items.rocket_cone_deg".into(),
            (1.0..=35.0).contains(&i.rocket_cone_deg),
        );
        for (name, x) in [
            ("rocket_range_u", i.rocket_range_u),
            ("rocket_back_range_u", i.rocket_back_range_u),
            ("leech_range_u", i.leech_range_u),
            ("orb_range_u", i.orb_range_u),
            ("emp_radius_u", i.emp_radius_u),
            ("drop_behind_u", i.drop_behind_u),
            ("goo_seek_u", i.goo_seek_u),
            ("turbo_straight_secs", i.turbo_straight_secs),
        ] {
            check(format!("items.{name}"), positive(x));
        }
        check("items.hoard_secs".into(), i.hoard_secs >= 0.0);
        let a = &self.assist;
        check("assist.strength".into(), unit(a.strength));
        check("assist.recovery_strength".into(), unit(a.recovery_strength));
        check("assist.recovery_secs".into(), positive(a.recovery_secs));
        out
    }
}

/// Where the tuning came from (for the dev hot reload).
#[derive(Resource, Debug, Clone)]
pub(crate) struct AiTuningFile {
    path: PathBuf,
    mtime: Option<SystemTime>,
}

fn mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// `OnEnter(AppState::Loading)`: read and validate the tuning.
pub(crate) fn load_tuning(
    paths: Option<Res<UserPaths>>,
    mut commands: Commands,
    mut done: MessageWriter<LoadingDone>,
    mut failed: MessageWriter<LoadingFailed>,
) {
    let result = paths
        .ok_or_else(|| "no UserPaths resource".to_owned())
        .and_then(|p| AiTuning::load(&p.assets_dir).map(|t| (t, p.assets_dir.join(AI_FILE))));
    match result {
        Ok((tuning, path)) => {
            info!("ai: tuning loaded from {}", path.display());
            commands.insert_resource(AiTuningFile {
                mtime: mtime(&path),
                path,
            });
            commands.insert_resource(tuning);
            done.write(LoadingDone(LOADING_KEY));
        }
        Err(error) => {
            error!("ai: {error}");
            failed.write(LoadingFailed {
                key: LOADING_KEY,
                error,
            });
        }
    }
}

/// `OnEnter(AppState::Unloading)`: drop the session's tuning.
pub(crate) fn unload_tuning(mut commands: Commands) {
    commands.remove_resource::<AiTuning>();
    commands.remove_resource::<AiTuningFile>();
}

/// Seconds between modification-time polls of the dev hot reload.
#[cfg(debug_assertions)]
const HOT_RELOAD_POLL_SECS: f32 = 1.0;

/// Debug builds: poll the file's modification time (≤ 1 Hz) and reload it when it changed; a
/// broken file keeps the previous tuning (logged).
#[cfg(debug_assertions)]
pub(crate) fn hot_reload(
    time: Res<Time<Real>>,
    mut next_poll: Local<f32>,
    mut file: ResMut<AiTuningFile>,
    mut tuning: ResMut<AiTuning>,
) {
    if time.elapsed_secs() < *next_poll {
        return;
    }
    *next_poll = time.elapsed_secs() + HOT_RELOAD_POLL_SECS;
    let now = mtime(&file.path);
    if now == file.mtime {
        return;
    }
    file.mtime = now;
    let loaded = std::fs::read_to_string(&file.path)
        .map_err(|e| format!("{}: {e}", file.path.display()))
        .and_then(|text| AiTuning::parse(&text, &file.path));
    match loaded {
        Ok(new) if new != *tuning => {
            info!("ai hot reload: {}", file.path.display());
            *tuning = new;
        }
        Ok(_) => {}
        Err(e) => warn!("ai hot reload: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE: &str = include_str!("../../../assets/data/ai.ron");

    fn sample() -> AiTuning {
        AiTuning::parse(FILE, Path::new("ai.ron")).expect("the shipped file is valid")
    }

    #[test]
    fn shipped_file_is_valid_and_covers_every_catalog_rival() {
        let tuning = sample();
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let catalog = Catalog::load(&root).expect("catalog loads");
        for rival in &catalog.rivals {
            assert!(
                tuning.personalities.contains_key(rival.id.as_str()),
                "no personality for rival {}",
                rival.id
            );
        }
        assert_eq!(tuning.personality("autopilot"), tuning.neutral);
    }

    #[test]
    fn difficulty_follows_the_gdd() {
        let d = sample().difficulty;
        let scales = [&d.rookie, &d.pro, &d.elite, &d.elite_plus].map(|p| p.speed_scale);
        assert_eq!(scales, [0.86, 0.94, 1.0, 1.02], "GDD §8.2 speed scales");
        let starts = [&d.rookie, &d.pro, &d.elite, &d.elite_plus].map(|p| p.perfect_start);
        assert_eq!(starts, [0.05, 0.20, 0.40, 0.60], "GDD §3.2 perfect starts");
        assert_eq!(d.rookie.reaction_secs, (0.8, 1.5));
        assert_eq!(d.elite_plus.reaction_secs, (0.15, 0.3));
        assert!(d.get(Difficulty::Rookie).corner_grip < d.get(Difficulty::Elite).corner_grip);
    }

    #[cfg(debug_assertions)]
    #[test]
    fn hot_reload_picks_up_edits_and_survives_broken_ones() {
        use bevy::ecs::system::RunSystemOnce;
        let dir = std::env::temp_dir().join(format!("rc_ai_hot_reload_{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("ai.ron");
        let mut world = World::new();
        world.insert_resource(Time::<Real>::default());
        world.insert_resource(sample());
        let edit = |text: &str, world: &mut World| {
            std::fs::write(&path, text).expect("write");
            // A fresh file record: the next poll sees a changed modification time.
            world.insert_resource(AiTuningFile {
                path: path.clone(),
                mtime: None,
            });
            world.run_system_once(hot_reload).expect("runs");
        };
        edit(
            &FILE.replacen("lookahead_secs: 0.45", "lookahead_secs: 0.5", 1),
            &mut world,
        );
        assert_eq!(world.resource::<AiTuning>().driving.lookahead_secs, 0.5);
        edit("(broken", &mut world);
        assert_eq!(
            world.resource::<AiTuning>().driving.lookahead_secs,
            0.5,
            "kept"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn errors_name_the_file_and_field() {
        let typo = FILE.replacen("lookahead_secs", "lookahead_sec", 1);
        let err = AiTuning::parse(&typo, Path::new("ai.ron")).expect_err("unknown field");
        assert!(
            err.contains("ai.ron") && err.contains("lookahead_sec"),
            "{err}"
        );
        let bad = FILE.replacen("speed_scale: 0.86", "speed_scale: 3.0", 1);
        let err = AiTuning::parse(&bad, Path::new("ai.ron")).expect_err("out of range");
        assert!(err.contains("difficulty.rookie.speed_scale"), "{err}");
    }
}
