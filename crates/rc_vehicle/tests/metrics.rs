//! The metrics tables of `docs/design/rc_vehicle.md` (every car side by side on flat
//! asphalt, printed as Markdown) and robustness probes. `#[ignore]`d (they only print):
//! `cargo test -p rc_vehicle --test metrics -- --ignored --nocapture`.

mod common;

use bevy::prelude::*;
use common::*;

#[test]
#[ignore = "prints the metrics table; run with --ignored --nocapture"]
fn print_metrics_table() {
    let mut app = app();
    let cars = lanes(&mut app, &CARS);
    let p: Vec<_> = CARS.iter().map(|c| params(&app, c)).collect();
    let tops: Vec<f32> = p.iter().map(|p| p.top_speed).collect();
    let straight = straight(&mut app, &cars, &tops);
    let half: Vec<f32> = tops.iter().map(|t| 0.5 * t).collect();
    let turn = turn(&mut app, &cars, &half);
    let fast = drift(
        &mut app,
        &cars,
        &tops.iter().map(|t| 0.7 * t).collect::<Vec<_>>(),
    );
    println!(
        "| Car | Top speed u/s (target) | 0→99 % s (target) | Braking from top, u | \
         Turn radius @50 %, u | Lateral accel @50 %, u/s² | Yaw response, s | \
         Drift entry / hold, ° | Drift lateral accel | Drift recovery, s | Drift speed kept |"
    );
    println!("|---|---|---|---|---|---|---|---|---|---|---|");
    for (i, id) in CARS.iter().enumerate() {
        let (s, t, d) = (straight[i], turn[i], fast[i]);
        println!(
            "| {id} | {:.1} ({:.0}) | {:.2} ({:.2}) | {:.1} | {:.1} | {:.1} | {:.2} | {:.0} / {:.0}–{:.0} | {:.1} | {:.2} | {:.0} % |",
            s.top,
            tops[i],
            s.zero_to_top.unwrap_or(f32::NAN),
            p[i].accel_time,
            s.braking.unwrap_or(f32::NAN),
            t.radius,
            t.lateral,
            t.response.unwrap_or(f32::NAN),
            d.entry_max,
            d.hold.0,
            d.hold.1,
            d.lateral,
            d.recover.unwrap_or(f32::NAN),
            d.speed_kept * 100.0
        );
    }
}

#[test]
#[ignore = "prints the metrics table; run with --ignored --nocapture"]
fn print_air_table() {
    // Drops (3 u), flips (roof) and a long jump's clean landing for every car.
    let mut app = app();
    let drops: Vec<Entity> = CARS
        .iter()
        .enumerate()
        .map(|(i, id)| spawn_car(&mut app, id, Transform::from_xyz(i as f32 * 20.0, 3.0, 0.0)))
        .collect();
    let flips: Vec<Entity> = CARS
        .iter()
        .enumerate()
        .map(|(i, id)| {
            let roof = Quat::from_rotation_z(std::f32::consts::PI);
            spawn_car(
                &mut app,
                id,
                Transform::from_xyz(i as f32 * 20.0, 1.5, 40.0).with_rotation(roof),
            )
        })
        .collect();
    let mut y = vec![Vec::new(); 8];
    let mut up = vec![Vec::new(); 8];
    for _ in 0..(5 * 60) {
        rc_testkit::step(&mut app, 1);
        for i in 0..8 {
            y[i].push(pose(&app, drops[i]).translation.y);
            up[i].push(telemetry(&app, flips[i]).up_dot);
        }
    }
    println!("| Car | 3 u drop: deepest, u | settled (±0.02 u) after, s | Roof → upright, s |");
    println!("|---|---|---|---|");
    for (i, id) in CARS.iter().enumerate() {
        let rest = *y[i].last().expect("samples");
        let settle = y[i]
            .iter()
            .rposition(|v| (v - rest).abs() > 0.02)
            .map(|n| (n + 1) as f32 / 60.0);
        let min = y[i].iter().copied().fold(f32::MAX, f32::min);
        let upright = up[i]
            .iter()
            .position(|u| *u > 0.95)
            .map(|n| n as f32 / 60.0);
        println!(
            "| {id} | {min:+.2} | {:.2} | {:.2} |",
            settle.unwrap_or(f32::NAN),
            upright.unwrap_or(f32::NAN)
        );
    }
}

#[test]
#[ignore = "prints the jump table; run with --ignored --nocapture"]
fn print_jump_table() {
    let mut app = app();
    let jumps = kicker_jumps(&mut app, &CARS);
    println!(
        "| Car | take-off speed, u/s | air time, s | landing strength | clean | speed after, u/s | final up |"
    );
    println!("|---|---|---|---|---|---|---|");
    for (id, j) in CARS.iter().zip(jumps) {
        println!(
            "| {id} | {:.1} | {:.2} | {:.2} | {} | {:.1} | {:+.2} |",
            j.takeoff,
            j.landing.map_or(f32::NAN, |l| l.air_time),
            j.landing.map_or(f32::NAN, |l| l.strength),
            j.landing.is_some_and(|l| l.clean),
            j.speed_after,
            j.final_up
        );
    }
}

#[test]
#[ignore = "robustness probe: every car jumps alone; run with --ignored --nocapture"]
fn probe_jump_robustness() {
    for id in CARS {
        let mut app = app();
        let j = kicker_jumps(&mut app, &[id])[0];
        println!(
            "{id:9} alone: air {:.2} strength {:.2} clean {} final up {:+.2}",
            j.landing.map_or(0.0, |l| l.air_time),
            j.landing.map_or(0.0, |l| l.strength),
            j.landing.is_some_and(|l| l.clean),
            j.final_up
        );
    }
}

#[test]
#[ignore = "robustness probe: air inputs never flip a car; run with --ignored --nocapture"]
fn probe_air_inputs() {
    // In the air: 0 coast, 1 brake held, 2 full steer, 3 handbrake + steer (roll).
    for variant in 0..4 {
        let mut app = app();
        let mut cars = Vec::new();
        for (i, id) in CARS.iter().enumerate() {
            let x = i as f32 * 20.0;
            kicker(&mut app, Vec3::new(x, 0.0, -20.0));
            cars.push(spawn_car(&mut app, id, Transform::from_xyz(x, 0.02, 30.0)));
        }
        rc_testkit::step_seconds(&mut app, 0.5);
        let mut min_up = [1.0f32; 8];
        for _ in 0..(6 * 60) {
            for &car in &cars {
                let airborne = telemetry(&app, car).grounded_wheels == 0;
                let mut c = control(&mut app, car);
                *c = rc_core::prelude::CarControl {
                    throttle: 1.0,
                    ..Default::default()
                };
                if airborne {
                    match variant {
                        0 => c.throttle = 0.0,
                        1 => {
                            c.throttle = 0.0;
                            c.brake = 1.0
                        }
                        2 => c.steer = 1.0,
                        _ => {
                            c.steer = 1.0;
                            c.drift = true
                        }
                    }
                }
            }
            rc_testkit::step(&mut app, 1);
            for (i, &car) in cars.iter().enumerate() {
                min_up[i] = min_up[i].min(telemetry(&app, car).up_dot);
            }
        }
        let finals: Vec<String> = cars
            .iter()
            .map(|&c| format!("{:+.2}", telemetry(&app, c).up_dot))
            .collect();
        println!(
            "variant {variant}: final up {finals:?} min up {:?}",
            min_up.map(|u| (u * 100.0).round() / 100.0)
        );
    }
}
