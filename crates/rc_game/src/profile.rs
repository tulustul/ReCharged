//! The player profile (GDD §9): unlocks, trophies, medals, Golden Batteries, records, stats and
//! seen hints, plus the pure unlock rules. Persisted as `profile.ron` (see `save`).

use std::collections::BTreeMap;

use bevy::prelude::*;
use rc_core::prelude::*;
use serde::{Deserialize, Serialize};

/// Anything that can be locked (GDD §9 unlock table).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Content {
    /// A car.
    Car(CarId),
    /// The forward layout of a track (its reverse/mirror layouts need `Feature(Reverse/Mirror)`).
    Track(TrackId),
    /// A cup.
    Cup(CupId),
    /// A global unlockable: Reverse, Mirror, a difficulty, a paint set, the credits.
    Feature(Unlockable),
}

/// A cup trophy (GDD §3.1): 1st Gold, 2nd Silver, 3rd Bronze. Ordered worst to best.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Trophy {
    /// 3rd place.
    Bronze,
    /// 2nd place.
    Silver,
    /// Cup win.
    Gold,
}

impl Trophy {
    /// The trophy for a final cup position (1-based); `None` off the podium.
    pub fn for_position(position: u8) -> Option<Self> {
        match position {
            1 => Some(Self::Gold),
            2 => Some(Self::Silver),
            3 => Some(Self::Bronze),
            _ => None,
        }
    }
}

/// A Time Trial medal (GDD §9). Ordered worst to best; Gold and Platinum are "gold medals".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Medal {
    /// Reference time × 1.15.
    Bronze,
    /// Reference time × 1.05.
    Silver,
    /// Reference time × 0.98.
    Gold,
    /// Reference time × 0.95.
    Platinum,
}

impl Medal {
    /// Best first.
    pub const BEST_FIRST: [Medal; 4] = [Self::Platinum, Self::Gold, Self::Silver, Self::Bronze];

    /// Target time = the layout's reference time × this factor (GDD §9).
    pub fn factor(self) -> f32 {
        match self {
            Self::Platinum => 0.95,
            Self::Gold => 0.98,
            Self::Silver => 1.05,
            Self::Bronze => 1.15,
        }
    }

    /// Counts for `UnlockRule::GoldMedals` (Gold or Platinum).
    pub fn is_gold(self) -> bool {
        self >= Self::Gold
    }
}

/// A timed record and the car that set it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimedRecord {
    /// Time in simulation ticks (`ticks_to_secs`).
    pub ticks: u32,
    /// Car used.
    pub car: CarId,
}

/// Records of one layout (see [`variant_key`]).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LayoutRecord {
    /// Geometry the times were set on; a race on other geometry discards them (GDD §9).
    pub geometry_hash: u64,
    /// Best lap from any single-player mode.
    pub best_lap: Option<TimedRecord>,
    /// Best race time, Time Trial only (its ghost is `ghosts/<variant>.ghost`).
    pub best_race: Option<TimedRecord>,
    /// Best Time Trial medal (kept when the times are discarded).
    pub medal: Option<Medal>,
}

/// Lifetime statistics of the profile owner (the player in slot 0).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Stats {
    /// Races finished (results processed).
    pub races: u32,
    /// 1st places.
    pub wins: u32,
    /// Top-3 places.
    pub podiums: u32,
    /// Distance driven in finished races, in u.
    pub distance: f64,
    /// Items used.
    pub items_used: u32,
    /// Item hits on other cars (not blocked by a shield).
    pub hits_landed: u32,
    /// Seconds the game has been running, over all sessions.
    pub time_played_secs: f64,
}

