//! Item decisions (GDD §8.1, pure): when an AI uses its item, with reaction delays by difficulty
//! and hoarding / aggression by personality.

use rc_core::prelude::*;

use crate::data::{DifficultyProfile, ItemTuning};

/// RNG key offset of the reaction delay draw.
const REACTION_KEY: u64 = 8;
/// Without a reason to wait, a defensive item (shield, oil, spring pad) is used after this long
/// (s) by a driver who does not hoard at all.
const IDLE_USE_SECS: f32 = 4.0;
/// Aggression widens the attack ranges between these shares (0 → 1).
const RANGE_SHARE: (f32, f32) = (0.75, 1.2);
/// The widest share of an attack range any driver uses (targets beyond it are never shot at).
pub(crate) const MAX_RANGE_SHARE: f32 = RANGE_SHARE.1;
/// Hoarders wait for surer shots: attack ranges shrink by up to this share.
const HOARD_RANGE_CUT: f32 = 0.35;

/// What the driver sees around it this tick (track and race geometry already evaluated).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ItemView {
    /// Nearest car inside the forward rocket cone (distance, u); while a Spark Rocket is held,
    /// only cars the rocket could see (no wall or prop in between) count.
    pub cone_ahead: Option<f32>,
    /// Nearest car inside the backward rocket cone (distance, u; in sight, as above).
    pub cone_behind: Option<f32>,
    /// Distance (u) to the car directly ahead in the standings (while a Leech Coil is held: only
    /// when in sight).
    pub next_in_race: Option<f32>,
    /// Nearest car ahead along the track (u).
    pub ahead_on_track: Option<f32>,
    /// Nearest car behind along the track near the own line (u).
    pub behind_on_line: Option<f32>,
    /// Nearest car in any direction (u).
    pub nearest: Option<f32>,
    /// Cars within the EMP radius.
    pub in_emp_radius: usize,
    /// A car a Goo Bomb carrier could bump soon (alongside or a little ahead, close laterally).
    pub goo_in_reach: bool,
    /// A rocket is locked onto this car.
    pub locked_on: bool,
    /// This car is shielded.
    pub shielded: bool,
    /// This car is boosting (any boost).
    pub boosting: bool,
    /// Seconds of straight road ahead at full throttle.
    pub straight_secs: f32,
    /// Race position (1 = leading).
    pub rank: u8,
    /// On a jump's run-up or landing: nothing is dropped here (a pad on the ramp throws the
    /// follower off it).
    pub near_jump: bool,
}

/// How a driver uses items.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Temper {
    /// Rival aggression, 0–1.
    pub aggression: f32,
    /// Hoarding (personality + difficulty), 0–1.
    pub hoard: f32,
}

// ponytail: incoming rockets are answered with a shield only, never a dodge.

/// Use the item now? `Some(backward)` = yes (fired backward with look back).
pub fn decide(
    item: ItemKind,
    v: &ItemView,
    temper: Temper,
    held_secs: f32,
    t: &ItemTuning,
) -> Option<bool> {
    let reach = (RANGE_SHARE.0 + (RANGE_SHARE.1 - RANGE_SHARE.0) * temper.aggression)
        * (1.0 - HOARD_RANGE_CUT * temper.hoard);
    let within = |d: Option<f32>, range: f32| d.is_some_and(|d| d <= range * reach);
    let idle = held_secs >= IDLE_USE_SECS + temper.hoard * t.hoard_secs;
    let forward = Some(false);
    match item {
        ItemKind::SparkRocket => {
            if within(v.cone_ahead, t.rocket_range_u) {
                forward
            } else {
                within(v.cone_behind, t.rocket_back_range_u).then_some(true)
            }
        }
        ItemKind::LeechCoil => within(v.next_in_race, t.leech_range_u).then_some(false),
        ItemKind::ShockOrb => within(v.ahead_on_track, t.orb_range_u).then_some(false),
        ItemKind::EmpPulse => (v.in_emp_radius > 0).then_some(false),
        // Only with a bump in sight: carriers that had to chase blew themselves up.
        ItemKind::GooBomb => v.goo_in_reach.then_some(false),
        ItemKind::OilSlick | ItemKind::SpringPad => {
            let close = v.behind_on_line.is_some_and(|d| d <= t.drop_behind_u);
            // A leader keeps a hazard to defend with; anyone else drops it after a while.
            (!v.near_jump && (close || (idle && v.rank > 1))).then_some(false)
        }
        ItemKind::ShieldBubble => (!v.shielded && (v.locked_on || idle)).then_some(false),
        ItemKind::TurboCell => {
            let need = t.turbo_straight_secs * (1.0 + temper.hoard);
            (!v.boosting && v.straight_secs >= need).then_some(false)
        }
        ItemKind::StormStrike => (v.rank > 1).then_some(false),
    }
}

