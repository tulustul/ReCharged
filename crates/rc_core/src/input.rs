//! Input contracts: devices, bindings (GDD §12.1 defaults), local players, menu navigation and
//! input messages (ARCHITECTURE §4.5, §4.6, §5.3).

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::catalog::PlayerSlot;

/// A concrete input device a player can own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputDevice {
    /// Keyboard scheme A (WASD side).
    KeyboardA,
    /// Keyboard scheme B (arrows side).
    KeyboardB,
    /// A gamepad entity.
    Gamepad(Entity),
}

/// Device family, for glyphs and `UiNav` tagging.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeviceKind {
    /// Keyboard scheme A.
    #[default]
    KeyboardA,
    /// Keyboard scheme B.
    KeyboardB,
    /// Any gamepad (generic glyphs, no platform logos).
    Gamepad,
    /// Mouse pointer (menus only).
    Mouse,
}

/// A rebindable binding scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ControlScheme {
    /// Keyboard A.
    KeyboardA,
    /// Keyboard B.
    KeyboardB,
    /// Gamepads.
    Gamepad,
}

/// A rebindable driving action (GDD §12.1). Steering is two half-axis actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DriveAction {
    /// Throttle.
    Throttle,
    /// Brake / reverse.
    Brake,
    /// Steer left.
    SteerLeft,
    /// Steer right.
    SteerRight,
    /// Drift (handbrake).
    Drift,
    /// Use item.
    UseItem,
    /// Boost (spend charge).
    Boost,
    /// Look back (also fire backward).
    LookBack,
    /// Reset (tap = flip, hold 0.4 s = respawn).
    Reset,
    /// Cycle camera mode.
    Camera,
    /// Pause.
    Pause,
}

impl DriveAction {
    /// Every action in menu order.
    pub const ALL: [DriveAction; 11] = [
        Self::Throttle,
        Self::Brake,
        Self::SteerLeft,
        Self::SteerRight,
        Self::Drift,
        Self::UseItem,
        Self::Boost,
        Self::LookBack,
        Self::Reset,
        Self::Camera,
        Self::Pause,
    ];
}

/// One physical input bound to an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Binding {
    /// A keyboard key.
    Key(KeyCode),
    /// A gamepad button (analog triggers report their value).
    Button(GamepadButton),
    /// One direction of a gamepad axis; the action's value is the axis value that way.
    Axis {
        /// The axis.
        axis: GamepadAxis,
        /// `true` = positive half.
        positive: bool,
    },
}

/// Bindings of one scheme: every action may have several bindings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionBindings {
    /// [`DriveAction::Throttle`].
    pub throttle: Vec<Binding>,
    /// [`DriveAction::Brake`].
    pub brake: Vec<Binding>,
    /// [`DriveAction::SteerLeft`].
    pub steer_left: Vec<Binding>,
    /// [`DriveAction::SteerRight`].
    pub steer_right: Vec<Binding>,
    /// [`DriveAction::Drift`].
    pub drift: Vec<Binding>,
    /// [`DriveAction::UseItem`].
    pub use_item: Vec<Binding>,
    /// [`DriveAction::Boost`].
    pub boost: Vec<Binding>,
    /// [`DriveAction::LookBack`].
    pub look_back: Vec<Binding>,
    /// [`DriveAction::Reset`].
    pub reset: Vec<Binding>,
    /// [`DriveAction::Camera`].
    pub camera: Vec<Binding>,
    /// [`DriveAction::Pause`].
    pub pause: Vec<Binding>,
}

impl ActionBindings {
    /// Bindings of `action`.
    pub fn get(&self, action: DriveAction) -> &[Binding] {
        match action {
            DriveAction::Throttle => &self.throttle,
            DriveAction::Brake => &self.brake,
            DriveAction::SteerLeft => &self.steer_left,
            DriveAction::SteerRight => &self.steer_right,
            DriveAction::Drift => &self.drift,
            DriveAction::UseItem => &self.use_item,
            DriveAction::Boost => &self.boost,
            DriveAction::LookBack => &self.look_back,
            DriveAction::Reset => &self.reset,
            DriveAction::Camera => &self.camera,
            DriveAction::Pause => &self.pause,
        }
    }

