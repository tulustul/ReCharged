//! Headless AI races on the full sim stack (`rc_testkit::SimPlugins`): grid, countdown, laps and
//! respawns by rc_race, physics by rc_vehicle, items by rc_items.

mod common;

use bevy::prelude::{Entity, Transform};
use common::*;
use rc_core::prelude::*;
use rc_testkit::rc_track::Track;
use rc_testkit::{load_race, race_phase, step};

/// GDD DoD-2: within 1.5× the expected time (laps × the track's reference lap) …
const MAX_RATIO: f32 = 1.5;
/// … with at most 2 respawns per lap.
const MAX_RESPAWNS_PER_LAP: u32 = 2;

fn assert_clean(run: &RaceRun, what: &str) {
    let expected = run.reference_lap * run.laps as f32;
    for r in &run.racers {
        assert!(
            r.finished,
            "{what}: {} did not finish ({:?})",
            r.name, r.laps
        );
        assert!(
            r.total <= MAX_RATIO * expected,
            "{what}: {} took {:.1} s, more than {MAX_RATIO}× {expected:.1} s",
            r.name,
            r.total
        );
        assert!(
            r.respawns <= MAX_RESPAWNS_PER_LAP * u32::from(run.laps),
            "{what}: {} respawned {} times",
            r.name,
            r.respawns
        );
    }
}

fn field_race(track: &str, difficulty: Difficulty, laps: u8, seed: u64) -> RaceRun {
    let mut app = race_app();
    let racers = ai_field(&app, 8, difficulty, CarClass::Pro);
    let run = run_race(&mut app, config(track, racers, laps, seed), 300.0);
    run.print(&format!("{track} {difficulty:?} seed {seed}"));
    run
}

#[test]
fn eight_ai_finish_three_laps_of_test_oval_and_elite_beats_rookie() {
    let rookie = field_race("test_oval", Difficulty::Rookie, 3, 11);
    let elite = field_race("test_oval", Difficulty::Elite, 3, 11);
    assert_clean(&rookie, "test_oval rookie");
    assert_clean(&elite, "test_oval elite");
    let (r, e) = (rookie.mean_lap(), elite.mean_lap());
    assert!(
        e < 0.92 * r,
        "Elite laps {e:.2} s vs Rookie {r:.2} s: no clear margin"
    );
}

#[test]
fn eight_ai_finish_three_laps_of_toy_town() {
    assert_clean(
        &field_race("toy_town", Difficulty::Pro, 3, 12),
        "toy_town pro",
    );
}

#[test]
fn a_seeded_race_is_deterministic() {
    let run = |seed| {
        let mut app = race_app();
        let racers = ai_field(&app, 8, Difficulty::Pro, CarClass::Rookie);
        let mut cfg = config("test_oval", racers, 1, seed);
        cfg.items = true;
        let run = run_race(&mut app, cfg, 60.0);
        let world = app.world_mut();
        let mut q = world.query::<(&Racer, &bevy::prelude::Transform)>();
        let mut poses: Vec<(usize, [u32; 3])> = q
            .iter(world)
            .map(|(r, t)| (r.index, t.translation.to_array().map(f32::to_bits)))
            .collect();
        poses.sort();
        let laps: Vec<Vec<u32>> = run
            .racers
            .iter()
            .map(|r| r.laps.iter().map(|l| l.to_bits()).collect())
            .collect();
        (laps, poses, run.order, run.items_used)
    };
    let a = run(5);
    assert_eq!(a, run(5), "same seed, same race");
    assert_ne!(a.1, run(6).1, "another seed races differently");
}

/// A player who missed GO: nobody drives this car.
fn parked_player() -> RacerSetup {
    RacerSetup {
        name: "Parked".into(),
        car: CarId::new("hornet"),
        controller: Controller::Human {
            slot: PlayerSlot(0),
            autopilot: false,
        },
        ..Default::default()
    }
}

/// An AI has reached the parked car once it is this close behind it along the track (u).
const REACH_U: f32 = 6.0;

