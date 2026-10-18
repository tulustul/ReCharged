//! Binding resolution (GDD §12.1): physical input → per-action values, steering shaping and the
//! digital steering ramp. Pure functions over Bevy's input state.

use bevy::input::gamepad::Gamepad;
use bevy::prelude::*;
use rc_core::prelude::*;

/// An action counts as pressed at this value (analog triggers, stick halves).
pub const PRESS_THRESHOLD: f32 = 0.5;

/// Digital steering reaches full lock in this time, s (GDD §5.1 "~0.12 s").
pub const STEER_RAMP_S: f32 = 0.12;

/// Number of driving actions.
pub const ACTIONS: usize = DriveAction::ALL.len();

/// One action's input on one device (or merged over a player's devices).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ActionValue {
    /// From keys and buttons, 0–1 (analog triggers report their travel).
    pub digital: f32,
    /// From stick axes, 0–1.
    pub analog: f32,
    /// A key or button went down this frame (catches taps shorter than a frame).
    pub tapped: bool,
}

impl ActionValue {
    /// The action's value 0–1.
    pub fn value(self) -> f32 {
        self.digital.max(self.analog)
    }

    /// Held past [`PRESS_THRESHOLD`].
    pub fn pressed(self) -> bool {
        self.value() >= PRESS_THRESHOLD
    }

    fn merge(self, other: Self) -> Self {
        Self {
            digital: self.digital.max(other.digital),
            analog: self.analog.max(other.analog),
            tapped: self.tapped || other.tapped,
        }
    }
}

/// Every action of one device in one frame, indexed by `DriveAction as usize`.
pub type ActionFrame = [ActionValue; ACTIONS];

/// Value of one binding. Keyboard devices pass `pad = None`.
pub fn binding_value(
    binding: Binding,
    keys: &ButtonInput<KeyCode>,
    pad: Option<&Gamepad>,
) -> ActionValue {
    match (binding, pad) {
        (Binding::Key(key), _) => ActionValue {
            digital: if keys.pressed(key) { 1.0 } else { 0.0 },
            analog: 0.0,
            tapped: keys.just_pressed(key),
        },
        (Binding::Button(button), Some(pad)) => ActionValue {
            digital: pad
                .get(button)
                .unwrap_or(if pad.pressed(button) { 1.0 } else { 0.0 }),
            analog: 0.0,
            tapped: pad.just_pressed(button),
        },
        (Binding::Axis { axis, positive }, Some(pad)) => {
            let v = pad.get(axis).unwrap_or(0.0);
            ActionValue {
                digital: 0.0,
                analog: (if positive { v } else { -v }).clamp(0.0, 1.0),
                tapped: false,
            }
        }
        (Binding::Button(_) | Binding::Axis { .. }, None) => ActionValue::default(),
    }
}

/// Every action of a device with `bindings`.
pub fn device_frame(
    bindings: &ActionBindings,
    keys: &ButtonInput<KeyCode>,
    pad: Option<&Gamepad>,
) -> ActionFrame {
    let mut frame = [ActionValue::default(); ACTIONS];
    for (slot, action) in frame.iter_mut().zip(DriveAction::ALL) {
        *slot = bindings
            .get(action)
            .iter()
            .fold(ActionValue::default(), |acc, &b| {
                acc.merge(binding_value(b, keys, pad))
            });
    }
    frame
}

/// Merges `other` into `into` (a player holding two devices).
pub fn merge_frames(into: &mut ActionFrame, other: &ActionFrame) {
    for (a, b) in into.iter_mut().zip(other) {
        *a = a.merge(*b);
    }
}

/// A stick half (0–1) after the dead zone (rescaled so the response starts at 0) and the
/// sensitivity curve (`v^(1/sensitivity)`: 1 = linear, > 1 = more response near the centre).
pub fn shape_axis(value: f32, dead_zone: f32, sensitivity: f32) -> f32 {
    let dz = dead_zone.clamp(0.0, 0.95);
    if value <= dz {
        return 0.0;
    }
    let t = ((value - dz) / (1.0 - dz)).min(1.0);
    t.powf(1.0 / sensitivity.max(0.1))
}

/// Steering of a frame: the shaped analog part (−1 left … 1 right) and the digital target
/// (−1, 0, 1) that the fixed tick ramps toward.
pub fn steer_inputs(frame: &ActionFrame, dead_zone: f32, sensitivity: f32) -> (f32, f32) {
    let (left, right) = (
        frame[DriveAction::SteerLeft as usize],
        frame[DriveAction::SteerRight as usize],
    );
    let analog = shape_axis(right.analog, dead_zone, sensitivity)
        - shape_axis(left.analog, dead_zone, sensitivity);
    let digital = f32::from(u8::from(right.digital >= PRESS_THRESHOLD))
        - f32::from(u8::from(left.digital >= PRESS_THRESHOLD));
    (analog, digital)
}

/// Moves the digital steering `current` toward `target` at full lock per [`STEER_RAMP_S`].
pub fn ramp(current: f32, target: f32, dt: f32) -> f32 {
    let step = dt / STEER_RAMP_S;
    current + (target - current).clamp(-step, step)
}