/// Progress that persists across sessions. Written only by rc_game; read by rc_ui.
#[derive(Resource, Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Profile {
    /// Content earned so far (announced once). Stays unlocked even if the progress behind it is
    /// later discarded.
    pub unlocked: Vec<Content>,
    /// Earned content not yet viewed in a menu ("NEW" badge; `GameCommand::MarkSeen` clears it).
    pub new_unlocks: Vec<Content>,
    /// Best trophy per cup.
    pub trophies: BTreeMap<CupId, Trophy>,
    /// Tracks whose Golden Battery was collected.
    pub golden_batteries: Vec<TrackId>,
    /// Records per layout, keyed by [`variant_key`] (use [`Profile::record`]).
    pub records: BTreeMap<String, LayoutRecord>,
    /// Lifetime statistics.
    pub stats: Stats,
    /// Onboarding hints already shown.
    pub hints_seen: Vec<HintKind>,
    /// `--unlock-all`: everything is usable. In memory only — never saved, never earns anything.
    #[serde(skip)]
    pub unlock_all: bool,
}

impl Profile {
    /// Whether `rule` holds for this profile's progress (ignores `unlock_all`).
    pub fn rule_met(&self, rule: &UnlockRule) -> bool {
        match rule {
            UnlockRule::Start => true,
            UnlockRule::CupPodium(cup) => self.trophies.contains_key(cup),
            UnlockRule::CupGold(cup) => self.trophies.get(cup) == Some(&Trophy::Gold),
            UnlockRule::GoldMedals(n) => self.gold_medals() >= *n,
            UnlockRule::GoldenBatteries(n) => self.golden_batteries.len() >= *n as usize,
        }
    }

    /// Whether `content` may be used: `--unlock-all`, earned before, or its rule holds. Content
    /// unknown to the catalog, and the Elite+ difficulty (cup-only), are never unlocked.
    pub fn is_unlocked(&self, catalog: &Catalog, content: &Content) -> bool {
        unlock_rule(catalog, content).is_some_and(|rule| {
            self.unlock_all || self.unlocked.contains(content) || self.rule_met(rule)
        })
    }

    /// Whether a layout may be raced: its track, plus Reverse / Mirror when used.
    pub fn variant_unlocked(&self, catalog: &Catalog, variant: &TrackVariant) -> bool {
        let feature = |u| self.is_unlocked(catalog, &Content::Feature(u));
        self.is_unlocked(catalog, &Content::Track(variant.track.clone()))
            && (!variant.reverse || feature(Unlockable::Reverse))
            && (!variant.mirror || feature(Unlockable::Mirror))
    }

    /// Whether a paint may be used (base paints always; others need their paint set).
    pub fn paint_unlocked(&self, catalog: &Catalog, car: &CarId, paint: PaintId) -> bool {
        let Some(info) = catalog
            .car(car)
            .and_then(|c| c.paints.iter().find(|p| p.id == paint))
        else {
            return false;
        };
        info.set == PaintSet::Base
            || self.is_unlocked(catalog, &Content::Feature(Unlockable::PaintSet(info.set)))
    }

    /// Gold or Platinum medals over all layouts.
    pub fn gold_medals(&self) -> u32 {
        let golds = self.records.values().filter_map(|r| r.medal);
        golds.filter(|m| m.is_gold()).count() as u32
    }

    /// Records of a layout.
    pub fn record(&self, variant: &TrackVariant) -> Option<&LayoutRecord> {
        self.records.get(&variant_key(variant))
    }

    /// Best trophy of a cup.
    pub fn trophy(&self, cup: &CupId) -> Option<Trophy> {
        self.trophies.get(cup).copied()
    }

    /// The track's Golden Battery was collected.
    pub fn has_golden_battery(&self, track: &TrackId) -> bool {
        self.golden_batteries.contains(track)
    }

    /// Records newly satisfied unlocks (never `Start` content or hidden tracks) as earned and NEW;
    /// returns them for announcement.
    pub(crate) fn refresh_unlocks(&mut self, catalog: &Catalog) -> Vec<Content> {
        let fresh: Vec<Content> = lockable(catalog)
            .filter(|(content, rule)| {
                **rule != UnlockRule::Start
                    && self.rule_met(rule)
                    && !self.unlocked.contains(content)
            })
            .map(|(content, _)| content)
            .collect();
        self.unlocked.extend(fresh.iter().cloned());
        self.new_unlocks.extend(fresh.iter().cloned());
        fresh
    }
}

