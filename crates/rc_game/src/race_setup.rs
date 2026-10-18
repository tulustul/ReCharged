//! `RaceRequest` → `RaceConfig` (pure).

use rc_core::prelude::*;

use crate::RaceRequest;

/// Time Trial always runs 3 laps (GDD §3).
const TIME_TRIAL_LAPS: u8 = 3;

/// Builds the session setup: local players first (slot colours), then rivals in catalog order
/// driving the class of the human car. Weather/time outside the track's allowed presets fall back
/// to its first allowed preset (Noon when allowed). Errors name the unknown track or car.
pub fn build_race_config(
    request: &RaceRequest,
    catalog: &Catalog,
    settings: &Settings,
    autopilot: bool,
) -> Result<RaceConfig, String> {
    let track = catalog
        .track(&request.variant.track)
        .ok_or_else(|| format!("unknown track \"{}\"", request.variant.track))?;
    let car_id = match &request.car {
        Some(car) => car.clone(),
        None => catalog
            .cars
            .first()
            .ok_or("the catalog has no cars")?
            .id
            .clone(),
    };
    let car = catalog
        .car(&car_id)
        .ok_or_else(|| format!("unknown car \"{car_id}\""))?;
    let time_trial = request.mode == GameMode::TimeTrial;
    let humans = request.players.clamp(1, MAX_LOCAL_PLAYERS as u8);
    let free = MAX_RACERS - usize::from(humans);
    let ai = if time_trial {
        0
    } else {
        request.ai.map_or(free, usize::from).min(free)
    };
    let difficulty = request
        .difficulty
        .unwrap_or(settings.gameplay.ai_difficulty);

    let mut racers: Vec<RacerSetup> = (0..humans)
        .map(PlayerSlot)
        .map(|slot| RacerSetup {
            name: if humans == 1 {
                "Player".into()
            } else {
                format!("P{}", slot.0 + 1)
            },
            car: car_id.clone(),
            paint: request.paint,
            color: slot.color(),
            controller: Controller::Human { slot, autopilot },
        })
        .collect();
    // ponytail: rivals in catalog order; T07 picks them per mode (seeded) and per cup class.
    racers.extend(catalog.rivals.iter().take(ai).map(|rival| RacerSetup {
        name: rival.name.clone(),
        car: rival.cars.for_class(car.class).clone(),
        paint: PaintId(0),
        color: rival.color,
        controller: Controller::Ai {
            rival: rival.id.clone(),
            difficulty,
        },
    }));

    let allowed = |w: &WeatherChoice| match *w {
        WeatherChoice::Fixed(kind) => track.allowed_weather.contains(&kind),
        WeatherChoice::Random => true,
        WeatherChoice::Dynamic { start } => !track.indoor && track.allowed_weather.contains(&start),
    };
    let weather = request
        .weather
        .filter(allowed)
        .or_else(|| {
            track
                .allowed_weather
                .first()
                .map(|&k| WeatherChoice::Fixed(k))
        })
        .unwrap_or_default();
    let time = request
        .time
        .filter(|t| track.allowed_times.contains(t))
        .or_else(|| {
            track
                .allowed_times
                .contains(&TimeOfDay::Noon)
                .then_some(TimeOfDay::Noon)
        })
        .or_else(|| track.allowed_times.first().copied())
        .unwrap_or_default();

    Ok(RaceConfig {
        mode: request.mode.clone(),
        variant: request.variant.clone(),
        laps: if time_trial {
            TIME_TRIAL_LAPS
        } else {
            request.laps.unwrap_or(track.default_laps).clamp(1, 9)
        },
        racers,
        items: !time_trial && request.items.unwrap_or(true),
        weather: WeatherSetup { weather, time },
        ai_catch_up: !time_trial && settings.gameplay.ai_catch_up,
        seed: request.seed.unwrap_or_default(),
        golden_battery: false,
        ghost: None,
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn catalog() -> Catalog {
        Catalog::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")).expect("catalog")
    }

    #[test]
    fn quick_race_fills_the_grid_with_rivals_of_the_player_class() {
        let c = catalog();
        let mut request = RaceRequest::quick("test_oval");
        request.car = Some("torque".into());
        let config = build_race_config(&request, &c, &Settings::default(), false).expect("valid");
        assert_eq!(config.racers.len(), MAX_RACERS);
        assert!(matches!(
            config.racers[0].controller,
            Controller::Human {
                slot: PlayerSlot(0),
                autopilot: false
            }
        ));
        for racer in &config.racers[1..] {
            assert_eq!(
                c.car(&racer.car).map(|car| car.class),
                Some(CarClass::Pro),
                "{}",
                racer.name
            );
        }
        assert_eq!(
            config.laps,
            c.track(&"test_oval".into()).expect("track").default_laps
        );
        assert!(config.items && config.ai_catch_up);
    }

    #[test]
    fn time_trial_is_solo_without_items_over_three_laps() {
        let request = RaceRequest {
            mode: GameMode::TimeTrial,
            ai: Some(5),
            laps: Some(7),
            ..RaceRequest::quick("toy_town")
        };
        let config =
            build_race_config(&request, &catalog(), &Settings::default(), false).expect("valid");
        assert_eq!(
            (config.racers.len(), config.laps, config.items),
            (1, 3, false)
        );
    }

    #[test]
    fn versus_players_get_slot_colours_and_presets_fall_back() {
        let request = RaceRequest {
            players: 3,
            ai: Some(10),
            weather: Some(WeatherChoice::Fixed(WeatherKind::Snow)), // not allowed indoors
            time: Some(TimeOfDay::Morning),                         // not allowed in Aisle Rush
            ..RaceRequest::quick("aisle_rush")
        };
        let config =
            build_race_config(&request, &catalog(), &Settings::default(), true).expect("valid");
        assert_eq!(config.racers.len(), MAX_RACERS);
        assert_eq!(config.racers[2].name, "P3");
        assert_eq!(config.racers[2].color, SLOT_COLORS[2]);
        assert_eq!(
            config.weather.weather,
            WeatherChoice::Fixed(WeatherKind::Clear)
        );
        assert_eq!(config.weather.time, TimeOfDay::Sunset);
    }

    #[test]
    fn unknown_track_or_car_is_an_error() {
        let c = catalog();
        assert!(
            build_race_config(
                &RaceRequest::quick("nowhere"),
                &c,
                &Settings::default(),
                false
            )
            .is_err()
        );
        let mut request = RaceRequest::quick("test_oval");
        request.car = Some("hovercraft".into());
        let err =
            build_race_config(&request, &c, &Settings::default(), false).expect_err("bad car");
        assert!(err.contains("hovercraft"));
    }
}