#[cfg(test)]
mod tests {
    use bevy::input::gamepad::{GamepadAxis, GamepadButton, GamepadInput};

    use super::*;

    fn pad_with(buttons: &[(GamepadButton, f32)], axes: &[(GamepadAxis, f32)]) -> Gamepad {
        let mut pad = Gamepad::default();
        for &(b, v) in buttons {
            pad.analog_mut().set(GamepadInput::Button(b), v);
            if v >= 0.75 {
                pad.digital_mut().press(b);
            }
        }
        for &(a, v) in axes {
            pad.analog_mut().set(GamepadInput::Axis(a), v);
        }
        pad
    }

    #[test]
    fn keyboard_bindings_resolve_for_their_scheme_only() {
        let mut keys = ButtonInput::default();
        keys.press(KeyCode::KeyW);
        keys.press(KeyCode::ArrowLeft);
        let a = device_frame(&ActionBindings::keyboard_a(), &keys, None);
        let b = device_frame(&ActionBindings::keyboard_b(), &keys, None);
        assert_eq!(a[DriveAction::Throttle as usize].value(), 1.0);
        assert!(a[DriveAction::Throttle as usize].tapped);
        assert_eq!(a[DriveAction::SteerLeft as usize].value(), 0.0);
        assert_eq!(b[DriveAction::SteerLeft as usize].value(), 1.0);
        assert_eq!(b[DriveAction::Throttle as usize].value(), 0.0);
    }

    #[test]
    fn gamepad_triggers_are_analog_and_sticks_split_into_halves() {
        let keys = ButtonInput::default();
        let pad = pad_with(
            &[(GamepadButton::RightTrigger2, 0.4)],
            &[(GamepadAxis::LeftStickX, -0.8)],
        );
        let f = device_frame(&ActionBindings::gamepad(), &keys, Some(&pad));
        assert!((f[DriveAction::Throttle as usize].value() - 0.4).abs() < 1e-6);
        assert!(!f[DriveAction::Throttle as usize].pressed());
        assert!((f[DriveAction::SteerLeft as usize].analog - 0.8).abs() < 1e-6);
        assert_eq!(f[DriveAction::SteerRight as usize].value(), 0.0);
        let keyboard = device_frame(&ActionBindings::keyboard_a(), &keys, None);
        assert_eq!(keyboard[DriveAction::Throttle as usize].value(), 0.0);
    }

    #[test]
    fn several_bindings_and_devices_take_the_strongest() {
        let keys = ButtonInput::default();
        let pad = pad_with(
            &[(GamepadButton::DPadLeft, 1.0)],
            &[(GamepadAxis::LeftStickX, -0.3)],
        );
        let mut f = device_frame(&ActionBindings::gamepad(), &keys, Some(&pad));
        let steer = f[DriveAction::SteerLeft as usize];
        assert_eq!((steer.digital, steer.analog), (1.0, 0.3));
        let mut keys = ButtonInput::default();
        keys.press(KeyCode::KeyD);
        merge_frames(
            &mut f,
            &device_frame(&ActionBindings::keyboard_a(), &keys, None),
        );
        assert_eq!(f[DriveAction::SteerRight as usize].digital, 1.0);
    }

    #[test]
    fn dead_zone_rescales_and_sensitivity_bends_the_curve() {
        assert_eq!(shape_axis(0.1, 0.12, 1.0), 0.0);
        assert_eq!(shape_axis(0.12, 0.12, 1.0), 0.0);
        assert!(
            (shape_axis(0.56, 0.12, 1.0) - 0.5).abs() < 1e-6,
            "linear after the dead zone"
        );
        assert_eq!(shape_axis(1.0, 0.12, 1.0), 1.0);
        assert!(
            shape_axis(0.56, 0.12, 2.0) > 0.7,
            "more sensitive near the centre"
        );
        assert!(
            shape_axis(0.56, 0.12, 0.5) < 0.3,
            "less sensitive near the centre"
        );
        assert_eq!(shape_axis(1.0, 0.12, 2.0), 1.0, "full lock stays reachable");
        assert!(shape_axis(0.5, 2.0, 0.0).is_finite());
    }

    #[test]
    fn steering_combines_stick_and_digital_target() {
        let mut f = [ActionValue::default(); ACTIONS];
        f[DriveAction::SteerLeft as usize].analog = 1.0;
        f[DriveAction::SteerRight as usize].digital = 1.0;
        let (analog, digital) = steer_inputs(&f, 0.12, 1.0);
        assert_eq!((analog, digital), (-1.0, 1.0));
    }

    #[test]
    fn digital_ramp_reaches_full_lock_in_about_0_12_s() {
        let dt = SIM_DT;
        let mut s = 0.0;
        let mut ticks = 0;
        while s < 1.0 {
            s = ramp(s, 1.0, dt);
            ticks += 1;
        }
        assert_eq!(ticks, 8, "0.12 s at 60 Hz rounds up to 8 ticks");
        assert_eq!(ramp(1.0, 1.0, dt), 1.0, "holds");
        let back = ramp(1.0, 0.0, dt);
        assert!(back < 1.0 && back > 0.8, "returns at the same rate: {back}");
        assert_eq!(ramp(0.3, 0.3, 10.0), 0.3);
    }
}
