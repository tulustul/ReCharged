//! `RaceRequest` → `RaceConfig` per mode (pure), and validation against the profile's unlocks.
//!
//! Grid (GDD §3.2): rivals in a seeded order (`RngStream::RaceSetup`) fill the front, the local
//! players start behind them in slot order (slot colours; "Player" alone, P1–P4 otherwise) — a
//! Versus session re-sorts its players by the session points at every attempt
//! (`series::versus_grid`). Rivals drive the cup's class in cups and the class of the best human
//! car elsewhere (Secret counts as Elite, GDD §8.3); cars shared by several racers get distinct
//! base paints.

use rc_core::prelude::*;

use crate::profile::{Content, Profile};
use crate::{PlayerCar, RaceRequest};

/// Time Trial always runs 3 laps (GDD §3).
pub const TIME_TRIAL_LAPS: u8 = 3;

/// What building a `RaceConfig` reads besides the request.
pub(crate) struct SetupContext<'a> {
    pub catalog: &'a Catalog,
    pub settings: &'a Settings,
    pub profile: &'a Profile,
    /// `--autopilot`: every local player is driven by the AI.
    pub autopilot: bool,
    /// Versus players whose car the AI took over.
    pub dropped: &'a [PlayerSlot],
    /// The layout's saved Time Trial ghost.
    pub ghost: Option<GhostData>,
}

/// The local players of a request.
pub(crate) fn human_cars(
    request: &RaceRequest,
    catalog: &Catalog,
) -> Result<Vec<PlayerCar>, String> {
    if !request.humans.is_empty() {
        return Ok(request.humans.clone());
    }
    let car = match &request.car {
        Some(car) => car.clone(),
        None => catalog
            .cars
            .first()
            .ok_or("the catalog has no cars")?
            .id
            .clone(),
    };
    let players = request.players.clamp(1, MAX_LOCAL_PLAYERS as u8);
    let player = |slot| PlayerCar {
        slot: PlayerSlot(slot),
        car: car.clone(),
        paint: request.paint,
    };
    Ok((0..players).map(player).collect())
}

/// Checks a request: the mode's player count, known content, and unlocks (cups: the cup; other
/// modes: the layout and an explicitly requested difficulty; always: cars and paints).
pub(crate) fn validate(
    request: &RaceRequest,
    catalog: &Catalog,
    profile: &Profile,
) -> Result<(), String> {
    let humans = human_cars(request, catalog)?;
    let (min, max) = match request.mode {
        GameMode::Versus => (2, MAX_LOCAL_PLAYERS),
        GameMode::QuickRace => (1, MAX_LOCAL_PLAYERS),
        GameMode::TimeTrial | GameMode::Championship { .. } => (1, 1),
    };
    if !(min..=max).contains(&humans.len()) {
        return Err(format!(
            "{:?} needs {min}–{max} players, got {}",
            request.mode,
            humans.len()
        ));
    }
    for (i, h) in humans.iter().enumerate() {
        if humans[..i].iter().any(|o| o.slot == h.slot) || h.slot.index() >= MAX_LOCAL_PLAYERS {
            return Err(format!("player slot {} is invalid or used twice", h.slot.0));
        }
        if catalog.car(&h.car).is_none() {
            return Err(format!("unknown car \"{}\"", h.car));
        }
        if !profile.is_unlocked(catalog, &Content::Car(h.car.clone())) {
            return Err(format!("car \"{}\" is locked", h.car));
        }
        if !profile.paint_unlocked(catalog, &h.car, h.paint) {
            return Err(format!(
                "paint {} of \"{}\" is locked or unknown",
                h.paint.0, h.car
            ));
        }
    }
    match &request.mode {
        GameMode::Championship { cup, race_index } => {
            let info = catalog
                .cup(cup)
                .ok_or_else(|| format!("unknown cup \"{cup}\""))?;
            if *race_index >= info.races.len() {
                return Err(format!("cup \"{cup}\" has no race {race_index}"));
            }
            if !profile.is_unlocked(catalog, &Content::Cup(cup.clone())) {
                return Err(format!("cup \"{cup}\" is locked"));
            }
        }
        _ => {
            let track = &request.variant.track;
            if catalog.track(track).is_none() {
                return Err(format!("unknown track \"{track}\""));
            }
            if !profile.variant_unlocked(catalog, &request.variant) {
                return Err(format!("layout {:?} is locked", request.variant));
            }
            if let Some(d) = request.difficulty
                && !profile.is_unlocked(catalog, &Content::Feature(Unlockable::Difficulty(d)))
            {
                return Err(format!("difficulty {d:?} is locked"));
            }
        }
    }
    Ok(())
}