    /// Mutable bindings of `action` (rebinding; rc_game is the only writer of `Settings`).
    pub fn get_mut(&mut self, action: DriveAction) -> &mut Vec<Binding> {
        match action {
            DriveAction::Throttle => &mut self.throttle,
            DriveAction::Brake => &mut self.brake,
            DriveAction::SteerLeft => &mut self.steer_left,
            DriveAction::SteerRight => &mut self.steer_right,
            DriveAction::Drift => &mut self.drift,
            DriveAction::UseItem => &mut self.use_item,
            DriveAction::Boost => &mut self.boost,
            DriveAction::LookBack => &mut self.look_back,
            DriveAction::Reset => &mut self.reset,
            DriveAction::Camera => &mut self.camera,
            DriveAction::Pause => &mut self.pause,
        }
    }

    /// Default keyboard A scheme (GDD §12.1).
    pub fn keyboard_a() -> Self {
        use KeyCode::*;
        let k = |keys: &[KeyCode]| keys.iter().map(|&c| Binding::Key(c)).collect();
        Self {
            throttle: k(&[KeyW]),
            brake: k(&[KeyS]),
            steer_left: k(&[KeyA]),
            steer_right: k(&[KeyD]),
            drift: k(&[Space]),
            use_item: k(&[KeyE]),
            boost: k(&[ShiftLeft]),
            look_back: k(&[KeyQ]),
            reset: k(&[KeyR]),
            camera: k(&[KeyC]),
            pause: k(&[Escape]),
        }
    }

    /// Default keyboard B scheme (GDD §12.1).
    pub fn keyboard_b() -> Self {
        use KeyCode::*;
        let k = |keys: &[KeyCode]| keys.iter().map(|&c| Binding::Key(c)).collect();
        Self {
            throttle: k(&[ArrowUp]),
            brake: k(&[ArrowDown]),
            steer_left: k(&[ArrowLeft]),
            steer_right: k(&[ArrowRight]),
            drift: k(&[ShiftRight]),
            use_item: k(&[Enter]),
            boost: k(&[ControlRight]),
            look_back: k(&[Numpad0, Delete]),
            reset: k(&[Backspace]),
            camera: k(&[NumpadDecimal, End]),
            pause: k(&[Escape]),
        }
    }

    /// Default gamepad scheme (GDD §12.1).
    pub fn gamepad() -> Self {
        use GamepadButton::*;
        let b = |buttons: &[GamepadButton]| buttons.iter().map(|&c| Binding::Button(c)).collect();
        let stick = |positive| Binding::Axis {
            axis: GamepadAxis::LeftStickX,
            positive,
        };
        Self {
            throttle: b(&[RightTrigger2]),
            brake: b(&[LeftTrigger2]),
            steer_left: vec![stick(false), Binding::Button(DPadLeft)],
            steer_right: vec![stick(true), Binding::Button(DPadRight)],
            drift: b(&[East, RightTrigger]),
            use_item: b(&[South]),
            boost: b(&[West, LeftTrigger]),
            look_back: b(&[North]),
            reset: b(&[DPadUp]),
            camera: b(&[Select]),
            pause: b(&[Start]),
        }
    }
}

/// Logical menu navigation (fixed keys, GDD §12.2), forwarded to React as `ui.nav`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NavAction {
    /// Move focus up.
    Up,
    /// Move focus down.
    Down,
    /// Move focus left.
    Left,
    /// Move focus right.
    Right,
    /// Activate (A / Enter / Space; Start in menus).
    Confirm,
    /// Up one level (B / Esc / Backspace).
    Back,
    /// Previous tab (LB / Q).
    PrevTab,
    /// Next tab (RB / E).
    NextTab,
}

