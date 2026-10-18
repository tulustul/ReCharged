//! UI-side contracts shared with Bevy crates (ARCHITECTURE §4.5, §4.6). The React channels
//! themselves live in rc_ui.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::catalog::{CarId, PaintId};

/// UI sound cues (GDD §16.2); each plays `assets/audio/sfx/<stem>.ogg` (`docs/design/sfx.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UiCue {
    /// Focus moved.
    Hover,
    /// Activated.
    Select,
    /// Went back.
    Back,
    /// Toggled a switch.
    Toggle,
    /// Slider step.
    SliderTick,
    /// Not allowed / failed.
    Error,
    /// Something was unlocked.
    Unlock,
}

impl UiCue {
    /// File stem under `audio/sfx/`, e.g. `"ui_slider_tick"`.
    pub fn stem(self) -> &'static str {
        match self {
            Self::Hover => "ui_hover",
            Self::Select => "ui_select",
            Self::Back => "ui_back",
            Self::Toggle => "ui_toggle",
            Self::SliderTick => "ui_slider_tick",
            Self::Error => "ui_error",
            Self::Unlock => "ui_unlock",
        }
    }
}

/// Play a UI cue (producer ui; consumer audio).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiSound(pub UiCue);

/// Camera framing of the menu showroom.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BackdropFocus {
    /// Wide, slowly drifting shot.
    #[default]
    Wide,
    /// Close on the car (car select, garage).
    Car,
}

/// Present while the 3D menu showroom actually renders the [`MenuBackdrop`] car. Inserted and
/// removed only by rc_render; rc_ui reads it: without it the menus draw their own background and
/// 2D car art instead of leaving room for the 3D car.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ShowroomReady;

/// What the menu showroom shows. Written only by rc_ui; read by render and camera.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct MenuBackdrop {
    /// Car on the turntable.
    pub car: CarId,
    /// Its paint.
    pub paint: PaintId,
    /// Camera framing.
    pub focus: BackdropFocus,
}
