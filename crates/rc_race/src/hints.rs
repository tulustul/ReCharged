//! Onboarding hints (GDD §13): each kind once, for a human-driven local racer, only with
//! `Settings.gameplay.hints` on and not yet in rc_game's `HintsSeen`; a session remembers what it
//! already sent (rc_game adds shown hints to `HintsSeen` for good).

use bevy::prelude::*;
use rc_core::prelude::*;

use crate::recovery::{Recovery, STUCK_HINT_TICKS};

/// The charge boost can start from this charge (GDD §5.3).
const BOOST_READY_CHARGE: f32 = 0.15;

/// Hints sent in this session (removed in `OnEnter(Unloading)`).
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct HintsShown(Vec<HintKind>);

/// The hints a racer's state calls for.
pub(crate) fn triggers(
    telemetry: &VehicleTelemetry,
    charge: f32,
    item: &ItemSlot,
    wrong_way: bool,
    stuck_ticks: u32,
) -> impl Iterator<Item = HintKind> {
    [
        (
            telemetry.drifting && telemetry.charge_rate > 0.0,
            HintKind::DriftCharge,
        ),
        (charge >= BOOST_READY_CHARGE, HintKind::BoostReady),
        (
            item.item.is_some() && item.roulette <= 0.0,
            HintKind::ItemReady,
        ),
        (stuck_ticks >= STUCK_HINT_TICKS, HintKind::HoldReset),
        (wrong_way, HintKind::WrongWay),
    ]
    .into_iter()
    .filter_map(|(on, kind)| on.then_some(kind))
}

type HintItem<'a> = (
    Entity,
    &'a VehicleTelemetry,
    &'a Charge,
    &'a ItemSlot,
    &'a RaceProgress,
    &'a Recovery,
);

/// `SimSet::Race` (Racing): `RaceEvent::Hint` for the racer that triggered it.
pub(crate) fn emit_hints(
    settings: Option<Res<Settings>>,
    seen: Option<Res<HintsSeen>>,
    mut shown: ResMut<HintsShown>,
    racers: Query<HintItem, (With<LocalPlayer>, Without<AiDriver>)>,
    mut events: MessageWriter<RaceEvent>,
) {
    if settings.is_some_and(|s| !s.gameplay.hints) {
        return;
    }
    let seen = seen.as_deref().map_or(&[][..], |s| &s.0[..]);
    for (car, telemetry, charge, item, progress, recovery) in &racers {
        let stuck = recovery.stuck;
        for kind in triggers(telemetry, charge.0, item, progress.wrong_way, stuck) {
            if shown.0.contains(&kind) || seen.contains(&kind) {
                continue;
            }
            shown.0.push(kind);
            events.write(RaceEvent {
                racer: Some(car),
                kind: RaceEventKind::Hint(kind),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_trigger_maps_to_its_hint() {
        let calm = VehicleTelemetry::default();
        let none = ItemSlot::default();
        assert_eq!(triggers(&calm, 0.1, &none, false, 0).count(), 0);
        let drifting = VehicleTelemetry {
            drifting: true,
            charge_rate: 0.2,
            ..calm
        };
        let ready = ItemSlot {
            item: Some(ItemKind::TurboCell),
            roulette: 0.0,
        };
        let rolling = ItemSlot {
            roulette: 0.5,
            ..ready
        };
        let all: Vec<HintKind> = triggers(&drifting, 0.15, &ready, true, 180).collect();
        use HintKind::*;
        assert_eq!(
            all,
            [DriftCharge, BoostReady, ItemReady, HoldReset, WrongWay]
        );
        assert_eq!(triggers(&calm, 0.0, &rolling, false, 179).count(), 0);
    }
}
