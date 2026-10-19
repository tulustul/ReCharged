//! Headless behaviour of `ItemsPlugin` on `test_oval` with box cars: pickups and the seeded
//! roulette, rockets, shields, goo, oil, spring pads, leech, orb, storm, EMP, restart.

mod common;

use bevy::prelude::*;
use bevy_rapier3d::prelude::{Collider, CollisionGroups, Group};
use common::*;
use rc_core::prelude::*;
use rc_items::{Band, ItemClock, ItemTuning};
use rc_testkit::*;

/// On the main straight (heading −Z, outer wall on the left at x ≈ −6.9).
fn straight(z: f32) -> Vec3 {
    Vec3::new(0.0, ROAD_Y, z)
}

#[test]
fn rocket_hits_a_stationary_target_and_tumbles_it() {
    let mut app = race(1);
    let shooter = spawn_car(&mut app, straight(5.0), 0.0);
    let target = spawn_car(&mut app, straight(-25.0), 0.0);
    step(&mut app, 20);
    let y0 = pose(&app, target).translation.y;
    fire(&mut app, shooter, ItemKind::SparkRocket, false);
    let lock = ItemEvent {
        racer: shooter,
        kind: ItemEventKind::LockOn { target },
    };
    assert!(log(&app).items.contains(&lock), "{:?}", log(&app).items);
    assert_eq!(all::<Projectile>(&mut app).len(), 1);
    step_until(&mut app, "the rocket hits", |w| {
        w.resource::<Log>()
            .hits
            .iter()
            .any(|(_, h)| h.victim == target)
    });
    let hit = hits_on(&app, target, ItemKind::SparkRocket)[0];
    assert!(!hit.blocked && hit.attacker == Some(shooter));
    assert!(hits_on(&app, shooter, ItemKind::SparkRocket).is_empty());
    assert!(
        log(&app)
            .explosions
            .iter()
            .any(|e| e.kind == ItemKind::SparkRocket)
    );
    step_seconds(&mut app, 0.3);
    let (t, v) = (pose(&app, target), velocity(&app, target));
    assert!(
        t.translation.y > y0 + 1.0,
        "lifted: {} → {}",
        y0,
        t.translation.y
    );
    assert!(v.angular.length() > 1.0, "tumbling: {}", v.angular);
    assert!(
        all::<Projectile>(&mut app).is_empty(),
        "the rocket is spent"
    );
}

#[test]
fn unlocked_rocket_flies_straight_and_explodes_on_the_wall() {
    let mut app = race(1);
    // Facing the outer wall (−X) on the main straight; nobody in sight to lock onto: the car
    // parked on the grass behind the wall is in the cone and within the blast radius of the
    // impact, but the wall hides it from the lock and shields it from the blast.
    let shooter = spawn_car(&mut app, straight(0.0), 90f32.to_radians());
    let hidden = spawn_car(&mut app, Vec3::new(-8.3, 0.0, 0.0), 0.0);
    step(&mut app, 10);
    fire(&mut app, shooter, ItemKind::SparkRocket, false);
    step_until(&mut app, "the rocket explodes", |w| {
        !w.resource::<Log>().explosions.is_empty()
    });
    let blast = log(&app).explosions[0];
    assert!(
        blast.position.x < -5.0 && blast.position.x > -8.0,
        "at the wall: {blast:?}"
    );
    assert!(
        (blast.position.z - 0.0).abs() < 0.5,
        "flew straight: {blast:?}"
    );
    assert!(log(&app).hits.is_empty(), "the owner is immune and alone");
    assert!(
        !log(&app)
            .items
            .iter()
            .any(|e| e.kind == ItemEventKind::LockOn { target: hidden }),
        "no lock through the wall"
    );
}

