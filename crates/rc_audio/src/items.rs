//! Item sounds: pickups and the roulette (local players hear the ticks and the lock), item use,
//! hits and explosions, the lock-on warning (a beep while a rocket is locked on a local player),
//! goo-fuse beeps speeding up toward the blast, and loops riding on projectiles.

use std::collections::HashMap;

use bevy::prelude::*;
use bevy_kira_audio::prelude::*;
use rc_core::prelude::*;

use crate::Step;
use crate::bank::Bank;
use crate::buses::{SfxBus, tween_ms};
use crate::output::audio_on;
use crate::shots::{Pending, Shots};
use crate::spatial::{Ears, REACH_CAR, REACH_LOUD, REACH_SMALL, spot};

/// Another racer's pickup, dB.
const OTHER_PICKUP_DB: f32 = -4.0;
/// Roulette: duration (GDD §6.1) and tick count; ticks slow down like a wheel.
const ROULETTE_S: f64 = 1.0;
const ROULETTE_TICKS: u32 = 12;
const ROULETTE_EASE: f64 = 1.6;
/// Shield pop follows the shield hit by this, s.
const SHIELD_POP_DELAY_S: f64 = 0.08;
/// Lock-on warning beep period, s; locks expire after `LOCK_MAX_S`, or after `LOCK_GRACE_S`
/// when the shooter has no rocket in flight.
const LOCK_BEEP_S: f64 = 0.22;
const LOCK_MAX_S: f64 = 6.0;
const LOCK_GRACE_S: f64 = 0.3;
/// Goo fuse (GDD §6.2) and its beep period from lit to blast, s.
const GOO_FUSE_S: f64 = 6.0;
const GOO_BEEP_S: (f64, f64) = (1.0, 0.12);
/// Projectile loops: parameter tween and fade-out when the projectile is gone, ms.
const LOOP_TWEEN_MS: u64 = 60;
const LOOP_STOP_MS: u64 = 100;

/// Sound of using an item (Turbo Cell's whoosh comes from the boost edge in `cars.rs`; Storm
/// Strike sounds on every hit).
pub(crate) fn used_cue(kind: ItemKind) -> Option<&'static str> {
    match kind {
        ItemKind::SparkRocket => Some("rocket_launch"),
        ItemKind::LeechCoil => Some("leech_zap"),
        ItemKind::ShockOrb => Some("orb_launch"),
        ItemKind::EmpPulse => Some("emp_charge"),
        ItemKind::GooBomb | ItemKind::SpringPad => Some("item_drop"),
        ItemKind::OilSlick => Some("oil_splat"),
        ItemKind::ShieldBubble => Some("shield_up"),
        ItemKind::TurboCell | ItemKind::StormStrike => None,
    }
}

/// Sound of an unblocked hit (rockets, goo and EMP sound through their `Explosion`).
pub(crate) fn hit_cue(by: ItemKind) -> Option<&'static str> {
    match by {
        ItemKind::ShockOrb => Some("orb_zap"),
        ItemKind::OilSlick => Some("oil_slip"),
        ItemKind::SpringPad => Some("spring_boing"),
        ItemKind::LeechCoil => Some("leech_zap"),
        ItemKind::StormStrike => Some("storm_strike"),
        _ => None,
    }
}

/// Sound of an `Explosion`.
pub(crate) fn explosion_cue(kind: ItemKind) -> &'static str {
    match kind {
        ItemKind::GooBomb => "goo_explosion",
        ItemKind::EmpPulse => "emp_blast",
        _ => "rocket_explosion",
    }
}

/// Loop riding on a projectile.
pub(crate) fn projectile_loop(kind: ItemKind) -> Option<&'static str> {
    match kind {
        ItemKind::SparkRocket => Some("rocket_flight_loop"),
        ItemKind::ShockOrb => Some("orb_hum_loop"),
        ItemKind::LeechCoil => Some("leech_hum_loop"),
        _ => None,
    }
}