/// The unlock rule of `content` (`None` = unknown or not unlockable).
pub fn unlock_rule<'a>(catalog: &'a Catalog, content: &Content) -> Option<&'a UnlockRule> {
    match content {
        Content::Car(id) => catalog.car(id).map(|c| &c.unlock),
        Content::Track(id) => catalog.track(id).map(|t| &t.unlock),
        Content::Cup(id) => catalog.cup(id).map(|c| &c.unlock),
        Content::Feature(u) => catalog.unlock_rule(*u),
    }
}

/// Every lockable item of the catalog with its rule (hidden dev tracks excluded).
fn lockable(catalog: &Catalog) -> impl Iterator<Item = (Content, &UnlockRule)> {
    let cars = catalog
        .cars
        .iter()
        .map(|c| (Content::Car(c.id.clone()), &c.unlock));
    let tracks = catalog.tracks.iter().filter(|t| !t.hidden);
    let tracks = tracks.map(|t| (Content::Track(t.id.clone()), &t.unlock));
    let cups = catalog
        .cups
        .iter()
        .map(|c| (Content::Cup(c.id.clone()), &c.unlock));
    let features = catalog
        .unlocks
        .iter()
        .map(|(u, r)| (Content::Feature(*u), r));
    cars.chain(tracks).chain(cups).chain(features)
}

/// Key of a layout for records, medal targets and ghost files: `toy_town`, `toy_town_rev`,
/// `toy_town_mirror`, `toy_town_rev_mirror`.
pub fn variant_key(variant: &TrackVariant) -> String {
    let mut key = variant.track.to_string();
    if variant.reverse {
        key.push_str("_rev");
    }
    if variant.mirror {
        key.push_str("_mirror");
    }
    key
}

#[cfg(test)]
pub(crate) mod tests {
    use std::path::Path;

    use super::*;

    pub(crate) fn catalog() -> Catalog {
        Catalog::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")).expect("catalog")
    }

    fn gold(profile: &mut Profile, layouts: &[&str]) {
        for key in layouts {
            profile.records.entry((*key).into()).or_default().medal = Some(Medal::Gold);
        }
    }

    #[test]
    fn every_rule_kind_is_evaluated() {
        let mut p = Profile::default();
        let rookie = CupId::new("rookie_cup");
        assert!(p.rule_met(&UnlockRule::Start));
        assert!(!p.rule_met(&UnlockRule::CupPodium(rookie.clone())));
        p.trophies.insert(rookie.clone(), Trophy::Bronze);
        assert!(p.rule_met(&UnlockRule::CupPodium(rookie.clone())));
        assert!(!p.rule_met(&UnlockRule::CupGold(rookie.clone())));
        p.trophies.insert(rookie.clone(), Trophy::Gold);
        assert!(p.rule_met(&UnlockRule::CupGold(rookie)));

        gold(&mut p, &["toy_town", "toy_town_rev"]);
        p.records.entry("sofa_circuit".into()).or_default().medal = Some(Medal::Silver);
        assert_eq!(p.gold_medals(), 2, "silver does not count");
        assert!(!p.rule_met(&UnlockRule::GoldMedals(3)));
        p.records.entry("aisle_rush".into()).or_default().medal = Some(Medal::Platinum);
        assert!(
            p.rule_met(&UnlockRule::GoldMedals(3)),
            "platinum counts as gold"
        );

        p.golden_batteries = vec!["toy_town".into(); 5];
        assert!(!p.rule_met(&UnlockRule::GoldenBatteries(6)));
        p.golden_batteries.push("sofa_circuit".into());
        assert!(p.rule_met(&UnlockRule::GoldenBatteries(6)));
    }

