//! Status effects on the car body (GDD §5.5): the `paint` / `paint_accent` materials of every car
//! model are [`CarBodyMaterial`]s (`assets/shaders/render/car_status.wesl`), so effects show on the
//! car itself instead of on meshes attached to it:
//! - **Shield** — a cyan energy rim with bands rising over the body; flares when it blocks a hit,
//!   blinks before it expires;
//! - **charge boost** — a fainter electric rim while the racer spends charge;
//! - **power loss** (EMP, Storm Strike) — flickering electric veins;
//! - **grip loss** (Oil Slick) — darker, glossier paint with a rainbow oil film.

use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use rc_core::prelude::*;

use super::{CarParts, DrivenBy};

/// Shield rim colour (linear HDR) and its blink during the last seconds.
const SHIELD_RIM: Vec3 = Vec3::new(2.5, 8.0, 15.0);
const SHIELD_WARN_S: f32 = 2.0;
const SHIELD_BLINK_DIM: f32 = 0.35;
/// Rim while spending charge (linear HDR).
const CHARGE_RIM: Vec3 = Vec3::new(1.2, 4.5, 9.0);
/// Band shimmer of the shield / charge rims (0–1).
const SHIELD_BANDS: f32 = 1.0;
const CHARGE_BANDS: f32 = 0.5;
/// A blocked hit's flare decays at this rate (1/s).
const FLASH_DECAY: f32 = 3.0;
/// Grip multiplier of a full oil film (Oil Slick).
const OIL_GRIP: f32 = 0.2;
/// *Reduce flashing*: hit flare and crackle strength.
const CALM_FLARE: f32 = 0.3;
const CALM_CRACKLE: f32 = 0.5;

/// Car paint with status effects.
pub type CarBodyMaterial = ExtendedMaterial<StandardMaterial, StatusExtension>;

/// Shader parameters of a [`CarBodyMaterial`] (all zero: plain paint).
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone, Copy, Default, PartialEq)]
pub struct StatusExtension {
    /// rgb: energy rim colour (linear HDR, 0 = none), w: band shimmer 0–1.
    #[uniform(100)]
    pub rim: Vec4,
    /// x: electric crackle 0–1, y: oil film 0–1, z: hit flare 0–1.
    #[uniform(101)]
    pub status: Vec4,
}

impl MaterialExtension for StatusExtension {
    fn fragment_shader() -> ShaderRef {
        "shaders/render/car_status.wesl".into()
    }
}

/// A car model's blocked-hit flare (0–1).
#[derive(Component, Debug, Clone, Copy, Default)]
pub(super) struct Flare(f32);

pub(super) fn build(app: &mut App) {
    app.add_plugins(MaterialPlugin::<CarBodyMaterial>::default());
}

/// The body look of a racer with `effects`, spending charge or not, a hit flare of `flash` and
/// `now` seconds on the clock (shield blink); `calm` = *Reduce flashing*.
pub(crate) fn status_look(
    effects: &StatusEffects,
    charging: bool,
    flash: f32,
    now: f32,
    calm: bool,
) -> StatusExtension {
    let rim = match effects.shield {
        Some(left) => {
            let blink = if left < SHIELD_WARN_S && (now * 8.0).sin() < 0.0 {
                SHIELD_BLINK_DIM
            } else {
                1.0
            };
            (SHIELD_RIM * blink).extend(SHIELD_BANDS)
        }
        None if charging => CHARGE_RIM.extend(CHARGE_BANDS),
        None => Vec4::ZERO,
    };
    let (flare, crackle) = if calm {
        (CALM_FLARE, CALM_CRACKLE)
    } else {
        (1.0, 1.0)
    };
    let oil = ((1.0 - effects.grip_mul()) / (1.0 - OIL_GRIP)).clamp(0.0, 1.0);
    StatusExtension {
        rim,
        status: Vec4::new(
            (1.0 - effects.power_mul()).clamp(0.0, 1.0) * crackle,
            oil,
            flash.clamp(0.0, 1.0) * flare,
            0.0,
        ),
    }
}

/// `VisualSet`: every racer's body materials follow its status effects.
#[allow(clippy::type_complexity)]
pub(super) fn shade(
    time: Res<Time>,
    settings: Option<Res<Settings>>,
    mut hits: MessageReader<HitEvent>,
    mut models: Query<(&DrivenBy, &CarParts, &mut Flare)>,
    racers: Query<(&StatusEffects, Option<&VehicleTelemetry>)>,
    mut materials: ResMut<Assets<CarBodyMaterial>>,
) {
    let blocked: Vec<Entity> = hits
        .read()
        .filter(|h| h.blocked)
        .map(|h| h.victim)
        .collect();
    let (dt, now) = (time.delta_secs(), time.elapsed_secs());
    let calm = settings.is_some_and(|s| s.accessibility.reduce_flashing);
    for (driven, parts, mut flare) in &mut models {
        let Ok((effects, telemetry)) = racers.get(driven.0) else {
            continue;
        };
        flare.0 = if blocked.contains(&driven.0) {
            1.0
        } else {
            (flare.0 - dt * FLASH_DECAY).max(0.0)
        };
        let charging = telemetry.is_some_and(|t| t.charge_boost);
        let look = status_look(effects, charging, flare.0, now, calm);
        for handle in &parts.body {
            if materials.get(handle).is_some_and(|m| m.extension != look)
                && let Some(mut m) = materials.get_mut(handle)
            {
                m.extension = look;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_racers_keep_plain_paint() {
        let look = status_look(&StatusEffects::default(), false, 0.0, 0.0, false);
        assert_eq!(look, StatusExtension::default());
    }

    #[test]
    fn effects_show_on_the_body() {
        let shielded = StatusEffects {
            shield: Some(5.0),
            ..default()
        };
        let look = status_look(&shielded, true, 0.0, 0.0, false);
        assert_eq!(
            look.rim,
            SHIELD_RIM.extend(SHIELD_BANDS),
            "shield wins over charge"
        );
        let charging = status_look(&StatusEffects::default(), true, 0.0, 0.0, false);
        assert_eq!(charging.rim, CHARGE_RIM.extend(CHARGE_BANDS));

        let stunned = StatusEffects {
            power_loss: Some(TimedMul {
                mul: 0.0,
                remaining: 1.0,
            }),
            grip: Some(TimedMul {
                mul: OIL_GRIP,
                remaining: 1.0,
            }),
            ..default()
        };
        let look = status_look(&stunned, false, 2.0, 0.0, false);
        assert_eq!(look.status, Vec4::new(1.0, 1.0, 1.0, 0.0));
        let calm = status_look(&stunned, false, 2.0, 0.0, true);
        assert!(
            calm.status.x < 1.0 && calm.status.z < 1.0,
            "reduce flashing: softer"
        );
    }

    #[test]
    fn an_expiring_shield_blinks() {
        let ending = StatusEffects {
            shield: Some(1.0),
            ..default()
        };
        let lit = status_look(&ending, false, 0.0, 0.1, false).rim;
        let dim = status_look(&ending, false, 0.0, 0.5, false).rim;
        assert!(dim.x < lit.x, "{dim} vs {lit}");
    }
}
