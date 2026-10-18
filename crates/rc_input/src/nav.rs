//! Menu navigation (GDD §12.2, §12.3): the fixed menu keys → `UiNav` (slot + device tagged,
//! hold-repeat for directions), the lobby's join (Confirm on an unassigned device) and leave (Back
//! on a joined one), and the takeover of a disconnected player's slot. Suppressed while a text
//! field has focus, while a rebind is captured and while racing unpaused; in a running race the
//! keys bound to Pause request the pause instead.

use bevy::ecs::system::SystemParam;
use bevy::input::gamepad::{Gamepad, GamepadButton};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use rc_core::prelude::*;

use crate::intents::{Flow, Typing};
use crate::players::{Roster, kind_of};
use crate::rebind::RebindCapture;

/// Left stick: a direction starts past this deflection …
pub const STICK_PRESS: f32 = 0.5;
/// … and ends below this one (hysteresis).
pub const STICK_RELEASE: f32 = 0.3;
/// A held direction first repeats after this, s …
pub const REPEAT_DELAY_S: f32 = 0.35;
/// … then every this, s.
pub const REPEAT_EVERY_S: f32 = 0.1;

/// How a menu key behaves in the Versus lobby.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    /// Belongs to its scheme's player (joins, leaves, per-slot focus).
    Scheme,
    /// Nobody's in the lobby (sent with `slot: None`).
    Shared,
    /// A scheme key that exists only in the lobby (scheme A's Back).
    LobbyOnly,
}

/// One fixed menu key.
#[derive(Debug, Clone, Copy)]
struct NavKey {
    key: KeyCode,
    action: NavAction,
    scheme: InputDevice,
    role: Role,
}

const fn nav_key(key: KeyCode, action: NavAction, scheme: InputDevice, role: Role) -> NavKey {
    NavKey {
        key,
        action,
        scheme,
        role,
    }
}

/// GDD §12.2: arrows or WASD move, Enter or Space confirm, Esc or Backspace back, Q/E tabs; in the
/// lobby scheme A is WASD + Space + Left Ctrl and scheme B arrows + Enter + Backspace.
const KEYS: [NavKey; 16] = {
    use InputDevice::{KeyboardA as A, KeyboardB as B};
    use KeyCode as K;
    use NavAction as N;
    use Role::{LobbyOnly, Scheme, Shared};
    [
        nav_key(K::KeyW, N::Up, A, Scheme),
        nav_key(K::KeyS, N::Down, A, Scheme),
        nav_key(K::KeyA, N::Left, A, Scheme),
        nav_key(K::KeyD, N::Right, A, Scheme),
        nav_key(K::ArrowUp, N::Up, B, Scheme),
        nav_key(K::ArrowDown, N::Down, B, Scheme),
        nav_key(K::ArrowLeft, N::Left, B, Scheme),
        nav_key(K::ArrowRight, N::Right, B, Scheme),
        nav_key(K::Space, N::Confirm, A, Scheme),
        nav_key(K::Enter, N::Confirm, B, Scheme),
        nav_key(K::NumpadEnter, N::Confirm, B, Scheme),
        nav_key(K::Backspace, N::Back, B, Scheme),
        nav_key(K::ControlLeft, N::Back, A, LobbyOnly),
        nav_key(K::Escape, N::Back, A, Shared),
        nav_key(K::KeyQ, N::PrevTab, A, Shared),
        nav_key(K::KeyE, N::NextTab, A, Shared),
    ]
};

/// Gamepad: D-pad moves, A confirms, B backs, LB/RB switch tabs, Start confirms (or pauses).
const BUTTONS: [(GamepadButton, NavAction); 9] = [
    (GamepadButton::DPadUp, NavAction::Up),
    (GamepadButton::DPadDown, NavAction::Down),
    (GamepadButton::DPadLeft, NavAction::Left),
    (GamepadButton::DPadRight, NavAction::Right),
    (GamepadButton::South, NavAction::Confirm),
    (GamepadButton::East, NavAction::Back),
    (GamepadButton::LeftTrigger, NavAction::PrevTab),
    (GamepadButton::RightTrigger, NavAction::NextTab),
    (GamepadButton::Start, NavAction::Confirm),
];

fn is_direction(action: NavAction) -> bool {
    matches!(
        action,
        NavAction::Up | NavAction::Down | NavAction::Left | NavAction::Right
    )
}

/// Hold-repeat timer of one held direction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Repeat {
    held: f32,
    next: f32,
}

impl Default for Repeat {
    fn default() -> Self {
        Self {
            held: 0.0,
            next: REPEAT_DELAY_S,
        }
    }
}

impl Repeat {
    /// Advances by `dt`; `true` when a repeat is due (at most one per call, no catch-up bursts).
    pub fn tick(&mut self, dt: f32) -> bool {
        self.held += dt;
        if self.held < self.next {
            return false;
        }
        self.next += REPEAT_EVERY_S;
        if self.next <= self.held {
            self.next = self.held + REPEAT_EVERY_S;
        }
        true
    }
}

