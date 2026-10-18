//! `assets/data/items.ron` → [`ItemTuning`]: the odds table (GDD §6.3) and every item's numbers
//! (GDD §6.2). Loaded in `OnEnter(AppState::Loading)`; dev builds poll the file once a second.

use std::path::Path;

use bevy::prelude::*;
use rc_core::prelude::*;
use serde::Deserialize;

use crate::odds::{Band, Odds};

/// The data file, relative to the assets dir.
pub const ITEMS_FILE: &str = "data/items.ron";

/// Loading-gate key of this crate.
pub const LOADING_KEY: &str = "items";

/// A knock-back: Δv of the reference car (`up`, `push` along the hit direction, u/s) and an
/// angular impulse (`spin`) about the horizontal axis across the push.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tumble {
    /// Upward Δv (u/s).
    pub up: f32,
    /// Δv along the push (u/s).
    pub push: f32,
    /// Angular impulse.
    pub spin: f32,
}

/// Volt Cubes.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PickupTuning {
    /// Car centre to cube centre that takes it (u).
    pub radius: f32,
    /// Seconds until a taken cube is back.
    pub respawn_secs: f32,
    /// Roulette length (s).
    pub roulette_secs: f32,
}

/// Turbo Cell boost.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TurboTuning {
    /// Top-speed multiplier.
    pub top_speed: f32,
    /// Drive-force multiplier.
    pub force: f32,
    /// Duration (s).
    pub secs: f32,
}

/// Spark Rocket.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RocketTuning {
    /// u/s.
    pub speed: f32,
    /// Lock cone half-angle (degrees).
    pub lock_cone_deg: f32,
    /// Lock range fired forward (u).
    pub lock_range: f32,
    /// Lock range fired backward (u).
    pub lock_range_back: f32,
    /// Homing turn rate (degrees/s).
    pub turn_rate_deg: f32,
    /// Body radius against the world (u).
    pub radius: f32,
    /// Car centre to the flight path that detonates it (u).
    pub hit_radius: f32,
    /// Cars within this of the detonation tumble (u).
    pub blast_radius: f32,
    /// Seconds of flight before it self-detonates.
    pub life_secs: f32,
    /// Knock-back.
    pub tumble: Tumble,
}

/// Leech Coil.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LeechTuning {
    /// Reach to a car ahead in the race (u).
    pub ahead_range: f32,
    /// Reach to any car when none is ahead (u).
    pub near_range: f32,
    /// Seconds spent searching.
    pub wait_secs: f32,
    /// Seconds tethered.
    pub secs: f32,
    /// Share of the target's charge moved over `secs`.
    pub share: f32,
    /// Top-speed multiplier while tethered.
    pub tow: f32,
    /// The tether breaks beyond this (u).
    pub break_range: f32,
}

/// Shock Orb.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrbTuning {
    /// u/s.
    pub speed: f32,
    /// Cars within this are knocked (u).
    pub radius: f32,
    /// Lifetime (s).
    pub life_secs: f32,
    /// Hits before it is spent.
    pub hits: u32,
    /// Height above the track surface (u).
    pub height: f32,
    /// Bounces this far inside the racing width (u).
    pub edge_margin: f32,
    /// Largest launch angle to the track direction (degrees).
    pub max_angle_deg: f32,
    /// Sideways knock-back.
    pub knock: Tumble,
}

/// EMP Pulse.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmpTuning {
    /// Wind-up before the pulse (s).
    pub windup_secs: f32,
    /// Pulse radius (u).
    pub radius: f32,
    /// Drive-force multiplier of the victims.
    pub power: f32,
    /// Power loss duration (s).
    pub secs: f32,
    /// Share of the victims' charge drained.
    pub drain: f32,
}

/// Goo Bomb.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GooTuning {
    /// Fuse (s).
    pub fuse_secs: f32,
    /// The new carrier cannot pass it back to the giver for this long (s).
    pub no_pass_back_secs: f32,
    /// Explosion radius (u).
    pub radius: f32,
    /// Knock-back.
    pub tumble: Tumble,
}

/// Oil Slick.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OilTuning {
    /// Puddle radius (u).
    pub radius: f32,
    /// Lifetime (s).
    pub life_secs: f32,
    /// Grip multiplier.
    pub grip: f32,
    /// Grip loss duration (s).
    pub secs: f32,
    /// Yaw impulse on entry.
    pub spin: f32,
}

/// Spring Pad.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpringTuning {
    /// Pad radius (u).
    pub radius: f32,
    /// Lifetime (s).
    pub life_secs: f32,
    /// Upward Δv (u/s).
    pub up: f32,
    /// Δv along the car's heading (u/s).
    pub forward: f32,
    /// Random angular impulse, at most per axis.
    pub spin: f32,
}

