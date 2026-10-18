//! The vehicle data files: every file loads, matches the catalog's stars and drivetrain, the
//! binding geometry of `docs/design/car-dimensions.md` and each model's `<car>.wheels.ron`;
//! the loader reports bad files and unknown cars through the loading gate.

use std::path::Path;

use bevy::prelude::*;
use rc_core::prelude::*;
use rc_testkit::*;
use rc_vehicle::{VEHICLES_DIR, VehicleLibrary, VehicleParams};
use serde::Deserialize;

fn library() -> VehicleLibrary {
    VehicleLibrary::load(&assets_dir().join(VEHICLES_DIR)).expect("vehicle files load")
}

#[test]
fn every_catalog_car_has_matching_vehicle_params() {
    let lib = library();
    let catalog = Catalog::load(&assets_dir()).expect("catalog");
    let ids: Vec<&str> = lib.cars.keys().map(CarId::as_str).collect();
    let mut catalog_ids: Vec<&str> = catalog.cars.iter().map(|c| c.id.as_str()).collect();
    catalog_ids.sort();
    assert_eq!(ids, catalog_ids, "one vehicle file per catalog car");
    for car in &catalog.cars {
        let p = &lib.cars[&car.id];
        assert_eq!(p.drive, car.drive, "{}: drivetrain", car.id);
        assert_eq!(
            p.top_speed,
            top_speed_for_stars(car.stars.speed),
            "{}: Speed stars",
            car.id
        );
        assert_eq!(
            p.accel_time,
            zero_to_top_secs_for_stars(car.stars.accel),
            "{}: Accel stars",
            car.id
        );
    }
    // Weight trade-offs: heavier → more mass, less air control, more landing loss.
    let mut by_weight: Vec<(u8, &VehicleParams)> = catalog
        .cars
        .iter()
        .map(|c| (c.stars.weight, &lib.cars[&c.id]))
        .collect();
    by_weight.sort_by_key(|(w, _)| *w);
    for pair in by_weight.windows(2) {
        let ((wa, a), (wb, b)) = (pair[0], pair[1]);
        if wa < wb {
            assert!(a.mass < b.mass, "{} vs {}: mass by weight", a.id, b.id);
            assert!(
                a.air_control > b.air_control,
                "{} vs {}: air control",
                a.id,
                b.id
            );
            assert!(
                a.landing_loss < b.landing_loss,
                "{} vs {}: landing loss",
                a.id,
                b.id
            );
        }
    }
    // Handling: more stars → more grip.
    for a in &catalog.cars {
        for b in &catalog.cars {
            if a.stars.handling > b.stars.handling {
                assert!(
                    lib.cars[&a.id].tyres.grip > lib.cars[&b.id].tyres.grip,
                    "{} vs {}",
                    a.id,
                    b.id
                );
            }
        }
    }
}

/// One row of the car-dimensions table.
fn dimension_rows() -> Vec<(String, Vec<f32>)> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/design/car-dimensions.md");
    let text = std::fs::read_to_string(path).expect("car-dimensions.md");
    text.lines()
        .filter(|l| l.starts_with("| ") && !l.starts_with("| Car") && !l.starts_with("|---"))
        .map(|l| {
            let cells: Vec<&str> = l
                .split('|')
                .map(str::trim)
                .filter(|c| !c.is_empty())
                .collect();
            let numbers = cells[2..]
                .iter()
                .flat_map(|c| c.split('/'))
                .map(|n| n.trim().parse::<f32>().expect("number"))
                .collect();
            (cells[0].to_owned(), numbers)
        })
        .collect()
}

#[test]
fn geometry_matches_car_dimensions() {
    let lib = library();
    let rows = dimension_rows();
    assert_eq!(rows.len(), lib.cars.len(), "a table row per car");
    for (id, n) in rows {
        let p = &lib.cars[&CarId::new(id.as_str())];
        // Length, Width, Height, Wheelbase, Track F/R, Radius F/R, Width F/R, Clearance.
        let ours = [
            p.body.length,
            p.body.width,
            p.body.height,
            p.wheelbase,
            p.front.track,
            p.rear.track,
            p.front.radius,
            p.rear.radius,
            p.front.width,
            p.rear.width,
            p.body.clearance,
        ];
        for (k, (a, b)) in ours.iter().zip(&n).enumerate() {
            assert!(
                (a - b).abs() < 1e-4,
                "{id}: column {k} is {a}, the table says {b}"
            );
        }
    }
}

#[derive(Deserialize)]
struct WheelsFile {
    car: String,
    wheels: Vec<WheelEntry>,
}

#[derive(Deserialize)]
struct WheelEntry {
    name: String,
    center: (f32, f32, f32),
    radius: f32,
    width: f32,
}