/// Direction of the left stick with press/release hysteresis on the dominant axis (stick up = +Y).
pub fn stick_direction(previous: Option<NavAction>, stick: Vec2) -> Option<NavAction> {
    let along = |d: NavAction| match d {
        NavAction::Up => stick.y,
        NavAction::Down => -stick.y,
        NavAction::Right => stick.x,
        NavAction::Left => -stick.x,
        _ => 0.0,
    };
    if let Some(d) = previous
        && along(d) >= STICK_RELEASE
    {
        return Some(d);
    }
    if stick.x.abs().max(stick.y.abs()) < STICK_PRESS {
        return None;
    }
    Some(if stick.x.abs() >= stick.y.abs() {
        if stick.x > 0.0 {
            NavAction::Right
        } else {
            NavAction::Left
        }
    } else if stick.y > 0.0 {
        NavAction::Up
    } else {
        NavAction::Down
    })
}

/// A press of a menu input this frame.
#[derive(Debug, Clone, Copy)]
struct Press {
    device: InputDevice,
    action: NavAction,
    role: Role,
    repeat: bool,
}

/// Repeat bookkeeping key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Source {
    Key(KeyCode),
    Button(Entity, GamepadButton),
    Stick(Entity),
}

/// Per-source hold state.
#[derive(Default)]
pub(crate) struct NavState {
    repeats: HashMap<Source, Repeat>,
    sticks: HashMap<Entity, NavAction>,
    presses: Vec<Press>,
}

impl NavState {
    /// A digital source: fires on press, repeats directions while held.
    fn digital(&mut self, source: Source, press: Press, down: (bool, bool), dt: f32) {
        let (just_pressed, pressed) = down;
        if just_pressed {
            self.presses.push(press);
            if is_direction(press.action) {
                self.repeats.insert(source, Repeat::default());
            }
        } else if pressed {
            if let Some(r) = self.repeats.get_mut(&source)
                && r.tick(dt)
            {
                self.presses.push(Press {
                    repeat: true,
                    ..press
                });
            }
        } else {
            self.repeats.remove(&source);
        }
    }
}

