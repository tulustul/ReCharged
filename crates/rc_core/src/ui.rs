//! UI-side contracts shared with Bevy crates (ARCHITECTURE §4.5, §4.6). The React channels
//! themselves live in rc_ui.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::catalog::{CarId, PaintId, PlayerSlot};

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
    /// Close on the car (car select).
    Car,
    /// The wide shot without the car: screens whose content covers the turntable (Garage grid,
    /// Credits roll, Cup-end podium) or whose translucent cards it would show through (Cup select,
    /// Track select, Versus lobby).
    Room,
}

/// Colour-blind marker shapes (GDD §18), in grid order: ● ▲ ■ ◆ ★ ✚ ⬟ ⬢.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MarkerShape {
    /// ●
    Circle,
    /// ▲
    Triangle,
    /// ■
    Square,
    /// ◆
    Diamond,
    /// ★
    Star,
    /// ✚
    Cross,
    /// ⬟
    Pentagon,
    /// ⬢
    Hexagon,
}

impl MarkerShape {
    /// Every shape, in grid order.
    pub const ALL: [Self; 8] = [
        Self::Circle,
        Self::Triangle,
        Self::Square,
        Self::Diamond,
        Self::Star,
        Self::Cross,
        Self::Pentagon,
        Self::Hexagon,
    ];
}

/// A racer's colour-blind marker (Settings → Accessibility): the player number for humans, else
/// a shape by grid index. rc_vfx (name tags) and rc_ui (minimap, standings) both use
/// [`RacerMarker::of`], so the shapes always agree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RacerMarker {
    /// P1–P4.
    Player(PlayerSlot),
    /// An AI rival's shape.
    Shape(MarkerShape),
}

impl RacerMarker {
    /// The marker of the racer at `grid_index` (`slot`: its local player, if human).
    pub fn of(grid_index: usize, slot: Option<PlayerSlot>) -> Self {
        match slot {
            Some(slot) => Self::Player(slot),
            None => Self::Shape(MarkerShape::ALL[grid_index % MarkerShape::ALL.len()]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markers_follow_the_grid_and_humans_keep_their_number() {
        assert_eq!(
            RacerMarker::of(0, None),
            RacerMarker::Shape(MarkerShape::Circle)
        );
        assert_eq!(
            RacerMarker::of(4, None),
            RacerMarker::Shape(MarkerShape::Star)
        );
        assert_eq!(
            RacerMarker::of(7, None),
            RacerMarker::Shape(MarkerShape::Hexagon)
        );
        assert_eq!(
            RacerMarker::of(1, Some(PlayerSlot(1))),
            RacerMarker::Player(PlayerSlot(1))
        );
        let shapes: std::collections::HashSet<_> =
            (0..8).map(|i| RacerMarker::of(i, None)).collect();
        assert_eq!(shapes.len(), 8, "a full grid gets eight different shapes");
    }
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