/// 7 Pro AI behind a car parked on the racing line `ahead` u past the pole slot (0 = on pole): for
/// every AI, the seconds from reaching the parked car (GO for the cars that start that close) to
/// getting past it (`None` = not within 20 s). `AI_TRACE_ALL=1` prints each AI's distance to the
/// parked car, lateral, speed and recovery (`R`) every 0.2 s; `AI_TRACE=<index>` traces one car.
fn parked_car_race(track: &str, seed: u64, ahead: f32) -> Vec<(String, Option<f32>)> {
    let mut app = race_app();
    let mut racers = vec![parked_player()];
    racers.extend(ai_field(&app, 7, Difficulty::Pro, CarClass::Pro));
    load_race(&mut app, config(track, racers, 1, seed)).expect("loads");
    while race_phase(&app) != Some(RacePhase::Intro) {
        step(&mut app, 1);
    }
    let line = app.world().resource::<Track>().clone();
    let s = line.project(line.grid()[0].translation, None).s + ahead;
    let frame = line.frame_at(s);
    let on_line = frame.point(line.racing_line_offset_at(s), 0.15);
    let cars: Vec<Entity> = {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &Racer)>();
        let mut cars: Vec<(usize, Entity)> = q.iter(world).map(|(e, r)| (r.index, e)).collect();
        cars.sort();
        cars.into_iter().map(|(_, e)| e).collect()
    };
    app.world_mut().write_message(RespawnCar {
        car: cars[0],
        transform: Transform::from_translation(on_line).with_rotation(frame.rotation()),
        kind: RespawnKind::ToTrack,
    });
    while race_phase(&app) != Some(RacePhase::Racing) {
        step(&mut app, 1);
    }
    let trace = std::env::var("AI_TRACE_ALL").is_ok();
    let traced: Option<usize> = std::env::var("AI_TRACE").ok().and_then(|v| v.parse().ok());
    let mut reached: Vec<Option<f32>> = vec![None; cars.len()];
    let mut passed: Vec<Option<f32>> = vec![None; cars.len()];
    for tick in 0..secs_to_ticks(20.0) {
        step(&mut app, 1);
        if let Some(index) = traced {
            trace_racer(&mut app, index, tick);
        }
        let t = ticks_to_secs(tick + 1);
        let w = app.world();
        let view = |e: Entity| *w.get::<rc_ai::TrackView>(e).expect("track view");
        let parked = view(cars[0]).s;
        if trace && tick.is_multiple_of(12) {
            let row: Vec<String> = cars[1..]
                .iter()
                .map(|&e| {
                    let (v, brain) = (view(e), w.get::<rc_ai::AiBrain>(e).expect("brain"));
                    let driving = format!("{:?}", brain.recovery).contains("Driving");
                    let rec = if driving { ' ' } else { 'R' };
                    let d = line.delta_s(parked, v.s);
                    format!("{d:6.1}/{:5.2}/{:4.1}{rec}", v.lateral, v.vel.length())
                })
                .collect();
            println!("t {t:5.2} {}", row.join(" "));
        }
        for (i, &car) in cars.iter().enumerate().skip(1) {
            let ahead = line.delta_s(parked, view(car).s);
            if reached[i].is_none() && ahead > -REACH_U {
                reached[i] = Some(if tick == 0 { 0.0 } else { t });
            }
            if passed[i].is_none() && ahead > 0.0 {
                passed[i] = Some(t);
            }
        }
        if passed[1..].iter().all(Option::is_some) {
            break;
        }
    }
    let name = |e: Entity| app.world().get::<Racer>(e).map(|r| r.name.clone());
    (1..cars.len())
        .map(|i| {
            let secs = passed[i].zip(reached[i]).map(|(p, r)| p - r);
            (name(cars[i]).unwrap_or_default(), secs)
        })
        .collect()
}

/// GDD §8.1 (R2): nobody queues behind a stopped car — a player who missed GO, parked on the
/// racing line on pole, is passed by each of the 7 AI behind within about 3 s of reaching it.
#[test]
fn every_ai_passes_a_car_parked_on_pole_within_3_s() {
    for track in ["toy_town", "test_oval"] {
        let times = parked_car_race(track, 31, 0.0);
        for (name, secs) in &times {
            assert!(
                secs.is_some_and(|s| s <= 3.0),
                "{track}: {name} took {secs:?} s to get past the parked car: {times:?}"
            );
        }
    }
}

/// [`parked_car_race`] with `AI_TRACK`, `AI_SEED` and `AI_PARK_AHEAD` (u past the pole slot),
/// printing each AI's time: `cargo test -p rc_ai --test race -- --ignored --nocapture
/// parked_probe` (`AI_TRACE_ALL=1` for the trace).
#[test]
#[ignore = "tuning probe"]
fn parked_probe() {
    let env = |k: &str, d: &str| std::env::var(k).unwrap_or_else(|_| d.to_owned());
    let track = env("AI_TRACK", "toy_town");
    let seed: u64 = env("AI_SEED", "31").parse().expect("AI_SEED");
    let ahead: f32 = env("AI_PARK_AHEAD", "0").parse().expect("AI_PARK_AHEAD");
    let times = parked_car_race(&track, seed, ahead);
    println!("{track} seed {seed} ahead {ahead}: {times:?}");
}