#[test]
fn a_rocket_that_loses_its_lock_locks_onto_the_next_car_in_its_cone() {
    let mut app = race(1);
    let shooter = spawn_car(&mut app, straight(5.0), 0.0);
    let near = spawn_car(&mut app, straight(-20.0), 0.0);
    let far = spawn_car(&mut app, straight(-50.0), 0.0);
    step(&mut app, 20);
    fire(&mut app, shooter, ItemKind::SparkRocket, false);
    let lock = |target| ItemEvent {
        racer: shooter,
        kind: ItemEventKind::LockOn { target },
    };
    assert!(
        log(&app).items.contains(&lock(near)),
        "the nearest car first"
    );
    app.world_mut().despawn(near); // the target is gone: the lock is lost…
    step(&mut app, 2);
    let items = &log(&app).items;
    let lost = ItemEvent {
        racer: shooter,
        kind: ItemEventKind::LockLost,
    };
    assert!(items.contains(&lost), "{items:?}");
    assert!(items.contains(&lock(far)), "…and re-acquired: {items:?}");
    let rocket = all::<Projectile>(&mut app)[0];
    assert_eq!(
        app.world().get::<ItemTarget>(rocket),
        Some(&ItemTarget(Some(far)))
    );
    step_until(&mut app, "the rocket hits the new target", |w| {
        w.resource::<Log>()
            .hits
            .iter()
            .any(|(_, h)| h.victim == far)
    });
}

#[test]
fn an_emp_does_not_reach_through_a_wall() {
    let mut app = race(1);
    // The user stands on the grass outside the outer wall (x −6.9…−7.5); one car is 9.5 u away
    // on the track behind the wall, another in plain view outside.
    let user = spawn_car(&mut app, Vec3::new(-9.5, 0.0, 0.0), 0.0);
    let behind_wall = spawn_car(&mut app, straight(0.0), 0.0);
    let in_view = spawn_car(&mut app, Vec3::new(-9.5, 0.0, -6.0), 0.0);
    step(&mut app, 10);
    fire(&mut app, user, ItemKind::EmpPulse, false);
    step_seconds(&mut app, 0.6);
    assert_eq!(hits_on(&app, in_view, ItemKind::EmpPulse).len(), 1);
    assert!(
        hits_on(&app, behind_wall, ItemKind::EmpPulse).is_empty(),
        "the wall shields it"
    );
}

#[test]
fn storm_strike_spares_a_car_under_cover() {
    let mut app = race(1);
    let cars: Vec<Entity> = (0..3)
        .map(|i| spawn_car(&mut app, straight(-12.0 + 6.0 * i as f32), 0.0))
        .collect();
    let (covered, open, user) = (cars[0], cars[1], cars[2]);
    // A roof (a static prop collider) over the leader.
    app.world_mut().spawn((
        Transform::from_xyz(0.0, 3.0, -12.0),
        Collider::cuboid(2.0, 0.2, 2.0),
        CollisionGroups::new(Group::from_bits_truncate(layers::PROP), Group::ALL),
    ));
    app.world_mut().insert_resource(Standings(cars.clone()));
    step(&mut app, 10);
    fire(&mut app, user, ItemKind::StormStrike, false);
    step_seconds(&mut app, 1.0);
    assert!(
        hits_on(&app, covered, ItemKind::StormStrike).is_empty(),
        "sheltered"
    );
    assert_eq!(hits_on(&app, open, ItemKind::StormStrike).len(), 1);
}

#[test]
fn shield_blocks_a_hit_and_pops() {
    let mut app = race(1);
    let shooter = spawn_car(&mut app, straight(5.0), 0.0);
    let target = spawn_car(&mut app, straight(-25.0), 0.0);
    step(&mut app, 20);
    fire(&mut app, target, ItemKind::ShieldBubble, false);
    step(&mut app, 1);
    assert!(effects(&app, target).shielded());
    let y0 = pose(&app, target).translation.y;
    fire(&mut app, shooter, ItemKind::SparkRocket, false);
    step_until(&mut app, "the rocket hits", |w| {
        w.resource::<Log>()
            .hits
            .iter()
            .any(|(_, h)| h.victim == target)
    });
    let hit = hits_on(&app, target, ItemKind::SparkRocket)[0];
    assert!(hit.blocked, "{hit:?}");
    step(&mut app, 2);
    assert!(!effects(&app, target).shielded(), "the shield popped");
    step_seconds(&mut app, 0.3);
    assert!(
        (pose(&app, target).translation.y - y0).abs() < 0.1,
        "no tumble"
    );
    assert!(velocity(&app, target).angular.length() < 0.2);
}