#[test]
fn wheel_layout_matches_the_models_wheel_files() {
    let lib = library();
    let mut checked = 0;
    for (id, p) in &lib.cars {
        let path = assets_dir().join(format!("models/cars/{id}.wheels.ron"));
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue; // the model is not finished yet
        };
        let file: WheelsFile = ron::from_str(&text).expect("wheels.ron parses");
        assert_eq!(file.car, id.as_str());
        let names = ["wheel_fl", "wheel_fr", "wheel_rl", "wheel_rr"];
        for ((w, (centre, radius, width)), name) in
            file.wheels.iter().zip(p.wheel_layout()).zip(names)
        {
            assert_eq!(w.name, name, "{id}: wheel order");
            let c = Vec3::new(w.center.0, w.center.1, w.center.2);
            assert!(
                c.distance(centre) < 0.006,
                "{id} {name}: model {c}, physics {centre}"
            );
            assert!((w.radius - radius).abs() < 0.006, "{id} {name}: radius");
            assert!((w.width - width).abs() < 0.006, "{id} {name}: width");
        }
        checked += 1;
    }
    println!("checked {checked} wheel files");
}

#[test]
fn broken_files_name_the_file_and_field() {
    let dir = std::env::temp_dir().join(format!("rc_vehicle_bad_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let good =
        std::fs::read_to_string(assets_dir().join(VEHICLES_DIR).join("zip.ron")).expect("zip");
    std::fs::write(dir.join("zip.ron"), good.replace("mass: 2.0", "mass: -1.0")).expect("write");
    let err = VehicleLibrary::load(&dir).expect_err("negative mass is rejected");
    assert!(err.contains("zip.ron") && err.contains("`mass`"), "{err}");
    std::fs::write(dir.join("zip.ron"), good.replace("top_speed:", "top_sped:")).expect("write");
    let err = VehicleLibrary::load(&dir).expect_err("typo is rejected");
    assert!(err.contains("zip.ron") && err.contains("top_sped"), "{err}");
    std::fs::write(dir.join("zip.ron"), good.replace("\"zip\"", "\"zap\"")).expect("write");
    let err = VehicleLibrary::load(&dir).expect_err("id must match the file");
    assert!(err.contains("`id`"), "{err}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn loading_gate_reports_unknown_cars() {
    let mut app = headless_app(rc_vehicle::VehiclePlugin);
    let config = |car: &str| RaceConfig {
        racers: vec![RacerSetup {
            car: CarId::new(car),
            ..default()
        }],
        ..default()
    };
    let failure = load_race(&mut app, config("hovercraft")).expect_err("unknown car fails");
    assert_eq!(failure.key, rc_vehicle::LOADING_KEY);
    assert!(failure.error.contains("hovercraft"), "{}", failure.error);
    load_race(&mut app, config("zip")).expect("a known car loads");
    assert!(app.world().contains_resource::<VehicleLibrary>());
    quit_to_menu(&mut app);
    assert!(
        !app.world().contains_resource::<VehicleLibrary>(),
        "session resource removed"
    );
}

#[test]
fn hot_reload_picks_up_changed_files_and_keeps_old_params_on_errors() {
    let dir = std::env::temp_dir().join(format!("rc_vehicle_reload_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let src = assets_dir().join(VEHICLES_DIR).join("hornet.ron");
    let good = std::fs::read_to_string(src).expect("hornet");
    let file = dir.join("hornet.ron");
    std::fs::write(&file, &good).expect("write");
    let mut lib = VehicleLibrary::load(&dir).expect("loads");
    assert_eq!(
        lib.reload_changed(),
        (vec![], vec![]),
        "nothing changed yet"
    );
    let touch = |text: &str| {
        std::fs::write(&file, text).expect("write");
        // Make the modification time differ even on coarse file systems.
        let later = std::time::SystemTime::now() + std::time::Duration::from_secs(5);
        std::fs::File::options()
            .write(true)
            .open(&file)
            .and_then(|f| f.set_modified(later))
            .expect("set mtime");
    };
    touch(&good.replace("mass: 2.6", "mass: 2.9"));
    let (changed, errors) = lib.reload_changed();
    assert_eq!((changed, errors.len()), (vec![CarId::new("hornet")], 0));
    assert_eq!(lib.get(&CarId::new("hornet")).expect("hornet").mass, 2.9);
    touch(&good.replace("mass: 2.6", "mass: oops"));
    let (changed, errors) = lib.reload_changed();
    assert!(changed.is_empty() && errors.len() == 1, "{errors:?}");
    assert_eq!(lib.get(&CarId::new("hornet")).expect("kept").mass, 2.9);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn changed_params_rebuild_live_bodies() {
    let mut app = headless_app(rc_vehicle::VehiclePlugin);
    start_race(&mut app, RaceConfig::default()).expect("race loads");
    let car = app
        .world_mut()
        .spawn((
            Racer {
                car: CarId::new("zip"),
                ..default()
            },
            Transform::from_xyz(0.0, 1.0, 0.0),
        ))
        .id();
    step(&mut app, 2);
    let before = app
        .world()
        .get::<rc_vehicle::CarSpec>(car)
        .expect("body")
        .mass();
    app.world_mut()
        .resource_mut::<VehicleLibrary>()
        .cars
        .get_mut(&CarId::new("zip"))
        .expect("zip")
        .mass = 4.0;
    step(&mut app, 2);
    let after = app
        .world()
        .get::<rc_vehicle::CarSpec>(car)
        .expect("body")
        .mass();
    assert_eq!((before, after), (2.0, 4.0));
}