/// Builds the session setup of a validated request. Errors name the unknown cup, track or car.
pub(crate) fn build_race_config(
    request: &RaceRequest,
    ctx: &SetupContext,
) -> Result<RaceConfig, String> {
    let catalog = ctx.catalog;
    let cup_race = match &request.mode {
        GameMode::Championship { cup, race_index } => {
            let info = catalog
                .cup(cup)
                .ok_or_else(|| format!("unknown cup \"{cup}\""))?;
            let race = info.races.get(*race_index);
            Some((
                info,
                race.ok_or_else(|| format!("cup \"{cup}\" has no race {race_index}"))?,
            ))
        }
        _ => None,
    };
    let variant = cup_race.map_or(&request.variant, |(_, race)| &race.variant);
    let track = catalog
        .track(&variant.track)
        .ok_or_else(|| format!("unknown track \"{}\"", variant.track))?;
    let humans = human_cars(request, catalog)?;
    let mut classes = Vec::new();
    for h in &humans {
        let car = catalog
            .car(&h.car)
            .ok_or_else(|| format!("unknown car \"{}\"", h.car))?;
        classes.push(car.class);
    }
    let time_trial = request.mode == GameMode::TimeTrial;
    let free = MAX_RACERS.saturating_sub(humans.len());
    let (ai, class, difficulty) = match cup_race {
        Some((cup, _)) => (free, cup_class(cup.difficulty), cup.difficulty),
        None => (
            if time_trial {
                0
            } else {
                request.ai.map_or(free, usize::from).min(free)
            },
            classes.iter().copied().max().unwrap_or_default(),
            request
                .difficulty
                .unwrap_or_else(|| default_difficulty(ctx)),
        ),
    };
    let seed = request.seed.unwrap_or_default();

    let mut racers: Vec<RacerSetup> = humans
        .iter()
        .map(|h| RacerSetup {
            name: match humans.len() {
                1 => "Player".into(),
                _ => format!("P{}", h.slot.0 + 1),
            },
            car: h.car.clone(),
            paint: h.paint,
            color: h.slot.color(),
            controller: Controller::Human {
                slot: h.slot,
                autopilot: ctx.autopilot || ctx.dropped.contains(&h.slot),
            },
        })
        .collect();
    for rival in seeded_rivals(catalog, seed).into_iter().take(ai) {
        let car = rival.cars.for_class(class).clone();
        let paint = free_paint(catalog, &racers, &car);
        racers.push(RacerSetup {
            name: rival.name.clone(),
            car,
            paint,
            color: rival.color,
            controller: Controller::Ai {
                rival: rival.id.clone(),
                difficulty,
            },
        });
    }
    // The rivals fill the front of the grid, the players start behind them.
    racers.rotate_left(humans.len());

    let weather = match cup_race {
        Some((_, race)) => race.weather,
        None => WeatherSetup {
            weather: weather_choice(request.weather, track),
            time: time_of_day(request.time, track),
        },
    };
    let laps = match cup_race {
        _ if time_trial => TIME_TRIAL_LAPS,
        Some((_, race)) => race.laps.unwrap_or(track.default_laps),
        None => request.laps.unwrap_or(track.default_laps).clamp(1, 9),
    };
    let single_player = request.mode != GameMode::Versus && humans.len() == 1;
    Ok(RaceConfig {
        mode: request.mode.clone(),
        variant: variant.clone(),
        laps,
        racers,
        items: cup_race.is_some() || (!time_trial && request.items.unwrap_or(true)),
        weather,
        ai_catch_up: !time_trial && ctx.settings.gameplay.ai_catch_up,
        seed,
        golden_battery: single_player
            && !variant.reverse
            && !ctx.profile.has_golden_battery(&variant.track),
        ghost: if time_trial { ctx.ghost.clone() } else { None },
    })
}