#[test]
fn every_hit_in_the_tick_a_shield_pops_is_blocked_and_emp_drains_the_rest() {
    let mut app = race(1);
    let victim = spawn_car(&mut app, straight(0.0), 0.0);
    let bystander = spawn_car(&mut app, straight(-4.0), 0.0);
    let emps = [
        spawn_car(&mut app, straight(6.0), 0.0),
        spawn_car(&mut app, Vec3::new(3.0, ROAD_Y, 4.0), 0.0),
    ];
    step(&mut app, 10);
    fire(&mut app, victim, ItemKind::ShieldBubble, false);
    step(&mut app, 1);
    for car in [victim, bystander] {
        app.world_mut().get_mut::<Charge>(car).expect("racer").0 = 0.8;
    }
    // Both EMPs go off in the same tick.
    for car in emps {
        app.world_mut()
            .get_mut::<ItemSlot>(car)
            .expect("racer")
            .item = Some(ItemKind::EmpPulse);
        app.world_mut()
            .get_mut::<CarControl>(car)
            .expect("racer")
            .use_item = true;
    }
    step(&mut app, 1);
    let used = frame(&app);
    for car in emps {
        app.world_mut()
            .get_mut::<CarControl>(car)
            .expect("racer")
            .use_item = false;
    }
    step_seconds(&mut app, 0.6);
    let blocked = hits_on(&app, victim, ItemKind::EmpPulse);
    assert_eq!(blocked.len(), 2);
    assert!(blocked.iter().all(|h| h.blocked), "{blocked:?}");
    let ticks: Vec<u32> = log(&app)
        .hits
        .iter()
        .filter(|(_, h)| h.victim == victim)
        .map(|(f, _)| *f)
        .collect();
    assert_eq!(ticks[0], ticks[1], "same tick");
    assert_eq!(ticks[0] - used, 24, "after the 0.4 s wind-up");
    let v = effects(&app, victim);
    assert!(!v.shielded() && v.power_loss.is_none());
    assert_eq!(app.world().get::<Charge>(victim).expect("racer").0, 0.8);
    // The unshielded bystander loses power (one effect) and 50 % of its charge, twice.
    assert_eq!(hits_on(&app, bystander, ItemKind::EmpPulse).len(), 2);
    assert_eq!(effects(&app, bystander).power_mul(), 0.0);
    let charge = app.world().get::<Charge>(bystander).expect("racer").0;
    assert!((charge - 0.2).abs() < 1e-4, "{charge}");
    let emp = &app.world().resource::<ItemTuning>().emp_pulse;
    assert_eq!(log(&app).explosions.len(), 2);
    assert!(log(&app).explosions.iter().all(|e| e.radius == emp.radius));
}

#[test]
fn goo_passes_on_contact_and_explodes_after_the_fuse() {
    let mut app = race(1);
    let a = spawn_car(&mut app, straight(0.0), 0.0);
    let b = spawn_car(&mut app, Vec3::new(3.0, ROAD_Y, 0.0), 0.0);
    step(&mut app, 10);
    fire(&mut app, a, ItemKind::GooBomb, false);
    let goo = all::<Hazard>(&mut app)[0];
    assert_eq!(
        app.world().get::<ItemTarget>(goo),
        Some(&ItemTarget(Some(a)))
    );
    set_velocity(&mut app, a, Vec3::X * 4.0);
    step_until(&mut app, "the goo is passed", |w| {
        w.get::<ItemTarget>(goo) == Some(&ItemTarget(Some(b)))
    });
    assert_eq!(app.world().get::<Hazard>(goo).map(|h| h.owner), Some(a));
    for car in [a, b] {
        set_velocity(&mut app, car, Vec3::ZERO); // frictionless test cars would slide apart
    }
    step(&mut app, 1);
    let t = pose(&app, goo).translation;
    assert!(
        t.distance(pose(&app, b).translation) < 1.5,
        "rides on its carrier"
    );
    step_until(&mut app, "the goo explodes", |w| {
        !w.resource::<Log>().explosions.is_empty()
    });
    let boom = log(&app).explosions[0];
    assert_eq!(boom.kind, ItemKind::GooBomb);
    assert!(
        log(&app)
            .hits
            .iter()
            .all(|(_, h)| h.by == ItemKind::GooBomb)
    );
    let carrier = hits_on(&app, b, ItemKind::GooBomb);
    assert_eq!(carrier.len(), 1, "the carrier is hit too");
    assert_eq!(carrier[0].attacker, Some(a));
    assert_eq!(
        hits_on(&app, a, ItemKind::GooBomb).len(),
        1,
        "a is within 7 u"
    );
    assert!(all::<Hazard>(&mut app).is_empty());
}

