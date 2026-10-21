//! Loading and validation of [`Catalog`]: `Catalog::load` (std::fs + ron), cross-references,
//! unlock order of cup content, allowed weather/time presets.

use std::fmt;
use std::path::{Path, PathBuf};

use super::*;
use crate::weather::WeatherChoice;

/// Why the catalog could not be loaded.
#[derive(Debug)]
pub enum CatalogError {
    /// The file could not be read.
    Io {
        /// File path.
        path: PathBuf,
        /// Underlying error.
        error: std::io::Error,
    },
    /// The file is not valid RON for [`Catalog`].
    Parse {
        /// File path.
        path: PathBuf,
        /// Parser error with position.
        error: Box<ron::error::SpannedError>,
    },
    /// The file parsed but references or rules are broken (one message per problem).
    Invalid {
        /// File path.
        path: PathBuf,
        /// Problems, each naming the offending entry and field.
        problems: Vec<String>,
    },
}

impl fmt::Display for CatalogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, error } => write!(f, "{}: {error}", path.display()),
            Self::Parse { path, error } => write!(f, "{}:{error}", path.display()),
            Self::Invalid { path, problems } => {
                write!(f, "{}: {} problem(s):", path.display(), problems.len())?;
                for p in problems {
                    write!(f, "\n  - {p}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for CatalogError {}

impl Catalog {
    /// Reads and validates `<assets_dir>/data/catalog.ron`.
    pub fn load(assets_dir: &Path) -> Result<Self, CatalogError> {
        let path = assets_dir.join(CATALOG_FILE);
        let text = std::fs::read_to_string(&path).map_err(|error| CatalogError::Io {
            path: path.clone(),
            error,
        })?;
        Self::parse(&text, &path)
    }

    /// Parses and validates catalog RON; `path` is only used in error messages.
    pub fn parse(text: &str, path: &Path) -> Result<Self, CatalogError> {
        let catalog: Catalog = ron::from_str(text).map_err(|error| CatalogError::Parse {
            path: path.to_owned(),
            error: Box::new(error),
        })?;
        catalog
            .validate()
            .map_err(|problems| CatalogError::Invalid {
                path: path.to_owned(),
                problems,
            })?;
        Ok(catalog)
    }

    /// The car with `id`.
    pub fn car(&self, id: &CarId) -> Option<&CarInfo> {
        self.cars.iter().find(|c| &c.id == id)
    }

    /// The track with `id`.
    pub fn track(&self, id: &TrackId) -> Option<&TrackInfo> {
        self.tracks.iter().find(|t| &t.id == id)
    }

    /// The cup with `id`.
    pub fn cup(&self, id: &CupId) -> Option<&CupInfo> {
        self.cups.iter().find(|c| &c.id == id)
    }

    /// The rival with `id`.
    pub fn rival(&self, id: &RivalId) -> Option<&RivalInfo> {
        self.rivals.iter().find(|r| &r.id == id)
    }

    /// The rule of a global unlockable (`None` = not unlockable, e.g. Elite+ difficulty).
    pub fn unlock_rule(&self, unlockable: Unlockable) -> Option<&UnlockRule> {
        self.unlocks
            .iter()
            .find(|(u, _)| *u == unlockable)
            .map(|(_, rule)| rule)
    }

    /// Whether `need` is necessarily satisfied whenever `have` is (unlock order).
    ///
    /// A cup podium/gold implies the cup was played, hence its own unlock rule held too.
    pub fn rule_implies(&self, have: &UnlockRule, need: &UnlockRule) -> bool {
        self.implies(have, need, 0)
    }

    fn implies(&self, have: &UnlockRule, need: &UnlockRule, depth: usize) -> bool {
        use UnlockRule::*;
        if depth > self.cups.len() {
            return false; // a cycle in cup unlocks (reported by `validate`)
        }
        let direct = match (have, need) {
            (_, Start) => true,
            (GoldMedals(m), GoldMedals(n)) | (GoldenBatteries(m), GoldenBatteries(n)) => m >= n,
            (CupGold(x), CupPodium(c)) => x == c,
            _ => have == need,
        };
        direct
            || match have {
                CupPodium(x) | CupGold(x) => self
                    .cup(x)
                    .is_some_and(|cup| self.implies(&cup.unlock, need, depth + 1)),
                _ => false,
            }
    }

    /// Checks every cross-reference and content rule; returns one message per problem.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut problems = Vec::new();
        let p = &mut problems;
        duplicates("cars", self.cars.iter().map(|c| c.id.as_str()), p);
        duplicates("tracks", self.tracks.iter().map(|t| t.id.as_str()), p);
        duplicates("cups", self.cups.iter().map(|c| c.id.as_str()), p);
        duplicates("rivals", self.rivals.iter().map(|r| r.id.as_str()), p);
        for car in &self.cars {
            self.validate_car(car, p);
        }
        for track in &self.tracks {
            self.validate_track(track, p);
        }
        for cup in &self.cups {
            self.validate_cup(cup, p);
        }
        for rival in &self.rivals {
            self.validate_rival(rival, p);
        }
        self.validate_unlocks(p);
        if problems.is_empty() {
            Ok(())
        } else {
            Err(problems)
        }
    }

    fn validate_rule(&self, at: &str, rule: &UnlockRule, p: &mut Vec<String>) {
        let tracks = self.tracks.iter().filter(|t| !t.hidden).count() as u32;
        match rule {
            UnlockRule::CupPodium(cup) | UnlockRule::CupGold(cup) if self.cup(cup).is_none() => {
                p.push(format!("{at}.unlock: unknown cup \"{cup}\""));
            }
            UnlockRule::GoldenBatteries(n) if *n > tracks => {
                p.push(format!(
                    "{at}.unlock: {n} golden batteries, only {tracks} exist"
                ));
            }
            UnlockRule::GoldMedals(n) if *n > tracks * 4 => {
                p.push(format!(
                    "{at}.unlock: {n} gold medals, only {} layouts exist",
                    tracks * 4
                ));
            }
            _ => {}
        }
    }

    fn validate_car(&self, car: &CarInfo, p: &mut Vec<String>) {
        let at = format!("cars[{}]", car.id);
        let s = car.stars;
        if [s.speed, s.accel, s.handling, s.weight]
            .iter()
            .any(|v| !(1..=5).contains(v))
        {
            p.push(format!("{at}.stars: every rating must be 1–5"));
        }
        if car.model.is_empty() {
            p.push(format!("{at}.model: empty"));
        }
        duplicates(
            &format!("{at}.paints"),
            car.paints.iter().map(|x| x.id.0.to_string()),
            p,
        );
        if !car.paints.iter().any(|x| x.set == PaintSet::Base) {
            p.push(format!("{at}.paints: no Base paint"));
        }
        for set in PaintSet::UNLOCKABLE {
            let n = car.paints.iter().filter(|x| x.set == set).count();
            if n != 1 {
                p.push(format!(
                    "{at}.paints: {n} {set:?} paints, expected exactly 1"
                ));
            }
        }
        for paint in &car.paints {
            let unit = |v: f32| (0.0..=1.0).contains(&v);
            let colors_ok = paint.base.iter().chain(&paint.accent).all(|&c| unit(c));
            if !colors_ok || !unit(paint.metallic) || !unit(paint.roughness) {
                p.push(format!(
                    "{at}.paints[{}]: values must be within 0–1",
                    paint.id.0
                ));
            }
        }
        self.validate_rule(&at, &car.unlock, p);
    }

    fn validate_track(&self, track: &TrackInfo, p: &mut Vec<String>) {
        let at = format!("tracks[{}]", track.id);
        if !(1..=3).contains(&track.difficulty) {
            p.push(format!("{at}.difficulty: must be 1–3"));
        }
        if !(1..=9).contains(&track.default_laps) {
            p.push(format!("{at}.default_laps: must be 1–9"));
        }
        if track.allowed_times.is_empty() {
            p.push(format!("{at}.allowed_times: empty"));
        }
        if track.allowed_weather.is_empty() {
            p.push(format!("{at}.allowed_weather: empty"));
        }
        self.validate_rule(&at, &track.unlock, p);
    }

    fn validate_cup(&self, cup: &CupInfo, p: &mut Vec<String>) {
        let at = format!("cups[{}]", cup.id);
        if cup.races.is_empty() {
            p.push(format!("{at}.races: empty"));
        }
        self.validate_rule(&at, &cup.unlock, p);
        if self.implies(&cup.unlock, &UnlockRule::CupPodium(cup.id.clone()), 0)
            && cup.unlock != UnlockRule::Start
        {
            p.push(format!("{at}.unlock: requires itself"));
        }
        for (i, race) in cup.races.iter().enumerate() {
            let at = format!("{at}.races[{i}]");
            let Some(track) = self.track(&race.variant.track) else {
                p.push(format!(
                    "{at}.variant.track: unknown track \"{}\"",
                    race.variant.track
                ));
                continue;
            };
            if track.hidden {
                p.push(format!(
                    "{at}.variant.track: \"{}\" is a hidden dev track",
                    track.id
                ));
            }
            if race.laps.is_some_and(|l| !(1..=9).contains(&l)) {
                p.push(format!("{at}.laps: must be 1–9"));
            }
            if !track.allowed_times.contains(&race.weather.time) {
                p.push(format!(
                    "{at}.weather.time: {:?} not allowed on {}",
                    race.weather.time, track.id
                ));
            }
            match race.weather.weather {
                WeatherChoice::Fixed(kind) if !track.allowed_weather.contains(&kind) => {
                    p.push(format!(
                        "{at}.weather: {kind:?} not allowed on {}",
                        track.id
                    ));
                }
                WeatherChoice::Dynamic { start } => {
                    if track.indoor {
                        p.push(format!(
                            "{at}.weather: Dynamic weather on indoor track {}",
                            track.id
                        ));
                    }
                    if !track.allowed_weather.contains(&start) {
                        p.push(format!(
                            "{at}.weather: Dynamic start {start:?} not allowed on {}",
                            track.id
                        ));
                    }
                }
                _ => {}
            }
            let mut needs = vec![("track", track.unlock.clone())];
            if race.variant.reverse {
                needs.push((
                    "reverse",
                    self.unlock_rule(Unlockable::Reverse)
                        .cloned()
                        .unwrap_or_default(),
                ));
            }
            if race.variant.mirror {
                needs.push((
                    "mirror",
                    self.unlock_rule(Unlockable::Mirror)
                        .cloned()
                        .unwrap_or_default(),
                ));
            }
            for (what, need) in needs {
                if !self.rule_implies(&cup.unlock, &need) {
                    p.push(format!(
                        "{at}: {what} ({need:?}) is unlocked later than the cup ({:?})",
                        cup.unlock
                    ));
                }
            }
        }
    }

    fn validate_rival(&self, rival: &RivalInfo, p: &mut Vec<String>) {
        let at = format!("rivals[{}]", rival.id);
        if !(0.0..=1.0).contains(&rival.aggression) || !(0.0..=1.0).contains(&rival.consistency) {
            p.push(format!("{at}: aggression and consistency must be 0–1"));
        }
        for (slot, class) in [
            ("rookie", CarClass::Rookie),
            ("pro", CarClass::Pro),
            ("elite", CarClass::Elite),
        ] {
            let id = rival.cars.for_class(class);
            match self.car(id) {
                None => p.push(format!("{at}.cars.{slot}: unknown car \"{id}\"")),
                Some(car) if car.class != class => {
                    p.push(format!(
                        "{at}.cars.{slot}: \"{id}\" is a {:?} car",
                        car.class
                    ));
                }
                Some(_) => {}
            }
        }
    }

    fn validate_unlocks(&self, p: &mut Vec<String>) {
        duplicates(
            "unlocks",
            self.unlocks.iter().map(|(u, _)| format!("{u:?}")),
            p,
        );
        for required in [Unlockable::Reverse, Unlockable::Mirror] {
            if self.unlock_rule(required).is_none() {
                p.push(format!("unlocks: missing {required:?}"));
            }
        }
        for (unlockable, rule) in &self.unlocks {
            self.validate_rule(&format!("unlocks[{unlockable:?}]"), rule, p);
        }
    }
}

fn duplicates<S: AsRef<str>>(at: &str, ids: impl Iterator<Item = S>, p: &mut Vec<String>) {
    let mut seen: Vec<String> = Vec::new();
    for id in ids {
        let id = id.as_ref();
        if seen.iter().any(|s| s == id) {
            p.push(format!("{at}: duplicate id \"{id}\""));
        } else {
            seen.push(id.to_owned());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assets() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
    }

    fn catalog() -> Catalog {
        Catalog::load(&assets()).unwrap_or_else(|e| panic!("{e}"))
    }

    fn problems(c: &Catalog) -> Vec<String> {
        c.validate().err().unwrap_or_default()
    }

    #[test]
    fn catalog_file_loads_and_validates() {
        let c = catalog();
        assert_eq!(c.cars.len(), 8);
        assert_eq!(c.tracks.iter().filter(|t| !t.hidden).count(), 7);
        assert!(c.track(&"test_oval".into()).is_some_and(|t| t.hidden));
        assert_eq!(
            c.cups.iter().map(|cup| cup.races.len()).collect::<Vec<_>>(),
            [4, 4, 4, 6]
        );
        assert_eq!(c.rivals.len(), 7);
        assert!(c.cars.iter().all(|car| car.paints.len() == 8));
    }

    #[test]
    fn unknown_references_are_reported_with_their_field() {
        let mut c = catalog();
        c.cups[0].races[0].variant.track = "nowhere".into();
        c.rivals[0].cars.pro = "ghost_car".into();
        c.cars[0].unlock = UnlockRule::CupPodium("no_cup".into());
        let p = problems(&c);
        assert!(
            p.iter()
                .any(|m| m.contains("races[0].variant.track") && m.contains("nowhere")),
            "{p:?}"
        );
        assert!(
            p.iter()
                .any(|m| m.contains("cars.pro") && m.contains("ghost_car")),
            "{p:?}"
        );
        assert!(
            p.iter().any(|m| m.contains("unknown cup \"no_cup\"")),
            "{p:?}"
        );
    }

    #[test]
    fn cup_races_must_not_use_content_unlocked_later() {
        let mut c = catalog();
        let rookie = c
            .cups
            .iter()
            .position(|cup| cup.unlock == UnlockRule::Start)
            .expect("start cup");
        c.cups[rookie].races[0].variant.mirror = true;
        let p = problems(&c);
        assert!(
            p.iter()
                .any(|m| m.contains("mirror") && m.contains("unlocked later")),
            "{p:?}"
        );
    }

    #[test]
    fn later_cups_may_use_earlier_unlocks() {
        let c = catalog();
        let pro_podium = UnlockRule::CupPodium("pro_cup".into());
        let rookie_podium = UnlockRule::CupPodium("rookie_cup".into());
        let elite_podium = UnlockRule::CupPodium("elite_cup".into());
        assert!(c.rule_implies(&elite_podium, &pro_podium));
        assert!(c.rule_implies(&elite_podium, &rookie_podium));
        assert!(c.rule_implies(&UnlockRule::CupGold("pro_cup".into()), &pro_podium));
        assert!(!c.rule_implies(&rookie_podium, &pro_podium));
        assert!(!c.rule_implies(&pro_podium, &UnlockRule::GoldMedals(3)));
        assert!(c.rule_implies(&UnlockRule::GoldMedals(6), &UnlockRule::GoldMedals(3)));
    }

    #[test]
    fn weather_presets_are_checked_per_track() {
        let mut c = catalog();
        let indoor = c
            .cups
            .iter()
            .flat_map(|cup| cup.races.iter())
            .position(|r| c.track(&r.variant.track).is_some_and(|t| t.indoor))
            .expect("an indoor cup race");
        let race = c
            .cups
            .iter_mut()
            .flat_map(|cup| cup.races.iter_mut())
            .nth(indoor)
            .expect("race");
        race.weather.weather = WeatherChoice::Dynamic {
            start: WeatherKind::Snow,
        };
        let p = problems(&c);
        assert!(
            p.iter().any(|m| m.contains("Dynamic weather on indoor")),
            "{p:?}"
        );
        assert!(p.iter().any(|m| m.contains("Dynamic start Snow")), "{p:?}");
    }

    #[test]
    fn rival_cars_must_match_their_class_and_ids_must_be_unique() {
        let mut c = catalog();
        c.rivals[0].cars.rookie = c.rivals[0].cars.elite.clone();
        c.cars.push(c.cars[0].clone());
        let p = problems(&c);
        assert!(
            p.iter()
                .any(|m| m.contains("cars.rookie") && m.contains("Elite car")),
            "{p:?}"
        );
        assert!(
            p.iter().any(|m| m.starts_with("cars: duplicate id")),
            "{p:?}"
        );
    }

    #[test]
    fn parse_errors_name_the_file() {
        let err = Catalog::parse("(cars: [", Path::new("x/catalog.ron")).expect_err("broken RON");
        assert!(err.to_string().starts_with("x/catalog.ron:"), "{err}");
    }
}
