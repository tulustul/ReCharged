//! The UI's own WESL effects, embedded in the binary (`embedded://rc_ui/shaders/…`) and typed
//! into `bevy.ts` like every other binding:
//! - `chargeSwap` (morph): the Main menu → mode transition, a crackling charge front sweeping
//!   the old screen away (GDD §14: the glitch/charge morph only there; plain fades elsewhere).

use bevy::prelude::*;
use bevy_react::filters::FilterColor;
use bevy_react::prelude::*;
use bevy_react::protocol::units::{Angle, Length};

/// Packing: `params[0]` = (angle, width, split), `params[1]` = tint, `params[2]` = core (see the
/// shader header).
#[react_morph_filter(shader = "embedded://rc_ui/shaders/charge_swap.wesl")]
struct ChargeSwap {
    /// Sweep direction, degrees clockwise from +X (screen space, y down).
    #[serde(default = "default_angle")]
    angle: Angle,
    /// Glow fall-off of the front, px.
    #[serde(default = "default_width")]
    width: Length,
    /// Channel split next to the front, px.
    #[serde(default = "default_split")]
    split: Length,
    /// Glow colour (alpha = strength).
    #[serde(default = "cyan")]
    tint: FilterColor,
    /// Colour of the hot front line (alpha = strength).
    #[serde(default = "volt")]
    core: FilterColor,
}

fn default_angle() -> Angle {
    Angle::from_radians(12f32.to_radians())
}

fn default_width() -> Length {
    Length::Px(26.0)
}

fn default_split() -> Length {
    Length::Px(9.0)
}

/// Electric cyan, linear RGB.
fn cyan() -> FilterColor {
    FilterColor([0.01, 0.69, 1.0, 0.85])
}

/// Volt yellow, linear RGB.
fn volt() -> FilterColor {
    FilterColor([0.89, 0.75, 0.005, 1.0])
}

pub(crate) fn register(app: &mut App) {
    app.add_react_morph_filter::<ChargeSwap>();
}
