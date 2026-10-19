//! Rebinding (GDD §12.1, §15): `RebindRequest` → capture the next press of the scheme's device
//! family (Esc / Start cancel) → conflict check (both keyboard schemes share one keyboard, so they
//! are checked together) → `RebindResult`. rc_game applies it to `Settings` (single writer).

use bevy::input::gamepad::{Gamepad, GamepadAxis, GamepadButton};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use rc_core::prelude::*;

use crate::bindings::PRESS_THRESHOLD;

/// Axes a stick or trigger rebind may capture.
const CAPTURE_AXES: [GamepadAxis; 6] = [
    GamepadAxis::LeftStickX,
    GamepadAxis::LeftStickY,
    GamepadAxis::RightStickX,
    GamepadAxis::RightStickY,
    GamepadAxis::LeftZ,
    GamepadAxis::RightZ,
];

/// The rebind in progress. `busy` is true while capturing and in the frame a capture ended, so
/// the captured press neither navigates nor drives.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct RebindCapture {
    /// The pending request.
    pub request: Option<RebindRequest>,
    /// Input is reserved for the capture this frame.
    pub busy: bool,
}

/// The first other action already using `binding`: in `scheme`, plus the other keyboard scheme
/// for keyboard schemes. The action being rebound itself is not a conflict.
pub fn find_conflict(
    controls: &ControlSettings,
    scheme: ControlScheme,
    action: DriveAction,
    binding: Binding,
) -> Option<(ControlScheme, DriveAction)> {
    let schemes: &[ControlScheme] = match scheme {
        ControlScheme::KeyboardA => &[ControlScheme::KeyboardA, ControlScheme::KeyboardB],
        ControlScheme::KeyboardB => &[ControlScheme::KeyboardB, ControlScheme::KeyboardA],
        ControlScheme::Gamepad => &[ControlScheme::Gamepad],
    };
    schemes.iter().find_map(|&s| {
        DriveAction::ALL
            .into_iter()
            .filter(|&a| (s, a) != (scheme, action))
            .find(|&a| controls.bindings(s).get(a).contains(&binding))
            .map(|a| (s, a))
    })
}

/// What a capture saw this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Captured {
    /// The cancel key/button.
    Cancel,
    /// A new binding.
    Binding(Binding),
}

/// The next keyboard press (Esc cancels).
pub fn capture_key(keys: &ButtonInput<KeyCode>) -> Option<Captured> {
    if keys.just_pressed(KeyCode::Escape) {
        return Some(Captured::Cancel);
    }
    keys.get_just_pressed()
        .next()
        .map(|&k| Captured::Binding(Binding::Key(k)))
}

/// The next gamepad press (Start cancels): a button, or an axis crossing [`PRESS_THRESHOLD`]
/// against its value in the previous frame (`previous`, updated).
pub fn capture_pad(pad: &Gamepad, previous: &mut [f32; CAPTURE_AXES.len()]) -> Option<Captured> {
    let mut found = None;
    for (prev, axis) in previous.iter_mut().zip(CAPTURE_AXES) {
        let v = pad.get(axis).unwrap_or(0.0);
        if found.is_none() && v.abs() >= PRESS_THRESHOLD && prev.abs() < PRESS_THRESHOLD {
            found = Some(Captured::Binding(Binding::Axis {
                axis,
                positive: v > 0.0,
            }));
        }
        *prev = v;
    }
    if pad.just_pressed(GamepadButton::Start) {
        return Some(Captured::Cancel);
    }
    pad.get_just_pressed()
        .find(|b| !matches!(b, GamepadButton::Other(_)))
        .map(|&b| Captured::Binding(Binding::Button(b)))
        .or(found)
}