/// Shield Bubble.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShieldTuning {
    /// Duration (s).
    pub secs: f32,
    /// Δv given to a car it bumps (u/s).
    pub push: f32,
}

/// Storm Strike.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StormTuning {
    /// Between strikes (s).
    pub interval_secs: f32,
    /// Drive-force multiplier of the victims.
    pub power: f32,
    /// Power loss duration (s).
    pub secs: f32,
    /// Upward Δv (u/s).
    pub hop: f32,
}

/// Everything in `items.ron` (session resource: inserted on `OnEnter(Loading)`, removed on
/// `OnEnter(Unloading)`).
#[derive(Resource, Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(missing_docs)]
pub struct ItemTuning {
    pub pickups: PickupTuning,
    /// Own rockets and hazards ignore their owner this long (s).
    pub owner_immunity_secs: f32,
    pub odds: Odds,
    pub turbo_cell: TurboTuning,
    pub spark_rocket: RocketTuning,
    pub leech_coil: LeechTuning,
    pub shock_orb: OrbTuning,
    pub emp_pulse: EmpTuning,
    pub goo_bomb: GooTuning,
    pub oil_slick: OilTuning,
    pub spring_pad: SpringTuning,
    pub shield_bubble: ShieldTuning,
    pub storm_strike: StormTuning,
}

impl ItemTuning {
    /// Reads and validates `<assets_dir>/data/items.ron`; errors name the file and the field.
    pub fn load(assets_dir: &Path) -> Result<Self, String> {
        let path = assets_dir.join(ITEMS_FILE);
        let text = std::fs::read_to_string(&path)
            .map_err(|e| format!("{ITEMS_FILE}: cannot read {}: {e}", path.display()))?;
        Self::parse(&text)
    }

    /// Parses and validates the file's text.
    pub fn parse(text: &str) -> Result<Self, String> {
        let tuning: Self = ron::from_str(text).map_err(|e| format!("{ITEMS_FILE}: {e}"))?;
        let problems = tuning.problems();
        if problems.is_empty() {
            Ok(tuning)
        } else {
            Err(format!("{ITEMS_FILE}: {}", problems.join("; ")))
        }
    }

    /// Validation messages naming the field (empty = valid).
    pub fn problems(&self) -> Vec<String> {
        let mut out = Vec::new();
        for (name, band) in [
            ("first", Band::First),
            ("top", Band::Top),
            ("middle", Band::Middle),
            ("last", Band::Last),
        ] {
            let sum: u32 = self.odds.weights(band).iter().sum();
            if sum != 100 {
                out.push(format!("`odds.{name}` sums to {sum}, expected 100"));
            }
        }
        let (r, l, o, e, g) = (
            &self.spark_rocket,
            &self.leech_coil,
            &self.shock_orb,
            &self.emp_pulse,
            &self.goo_bomb,
        );
        let positive = [
            ("pickups.radius", self.pickups.radius),
            ("pickups.respawn_secs", self.pickups.respawn_secs),
            ("pickups.roulette_secs", self.pickups.roulette_secs),
            ("turbo_cell.secs", self.turbo_cell.secs),
            ("spark_rocket.speed", r.speed),
            ("spark_rocket.lock_cone_deg", r.lock_cone_deg),
            ("spark_rocket.turn_rate_deg", r.turn_rate_deg),
            ("spark_rocket.radius", r.radius),
            ("spark_rocket.hit_radius", r.hit_radius),
            ("spark_rocket.blast_radius", r.blast_radius),
            ("spark_rocket.life_secs", r.life_secs),
            ("leech_coil.secs", l.secs),
            ("leech_coil.break_range", l.break_range),
            ("shock_orb.speed", o.speed),
            ("shock_orb.radius", o.radius),
            ("shock_orb.life_secs", o.life_secs),
            ("emp_pulse.radius", e.radius),
            ("emp_pulse.secs", e.secs),
            ("goo_bomb.fuse_secs", g.fuse_secs),
            ("goo_bomb.radius", g.radius),
            ("oil_slick.radius", self.oil_slick.radius),
            ("oil_slick.life_secs", self.oil_slick.life_secs),
            ("oil_slick.secs", self.oil_slick.secs),
            ("spring_pad.radius", self.spring_pad.radius),
            ("spring_pad.life_secs", self.spring_pad.life_secs),
            ("shield_bubble.secs", self.shield_bubble.secs),
            ("storm_strike.secs", self.storm_strike.secs),
        ];
        for (name, v) in positive {
            if !(v > 0.0 && v.is_finite()) {
                out.push(format!("`{name}` = {v}, must be > 0"));
            }
        }
        let shares = [
            ("leech_coil.share", l.share),
            ("emp_pulse.drain", e.drain),
            ("emp_pulse.power", e.power),
            ("oil_slick.grip", self.oil_slick.grip),
            ("storm_strike.power", self.storm_strike.power),
        ];
        for (name, v) in shares {
            if !(0.0..=1.0).contains(&v) {
                out.push(format!("`{name}` = {v}, must be within 0–1"));
            }
        }
        if !(1.0..=TOP_SPEED_CAP).contains(&self.turbo_cell.top_speed) {
            out.push(format!(
                "`turbo_cell.top_speed` must be within 1–{TOP_SPEED_CAP}"
            ));
        }
        if self.shock_orb.hits == 0 {
            out.push("`shock_orb.hits` must be ≥ 1".into());
        }
        out
    }
}