#[test]
fn goo_passes_on_a_near_miss_and_speeds_its_carrier() {
    // Side by side, chassis 1.1 u wide: 1.5 u apart leaves a 0.4 u gap, 2.1 u a 1.0 u gap.
    let near = |apart: f32| {
        let mut app = race(1);
        let a = spawn_car(&mut app, straight(0.0), 0.0);
        let b = spawn_car(&mut app, Vec3::new(apart, ROAD_Y, 0.0), 0.0);
        step(&mut app, 10);
        fire(&mut app, a, ItemKind::GooBomb, false);
        step(&mut app, 3);
        let goo = all::<Hazard>(&mut app)[0];
        let carrier = app.world().get::<ItemTarget>(goo).and_then(|t| t.0);
        let fast = |car| effects(&app, car).top_speed_mul() > 1.05;
        (carrier == Some(b), fast(a), fast(b))
    };
    assert_eq!(
        near(1.5),
        (true, false, true),
        "passed without a bump; the new carrier is fast"
    );
    assert_eq!(
        near(2.1),
        (false, true, false),
        "too far: kept, and its carrier is fast"
    );
}

#[test]
fn goo_explodes_on_time() {
    let mut app = race(1);
    let a = spawn_car(&mut app, straight(0.0), 0.0);
    step(&mut app, 10);
    let fuse = app.world().resource::<ItemTuning>().goo_bomb.fuse_secs;
    fire(&mut app, a, ItemKind::GooBomb, false);
    step(&mut app, secs_to_ticks(fuse) - 1);
    assert!(log(&app).explosions.is_empty(), "not before the fuse");
    step(&mut app, 1);
    assert_eq!(
        log(&app).explosions.len(),
        1,
        "exactly {fuse} s after the use"
    );
    assert_eq!(hits_on(&app, a, ItemKind::GooBomb).len(), 1, "carrier hit");
}

#[test]
fn oil_slick_makes_cars_slip_but_spares_its_owner() {
    let mut app = race(1);
    let dropper = spawn_car(&mut app, straight(0.0), 0.0);
    let victim = spawn_car(&mut app, straight(14.0), 0.0);
    step(&mut app, 10);
    fire(&mut app, dropper, ItemKind::OilSlick, false);
    let oil = all::<Hazard>(&mut app)[0];
    let radius = app.world().resource::<ItemTuning>().oil_slick.radius;
    assert_eq!(
        app.world().get::<ItemRadius>(oil),
        Some(&ItemRadius(radius))
    );
    let at = pose(&app, oil).translation;
    assert!(
        at.z > 3.0 && at.z < 4.5 && (at.y - ROAD_Y).abs() < 0.05,
        "behind, on the road: {at}"
    );
    set_velocity(&mut app, victim, Vec3::NEG_Z * 10.0);
    step_until(&mut app, "the victim enters the oil", |w| {
        w.resource::<Log>()
            .hits
            .iter()
            .any(|(_, h)| h.victim == victim)
    });
    let hit = hits_on(&app, victim, ItemKind::OilSlick);
    assert_eq!(hit.len(), 1);
    assert_eq!(hit[0].attacker, Some(dropper));
    step(&mut app, 2);
    let grip = effects(&app, victim).grip.expect("slipping");
    assert_eq!(grip.mul, 0.2);
    assert!(velocity(&app, victim).angular.y.abs() > 0.5, "spun");
    step_seconds(&mut app, 0.5);
    assert_eq!(
        hits_on(&app, victim, ItemKind::OilSlick).len(),
        1,
        "entering hits once"
    );
    assert!(hits_on(&app, dropper, ItemKind::OilSlick).is_empty());
}