/// PreUpdate `InputSet::Collect`, before navigation and driving.
pub(crate) fn capture(
    mut requests: MessageReader<RebindRequest>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    pads: Query<(Entity, &Gamepad)>,
    settings: Option<Res<Settings>>,
    mut state: ResMut<RebindCapture>,
    mut axes: Local<HashMap<Entity, [f32; CAPTURE_AXES.len()]>>,
    mut results: MessageWriter<RebindResult>,
) {
    state.busy = state.request.is_some();
    // Keep the axis history fresh, so a stick already held when a capture starts is not taken.
    let mut seen = None;
    for (entity, pad) in &pads {
        let found = capture_pad(pad, axes.entry(entity).or_default());
        seen = seen.or(found);
    }
    if let Some(request) = requests.read().last() {
        state.request = Some(*request);
        state.busy = true;
        return; // the press that opened the capture is not captured
    }
    let Some(request) = state.request else {
        return;
    };
    let captured = match request.scheme {
        ControlScheme::KeyboardA | ControlScheme::KeyboardB => {
            keys.as_deref().and_then(capture_key)
        }
        ControlScheme::Gamepad => seen,
    };
    let Some(captured) = captured else {
        return;
    };
    state.request = None;
    let binding = match captured {
        Captured::Cancel => None,
        Captured::Binding(b) => Some(b),
    };
    let controls = settings.map(|s| s.controls.clone()).unwrap_or_default();
    let conflict =
        binding.and_then(|b| find_conflict(&controls, request.scheme, request.action, b));
    let previous = controls
        .bindings(request.scheme)
        .get(request.action)
        .first()
        .copied();
    info!(
        "rebind {:?} {:?}: {binding:?} (conflict {conflict:?}, was {previous:?})",
        request.scheme, request.action
    );
    results.write(RebindResult {
        scheme: request.scheme,
        action: request.action,
        binding,
        conflict,
        previous,
    });
}

#[cfg(test)]
mod tests {
    use bevy::input::gamepad::GamepadInput;

    use super::*;

    #[test]
    fn conflicts_span_both_keyboard_schemes() {
        let c = ControlSettings::default();
        let up = Binding::Key(KeyCode::ArrowUp);
        assert_eq!(
            find_conflict(&c, ControlScheme::KeyboardA, DriveAction::Boost, up),
            Some((ControlScheme::KeyboardB, DriveAction::Throttle)),
            "keyboard A can't take keyboard B's throttle"
        );
        let w = Binding::Key(KeyCode::KeyW);
        assert_eq!(
            find_conflict(&c, ControlScheme::KeyboardA, DriveAction::Throttle, w),
            None,
            "itself"
        );
        assert_eq!(
            find_conflict(&c, ControlScheme::KeyboardA, DriveAction::Boost, w),
            Some((ControlScheme::KeyboardA, DriveAction::Throttle))
        );
        let free = Binding::Key(KeyCode::KeyZ);
        assert_eq!(
            find_conflict(&c, ControlScheme::KeyboardB, DriveAction::Boost, free),
            None
        );
    }

    #[test]
    fn gamepad_conflicts_stay_within_the_gamepad_scheme() {
        let c = ControlSettings::default();
        let a = Binding::Button(GamepadButton::South);
        assert_eq!(
            find_conflict(&c, ControlScheme::Gamepad, DriveAction::Boost, a),
            Some((ControlScheme::Gamepad, DriveAction::UseItem))
        );
        let w = Binding::Key(KeyCode::KeyW);
        assert_eq!(
            find_conflict(&c, ControlScheme::Gamepad, DriveAction::Boost, w),
            None
        );
    }

    #[test]
    fn keyboard_capture_takes_the_next_key_and_esc_cancels() {
        let mut keys = ButtonInput::default();
        assert_eq!(capture_key(&keys), None);
        keys.press(KeyCode::KeyK);
        assert_eq!(
            capture_key(&keys),
            Some(Captured::Binding(Binding::Key(KeyCode::KeyK)))
        );
        keys.clear();
        keys.press(KeyCode::Escape);
        assert_eq!(capture_key(&keys), Some(Captured::Cancel));
    }

    #[test]
    fn pad_capture_takes_buttons_and_fresh_axis_deflections() {
        let mut prev = [0.0; CAPTURE_AXES.len()];
        let mut pad = Gamepad::default();
        pad.analog_mut()
            .set(GamepadInput::Axis(GamepadAxis::RightStickY), -0.9);
        assert_eq!(
            capture_pad(&pad, &mut prev),
            Some(Captured::Binding(Binding::Axis {
                axis: GamepadAxis::RightStickY,
                positive: false
            }))
        );
        assert_eq!(
            capture_pad(&pad, &mut prev),
            None,
            "still held: not a new press"
        );
        pad.digital_mut().press(GamepadButton::North);
        assert_eq!(
            capture_pad(&pad, &mut prev),
            Some(Captured::Binding(Binding::Button(GamepadButton::North)))
        );
        let mut pad = Gamepad::default();
        pad.digital_mut().press(GamepadButton::Start);
        assert_eq!(capture_pad(&pad, &mut prev), Some(Captured::Cancel));
    }
}