/// `OnEnter(AppState::Loading)`: reads the tuning (`LoadingDone("items")` or `LoadingFailed`).
pub(crate) fn load_tuning(
    paths: Option<Res<UserPaths>>,
    mut commands: Commands,
    mut done: MessageWriter<LoadingDone>,
    mut failed: MessageWriter<LoadingFailed>,
) {
    let result = paths
        .ok_or_else(|| "no UserPaths resource".to_owned())
        .and_then(|p| ItemTuning::load(&p.assets_dir));
    match result {
        Ok(tuning) => {
            commands.insert_resource(tuning);
            done.write(LoadingDone(LOADING_KEY));
        }
        Err(error) => {
            error!("items: {error}");
            failed.write(LoadingFailed {
                key: LOADING_KEY,
                error,
            });
        }
    }
}

/// `OnEnter(AppState::Unloading)`: the session's tuning goes.
pub(crate) fn unload_tuning(mut commands: Commands) {
    commands.remove_resource::<ItemTuning>();
}

/// Debug builds: poll the file's modification time (≤ 1 Hz) and reload it when it changes; a bad
/// edit is reported and the old tuning kept.
#[cfg(debug_assertions)]
pub(crate) fn hot_reload(
    time: Res<Time<Real>>,
    paths: Option<Res<UserPaths>>,
    mut tuning: ResMut<ItemTuning>,
    mut poll: Local<(f32, Option<std::time::SystemTime>)>,
) {
    const POLL_SECS: f32 = 1.0;
    if time.elapsed_secs() < poll.0 {
        return;
    }
    poll.0 = time.elapsed_secs() + POLL_SECS;
    let Some(paths) = paths else { return };
    let path = paths.assets_dir.join(ITEMS_FILE);
    let Ok(modified) = std::fs::metadata(&path).and_then(|m| m.modified()) else {
        return;
    };
    if poll.1.replace(modified).is_none_or(|seen| seen == modified) {
        return;
    }
    match ItemTuning::load(&paths.assets_dir) {
        Ok(new) => {
            info!("items hot reload: {ITEMS_FILE}");
            *tuning = new;
        }
        Err(e) => warn!("items hot reload: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tuning() -> ItemTuning {
        ItemTuning::parse(include_str!("../../../assets/data/items.ron")).expect("items.ron")
    }

    #[test]
    fn data_file_is_valid_and_rows_sum_to_100() {
        let t = tuning();
        assert!(t.problems().is_empty(), "{:?}", t.problems());
        for band in [Band::First, Band::Top, Band::Middle, Band::Last] {
            assert_eq!(t.odds.weights(band).iter().sum::<u32>(), 100, "{band:?}");
        }
        // GDD §6.3 spot checks: the leader never gets attack items, Storm Strike only at the back.
        let first = t.odds.weights(Band::First);
        assert_eq!(
            first[2..6],
            [0, 0, 0, 0],
            "no Leech/Orb/EMP/Goo for the leader"
        );
        for band in [Band::First, Band::Top, Band::Middle] {
            assert_eq!(t.odds.weights(band)[9], 0, "Storm Strike in {band:?}");
        }
        assert_eq!(t.odds.weights(Band::Last)[9], 10);
    }

    #[test]
    fn problems_name_the_field() {
        let mut t = tuning();
        t.odds.first.insert(ItemKind::StormStrike, 5);
        t.spark_rocket.speed = 0.0;
        let p = t.problems().join("\n");
        assert!(p.contains("`odds.first` sums to 105"), "{p}");
        assert!(p.contains("`spark_rocket.speed`"), "{p}");
        assert!(ItemTuning::parse("(pickups: ())").is_err());
    }
}