/// Rivals drive the cup's class: Elite (and Elite+) cups field Elite cars.
fn cup_class(difficulty: Difficulty) -> CarClass {
    match difficulty {
        Difficulty::Rookie => CarClass::Rookie,
        Difficulty::Pro => CarClass::Pro,
        Difficulty::Elite | Difficulty::ElitePlus => CarClass::Elite,
    }
}

/// The settings' default difficulty, or the best unlocked one below it.
fn default_difficulty(ctx: &SetupContext) -> Difficulty {
    let wanted = ctx.settings.gameplay.ai_difficulty;
    let unlocked = |d: &Difficulty| {
        ctx.profile
            .is_unlocked(ctx.catalog, &Content::Feature(Unlockable::Difficulty(*d)))
    };
    [Difficulty::Elite, Difficulty::Pro, Difficulty::Rookie]
        .into_iter()
        .filter(|d| *d <= wanted)
        .find(unlocked)
        .unwrap_or(Difficulty::Rookie)
}

/// All rivals in a seeded order (Fisher–Yates over `RngStream::RaceSetup`).
fn seeded_rivals(catalog: &Catalog, seed: u64) -> Vec<&RivalInfo> {
    let mut pool: Vec<&RivalInfo> = catalog.rivals.iter().collect();
    for i in (1..pool.len()).rev() {
        pool.swap(
            i,
            rng::index(seed, 0, RngStream::RaceSetup, i as u64, i + 1),
        );
    }
    pool
}

/// The first base paint of `car` that no earlier racer with the same car uses.
fn free_paint(catalog: &Catalog, racers: &[RacerSetup], car: &CarId) -> PaintId {
    let used = |p: &PaintId| racers.iter().any(|r| &r.car == car && r.paint == *p);
    catalog
        .car(car)
        .and_then(|info| {
            let base = info.paints.iter().filter(|p| p.set == PaintSet::Base);
            base.map(|p| p.id).find(|p| !used(p))
        })
        .unwrap_or_default()
}

fn weather_choice(wanted: Option<WeatherChoice>, track: &TrackInfo) -> WeatherChoice {
    let allowed = |w: &WeatherChoice| match *w {
        WeatherChoice::Fixed(kind) => track.allowed_weather.contains(&kind),
        WeatherChoice::Random => true,
        WeatherChoice::Dynamic { start } => !track.indoor && track.allowed_weather.contains(&start),
    };
    let first = track
        .allowed_weather
        .first()
        .map(|&k| WeatherChoice::Fixed(k));
    wanted.filter(allowed).or(first).unwrap_or_default()
}

