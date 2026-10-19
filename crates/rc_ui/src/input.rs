//! `input.*` / `bindings.*` channels: device changes (glyphs, joins, disconnects), binding lists
//! and rebinding.

use bevy::prelude::*;
use bevy_react::prelude::*;
use rc_core::prelude::*;
use rc_game::GameCommand;

mirror_enum!(
    /// [`DeviceKind`]: device family for glyphs.
    UiDeviceKind = DeviceKind { KeyboardA, KeyboardB, Gamepad, Mouse }
);

mirror_enum!(
    /// [`ControlScheme`]: a rebindable scheme.
    UiScheme = ControlScheme { KeyboardA, KeyboardB, Gamepad }
);

mirror_enum!(
    /// [`DriveAction`]: a rebindable driving action.
    UiDriveAction = DriveAction {
        Throttle, Brake, SteerLeft, SteerRight, Drift, UseItem, Boost, LookBack, Reset, Camera,
        Pause,
    }
);

/// Device family of a concrete device.
pub fn device_kind(device: InputDevice) -> UiDeviceKind {
    match device {
        InputDevice::KeyboardA => UiDeviceKind::KeyboardA,
        InputDevice::KeyboardB => UiDeviceKind::KeyboardB,
        InputDevice::Gamepad(_) => UiDeviceKind::Gamepad,
    }
}

/// Bevy → React event `input.device`: a player's device changed (rc_input's `DeviceEvent`).
#[react_event(name = "input.device")]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(tag = "kind", rename_all = "camelCase")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceChange {
    /// A device joined a lobby slot.
    Joined {
        /// Slot 0–3.
        slot: u8,
        /// Its family.
        device: UiDeviceKind,
    },
    /// A player left the lobby.
    Left {
        /// Slot 0–3.
        slot: u8,
    },
    /// A player's gamepad disconnected (the race pauses).
    Disconnected {
        /// Slot 0–3.
        slot: u8,
    },
    /// A player used another device family.
    LastUsed {
        /// Slot 0–3.
        slot: u8,
        /// The family now in use.
        device: UiDeviceKind,
    },
}

impl From<&DeviceEvent> for DeviceChange {
    fn from(event: &DeviceEvent) -> Self {
        match *event {
            DeviceEvent::Joined { slot, device } => Self::Joined {
                slot: slot.0,
                device: device_kind(device),
            },
            DeviceEvent::Left { slot } => Self::Left { slot: slot.0 },
            DeviceEvent::Disconnected { slot } => Self::Disconnected { slot: slot.0 },
            DeviceEvent::LastUsed { slot, kind } => Self::LastUsed {
                slot: slot.0,
                device: kind.into(),
            },
        }
    }
}

/// One physical input, named as Bevy names it (`KeyCode` / `GamepadButton` / `GamepadAxis`
/// variant, e.g. `"KeyW"`, `"South"`, `"LeftStickX"`); React turns it into a glyph.
#[derive(serde::Serialize, ts_rs::TS, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(tag = "kind", rename_all = "camelCase")]
pub enum UiBinding {
    /// A keyboard key.
    Key {
        /// `KeyCode` variant (same names as the web's `KeyboardEvent.code`).
        code: String,
    },
    /// A gamepad button.
    Button {
        /// `GamepadButton` variant.
        button: String,
    },
    /// One direction of a gamepad axis.
    Axis {
        /// `GamepadAxis` variant.
        axis: String,
        /// The positive half.
        positive: bool,
    },
}

impl From<&Binding> for UiBinding {
    fn from(binding: &Binding) -> Self {
        match binding {
            Binding::Key(code) => Self::Key {
                code: format!("{code:?}"),
            },
            Binding::Button(button) => Self::Button {
                button: format!("{button:?}"),
            },
            Binding::Axis { axis, positive } => Self::Axis {
                axis: format!("{axis:?}"),
                positive: *positive,
            },
        }
    }
}

/// The bindings of one action.
#[derive(serde::Serialize, ts_rs::TS, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct UiActionBindings {
    /// The action.
    pub action: UiDriveAction,
    /// Its bindings (several allowed).
    pub bindings: Vec<UiBinding>,
}

/// Every action of one scheme, in menu order.
#[derive(serde::Serialize, ts_rs::TS, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct UiSchemeBindings {
    /// The scheme.
    pub scheme: UiScheme,
    /// Actions in [`DriveAction::ALL`] order.
    pub actions: Vec<UiActionBindings>,
}

/// Response of `bindings.get`; also the Bevy → React event `bindings.changed` (after a rebind or a
/// reset, once rc_game stored it).
#[react_event(name = "bindings.changed")]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiBindings {
    /// Keyboard A, keyboard B, gamepad.
    pub schemes: Vec<UiSchemeBindings>,
}