#[test]
fn spring_pad_launches_cars() {
    let mut app = race(1);
    let dropper = spawn_car(&mut app, straight(0.0), 0.0);
    let victim = spawn_car(&mut app, straight(12.0), 0.0);
    step(&mut app, 10);
    fire(&mut app, dropper, ItemKind::SpringPad, false);
    set_velocity(&mut app, victim, Vec3::NEG_Z * 10.0);
    step_until(&mut app, "the victim crosses the pad", |w| {
        w.resource::<Log>()
            .hits
            .iter()
            .any(|(_, h)| h.victim == victim)
    });
    step(&mut app, 2);
    let v = velocity(&app, victim).linear;
    // items.ron: up 8 u/s (0.64 s of air), less two ticks of gravity.
    assert!(v.y > 6.5, "launched: {v}");
    step_seconds(&mut app, 0.3);
    let y = pose(&app, victim).translation.y;
    assert!(y > ROAD_Y + 0.8, "in the air: {y}");
}

#[test]
fn leech_coil_moves_charge_and_tows() {
    let mut app = race(1);
    let me = spawn_car(&mut app, straight(0.0), 0.0);
    let ahead = spawn_car(&mut app, straight(-15.0), 0.0);
    let behind = spawn_car(&mut app, straight(6.0), 0.0); // nearer, but behind
    app.world_mut()
        .insert_resource(Standings(vec![ahead, me, behind]));
    step(&mut app, 10);
    app.world_mut().get_mut::<Charge>(ahead).expect("racer").0 = 0.8;
    fire(&mut app, me, ItemKind::LeechCoil, false);
    step(&mut app, 2);
    let hit = hits_on(&app, ahead, ItemKind::LeechCoil);
    assert_eq!(hit.len(), 1, "{:?}", log(&app).hits);
    let tether = all::<Projectile>(&mut app)[0];
    assert_eq!(
        app.world().get::<ItemTarget>(tether),
        Some(&ItemTarget(Some(ahead)))
    );
    step_seconds(&mut app, 1.0);
    assert_eq!(effects(&app, me).top_speed_mul(), 1.08, "towed");
    step_seconds(&mut app, 2.5);
    let charge = |car| app.world().get::<Charge>(car).expect("racer").0;
    assert!((charge(me) - 0.24).abs() < 0.01, "gained {}", charge(me));
    assert!(
        (charge(ahead) - 0.56).abs() < 0.01,
        "kept {}",
        charge(ahead)
    );
    assert!(all::<Projectile>(&mut app).is_empty(), "the tether ended");
    assert_eq!(effects(&app, me).top_speed_mul(), 1.0);
}

#[test]
fn shock_orb_follows_the_track_and_bounces() {
    let mut app = race(1);
    // Angled 25° toward the outer (left) wall on the main straight.
    let car = spawn_car(&mut app, straight(5.0), 25f32.to_radians());
    step(&mut app, 10);
    fire(&mut app, car, ItemKind::ShockOrb, false);
    let orb = all::<Projectile>(&mut app)[0];
    let (track_len, start) = {
        let t = track(&app);
        (t.length(), t.project(pose(&app, orb).translation, None))
    };
    let mut last = start;
    let (mut travelled, mut bounces, mut sign) = (0.0, 0, -1.0);
    let mut widest_left = 0.0f32;
    for _ in 0..secs_to_ticks(4.0) {
        step(&mut app, 1);
        let p = pose(&app, orb).translation;
        let t = track(&app);
        let at = t.project(p, Some(last.s));
        let ds = t.delta_s(last.s, at.s);
        assert!(ds > 0.0 && ds < 1.0, "moves forward along the track: {ds}");
        travelled += ds;
        let hw = t.frame_at(at.s).half_width;
        // Left: the outer barrier stands beyond the 0.9 u curb (the core keeps 0.5 u off it);
        // right: the infield is open, so it turns at the racing width's edge.
        assert!(
            at.lateral >= -(hw + 0.4 + 1e-3) && at.lateral <= hw - 0.5 + 1e-3,
            "between the barrier and the open edge: {} of {hw}",
            at.lateral
        );
        widest_left = widest_left.max(-at.lateral);
        assert!(
            (at.height - 0.7).abs() < 0.05,
            "glides above it: {}",
            at.height
        );
        let now = (at.lateral - last.lateral).signum();
        if now != 0.0 && now != sign {
            bounces += 1;
            sign = now;
        }
        last = at;
    }
    assert!(bounces >= 2, "{bounces} bounces");
    assert!(
        widest_left > 6.1,
        "it reached the barrier beyond the 6 u half width: {widest_left}"
    );
    assert!(travelled > 70.0 && travelled < track_len, "{travelled} u");
    // Turn 1 starts at z ≈ -40: the orb followed it round to the right (+X).
    assert!(
        pose(&app, orb).translation.x > 5.0,
        "{}",
        pose(&app, orb).translation
    );
}

