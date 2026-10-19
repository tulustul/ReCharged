//! Volt Cubes (GDD §6.1): spawned per attempt from the track's item rows; a racer holding
//! nothing that drives through one starts a 1 s roulette (the cube respawns after 2 s); the
//! roulette ends with an item drawn from the odds row of the racer's race position (GDD §6.3)
//! with the keyed RNG. Item use turns the `CarControl::use_item` edge into [`UseItem`].

use bevy::prelude::*;
use rc_core::prelude::*;
use rc_track::Track;

use crate::data::ItemTuning;
use crate::effects::{ItemClock, run_down};
use crate::hits::centre;
use crate::odds::Band;

/// A racer used its item this tick (rc_items-internal; the item systems spawn what it does).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct UseItem {
    pub racer: Entity,
    pub kind: ItemKind,
    /// Look back was held: fire backward where the item allows it.
    pub backward: bool,
}

/// `OnEnter(RaceAttempt)`: one cube per slot of every item row (when items are on).
pub(crate) fn spawn_pickups(
    mut commands: Commands,
    config: Option<Res<RaceConfig>>,
    track: Option<Res<Track>>,
) {
    if !config.is_some_and(|c| c.items) {
        return;
    }
    let Some(track) = track else {
        warn!("items: no Track when the attempt started; no Volt Cubes");
        return;
    };
    let cubes: usize = track.item_rows().iter().map(|r| r.slots.len()).sum();
    info!(
        "items: {cubes} Volt Cubes in {} rows",
        track.item_rows().len()
    );
    for (row, r) in track.item_rows().iter().enumerate() {
        for (slot, &pos) in r.slots.iter().enumerate() {
            commands.spawn((
                Name::new("volt cube"),
                Pickup {
                    row: row as u8,
                    slot: slot as u8,
                },
                ItemTimer(0.0),
                Transform::from_translation(pos),
                DespawnOnExit(RaceAttempt),
            ));
        }
    }
}

/// The racer holds an item or is spinning the roulette.
fn holding(slot: &ItemSlot) -> bool {
    slot.item.is_some() || slot.roulette > 0.0
}

/// Taken cubes count their respawn down; available ones are taken by the first racer inside
/// that holds nothing (a held item is never replaced, and such racers leave the cube).
pub(crate) fn take_pickups(
    tuning: Res<ItemTuning>,
    mut cubes: Query<(&Transform, &mut ItemTimer), With<Pickup>>,
    mut racers: Query<(Entity, &Transform, &RaceProgress, &mut ItemSlot), With<Racer>>,
    mut events: MessageWriter<ItemEvent>,
) {
    let p = &tuning.pickups;
    for (cube, mut timer) in &mut cubes {
        if timer.0 > 0.0 && run_down(&mut timer.0) {
            continue;
        }
        for (racer, t, progress, mut slot) in &mut racers {
            if progress.finished
                || holding(&slot)
                || centre(t).distance(cube.translation) > p.radius
            {
                continue;
            }
            *slot = ItemSlot {
                item: None,
                roulette: p.roulette_secs,
            };
            timer.0 = p.respawn_secs;
            events.write(ItemEvent {
                racer,
                kind: ItemEventKind::Picked,
            });
            break;
        }
    }
}

/// Band of `racer` from `Standings` (Middle when it is not ranked yet).
fn band_of(racer: Entity, standings: Option<&Standings>) -> Band {
    standings
        .and_then(|s| {
            let i = s.0.iter().position(|&e| e == racer)?;
            Some(Band::of(i + 1, s.0.len()))
        })
        .unwrap_or(Band::Middle)
}

/// Roulettes count down; at 0 the item is drawn for the racer's position right now.
pub(crate) fn roulette(
    tuning: Res<ItemTuning>,
    clock: Res<ItemClock>,
    config: Option<Res<RaceConfig>>,
    standings: Option<Res<Standings>>,
    mut racers: Query<(Entity, &Racer, &mut ItemSlot)>,
    mut events: MessageWriter<ItemEvent>,
) {
    let seed = config.map_or(0, |c| c.seed);
    for (entity, racer, mut slot) in &mut racers {
        if slot.item.is_some() || slot.roulette <= 0.0 || run_down(&mut slot.roulette) {
            continue;
        }
        let band = band_of(entity, standings.as_deref());
        slot.item = tuning.odds.roll(band, seed, clock.0, racer.index);
        if let Some(kind) = slot.item {
            debug!("items: {} got {kind:?} ({band:?})", racer.name);
            events.write(ItemEvent {
                racer: entity,
                kind: ItemEventKind::Ready(kind),
            });
        }
    }
}

/// `CarControl::use_item` with a ready item: the slot empties, `ItemEvent::Used` and
/// [`UseItem`] go out (racers who finished keep theirs).
pub(crate) fn use_items(
    status: Option<Res<RaceStatus>>,
    mut racers: Query<(Entity, &CarControl, &RaceProgress, &mut ItemSlot)>,
    mut events: MessageWriter<ItemEvent>,
    mut uses: MessageWriter<UseItem>,
) {
    let race_secs = status.map_or(0.0, |s| ticks_to_secs(s.race_ticks));
    for (racer, control, progress, mut slot) in &mut racers {
        let Some(kind) = slot.item else { continue };
        if !control.use_item || progress.finished || slot.roulette > 0.0 {
            continue;
        }
        slot.item = None;
        debug!("items: {racer} used {kind:?} at {race_secs:.1} s");
        events.write(ItemEvent {
            racer,
            kind: ItemEventKind::Used(kind),
        });
        uses.write(UseItem {
            racer,
            kind,
            backward: control.look_back,
        });
    }
}

/// Turbo Cell and Shield Bubble act on the user.
pub(crate) fn self_items(
    tuning: Res<ItemTuning>,
    mut uses: MessageReader<UseItem>,
    mut hits: crate::hits::Hits,
) {
    for u in uses.read() {
        let effect = match u.kind {
            ItemKind::TurboCell => Effect::Boost {
                top_speed: tuning.turbo_cell.top_speed,
                force: tuning.turbo_cell.force,
                secs: tuning.turbo_cell.secs,
                freeze_charge: true,
            },
            ItemKind::ShieldBubble => Effect::Shield {
                secs: tuning.shield_bubble.secs,
            },
            _ => continue,
        };
        hits.effect(u.racer, effect, u.racer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn band_comes_from_standings_or_defaults_to_middle() {
        let e = |i| Entity::from_raw_u32(i).expect("entity");
        let standings = Standings((1..=8).map(e).collect());
        assert_eq!(band_of(e(1), Some(&standings)), Band::First);
        assert_eq!(band_of(e(3), Some(&standings)), Band::Top);
        assert_eq!(band_of(e(5), Some(&standings)), Band::Middle);
        assert_eq!(band_of(e(6), Some(&standings)), Band::Last);
        assert_eq!(band_of(e(9), Some(&standings)), Band::Middle);
        assert_eq!(band_of(e(1), None), Band::Middle);
    }
}