/// Item decision state of one AI driver.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ItemBrain {
    /// The usable item last tick and the tick it became usable.
    pub held: Option<(ItemKind, u64)>,
    /// Planned use: fire at this tick (the reaction delay).
    pub pending: Option<u64>,
}

impl ItemBrain {
    /// One tick: returns `Some(backward)` on the tick the item is used. `usable` = the held item
    /// once its roulette is over. The first tick a use is wanted starts the reaction delay; the
    /// item goes when the delay is over and the reason still holds.
    #[allow(clippy::too_many_arguments)]
    pub fn step(
        &mut self,
        usable: Option<ItemKind>,
        view: &ItemView,
        temper: Temper,
        skill: &DifficultyProfile,
        t: &ItemTuning,
        (seed, tick, key): (u64, u64, u64),
    ) -> Option<bool> {
        let Some(item) = usable else {
            *self = Self::default();
            return None;
        };
        if self.held.is_none_or(|(k, _)| k != item) {
            *self = Self {
                held: Some((item, tick)),
                pending: None,
            };
        }
        let held_secs = self
            .held
            .map_or(0.0, |(_, since)| (tick - since) as f32 * SIM_DT);
        let want = decide(item, view, temper, held_secs, t);
        match (want, self.pending) {
            (None, _) => {
                self.pending = None;
                None
            }
            (Some(_), None) => {
                let (lo, hi) = skill.reaction_secs;
                let delay = rng::range(
                    seed,
                    tick,
                    RngStream::AiReaction,
                    key + REACTION_KEY,
                    lo,
                    hi,
                );
                self.pending = Some(tick + u64::from(secs_to_ticks(delay)));
                None
            }
            (Some(backward), Some(at)) if tick >= at => {
                self.pending = None;
                Some(backward)
            }
            (Some(_), Some(_)) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tuning() -> ItemTuning {
        ItemTuning {
            rocket_cone_deg: 30.0,
            rocket_range_u: 70.0,
            rocket_back_range_u: 40.0,
            leech_range_u: 38.0,
            orb_range_u: 45.0,
            emp_radius_u: 10.0,
            drop_behind_u: 14.0,
            goo_seek_u: 25.0,
            turbo_straight_secs: 1.5,
            hoard_secs: 20.0,
        }
    }

    const NEUTRAL: Temper = Temper {
        aggression: 0.5,
        hoard: 0.0,
    };

    fn skill() -> DifficultyProfile {
        DifficultyProfile {
            speed_scale: 1.0,
            corner_grip: 0.95,
            mistakes_per_min: 0.0,
            reaction_secs: (0.2, 0.4),
            perfect_start: 0.4,
            drift: 1.0,
            line_spread_u: 0.5,
            hoard: 0.0,
        }
    }

    fn view() -> ItemView {
        ItemView {
            rank: 4,
            ..Default::default()
        }
    }

    #[test]
    fn rockets_fire_at_targets_ahead_else_behind() {
        let t = tuning();
        let ahead = ItemView {
            cone_ahead: Some(30.0),
            cone_behind: Some(10.0),
            ..view()
        };
        assert_eq!(
            decide(ItemKind::SparkRocket, &ahead, NEUTRAL, 0.0, &t),
            Some(false)
        );
        let behind = ItemView {
            cone_behind: Some(10.0),
            ..view()
        };
        assert_eq!(
            decide(ItemKind::SparkRocket, &behind, NEUTRAL, 0.0, &t),
            Some(true)
        );
        assert_eq!(
            decide(ItemKind::SparkRocket, &view(), NEUTRAL, 99.0, &t),
            None,
            "no target"
        );
    }

    #[test]
    fn aggression_and_hoarding_change_the_ranges() {
        let t = tuning();
        let far = ItemView {
            cone_ahead: Some(62.0),
            ..view()
        };
        let mayhem = Temper {
            aggression: 0.95,
            hoard: 0.0,
        };
        let nova = Temper {
            aggression: 0.55,
            hoard: 0.95,
        };
        assert_eq!(
            decide(ItemKind::SparkRocket, &far, mayhem, 0.0, &t),
            Some(false)
        );
        assert_eq!(
            decide(ItemKind::SparkRocket, &far, nova, 0.0, &t),
            None,
            "waits for a surer shot"
        );
    }

    #[test]
    fn use_conditions_follow_the_gdd() {
        let t = tuning();
        let d = |k, v: ItemView| decide(k, &v, NEUTRAL, 0.0, &t);
        assert_eq!(
            d(
                ItemKind::LeechCoil,
                ItemView {
                    next_in_race: Some(20.0),
                    ..view()
                }
            ),
            Some(false)
        );
        assert_eq!(
            d(
                ItemKind::LeechCoil,
                ItemView {
                    next_in_race: Some(60.0),
                    ..view()
                }
            ),
            None
        );
        assert_eq!(
            d(
                ItemKind::ShockOrb,
                ItemView {
                    ahead_on_track: Some(20.0),
                    ..view()
                }
            ),
            Some(false)
        );
        assert_eq!(
            d(
                ItemKind::EmpPulse,
                ItemView {
                    in_emp_radius: 1,
                    ..view()
                }
            ),
            Some(false)
        );
        assert_eq!(d(ItemKind::EmpPulse, view()), None);
        assert_eq!(
            d(
                ItemKind::OilSlick,
                ItemView {
                    behind_on_line: Some(8.0),
                    ..view()
                }
            ),
            Some(false)
        );
        assert_eq!(d(ItemKind::SpringPad, view()), None, "nobody behind");
        let ramp = ItemView {
            behind_on_line: Some(8.0),
            near_jump: true,
            ..view()
        };
        assert_eq!(d(ItemKind::SpringPad, ramp), None, "never on a jump");
        assert_eq!(
            d(
                ItemKind::GooBomb,
                ItemView {
                    goo_in_reach: true,
                    ..view()
                }
            ),
            Some(false)
        );
        assert_eq!(
            d(
                ItemKind::GooBomb,
                ItemView {
                    nearest: Some(10.0),
                    ..view()
                }
            ),
            None,
            "a car near but out of bump reach (behind, or far to the side)"
        );
        assert_eq!(
            d(
                ItemKind::ShieldBubble,
                ItemView {
                    locked_on: true,
                    ..view()
                }
            ),
            Some(false)
        );
        assert_eq!(
            d(
                ItemKind::ShieldBubble,
                ItemView {
                    locked_on: true,
                    shielded: true,
                    ..view()
                }
            ),
            None
        );
        assert_eq!(
            d(
                ItemKind::TurboCell,
                ItemView {
                    straight_secs: 2.0,
                    ..view()
                }
            ),
            Some(false)
        );
        assert_eq!(
            d(
                ItemKind::TurboCell,
                ItemView {
                    straight_secs: 2.0,
                    boosting: true,
                    ..view()
                }
            ),
            None
        );
        assert_eq!(d(ItemKind::StormStrike, view()), Some(false));
        assert_eq!(
            d(ItemKind::StormStrike, ItemView { rank: 1, ..view() }),
            None
        );
    }

    #[test]
    fn hoarders_keep_defensive_items_longer() {
        let t = tuning();
        let calm = Temper {
            aggression: 0.5,
            hoard: 0.0,
        };
        let hoarder = Temper {
            aggression: 0.5,
            hoard: 0.95,
        };
        let held = 6.0;
        assert_eq!(
            decide(ItemKind::ShieldBubble, &view(), calm, held, &t),
            Some(false)
        );
        assert_eq!(
            decide(ItemKind::ShieldBubble, &view(), hoarder, held, &t),
            None
        );
        // Leaders keep a hazard to defend with.
        let leading = ItemView { rank: 1, ..view() };
        assert_eq!(decide(ItemKind::OilSlick, &leading, calm, 60.0, &t), None);
    }

    #[test]
    fn reaction_delay_comes_from_the_difficulty() {
        let t = tuning();
        let target = ItemView {
            cone_ahead: Some(20.0),
            ..view()
        };
        let mut brain = ItemBrain::default();
        let mut fired = None;
        for tick in 100..200 {
            if let Some(back) = brain.step(
                Some(ItemKind::SparkRocket),
                &target,
                NEUTRAL,
                &skill(),
                &t,
                (1, tick, 0),
            ) {
                fired = Some((tick, back));
                break;
            }
        }
        let (tick, back) = fired.expect("fires");
        let delay = (tick - 100) as f32 * SIM_DT;
        assert!((0.2..=0.42).contains(&delay), "delay {delay}");
        assert!(!back);
    }

    #[test]
    fn a_lost_reason_cancels_the_use() {
        let t = tuning();
        let target = ItemView {
            cone_ahead: Some(20.0),
            ..view()
        };
        let mut brain = ItemBrain::default();
        brain.step(
            Some(ItemKind::SparkRocket),
            &target,
            NEUTRAL,
            &skill(),
            &t,
            (1, 0, 0),
        );
        assert!(brain.pending.is_some());
        brain.step(
            Some(ItemKind::SparkRocket),
            &view(),
            NEUTRAL,
            &skill(),
            &t,
            (1, 1, 0),
        );
        assert_eq!(brain.pending, None, "target gone");
        assert_eq!(
            brain.step(None, &target, NEUTRAL, &skill(), &t, (1, 2, 0)),
            None
        );
        assert_eq!(brain, ItemBrain::default(), "nothing held: reset");
    }
}