/// Seconds until the next goo beep, `elapsed` s after the fuse was lit.
pub(crate) fn goo_interval(elapsed: f64) -> f64 {
    let t = (elapsed / GOO_FUSE_S).clamp(0.0, 1.0);
    GOO_BEEP_S.0 + (GOO_BEEP_S.1 - GOO_BEEP_S.0) * t
}

/// Roulette tick offsets from the pickup, s (dense first, slowing down).
pub(crate) fn roulette_ticks() -> impl Iterator<Item = f64> {
    (0..ROULETTE_TICKS)
        .map(|k| ROULETTE_S * (f64::from(k) / f64::from(ROULETTE_TICKS)).powf(ROULETTE_EASE))
}

/// Rocket locks by shooter: (target, since).
#[derive(Resource, Default)]
struct Locks {
    by_shooter: HashMap<Entity, (Entity, f64)>,
    next_beep: f64,
}

/// Lit goo fuses by hazard entity: (lit at, next beep at).
#[derive(Resource, Default)]
struct Fuses(HashMap<Entity, (f64, f64)>);

/// Loops riding on projectiles.
#[derive(Resource, Default)]
struct ItemLoops(HashMap<Entity, Handle<AudioInstance>>);

pub(crate) fn build(app: &mut App) {
    app.init_resource::<Locks>()
        .init_resource::<Fuses>()
        .init_resource::<ItemLoops>()
        .add_systems(
            Update,
            (item_shots, lock_beeps, goo_beeps, item_loops)
                .chain()
                .in_set(Step::Items)
                .run_if(audio_on),
        )
        .add_systems(OnEnter(RacePhase::Setup), clear_items)
        .add_systems(OnEnter(AppState::Unloading), clear_items);
}

fn clear_items(mut locks: ResMut<Locks>, mut fuses: ResMut<Fuses>) {
    locks.by_shooter.clear();
    fuses.0.clear();
}

fn item_shots(
    mut items: MessageReader<ItemEvent>,
    mut hits: MessageReader<HitEvent>,
    mut blasts: MessageReader<Explosion>,
    cars: Query<(&Transform, Has<LocalPlayer>)>,
    mut pending: ResMut<Pending>,
    mut locks: ResMut<Locks>,
    mut shots: Shots,
) {
    let now = shots.now();
    for e in items.read() {
        let (pos, local) = cars
            .get(e.racer)
            .map_or((None, false), |(t, l)| (Some(t.translation), l));
        match e.kind {
            ItemEventKind::Picked if local => {
                shots.play("pickup_grab", 0.0, None);
                for t in roulette_ticks().skip(1) {
                    pending.push(now + t, "roulette_tick", 0.0, None);
                }
            }
            ItemEventKind::Picked => {
                shots.play_reach("pickup_grab", OTHER_PICKUP_DB, pos, REACH_SMALL);
            }
            ItemEventKind::Ready(_) if local => shots.play("roulette_lock", 0.0, None),
            ItemEventKind::Ready(_) => {}
            ItemEventKind::Used(kind) => {
                if let Some(stem) = used_cue(kind) {
                    shots.play(stem, 0.0, pos);
                }
            }
            ItemEventKind::LockOn { target } => {
                locks.by_shooter.insert(e.racer, (target, now));
            }
            ItemEventKind::LockLost => {
                locks.by_shooter.remove(&e.racer);
            }
        }
    }
    for h in hits.read() {
        let pos = cars.get(h.victim).ok().map(|(t, _)| t.translation);
        if h.blocked {
            shots.play("shield_hit", 0.0, pos);
            pending.push(now + SHIELD_POP_DELAY_S, "shield_pop", 0.0, pos);
        } else if let Some(stem) = hit_cue(h.by) {
            shots.play(stem, 0.0, pos);
        }
    }
    for b in blasts.read() {
        shots.play_reach(explosion_cue(b.kind), 0.0, Some(b.position), REACH_LOUD);
    }
}