#[test]
fn shock_orb_knocks_cars_but_never_its_owner() {
    let mut app = race(1);
    let owner = spawn_car(&mut app, straight(5.0), 0.0);
    let victim = spawn_car(&mut app, straight(-10.0), 0.0);
    step(&mut app, 10);
    fire(&mut app, owner, ItemKind::ShockOrb, false);
    step_seconds(&mut app, 1.0);
    assert_eq!(hits_on(&app, victim, ItemKind::ShockOrb).len(), 1);
    assert!(
        velocity(&app, victim).linear.x.abs() > 3.0,
        "knocked sideways"
    );
    // Drive the owner through its own orb: nothing.
    let orb = all::<Projectile>(&mut app)[0];
    let at = pose(&app, orb).translation;
    app.world_mut()
        .get_mut::<Transform>(owner)
        .expect("car")
        .translation = at - Vec3::Y * 0.7;
    step(&mut app, 5);
    assert!(hits_on(&app, owner, ItemKind::ShockOrb).is_empty());
}

#[test]
fn storm_strike_hits_only_cars_ahead_one_by_one() {
    let mut app = race(1);
    let cars: Vec<Entity> = (0..4)
        .map(|i| spawn_car(&mut app, straight(-12.0 + 6.0 * i as f32), 0.0))
        .collect();
    let (first, second, user, last) = (cars[0], cars[1], cars[2], cars[3]);
    app.world_mut().insert_resource(Standings(cars.clone()));
    step(&mut app, 10);
    fire(&mut app, user, ItemKind::StormStrike, false);
    let used = frame(&app);
    step_seconds(&mut app, 1.0);
    let strikes: Vec<(u32, Entity)> = log(&app)
        .hits
        .iter()
        .filter(|(_, h)| h.by == ItemKind::StormStrike)
        .map(|(f, h)| (*f, h.victim))
        .collect();
    assert_eq!(
        strikes.iter().map(|s| s.1).collect::<Vec<_>>(),
        vec![first, second]
    );
    assert_eq!(
        strikes[0].0 - used,
        15,
        "the first bolt 0.25 s after the use"
    );
    assert_eq!(strikes[1].0 - strikes[0].0, 15, "0.25 s apart");
    assert_eq!(effects(&app, second).power_mul(), 0.0);
    assert_eq!(effects(&app, last).power_mul(), 1.0);
    assert_eq!(effects(&app, user).power_mul(), 1.0);
}

/// Racers parked on the first row's cubes: the item each one gets, and the tick it came.
fn roulette_run(seed: u64) -> (Vec<ItemKind>, Vec<u64>) {
    let mut app = race(seed);
    let slots = track(&app).item_rows()[0].slots.clone();
    let cars: Vec<Entity> = slots[..4]
        .iter()
        .map(|&s| spawn_car(&mut app, s - Vec3::Y * 0.8, 0.0))
        .collect();
    app.world_mut().insert_resource(Standings(cars.clone()));
    let picked_at = app.world().resource::<ItemClock>().0 + 1;
    step(&mut app, 1);
    let picked = log(&app)
        .items
        .iter()
        .filter(|e| e.kind == ItemEventKind::Picked)
        .count();
    assert_eq!(picked, 4, "every racer takes a cube");
    let taken = all::<Pickup>(&mut app)
        .into_iter()
        .filter(|&p| app.world().get::<ItemTimer>(p).is_some_and(|t| t.0 > 0.0))
        .count();
    assert_eq!(taken, 4);
    let mut items = vec![None; 4];
    let mut ready_at = vec![0; 4];
    for _ in 0..secs_to_ticks(1.0) {
        step(&mut app, 1);
        for (i, &car) in cars.iter().enumerate() {
            let slot = app.world().get::<ItemSlot>(car).expect("racer");
            if items[i].is_none() && slot.item.is_some() {
                items[i] = slot.item;
                ready_at[i] = app.world().resource::<ItemClock>().0 - picked_at;
            }
        }
    }
    let odds = &app.world().resource::<ItemTuning>().odds;
    let items: Vec<ItemKind> = items.into_iter().map(|i| i.expect("item ready")).collect();
    for (rank, kind) in items.iter().enumerate() {
        let band = Band::of(rank + 1, 4);
        let k = ItemKind::ALL.iter().position(|x| x == kind).expect("kind");
        assert!(
            odds.weights(band)[k] > 0,
            "{kind:?} is not in the {band:?} row"
        );
    }
    // Held items are never replaced, and taken cubes come back after 2 s.
    step_seconds(&mut app, 1.0);
    assert_eq!(
        log(&app)
            .items
            .iter()
            .filter(|e| e.kind == ItemEventKind::Picked)
            .count(),
        4
    );
    let cubes = all::<Pickup>(&mut app);
    let respawning = cubes
        .iter()
        .filter(|&&p| app.world().get::<ItemTimer>(p).is_some_and(|t| t.0 > 0.0))
        .count();
    assert_eq!(respawning, 0, "all {} cubes are back", cubes.len());
    (items, ready_at)
}

