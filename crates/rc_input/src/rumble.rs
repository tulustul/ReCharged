//! Rumble (GDD §12.1, §15): impacts, item hits, landings and boost start shake the pads of the
//! local player concerned, when that player is on a pad. Setting: vibration on/off + intensity.

use std::time::Duration;

use bevy::input::gamepad::{GamepadRumbleIntensity, GamepadRumbleRequest};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use rc_core::prelude::*;

use crate::players::Roster;

/// One rumble: strong and weak motor 0–1 (before the intensity setting) and duration, ms.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pulse {
    /// Low-frequency motor.
    pub strong: f32,
    /// High-frequency motor.
    pub weak: f32,
    /// Length in milliseconds.
    pub ms: u64,
}

/// Landings after shorter air time do not rumble, s.
const LANDING_MIN_AIR_S: f32 = 0.3;

/// A car impact of `strength` 0–1.
pub fn impact_pulse(strength: f32) -> Pulse {
    let s = strength.clamp(0.0, 1.0);
    Pulse {
        strong: 0.8 * s,
        weak: 0.4 * s,
        ms: 180,
    }
}

/// Hit by an item (a blocked hit is a short buzz).
pub fn hit_pulse(blocked: bool) -> Pulse {
    if blocked {
        Pulse {
            strong: 0.0,
            weak: 0.4,
            ms: 150,
        }
    } else {
        Pulse {
            strong: 0.9,
            weak: 0.5,
            ms: 320,
        }
    }
}

/// A landing (`None` for short hops).
pub fn landing_pulse(landing: &CarLanded) -> Option<Pulse> {
    let s = landing.strength.clamp(0.0, 1.0);
    (landing.air_time >= LANDING_MIN_AIR_S).then_some(Pulse {
        strong: 0.6 * s,
        weak: 0.3 * s,
        ms: 140,
    })
}

/// Boost kicking in.
pub const BOOST_PULSE: Pulse = Pulse {
    strong: 0.15,
    weak: 0.5,
    ms: 220,
};

/// Update: rumble the pads of the players whose cars were involved.
#[allow(clippy::too_many_arguments)]
pub(crate) fn rumble(
    settings: Option<Res<Settings>>,
    roster: Res<Roster>,
    locals: Query<&LocalPlayer>,
    boosting: Query<(Entity, &LocalPlayer, &VehicleTelemetry)>,
    (mut impacts, mut hits, mut landings): (
        MessageReader<CarImpact>,
        MessageReader<HitEvent>,
        MessageReader<CarLanded>,
    ),
    mut was_boosting: Local<HashMap<Entity, bool>>,
    mut out: MessageWriter<GamepadRumbleRequest>,
) {
    let (on, intensity) = settings.map_or((true, 1.0), |s| {
        (
            s.controls.vibration,
            s.controls.vibration_intensity.clamp(0.0, 1.0),
        )
    });
    let mut pulses: Vec<(PlayerSlot, Pulse)> = Vec::new();
    pulses.extend(
        impacts
            .read()
            .filter_map(|i| Some((locals.get(i.car).ok()?.0, impact_pulse(i.strength)))),
    );
    pulses.extend(
        hits.read()
            .filter_map(|h| Some((locals.get(h.victim).ok()?.0, hit_pulse(h.blocked)))),
    );
    pulses.extend(
        landings
            .read()
            .filter_map(|l| Some((locals.get(l.car).ok()?.0, landing_pulse(l)?))),
    );
    for (entity, local, telemetry) in &boosting {
        let before = was_boosting
            .insert(entity, telemetry.boosting)
            .unwrap_or(false);
        if telemetry.boosting && !before {
            pulses.push((local.0, BOOST_PULSE));
        }
    }
    was_boosting.retain(|e, _| boosting.contains(*e));
    if !on || intensity <= 0.0 {
        return;
    }
    for (slot, pulse) in pulses {
        if roster.last_used(slot) != Some(DeviceKind::Gamepad) {
            continue; // playing on the keyboard: leave the pad on the desk alone
        }
        for device in roster.devices() {
            if let InputDevice::Gamepad(gamepad) = device
                && roster.slot_of(device) == Some(slot)
            {
                out.write(GamepadRumbleRequest::Add {
                    gamepad,
                    duration: Duration::from_millis(pulse.ms),
                    intensity: GamepadRumbleIntensity {
                        strong_motor: pulse.strong * intensity,
                        weak_motor: pulse.weak * intensity,
                    },
                });
            }
        }
    }
}

/// `OnEnter(PauseState::Paused)` and session end: silence every pad.
pub(crate) fn stop_all(roster: Res<Roster>, mut out: MessageWriter<GamepadRumbleRequest>) {
    for device in roster.devices() {
        if let InputDevice::Gamepad(gamepad) = device {
            out.write(GamepadRumbleRequest::Stop { gamepad });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pulses_scale_with_strength_and_skip_short_hops() {
        assert_eq!(impact_pulse(0.0).strong, 0.0);
        assert!(impact_pulse(1.0).strong > impact_pulse(0.3).strong);
        assert_eq!(impact_pulse(7.0), impact_pulse(1.0));
        assert!(hit_pulse(false).strong > hit_pulse(true).strong);
        let land = |air_time| CarLanded {
            car: Entity::PLACEHOLDER,
            air_time,
            strength: 1.0,
            clean: true,
        };
        assert_eq!(landing_pulse(&land(0.1)), None);
        assert!(landing_pulse(&land(0.9)).is_some_and(|p| p.strong > 0.0));
    }
}