impl From<&ControlSettings> for UiBindings {
    fn from(controls: &ControlSettings) -> Self {
        let schemes = [
            ControlScheme::KeyboardA,
            ControlScheme::KeyboardB,
            ControlScheme::Gamepad,
        ];
        Self {
            schemes: schemes
                .into_iter()
                .map(|scheme| UiSchemeBindings {
                    scheme: scheme.into(),
                    actions: DriveAction::ALL
                        .into_iter()
                        .map(|action| UiActionBindings {
                            action: action.into(),
                            bindings: controls
                                .bindings(scheme)
                                .get(action)
                                .iter()
                                .map(UiBinding::from)
                                .collect(),
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}

/// React → Bevy request `bindings.get`: the current bindings of every scheme.
#[react_request(name = "bindings.get", response = UiBindings)]
pub struct GetBindings;

/// React → Bevy message `input.rebind`: capture the next press for `action` of `scheme`.
#[react_message(name = "input.rebind")]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct Rebind {
    /// Scheme.
    pub scheme: UiScheme,
    /// Action.
    pub action: UiDriveAction,
}

/// Another action that already uses a captured binding.
#[derive(serde::Serialize, ts_rs::TS, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct UiConflict {
    /// Its scheme.
    pub scheme: UiScheme,
    /// The action.
    pub action: UiDriveAction,
}

/// Bevy → React event `input.rebindResult`: the capture ended (rc_game stores the binding; on a
/// conflict the two actions swap).
#[react_event(name = "input.rebindResult")]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RebindDone {
    /// Scheme.
    pub scheme: UiScheme,
    /// Action.
    pub action: UiDriveAction,
    /// The captured binding (`null` = cancelled).
    pub binding: Option<UiBinding>,
    /// An action that already used it: the two swapped.
    pub conflict: Option<UiConflict>,
    /// The binding the conflicting action received (the rebound action's previous one); `null`
    /// with a conflict = that action lost the captured binding (it may be unbound now).
    pub previous: Option<UiBinding>,
}

impl From<&RebindResult> for RebindDone {
    fn from(result: &RebindResult) -> Self {
        Self {
            scheme: result.scheme.into(),
            action: result.action.into(),
            binding: result.binding.as_ref().map(UiBinding::from),
            conflict: result.conflict.map(|(scheme, action)| UiConflict {
                scheme: scheme.into(),
                action: action.into(),
            }),
            previous: result.previous.as_ref().map(UiBinding::from),
        }
    }
}

/// React → Bevy message `input.resetBindings`: restore the default bindings of `scheme`.
#[react_message(name = "input.resetBindings")]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ResetBindings {
    /// Scheme to reset.
    pub scheme: UiScheme,
}

/// React → Bevy message `input.dropPlayer`: a disconnected player leaves; the AI takes over.
#[react_message(name = "input.dropPlayer")]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct DropPlayer {
    /// Slot 0–3.
    pub slot: u8,
}

pub(crate) fn register(app: &mut App) {
    app.add_react_event::<DeviceChange>()
        .add_react_event::<RebindDone>()
        .add_react_event::<UiBindings>()
        .add_react_request_handler(
            |request: On<Request<GetBindings>>, settings: Option<Res<Settings>>| match settings {
                Some(settings) => request.respond(UiBindings::from(&settings.controls)),
                None => request.respond_err("settings not loaded"),
            },
        )
        .add_react_handler(
            |rebind: On<Rebind>, mut out: MessageWriter<RebindRequest>| {
                let rebind = rebind.event();
                out.write(RebindRequest {
                    scheme: rebind.scheme.into(),
                    action: rebind.action.into(),
                });
            },
        )
        .add_react_handler(
            |reset: On<ResetBindings>, mut out: MessageWriter<GameCommand>| {
                out.write(GameCommand::ResetBindings(reset.event().scheme.into()));
            },
        )
        .add_react_handler(
            |drop: On<DropPlayer>, mut out: MessageWriter<GameCommand>| {
                let slot = drop.event().slot.min(MAX_LOCAL_PLAYERS as u8 - 1);
                out.write(GameCommand::DropPlayer(PlayerSlot(slot)));
            },
        );
}

/// Forwards rc_input's `DeviceEvent`s as `input.device`.
pub(crate) fn forward_devices(mut devices: MessageReader<DeviceEvent>, events: ReactEvents) {
    for device in devices.read() {
        events.send(&DeviceChange::from(device));
    }
}

/// Sends `bindings.changed` when rc_game changed the bindings.
pub(crate) fn push_bindings(
    settings: Option<Res<Settings>>,
    mut last: Local<Option<ControlSettings>>,
    events: ReactEvents,
) {
    let Some(settings) = settings.filter(|s| s.is_changed()) else {
        return;
    };
    if last.as_ref() != Some(&settings.controls) {
        // The first value is what `bindings.get` reports; only changes are pushed.
        if last.is_some() {
            events.send(&UiBindings::from(&settings.controls));
        }
        *last = Some(settings.controls.clone());
    }
}

/// Forwards rc_input's `RebindResult`s as `input.rebindResult`.
pub(crate) fn forward_rebinds(mut results: MessageReader<RebindResult>, events: ReactEvents) {
    for result in results.read() {
        events.send(&RebindDone::from(result));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bindings_list_every_action_of_every_scheme_by_bevy_name() {
        let bindings = UiBindings::from(&ControlSettings::default());
        assert_eq!(bindings.schemes.len(), 3);
        for scheme in &bindings.schemes {
            assert_eq!(scheme.actions.len(), DriveAction::ALL.len());
        }
        let throttle = &bindings.schemes[0].actions[0];
        assert_eq!(throttle.action, UiDriveAction::Throttle);
        assert_eq!(
            throttle.bindings,
            [UiBinding::Key {
                code: "KeyW".into()
            }]
        );
        let steer_left = &bindings.schemes[2].actions[2].bindings;
        assert_eq!(
            steer_left[0],
            UiBinding::Axis {
                axis: "LeftStickX".into(),
                positive: false
            }
        );
        assert_eq!(
            steer_left[1],
            UiBinding::Button {
                button: "DPadLeft".into()
            }
        );
    }
}