fn lock_beeps(
    mut locks: ResMut<Locks>,
    rockets: Query<&Projectile>,
    locals: Query<(), With<LocalPlayer>>,
    mut shots: Shots,
) {
    let now = shots.now();
    locks.by_shooter.retain(|&shooter, &mut (_, since)| {
        let flying = rockets
            .iter()
            .any(|p| p.kind == ItemKind::SparkRocket && p.owner == shooter);
        now - since < LOCK_MAX_S && (flying || now - since < LOCK_GRACE_S)
    });
    let warn = locks
        .by_shooter
        .values()
        .any(|&(target, _)| locals.contains(target));
    if warn && now >= locks.next_beep {
        shots.play("lockon_beep", 0.0, None);
        locks.next_beep = now + LOCK_BEEP_S;
    }
}

fn goo_beeps(
    hazards: Query<(Entity, &Hazard, &GlobalTransform)>,
    mut fuses: ResMut<Fuses>,
    mut shots: Shots,
) {
    let now = shots.now();
    fuses.0.retain(|&e, _| hazards.contains(e));
    for (entity, hazard, at) in &hazards {
        if hazard.kind != HazardKind::GooBomb {
            continue;
        }
        let (lit, next) = fuses.0.entry(entity).or_insert((now, now));
        if now >= *next {
            shots.play_reach("goo_beep", 0.0, Some(at.translation()), REACH_SMALL);
            *next = now + goo_interval(now - *lit);
        }
    }
}

fn item_loops(
    projectiles: Query<(Entity, &Projectile, &GlobalTransform)>,
    bank: Res<Bank>,
    ears: Res<Ears>,
    sfx: Res<AudioChannel<SfxBus>>,
    mut loops: ResMut<ItemLoops>,
    mut instances: ResMut<Assets<AudioInstance>>,
) {
    loops.0.retain(|&e, h| {
        let alive = projectiles.contains(e);
        if !alive && let Some(mut i) = instances.get_mut(h) {
            i.stop(tween_ms(LOOP_STOP_MS));
        }
        alive
    });
    for (entity, projectile, at) in &projectiles {
        let Some(stem) = projectile_loop(projectile.kind) else {
            continue;
        };
        let place = spot(&ears.0, at.translation(), REACH_CAR);
        if let Some(h) = loops.0.get(&entity) {
            if let Some(mut i) = instances.get_mut(h) {
                i.set_decibels(Decibels(place.db), tween_ms(LOOP_TWEEN_MS));
                i.set_panning(place.pan, tween_ms(LOOP_TWEEN_MS));
            }
        } else if let Some(source) = bank.sfx(stem) {
            let h = sfx
                .play(source)
                .looped()
                .with_volume(Decibels(place.db))
                .with_panning(place.pan)
                .handle();
            loops.0.insert(entity, h);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn every_item_maps_to_existing_sfx() {
        let known: HashSet<&str> = crate::bank::SFX.into_iter().collect();
        for kind in ItemKind::ALL {
            let cues = [
                used_cue(kind),
                hit_cue(kind),
                Some(explosion_cue(kind)),
                projectile_loop(kind),
            ];
            for stem in cues.into_iter().flatten() {
                assert!(known.contains(stem), "{kind:?} → {stem} is not a file");
            }
        }
        assert_eq!(
            used_cue(ItemKind::TurboCell),
            None,
            "the boost edge plays the whoosh"
        );
        assert_eq!(explosion_cue(ItemKind::EmpPulse), "emp_blast");
        assert_eq!(
            hit_cue(ItemKind::SparkRocket),
            None,
            "its explosion sounds instead"
        );
    }

    #[test]
    fn roulette_slows_down_and_goo_beeps_speed_up() {
        let ticks: Vec<f64> = roulette_ticks().collect();
        assert_eq!(ticks.len(), ROULETTE_TICKS as usize);
        assert_eq!(ticks[0], 0.0);
        assert!(
            ticks.windows(3).all(|w| w[2] - w[1] > w[1] - w[0]),
            "gaps grow"
        );
        assert!(*ticks.last().unwrap() < ROULETTE_S);
        assert_eq!(goo_interval(0.0), 1.0);
        assert_eq!(goo_interval(GOO_FUSE_S), 0.12);
        assert!(goo_interval(3.0) < goo_interval(1.0));
    }
}