fn time_of_day(wanted: Option<TimeOfDay>, track: &TrackInfo) -> TimeOfDay {
    let allowed = |t: &TimeOfDay| track.allowed_times.contains(t);
    wanted
        .filter(allowed)
        .or(Some(TimeOfDay::Noon).filter(allowed))
        .or_else(|| track.allowed_times.first().copied())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use bevy::prelude::default;

    use super::*;
    use crate::profile::Trophy;
    use crate::profile::tests::catalog;

    fn build(request: &RaceRequest, profile: &Profile) -> Result<RaceConfig, String> {
        let c = catalog();
        validate(request, &c, profile)?;
        let ctx = SetupContext {
            catalog: &c,
            settings: &Settings::default(),
            profile,
            autopilot: false,
            dropped: &[],
            ghost: None,
        };
        build_race_config(request, &ctx)
    }

    fn fresh() -> Profile {
        Profile::default()
    }

    #[test]
    fn quick_race_fills_the_grid_with_rivals_of_the_best_human_class() {
        let all = Profile {
            unlock_all: true,
            ..default()
        };
        let mut request = RaceRequest::quick("test_oval");
        request.car = Some("torque".into());
        let config = build(&request, &all).expect("valid");
        assert_eq!(config.racers.len(), MAX_RACERS);
        assert!(matches!(
            config.racers[MAX_RACERS - 1].controller,
            Controller::Human {
                slot: PlayerSlot(0),
                autopilot: false
            }
        ));
        let c = catalog();
        for racer in &config.racers[..MAX_RACERS - 1] {
            assert_eq!(
                c.car(&racer.car).map(|car| car.class),
                Some(CarClass::Pro),
                "{}",
                racer.name
            );
            let twins = config
                .racers
                .iter()
                .filter(|r| r.car == racer.car && r.paint == racer.paint);
            assert_eq!(twins.count(), 1, "same car, same paint: {}", racer.name);
        }
        assert_eq!(
            config.laps,
            c.track(&"test_oval".into()).expect("track").default_laps
        );
        assert!(config.items && config.ai_catch_up && config.golden_battery);
    }

    #[test]
    fn rival_picks_and_grid_order_follow_the_seed() {
        let request = |seed| RaceRequest {
            ai: Some(3),
            seed: Some(seed),
            ..RaceRequest::quick("toy_town")
        };
        let names = |seed| -> Vec<String> {
            let config = build(&request(seed), &fresh()).expect("valid");
            config.racers[..3].iter().map(|r| r.name.clone()).collect()
        };
        assert_eq!(names(1), names(1), "deterministic");
        assert_eq!(names(1).len(), 3);
        assert!(
            (2..40).any(|s| names(s) != names(1)),
            "the seed changes the field"
        );
    }

    #[test]
    fn rivals_fill_the_front_of_the_grid_and_the_players_start_behind_in_slot_order() {
        let request = RaceRequest {
            players: 3,
            ai: Some(3),
            seed: Some(4),
            ..RaceRequest::quick("toy_town")
        };
        let config = build(&request, &fresh()).expect("valid");
        let grid: Vec<Option<u8>> = config
            .racers
            .iter()
            .map(|r| match r.controller {
                Controller::Human { slot, .. } => Some(slot.0),
                Controller::Ai { .. } => None,
            })
            .collect();
        assert_eq!(grid, [None, None, None, Some(0), Some(1), Some(2)]);
        let names: Vec<&str> = config.racers[3..].iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["P1", "P2", "P3"]);
    }

    #[test]
    fn time_trial_is_solo_without_items_over_three_laps_with_its_ghost() {
        let ghost = GhostData {
            time_ticks: 5000,
            ..default()
        };
        let request = RaceRequest {
            mode: GameMode::TimeTrial,
            ai: Some(5),
            laps: Some(7),
            ..RaceRequest::quick("toy_town")
        };
        let c = catalog();
        let profile = fresh();
        let ctx = SetupContext {
            catalog: &c,
            settings: &Settings::default(),
            profile: &profile,
            autopilot: false,
            dropped: &[],
            ghost: Some(ghost.clone()),
        };
        let config = build_race_config(&request, &ctx).expect("valid");
        assert_eq!(
            (config.racers.len(), config.laps, config.items),
            (1, 3, false)
        );
        assert!(!config.ai_catch_up);
        assert_eq!(config.ghost, Some(ghost));
    }

    #[test]
    fn cups_take_layout_weather_laps_class_and_difficulty_from_the_cup() {
        let mut p = fresh();
        p.trophies.insert("elite_cup".into(), Trophy::Bronze);
        let request = RaceRequest {
            mode: GameMode::Championship {
                cup: "grand_volt_cup".into(),
                race_index: 5,
            },
            car: Some("zip".into()),
            ai: Some(1),
            laps: Some(9),
            ..RaceRequest::quick("test_oval")
        };
        assert!(
            build(&request, &fresh())
                .expect_err("locked")
                .contains("locked")
        );
        let config = build(&request, &p).expect("valid");
        assert_eq!(config.variant.track.as_str(), "maple_street");
        assert!(config.variant.reverse && config.variant.mirror);
        assert_eq!(
            config.weather.weather,
            WeatherChoice::Dynamic {
                start: WeatherKind::Fog
            }
        );
        assert_eq!((config.racers.len(), config.laps), (MAX_RACERS, 3));
        let c = catalog();
        for racer in &config.racers[..MAX_RACERS - 1] {
            assert_eq!(c.car(&racer.car).map(|x| x.class), Some(CarClass::Elite));
            assert!(matches!(
                racer.controller,
                Controller::Ai {
                    difficulty: Difficulty::ElitePlus,
                    ..
                }
            ));
        }
        assert!(!config.golden_battery, "reverse layouts have none");
    }

    #[test]
    fn versus_players_keep_their_cars_slots_and_colours() {
        let request = RaceRequest {
            mode: GameMode::Versus,
            humans: vec![
                PlayerCar {
                    slot: PlayerSlot(0),
                    car: "zip".into(),
                    paint: PaintId(1),
                },
                PlayerCar {
                    slot: PlayerSlot(2),
                    car: "dumpster".into(),
                    paint: PaintId(0),
                },
            ],
            ai: Some(10),
            weather: Some(WeatherChoice::Fixed(WeatherKind::Snow)), // not allowed indoors
            time: Some(TimeOfDay::Morning),                         // not allowed in Aisle Rush
            ..RaceRequest::quick("aisle_rush")
        };
        let c = catalog();
        let profile = fresh();
        let ctx = SetupContext {
            catalog: &c,
            settings: &Settings::default(),
            profile: &profile,
            autopilot: false,
            dropped: &[PlayerSlot(2)],
            ghost: None,
        };
        validate(&request, &c, &profile).expect("valid");
        let config = build_race_config(&request, &ctx).expect("valid");
        assert_eq!(config.racers.len(), MAX_RACERS);
        let (p1, p3) = (&config.racers[6], &config.racers[7]);
        assert_eq!((p3.name.as_str(), p3.car.as_str()), ("P3", "dumpster"));
        assert_eq!(p3.color, SLOT_COLORS[2]);
        assert!(matches!(
            p3.controller,
            Controller::Human {
                autopilot: true,
                ..
            }
        ));
        assert!(matches!(
            p1.controller,
            Controller::Human {
                slot: PlayerSlot(0),
                autopilot: false,
            }
        ));
        assert_eq!(
            config.weather.weather,
            WeatherChoice::Fixed(WeatherKind::Clear)
        );
        assert_eq!(config.weather.time, TimeOfDay::Sunset);
        assert!(!config.golden_battery, "not single-player");
    }

    #[test]
    fn locked_or_unknown_content_is_rejected() {
        let p = fresh();
        let err = |r: RaceRequest| build(&r, &p).expect_err("rejected");
        assert!(err(RaceRequest::quick("nowhere")).contains("nowhere"));
        assert!(err(RaceRequest::quick("maple_street")).contains("locked"));
        let mut r = RaceRequest::quick("toy_town");
        r.variant.mirror = true;
        assert!(err(r).contains("locked"));
        let mut r = RaceRequest::quick("toy_town");
        r.car = Some("hovercraft".into());
        assert!(err(r).contains("hovercraft"));
        let mut r = RaceRequest::quick("toy_town");
        r.car = Some("phantom".into());
        assert!(err(r).contains("locked"));
        let mut r = RaceRequest::quick("toy_town");
        r.paint = PaintId(7); // Chrome
        assert!(err(r).contains("paint"));
        let mut r = RaceRequest::quick("toy_town");
        r.difficulty = Some(Difficulty::ElitePlus);
        assert!(err(r).contains("difficulty"));
        let versus_alone = RaceRequest {
            mode: GameMode::Versus,
            ..RaceRequest::quick("toy_town")
        };
        assert!(err(versus_alone).contains("players"));
    }

    #[test]
    fn a_locked_default_difficulty_falls_back() {
        let mut settings = Settings::default();
        settings.gameplay.ai_difficulty = Difficulty::Elite; // locked until the Pro Cup podium
        let (c, p) = (catalog(), fresh());
        let ctx = SetupContext {
            catalog: &c,
            settings: &settings,
            profile: &p,
            autopilot: true,
            dropped: &[],
            ghost: None,
        };
        let config = build_race_config(&RaceRequest::quick("toy_town"), &ctx).expect("valid");
        assert!(matches!(
            config.racers[0].controller,
            Controller::Ai {
                difficulty: Difficulty::Pro,
                ..
            }
        ));
        assert!(matches!(
            config.racers[MAX_RACERS - 1].controller,
            Controller::Human {
                autopilot: true,
                ..
            }
        ));
    }
}
