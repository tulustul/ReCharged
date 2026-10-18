//! Battery charge & boost in the running sim (GDD §5.3, §5.5): drift, air time, clean landings
//! and drafting charge; the boost spends and needs 15 %; Turbo freezes the drain; items' drains
//! and grants.

mod common;

use bevy::prelude::*;
use common::*;
use rc_core::prelude::*;
use rc_testkit::*;

fn set_charge(app: &mut App, car: Entity, value: f32) {
    app.world_mut().get_mut::<Charge>(car).expect("racer").0 = value;
}

#[test]
fn fast_drifts_charge_slow_ones_do_not() {
    let mut app = app();
    let cars = lanes(&mut app, &["hornet"]);
    let top = params(&app, "hornet").top_speed;
    drift(&mut app, &cars, &[0.7 * top]);
    // ~1.6 s of qualifying drift at up to 22 %/s.
    let fast = charge(&app, cars[0]);
    assert!(fast > 0.12 && fast < 0.4, "fast drift charged {fast}");

    // The same drift held at 30 % of top speed earns nothing.
    let mut app = common::app();
    let slow = lanes(&mut app, &["hornet"])[0];
    for tick in 0..(5 * 60) {
        hold_speed(&mut app, slow, 0.3 * top);
        let mut c = control(&mut app, slow);
        c.steer = if tick > 120 { 1.0 } else { 0.0 };
        c.drift = tick > 120;
        step(&mut app, 1);
    }
    assert!(telemetry(&app, slow).drifting, "it does drift");
    assert_eq!(
        charge(&app, slow),
        0.0,
        "below 40 % of top speed nothing charges"
    );
}

#[test]
fn air_time_and_a_clean_landing_charge() {
    let mut app = app();
    // 10 u: ≈ 0.89 s of free flight, flat → clean landing.
    let car = spawn_car(&mut app, "zip", Transform::from_xyz(0.0, 10.0, 0.0));
    let landed = step_collect::<CarLanded>(&mut app, 1.5);
    let l = landed.last().copied().expect("landed");
    assert!(l.clean && (0.8..1.0).contains(&l.air_time), "{l:?}");
    assert!(telemetry(&app, car).clean_landing);
    // 15 %/s after 0.3 s of flight (≈ 0.09) + 10 % bonus.
    let c = charge(&app, car);
    assert!((0.16..0.24).contains(&c), "charge after the jump: {c}");

    // A short hop (0.5 s) gives no bonus and its landing is not clean.
    let hop = spawn_car(&mut app, "zip", Transform::from_xyz(20.0, 3.0, 0.0));
    let landed = step_collect::<CarLanded>(&mut app, 1.0);
    let l = landed
        .iter()
        .find(|l| l.car == hop)
        .copied()
        .expect("hop landed");
    assert!(!l.clean && l.strength > 0.0, "{l:?}");
    assert!(
        charge(&app, hop) < 0.04,
        "short hop charge {}",
        charge(&app, hop)
    );
}

#[test]
fn boost_spends_charge_and_needs_15_percent() {
    let mut app = app();
    let cars = lanes(&mut app, &["torque", "torque", "torque"]);
    set_charge(&mut app, cars[0], 0.5);
    set_charge(&mut app, cars[1], 0.10);
    let top = params(&app, "torque").top_speed;
    for &car in &cars {
        control(&mut app, car).throttle = 1.0;
    }
    step_seconds(&mut app, 5.0);
    for &car in &cars[..2] {
        control(&mut app, car).boost = true;
    }
    step_seconds(&mut app, 1.0);
    let (boosted, low) = (charge(&app, cars[0]), charge(&app, cars[1]));
    assert!((boosted - 0.15).abs() < 0.02, "35 %/s for 1 s: {boosted}");
    assert!((low - 0.10).abs() < 1e-4, "no boost below 15 %: {low}");
    let t = telemetry(&app, cars[0]);
    assert!(
        t.boosting && t.forward_speed > 1.15 * top,
        "boost speed {:.1}",
        t.forward_speed
    );
    assert!(!telemetry(&app, cars[1]).boosting);
    step_seconds(&mut app, 1.0);
    assert_eq!(charge(&app, cars[0]), 0.0, "runs dry");
    assert!(!telemetry(&app, cars[0]).boosting);
}

#[test]
fn turbo_freezes_the_drain_and_items_drain_and_grant() {
    let mut app = app();
    let cars = lanes(&mut app, &["volt_gt", "volt_gt"]);
    set_charge(&mut app, cars[0], 0.6);
    set_charge(&mut app, cars[1], 0.6);
    effects(&mut app, cars[0]).boost = Some(TimedBoost {
        top_speed: 1.45,
        force: 1.8,
        freeze_charge: true,
        remaining: 2.5,
    });
    let mut c = control(&mut app, cars[0]);
    c.boost = true;
    c.throttle = 1.0;
    step_seconds(&mut app, 1.0);
    assert!(
        (charge(&app, cars[0]) - 0.6).abs() < 1e-4,
        "Turbo freezes the drain"
    );
    app.world_mut().write_message(DrainCharge {
        car: cars[1],
        fraction: 0.5,
    });
    app.world_mut().write_message(GrantCharge {
        car: cars[0],
        amount: 0.7,
    });
    step(&mut app, 1);
    assert!(
        (charge(&app, cars[1]) - 0.3).abs() < 1e-4,
        "EMP halves the charge"
    );
    assert_eq!(charge(&app, cars[0]), 1.0, "grants clamp at full");
}

#[test]
fn drafting_charges_the_car_behind() {
    let mut app = app();
    let lead = spawn_car(&mut app, "phantom", Transform::from_xyz(0.0, 0.02, -4.5));
    let follow = spawn_car(&mut app, "phantom", Transform::from_xyz(0.0, 0.02, 0.0));
    let alone = spawn_car(&mut app, "phantom", Transform::from_xyz(60.0, 0.02, 0.0));
    step_seconds(&mut app, 0.5);
    for _ in 0..(6 * 60) {
        for car in [lead, follow, alone] {
            hold_speed(&mut app, car, 25.0);
        }
        step(&mut app, 1);
    }
    let (behind, solo) = (charge(&app, follow), charge(&app, alone));
    assert_eq!(solo, 0.0);
    assert!(behind > 0.2, "drafting for several seconds: {behind}");
}