/// Keyboard and pads.
#[derive(SystemParam)]
pub(crate) struct NavDevices<'w, 's> {
    keys: Option<Res<'w, ButtonInput<KeyCode>>>,
    pads: Query<'w, 's, (Entity, &'static Gamepad)>,
}

fn pause_bound(controls: &ControlSettings, scheme: ControlScheme, binding: Binding) -> bool {
    controls
        .bindings(scheme)
        .get(DriveAction::Pause)
        .contains(&binding)
}

/// PreUpdate `InputSet::Collect`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn navigate(
    devices: NavDevices,
    time: Res<Time>,
    (flow, typing, capture): (Flow, Typing, Res<RebindCapture>),
    (lobby, settings): (Option<Res<LobbyState>>, Option<Res<Settings>>),
    mut roster: ResMut<Roster>,
    mut state: Local<NavState>,
    mut nav: MessageWriter<UiNav>,
    mut events: MessageWriter<DeviceEvent>,
) {
    let dt = time.delta_secs();
    let lobby_open = lobby.as_ref().is_some_and(|l| l.open);
    let max_slots = lobby
        .as_ref()
        .map_or(MAX_LOCAL_PLAYERS as u8, |l| l.max_slots);
    let defaults;
    let controls = match settings.as_deref() {
        Some(s) => &s.controls,
        None => {
            defaults = ControlSettings::default();
            &defaults
        }
    };
    // In a running race the Pause keys pause; elsewhere they navigate.
    let pause_keys = flow.pausable();
    let state = &mut *state;
    state.presses.clear();
    if let Some(keys) = devices.keys.as_deref() {
        for k in KEYS {
            let scheme = match k.scheme {
                InputDevice::KeyboardB => ControlScheme::KeyboardB,
                _ => ControlScheme::KeyboardA,
            };
            let skip = (k.role == Role::LobbyOnly && !lobby_open)
                || (pause_keys && pause_bound(controls, scheme, Binding::Key(k.key)));
            let down = if skip {
                (false, false)
            } else {
                (keys.just_pressed(k.key), keys.pressed(k.key))
            };
            let press = Press {
                device: k.scheme,
                action: k.action,
                role: k.role,
                repeat: false,
            };
            state.digital(Source::Key(k.key), press, down, dt);
        }
    }
    for (entity, pad) in &devices.pads {
        let device = InputDevice::Gamepad(entity);
        for (button, action) in BUTTONS {
            let skip = pause_keys
                && pause_bound(controls, ControlScheme::Gamepad, Binding::Button(button));
            let down = if skip {
                (false, false)
            } else {
                (pad.just_pressed(button), pad.pressed(button))
            };
            let press = Press {
                device,
                action,
                role: Role::Scheme,
                repeat: false,
            };
            state.digital(Source::Button(entity, button), press, down, dt);
        }
        let previous = state.sticks.get(&entity).copied();
        let now = stick_direction(previous, pad.left_stick());
        let source = Source::Stick(entity);
        match now {
            Some(d) if Some(d) != previous => {
                state.sticks.insert(entity, d);
                state.presses.push(Press {
                    device,
                    action: d,
                    role: Role::Scheme,
                    repeat: false,
                });
                state.repeats.insert(source, Repeat::default());
            }
            Some(d) => {
                if state.repeats.get_mut(&source).is_some_and(|r| r.tick(dt)) {
                    state.presses.push(Press {
                        device,
                        action: d,
                        role: Role::Scheme,
                        repeat: true,
                    });
                }
            }
            None => {
                state.sticks.remove(&entity);
                state.repeats.remove(&source);
            }
        }
    }
    if typing.active() || capture.busy || (flow.racing() && flow.running()) {
        state.repeats.clear(); // a key held through the suppression does not repeat afterwards
        return;
    }
    for press in &state.presses {
        let owner = roster.slot_of(press.device);
        let scheme_key = press.role != Role::Shared;
        if press.action == NavAction::Confirm && !press.repeat && owner.is_none() && scheme_key {
            let joined = if lobby_open {
                roster.join(press.device, max_slots)
            } else if roster.has_orphans() {
                roster.take_over(press.device)
            } else {
                None
            };
            if let Some(slot) = joined {
                info!("{:?} joined as P{}", press.device, slot.0 + 1);
                events.write(DeviceEvent::Joined {
                    slot,
                    device: press.device,
                });
                continue;
            }
        }
        if lobby_open
            && press.action == NavAction::Back
            && scheme_key
            && roster.is_versus()
            && let Some(slot) = owner
        {
            info!("P{} left the lobby", slot.0 + 1);
            roster.leave(slot);
            events.write(DeviceEvent::Left { slot });
            continue;
        }
        nav.write(UiNav {
            action: press.action,
            slot: if lobby_open && !scheme_key {
                None
            } else {
                owner
            },
            device: kind_of(press.device),
            repeat: press.repeat,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use NavAction::{Down, Right, Up};

    #[test]
    fn hold_repeat_waits_0_35_s_then_repeats_every_0_1_s() {
        let dt = 1.0 / 120.0;
        let mut r = Repeat::default();
        let mut t = 0.0;
        let mut fired = Vec::new();
        for _ in 0..120 {
            t += dt;
            if r.tick(dt) {
                fired.push(t);
            }
        }
        assert!((fired[0] - REPEAT_DELAY_S).abs() <= dt + 1e-4, "{fired:?}");
        for w in fired.windows(2) {
            assert!(
                (w[1] - w[0] - REPEAT_EVERY_S).abs() <= dt + 1e-4,
                "{fired:?}"
            );
        }
        assert_eq!(fired.len(), 7, "0.35, 0.45 … 0.95 s");
    }

    #[test]
    fn a_hitch_does_not_burst_repeats() {
        let mut r = Repeat::default();
        assert!(r.tick(2.0));
        assert!(!r.tick(0.01));
    }

    #[test]
    fn stick_needs_0_5_to_start_and_holds_until_below_0_3() {
        assert_eq!(stick_direction(None, Vec2::new(0.45, 0.0)), None);
        assert_eq!(stick_direction(None, Vec2::new(0.55, 0.1)), Some(Right));
        assert_eq!(
            stick_direction(Some(Right), Vec2::new(0.35, 0.0)),
            Some(Right)
        );
        assert_eq!(stick_direction(Some(Right), Vec2::new(0.25, 0.0)), None);
        assert_eq!(
            stick_direction(None, Vec2::new(0.0, 0.8)),
            Some(Up),
            "stick up = +Y"
        );
        assert_eq!(
            stick_direction(None, Vec2::new(0.6, -0.7)),
            Some(Down),
            "dominant axis"
        );
        assert_eq!(
            stick_direction(Some(Up), Vec2::new(0.9, 0.1)),
            Some(Right),
            "swing"
        );
    }

    #[test]
    fn the_key_table_follows_the_gdd() {
        let find = |key| KEYS.iter().find(|k| k.key == key).copied().expect("bound");
        assert_eq!(find(KeyCode::KeyW).action, NavAction::Up);
        assert_eq!(find(KeyCode::Enter).scheme, InputDevice::KeyboardB);
        assert_eq!(find(KeyCode::Space).scheme, InputDevice::KeyboardA);
        assert_eq!(find(KeyCode::ControlLeft).role, Role::LobbyOnly);
        assert_eq!(find(KeyCode::Escape).role, Role::Shared);
        for (i, a) in KEYS.iter().enumerate() {
            assert!(
                KEYS[i + 1..].iter().all(|b| b.key != a.key),
                "{:?} twice",
                a.key
            );
        }
    }
}
