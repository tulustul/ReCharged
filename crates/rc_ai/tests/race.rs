//! Headless AI races on the full sim stack (`rc_testkit::SimPlugins`): grid, countdown, laps and
//! respawns by rc_race, physics by rc_vehicle, items by rc_items.

mod common;

use common::*;
use rc_core::prelude::*;
use rc_testkit::rc_track::Track;

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