#[test]
fn roulette_is_deterministic_for_a_seed() {
    let (a, ticks) = roulette_run(7);
    let (b, _) = roulette_run(7);
    assert_eq!(a, b, "same seed, same items");
    assert!(ticks.iter().all(|&t| t == 60), "1 s roulette: {ticks:?}");
}

#[test]
fn restart_clears_items_and_respawns_cubes() {
    let mut app = race(1);
    let cubes = all::<Pickup>(&mut app).len();
    let expected: usize = track(&app).item_rows().iter().map(|r| r.slots.len()).sum();
    assert_eq!(cubes, expected);
    assert!(cubes >= 16);
    let car = spawn_car(&mut app, straight(0.0), 0.0);
    step(&mut app, 5);
    fire(&mut app, car, ItemKind::SparkRocket, false);
    fire(&mut app, car, ItemKind::OilSlick, false);
    fire(&mut app, car, ItemKind::StormStrike, false);
    assert!(!all::<Projectile>(&mut app).is_empty() && !all::<Hazard>(&mut app).is_empty());
    restart(&mut app);
    assert!(all::<Projectile>(&mut app).is_empty());
    assert!(all::<Hazard>(&mut app).is_empty());
    assert_eq!(all::<Pickup>(&mut app).len(), expected);
    assert!(
        app.world().resource::<ItemClock>().0 <= 2,
        "the clock restarts"
    );
}

#[test]
fn rocket_fired_backward_locks_onto_a_car_behind() {
    let mut app = race(1);
    let shooter = spawn_car(&mut app, straight(-10.0), 0.0);
    let ahead = spawn_car(&mut app, straight(-30.0), 0.0);
    let behind = spawn_car(&mut app, straight(15.0), 0.0);
    step(&mut app, 20);
    fire(&mut app, shooter, ItemKind::SparkRocket, true);
    let lock = ItemEvent {
        racer: shooter,
        kind: ItemEventKind::LockOn { target: behind },
    };
    assert!(log(&app).items.contains(&lock), "{:?}", log(&app).items);
    step_until(&mut app, "the rocket hits", |w| {
        !w.resource::<Log>().hits.is_empty()
    });
    assert_eq!(hits_on(&app, behind, ItemKind::SparkRocket).len(), 1);
    assert!(hits_on(&app, ahead, ItemKind::SparkRocket).is_empty());
    let lost = ItemEvent {
        racer: shooter,
        kind: ItemEventKind::LockLost,
    };
    assert!(
        log(&app).items.contains(&lost),
        "the lock ends with the rocket"
    );
}

#[test]
fn shielded_cars_push_the_cars_they_bump() {
    let mut app = race(1);
    let bully = spawn_car(&mut app, straight(0.0), 0.0);
    let other = spawn_car(&mut app, Vec3::new(3.0, ROAD_Y, 0.0), 0.0);
    step(&mut app, 10);
    fire(&mut app, bully, ItemKind::ShieldBubble, false);
    step(&mut app, 1);
    set_velocity(&mut app, bully, Vec3::X * 3.0);
    step_until(&mut app, "the bump", |w| {
        w.get::<bevy_rapier3d::prelude::Velocity>(other)
            .is_some_and(|v| v.linear.x > 0.5)
    });
    step(&mut app, 3);
    let v = velocity(&app, other).linear;
    let push = app.world().resource::<ItemTuning>().shield_bubble.push;
    assert!(
        v.x > push * 0.8,
        "pushed away harder than a plain bump: {v}"
    );
    assert!(effects(&app, bully).shielded(), "bumping is not a hit");
    assert!(log(&app).hits.is_empty());
}