/// A menu navigation step (producer input; consumer ui → `ui.nav`).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiNav {
    /// The step.
    pub action: NavAction,
    /// The player who pressed it (`None` = a device not assigned to a slot).
    pub slot: Option<PlayerSlot>,
    /// Device family that pressed it.
    pub device: DeviceKind,
    /// Generated by hold-repeat.
    pub repeat: bool,
}

/// Start capturing the next press for `action` of `scheme` (producer ui; consumer input).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RebindRequest {
    /// Scheme to rebind.
    pub scheme: ControlScheme,
    /// Action to rebind.
    pub action: DriveAction,
}

/// Outcome of a rebind capture (producer input; consumers ui, game — which writes `Settings`).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RebindResult {
    /// Scheme.
    pub scheme: ControlScheme,
    /// Action.
    pub action: DriveAction,
    /// Captured binding; `None` = cancelled.
    pub binding: Option<Binding>,
    /// Another action already uses it (keyboard schemes are checked together).
    pub conflict: Option<(ControlScheme, DriveAction)>,
    /// The rebound action's primary binding before the capture: on a conflict the two swap, so
    /// the conflicting action receives it (`None`: it is left without the captured binding).
    pub previous: Option<Binding>,
}

/// Device assignment changes (producer input; consumers game, ui).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceEvent {
    /// A device joined a lobby slot.
    Joined {
        /// The slot.
        slot: PlayerSlot,
        /// The device.
        device: InputDevice,
    },
    /// A player left the lobby.
    Left {
        /// The slot.
        slot: PlayerSlot,
    },
    /// A player's gamepad disconnected (rc_game pauses the race).
    Disconnected {
        /// The slot.
        slot: PlayerSlot,
    },
    /// A player used another device family (glyphs follow it).
    LastUsed {
        /// The slot.
        slot: PlayerSlot,
        /// The device family.
        kind: DeviceKind,
    },
}

/// Ask rc_game to pause (producers input — only while running — and game on focus loss).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PauseRequest {
    /// Who paused (`None` = the system, e.g. focus loss).
    pub slot: Option<PlayerSlot>,
}

/// The Camera action was pressed (producer input — only in a running race; consumer camera: next
/// camera mode, or the next spectated racer once the slot's racer finished). Registered by the
/// input and camera plugins.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CycleCamera {
    /// The player who pressed it.
    pub slot: PlayerSlot,
}

/// Devices owned by one local player.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotBinding {
    /// The slot.
    pub slot: PlayerSlot,
    /// Owned devices (single-player: every device).
    pub devices: Vec<InputDevice>,
    /// Last used device family.
    pub last_used: DeviceKind,
}

/// Local players and their devices. Written only by rc_input.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct LocalPlayers {
    /// Joined slots in slot order.
    pub slots: Vec<SlotBinding>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_schemes_bind_every_action() {
        for scheme in [
            ActionBindings::keyboard_a(),
            ActionBindings::keyboard_b(),
            ActionBindings::gamepad(),
        ] {
            for action in DriveAction::ALL {
                assert!(!scheme.get(action).is_empty(), "{action:?} unbound");
            }
        }
    }

    #[test]
    fn keyboard_schemes_share_only_pause() {
        let (a, b) = (ActionBindings::keyboard_a(), ActionBindings::keyboard_b());
        for x in DriveAction::ALL {
            for y in DriveAction::ALL {
                let shared = a.get(x).iter().any(|k| b.get(y).contains(k));
                assert_eq!(
                    shared,
                    x == DriveAction::Pause && y == DriveAction::Pause,
                    "{x:?}/{y:?}"
                );
            }
        }
    }

    #[test]
    fn get_mut_targets_the_same_list() {
        let mut a = ActionBindings::keyboard_a();
        a.get_mut(DriveAction::Boost)
            .push(Binding::Key(KeyCode::KeyB));
        assert_eq!(a.get(DriveAction::Boost).len(), 2);
    }
}