/// Every layout of every catalog track with a data file (forward, reverse, mirrored), 8 Elite AI
/// for 3 laps, GDD DoD-2. Long (a few minutes); QA runs it:
/// `cargo test -p rc_ai --test race -- --ignored --nocapture every_layout`.
#[test]
#[ignore = "long: every layout; run by QA"]
fn every_layout() {
    let app = race_app();
    let catalog = app.world().resource::<Catalog>().clone();
    let tracks_dir = rc_testkit::assets_dir().join("data/tracks");
    let mut failures = Vec::new();
    for info in &catalog.tracks {
        if !tracks_dir.join(format!("{}.ron", info.id)).exists() {
            println!("skip {}: no data file yet", info.id);
            continue;
        }
        for (reverse, mirror) in [(false, false), (true, false), (false, true), (true, true)] {
            let mut app = race_app();
            let racers = ai_field(&app, 8, Difficulty::Elite, CarClass::Elite);
            let mut cfg = config(info.id.as_str(), racers, 3, 21);
            cfg.variant.reverse = reverse;
            cfg.variant.mirror = mirror;
            let label = format!("{} rev {reverse} mirror {mirror}", info.id);
            if let Err(e) = Track::load(&rc_testkit::assets_dir(), &cfg.variant) {
                failures.push(format!("{label}: does not load: {e}"));
                continue;
            }
            let run = run_race(&mut app, cfg, 400.0);
            run.print(&label);
            if let Err(e) = std::panic::catch_unwind(|| assert_clean(&run, &label)) {
                failures.push(format!("{label}: {e:?}"));
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

/// The lap-time table of `docs/design/rc_ai.md` (mean race ratio and flying laps over seeds):
/// `cargo test -p rc_ai --test race -- --ignored --nocapture lap_table`
/// (`AI_TRACKS=a,b AI_DIFFS=Rookie,Pro AI_SEEDS=4 AI_ITEMS=1` narrow or widen it).
#[test]
#[ignore = "tuning table; long"]
fn lap_table() {
    let env = |k: &str, d: &str| std::env::var(k).unwrap_or_else(|_| d.to_owned());
    let seeds: u64 = env("AI_SEEDS", "3").parse().expect("AI_SEEDS");
    for track in env("AI_TRACKS", "test_oval,toy_town").split(',') {
        let reference = Track::load(&rc_testkit::assets_dir(), &TrackVariant::forward(track))
            .map(|t| reference_lap(&t))
            .unwrap_or(f32::NAN);
        for diff in env("AI_DIFFS", "Rookie,Pro,Elite,ElitePlus").split(',') {
            let difficulty = difficulty_named(diff);
            let (mut ratios, mut flying, mut respawns, mut dnf) = (Vec::new(), Vec::new(), 0, 0);
            let mut causes = Vec::new();
            for seed in 1..=seeds {
                let run = field_race(track, difficulty, 3, seed);
                for &(cause, n) in &run.causes {
                    for _ in 0..n {
                        tally(&mut causes, cause);
                    }
                }
                let expected = run.reference_lap * run.laps as f32;
                for r in &run.racers {
                    if r.finished {
                        ratios.push(r.total / expected);
                    } else {
                        dnf += 1;
                    }
                    flying.extend(r.laps.iter().skip(1).copied());
                    respawns += r.respawns;
                }
            }
            ratios.sort_by(f32::total_cmp);
            flying.sort_by(f32::total_cmp);
            println!(
                "| {track} | {diff} | {reference:.1} | {:.2} | {:.2} | {:.3} | {:.3} | {respawns} | {dnf} |",
                flying[flying.len() / 2],
                flying[0],
                ratios.iter().sum::<f32>() / ratios.len() as f32,
                ratios[ratios.len() - 1],
            );
            if !causes.is_empty() {
                println!("respawn causes ({track} {diff}): {causes:?}");
            }
        }
    }
}

/// One configurable race with an optional per-tick trace, for tuning:
/// `AI_TRACK=toy_town AI_DIFF=Elite AI_N=8 AI_LAPS=3 AI_SEED=7 [AI_REV=1] [AI_MIRROR=1]
/// [AI_ITEMS=1] [AI_TRACE=<index>]
/// cargo test -p rc_ai --test race -- --ignored --nocapture probe`.
#[test]
#[ignore = "tuning probe"]
fn probe() {
    let env = |k: &str, d: &str| std::env::var(k).unwrap_or_else(|_| d.to_owned());
    let track = env("AI_TRACK", "test_oval");
    let difficulty = difficulty_named(&env("AI_DIFF", "Elite"));
    let class = match env("AI_CLASS", "Pro").as_str() {
        "Rookie" => CarClass::Rookie,
        "Elite" => CarClass::Elite,
        _ => CarClass::Pro,
    };
    let n: usize = env("AI_N", "8").parse().expect("AI_N");
    let laps: u8 = env("AI_LAPS", "3").parse().expect("AI_LAPS");
    let seed: u64 = env("AI_SEED", "7").parse().expect("AI_SEED");
    let traced: Option<usize> = std::env::var("AI_TRACE").ok().and_then(|v| v.parse().ok());
    let mut app = race_app();
    // `AI_TT=1`: the Time Trial of `tt_table` (the autopilot alone in a Phantom).
    let tt = env("AI_TT", "0") == "1";
    let racers = match tt {
        true => vec![tt_racer()],
        false => ai_field(&app, n, difficulty, class),
    };
    let mut cfg = config(&track, racers, laps, seed);
    if tt {
        cfg.mode = GameMode::TimeTrial;
        cfg.items = false;
    }
    cfg.variant.reverse = env("AI_REV", "0") == "1";
    cfg.variant.mirror = env("AI_MIRROR", "0") == "1";
    let run = run_race_traced(&mut app, cfg, 300.0, |app, tick| {
        if let Some(index) = traced {
            trace_racer(app, index, tick);
        }
    });
    run.print(&format!("{track} {difficulty:?} {class:?}"));
}

/// The Time Trial racer of `recharged --calibrate-medals`: a human on autopilot in a Phantom.
fn tt_racer() -> RacerSetup {
    RacerSetup {
        name: "Phantom".into(),
        car: CarId::new("phantom"),
        controller: Controller::Human {
            slot: PlayerSlot(0),
            autopilot: true,
        },
        ..Default::default()
    }
}

/// Time Trials by the autopilot (Elite, neutral personality) in a Phantom — the setup of
/// `recharged --calibrate-medals` — on every layout of every track with a file (or `AI_TRACKS`),
/// seeds 1..=`AI_SEEDS` (3): race times, respawns and outliers (> 3 s over the layout's median):
/// `cargo test -p rc_ai --test race -- --ignored --nocapture tt_table`.
#[test]
#[ignore = "tuning table; long"]
fn tt_table() {
    let env = |k: &str, d: &str| std::env::var(k).unwrap_or_else(|_| d.to_owned());
    let seeds: u64 = env("AI_SEEDS", "3").parse().expect("AI_SEEDS");
    let tracks: Vec<String> = match std::env::var("AI_TRACKS") {
        Ok(list) => list.split(',').map(str::to_owned).collect(),
        Err(_) => {
            let app = race_app();
            let dir = rc_testkit::assets_dir().join("data/tracks");
            let catalog = app.world().resource::<Catalog>();
            catalog
                .tracks
                .iter()
                .map(|t| t.id.to_string())
                .filter(|id| dir.join(format!("{id}.ron")).exists())
                .collect()
        }
    };
    let mut outliers = 0;
    for track in &tracks {
        for (reverse, mirror) in [(false, false), (true, false), (false, true), (true, true)] {
            let mut runs = Vec::new();
            for seed in 1..=seeds {
                let mut app = race_app();
                let mut cfg = config(track, vec![tt_racer()], 3, seed);
                cfg.mode = GameMode::TimeTrial;
                cfg.items = false;
                cfg.variant.reverse = reverse;
                cfg.variant.mirror = mirror;
                let run = run_race(&mut app, cfg, 300.0);
                let r = &run.racers[0];
                runs.push((if r.finished { r.total } else { f32::NAN }, r.respawns));
            }
            let mut sorted: Vec<f32> = runs.iter().map(|r| r.0).collect();
            sorted.sort_by(f32::total_cmp);
            let median = sorted[sorted.len() / 2];
            let shown: Vec<String> = runs
                .iter()
                .map(|(t, n)| {
                    let flag = if t.is_nan() || *t > median + 3.0 {
                        outliers += 1;
                        "!"
                    } else {
                        ""
                    };
                    format!(
                        "{t:.2}{}{flag}",
                        if *n > 0 {
                            format!("/{n}r")
                        } else {
                            String::new()
                        }
                    )
                })
                .collect();
            println!(
                "{track:<14} rev {reverse:<5} mirror {mirror:<5} median {median:6.2}  [{}]",
                shown.join(", ")
            );
        }
    }
    println!("outliers (> median + 3 s or DNF): {outliers}");
}