#[test]
fn leech_tether_breaks_out_of_range() {
    let mut app = race(1);
    let me = spawn_car(&mut app, straight(0.0), 0.0);
    let ahead = spawn_car(&mut app, straight(-15.0), 0.0);
    app.world_mut().insert_resource(Standings(vec![ahead, me]));
    step(&mut app, 10);
    app.world_mut().get_mut::<Charge>(ahead).expect("racer").0 = 0.9;
    fire(&mut app, me, ItemKind::LeechCoil, false);
    step_seconds(&mut app, 0.5);
    assert_eq!(all::<Projectile>(&mut app).len(), 1);
    // The target escapes beyond 45 u.
    app.world_mut()
        .get_mut::<Transform>(ahead)
        .expect("car")
        .translation = Vec3::new(44.0, ROAD_Y, 20.0);
    step(&mut app, 2);
    assert!(all::<Projectile>(&mut app).is_empty(), "the tether broke");
    let kept = app.world().get::<Charge>(ahead).expect("racer").0;
    step_seconds(&mut app, 0.5);
    assert_eq!(
        app.world().get::<Charge>(ahead).expect("racer").0,
        kept,
        "no more transfer"
    );
    assert!(kept > 0.8, "only half a second of transfer: {kept}");
}

/// Ticks `check` holds after `effect` is sent the way rc_race sends it.
fn ticks_active(
    app: &mut App,
    car: Entity,
    effect: Effect,
    check: fn(&StatusEffects) -> bool,
) -> u32 {
    app.world_mut().write_message(ApplyEffect {
        car,
        effect,
        source: None,
    });
    let mut ticks = 0;
    loop {
        step(app, 1);
        if !check(&effects(app, car)) {
            return ticks;
        }
        ticks += 1;
        assert!(ticks < 1000, "{effect:?} never ends");
    }
}

#[test]
fn race_effects_stack_by_the_rules_and_are_not_hits() {
    let mut app = race(1);
    let car = spawn_car(&mut app, straight(0.0), 0.0);
    step(&mut app, 5);
    // rc_race (T06): perfect start, stall, respawn ghosting — exact durations in ticks.
    let perfect = Effect::Boost {
        top_speed: 1.30,
        force: 1.50,
        secs: 1.0,
        freeze_charge: false,
    };
    assert_eq!(
        ticks_active(&mut app, car, perfect, |e| e.boost.is_some()),
        60
    );
    let stall = Effect::PowerLoss {
        mul: 0.0,
        secs: 0.6,
    };
    assert_eq!(
        ticks_active(&mut app, car, stall, |e| e.power_mul() == 0.0),
        36
    );
    let ghost = Effect::Ghost { secs: 1.5 };
    assert_eq!(
        ticks_active(&mut app, car, ghost, StatusEffects::ghosted),
        90
    );
    // A Turbo Cell during a perfect start: boosts never stack, the strongest wins.
    app.world_mut().write_message(ApplyEffect {
        car,
        effect: perfect,
        source: None,
    });
    step(&mut app, 10);
    fire(&mut app, car, ItemKind::TurboCell, false);
    step(&mut app, 1);
    let b = effects(&app, car).boost.expect("boosting");
    assert_eq!((b.top_speed, b.force, b.freeze_charge), (1.45, 1.80, true));
    assert!((b.remaining - 2.5).abs() < 0.02, "{}", b.remaining);
    // A shield never meets race effects: no hit, no pop.
    fire(&mut app, car, ItemKind::ShieldBubble, false);
    step(&mut app, 1);
    for effect in [stall, ghost] {
        app.world_mut().write_message(ApplyEffect {
            car,
            effect,
            source: None,
        });
    }
    step(&mut app, 2);
    let e = effects(&app, car);
    assert!(e.shielded() && e.ghosted() && e.power_mul() == 0.0);
    assert!(log(&app).hits.is_empty());
}