    #[test]
    fn content_follows_the_catalog_rules() {
        let c = catalog();
        let mut p = Profile::default();
        let car = |id: &str| Content::Car(id.into());
        assert!(p.is_unlocked(&c, &car("sparky")));
        assert!(!p.is_unlocked(&c, &car("torque")));
        assert!(!p.is_unlocked(&c, &Content::Cup("pro_cup".into())));
        assert!(p.is_unlocked(&c, &Content::Track("test_oval".into())));
        assert!(!p.is_unlocked(&c, &car("hovercraft")), "unknown content");
        let elite_plus = Content::Feature(Unlockable::Difficulty(Difficulty::ElitePlus));
        assert!(!p.is_unlocked(&c, &elite_plus), "Elite+ is cup-only");
        let reverse = TrackVariant {
            reverse: true,
            ..TrackVariant::forward("toy_town")
        };
        assert!(!p.variant_unlocked(&c, &reverse));
        assert!(p.paint_unlocked(&c, &"zip".into(), PaintId(3)));
        assert!(!p.paint_unlocked(&c, &"zip".into(), PaintId(4)), "Neon set");

        p.trophies.insert("rookie_cup".into(), Trophy::Bronze);
        assert!(p.is_unlocked(&c, &car("torque")));
        assert!(p.variant_unlocked(&c, &reverse));

        let mut all = Profile {
            unlock_all: true,
            ..default()
        };
        assert!(all.is_unlocked(&c, &car("bolt")));
        assert!(all.paint_unlocked(&c, &"bolt".into(), PaintId(7)));
        assert!(!all.is_unlocked(&c, &elite_plus));
        assert!(
            all.refresh_unlocks(&c).is_empty(),
            "--unlock-all earns nothing"
        );
    }

    #[test]
    fn refresh_announces_each_unlock_once_and_keeps_it() {
        let c = catalog();
        let mut p = Profile::default();
        assert!(
            p.refresh_unlocks(&c).is_empty(),
            "start content is not announced"
        );
        p.trophies.insert("rookie_cup".into(), Trophy::Silver);
        let fresh = p.refresh_unlocks(&c);
        for expected in [
            Content::Cup("pro_cup".into()),
            Content::Car("torque".into()),
            Content::Car("hornet".into()),
            Content::Track("maple_street".into()),
            Content::Track("wrench_works".into()),
            Content::Feature(Unlockable::Reverse),
        ] {
            assert!(fresh.contains(&expected), "{expected:?} in {fresh:?}");
        }
        assert_eq!(fresh.len(), 6);
        assert_eq!(p.new_unlocks, fresh);
        assert!(p.refresh_unlocks(&c).is_empty(), "announced once");

        gold(&mut p, &["toy_town", "toy_town_rev", "toy_town_mirror"]);
        let neon = Content::Feature(Unlockable::PaintSet(PaintSet::Neon));
        assert_eq!(p.refresh_unlocks(&c), std::slice::from_ref(&neon));
        p.records.clear(); // e.g. discarded after a track change
        assert!(p.is_unlocked(&c, &neon), "earned unlocks stay");
    }

    #[test]
    fn variant_keys_are_distinct() {
        let mut v = TrackVariant::forward("toy_town");
        assert_eq!(variant_key(&v), "toy_town");
        v.mirror = true;
        assert_eq!(variant_key(&v), "toy_town_mirror");
        v.reverse = true;
        assert_eq!(variant_key(&v), "toy_town_rev_mirror");
        v.mirror = false;
        assert_eq!(variant_key(&v), "toy_town_rev");
    }

    #[test]
    fn trophies_and_medals_order() {
        assert_eq!(Trophy::for_position(1), Some(Trophy::Gold));
        assert_eq!(Trophy::for_position(3), Some(Trophy::Bronze));
        assert_eq!(Trophy::for_position(4), None);
        assert!(Trophy::Gold > Trophy::Silver && Medal::Platinum > Medal::Gold);
        assert!(Medal::Gold.is_gold() && !Medal::Silver.is_gold());
    }
}
