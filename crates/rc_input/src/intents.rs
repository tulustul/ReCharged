//! Driving intents (ARCHITECTURE §3.2, §5.3). PreUpdate reads every device into per-slot intents:
//! levels, plus edges latched only while driving (race running, Countdown/Racing, no menu or
//! text field); also pause requests, camera cycling and the last-used device. FixedUpdate
//! `SimSet::Drivers` writes `CarControl` of local human racers and consumes each edge in exactly
//! one tick. Latched edges are dropped on entering `PauseState::Running` and `RacePhase::Setup`.

use bevy::ecs::system::SystemParam;
use bevy::input::gamepad::Gamepad;
use bevy::input_focus::InputFocus;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::text::EditableText;
use rc_core::prelude::*;

use crate::bindings::{
    ACTIONS, ActionFrame, PRESS_THRESHOLD, device_frame, merge_frames, ramp, steer_inputs,
};
use crate::players::{Roster, kind_of};
use crate::rebind::RebindCapture;

/// A slot's driving intent: levels from the latest frame, the digital steering ramp (advanced per
/// fixed tick) and the latched `use_item` edge.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SlotIntent {
    /// Throttle 0–1 (1 with auto-accelerate after GO unless braking).
    pub throttle: f32,
    /// Brake / reverse 0–1.
    pub brake: f32,
    /// Shaped stick steering −1…1.
    pub steer_analog: f32,
    /// Digital steering target −1, 0, 1.
    pub steer_target: f32,
    /// Digital steering after the ramp.
    pub steer_ramped: f32,
    /// Drift held.
    pub drift: bool,
    /// Boost held.
    pub boost: bool,
    /// Look back held.
    pub look_back: bool,
    /// Reset held (rc_race times it).
    pub reset: bool,
    /// Use item pressed since the last fixed tick.
    pub use_item: bool,
}

/// Every slot's intent. Written by rc_input only.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct Intents {
    /// Indexed by `PlayerSlot::index()`.
    pub slots: [SlotIntent; MAX_LOCAL_PLAYERS],
}

impl Intents {
    /// Drops every latched edge.
    pub fn clear_edges(&mut self) {
        for slot in &mut self.slots {
            slot.use_item = false;
        }
    }
}

/// Levels of a slot from its merged frame.
pub fn levels(
    frame: &ActionFrame,
    controls: &ControlSettings,
    auto_accelerate: bool,
) -> SlotIntent {
    let v = |a: DriveAction| frame[a as usize];
    let (steer_analog, steer_target) =
        steer_inputs(frame, controls.steer_dead_zone, controls.steer_sensitivity);
    let brake = v(DriveAction::Brake).value();
    let throttle = if auto_accelerate && brake < PRESS_THRESHOLD {
        1.0
    } else {
        v(DriveAction::Throttle).value()
    };
    SlotIntent {
        throttle,
        brake,
        steer_analog,
        steer_target,
        drift: v(DriveAction::Drift).pressed(),
        boost: v(DriveAction::Boost).pressed(),
        look_back: v(DriveAction::LookBack).pressed(),
        reset: v(DriveAction::Reset).pressed(),
        ..default()
    }
}

/// One fixed tick of a slot: ramps the digital steering and consumes the latched edge.
pub fn tick(intent: &mut SlotIntent) -> CarControl {
    intent.steer_ramped = ramp(intent.steer_ramped, intent.steer_target, SIM_DT);
    CarControl {
        throttle: intent.throttle,
        brake: intent.brake,
        steer: (intent.steer_analog + intent.steer_ramped).clamp(-1.0, 1.0),
        drift: intent.drift,
        boost: intent.boost,
        use_item: std::mem::take(&mut intent.use_item),
        look_back: intent.look_back,
        reset_held: intent.reset,
    }
}

/// Edges of a frame against the held bits of the previous frame; updates `held`.
pub fn edges(frame: &ActionFrame, held: &mut u16) -> u16 {
    let mut pressed = 0u16;
    let mut tapped = 0u16;
    for (i, a) in frame.iter().enumerate() {
        pressed |= u16::from(a.pressed()) << i;
        tapped |= u16::from(a.tapped) << i;
    }
    let edges = (pressed & !*held) | tapped;
    *held = pressed;
    edges
}

fn has(mask: u16, action: DriveAction) -> bool {
    mask & (1 << action as usize) != 0
}

/// The session lifecycle as rc_input sees it.
#[derive(SystemParam)]
pub(crate) struct Flow<'w> {
    phase: Option<Res<'w, State<RacePhase>>>,
    pause: Option<Res<'w, State<PauseState>>>,
}

impl Flow<'_> {
    fn phase(&self) -> Option<RacePhase> {
        self.phase.as_ref().map(|p| *p.get())
    }

    /// In a race and not paused.
    pub fn running(&self) -> bool {
        self.pause
            .as_ref()
            .is_some_and(|p| *p.get() == PauseState::Running)
    }

    /// The Pause action opens the pause menu (not during Setup or the results).
    pub fn pausable(&self) -> bool {
        self.running()
            && matches!(
                self.phase(),
                Some(RacePhase::Intro | RacePhase::Countdown | RacePhase::Racing)
            )
    }

    /// Driving input reaches the car.
    pub fn driving(&self) -> bool {
        self.running() && matches!(self.phase(), Some(RacePhase::Countdown | RacePhase::Racing))
    }

    /// After GO (auto-accelerate may push the throttle).
    pub fn racing(&self) -> bool {
        self.phase() == Some(RacePhase::Racing)
    }
}

/// Whether a bevy-react text field has the input focus.
#[derive(SystemParam)]
pub(crate) struct Typing<'w, 's> {
    focus: Option<Res<'w, InputFocus>>,
    fields: Query<'w, 's, (), With<EditableText>>,
}

impl Typing<'_, '_> {
    /// A text field is focused.
    pub fn active(&self) -> bool {
        self.focus
            .as_ref()
            .and_then(|f| f.get())
            .is_some_and(|e| self.fields.contains(e))
    }
}

/// Raw device state.
#[derive(SystemParam)]
pub(crate) struct Devices<'w, 's> {
    keys: Option<Res<'w, ButtonInput<KeyCode>>>,
    pads: Query<'w, 's, &'static Gamepad>,
}

/// Messages rc_input produces from driving actions.
#[derive(SystemParam)]
pub(crate) struct Outputs<'w> {
    pause: MessageWriter<'w, PauseRequest>,
    camera: MessageWriter<'w, CycleCamera>,
    devices: MessageWriter<'w, DeviceEvent>,
}

/// PreUpdate `InputSet::Collect`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn collect(
    devices: Devices,
    flow: Flow,
    typing: Typing,
    capture: Res<RebindCapture>,
    settings: Option<Res<Settings>>,
    mut roster: ResMut<Roster>,
    mut intents: ResMut<Intents>,
    mut held: Local<HashMap<InputDevice, u16>>,
    mut device_list: Local<Vec<InputDevice>>,
    mut out: Outputs,
) {
    let empty = ButtonInput::<KeyCode>::default();
    let keys = devices.keys.as_deref().unwrap_or(&empty);
    let defaults;
    let controls = match settings.as_deref() {
        Some(s) => &s.controls,
        None => {
            defaults = ControlSettings::default();
            &defaults
        }
    };
    let auto = settings
        .as_ref()
        .is_some_and(|s| s.accessibility.auto_accelerate);
    let blocked = capture.busy || typing.active();
    let mut merged: [Option<(ActionFrame, u16)>; MAX_LOCAL_PLAYERS] = [None; MAX_LOCAL_PLAYERS];
    let mut paused = false;
    device_list.clear();
    device_list.extend(roster.devices());
    for &device in device_list.iter() {
        let (bindings, pad) = match device {
            InputDevice::KeyboardA => (&controls.keyboard_a, None),
            InputDevice::KeyboardB => (&controls.keyboard_b, None),
            InputDevice::Gamepad(e) => match devices.pads.get(e) {
                Ok(pad) => (&controls.gamepad, Some(pad)),
                Err(_) => continue,
            },
        };
        let frame = device_frame(bindings, keys, pad);
        let edge = edges(&frame, held.entry(device).or_default());
        if blocked {
            continue;
        }
        let slot = roster.slot_of(device);
        if has(edge, DriveAction::Pause) && flow.pausable() && !paused {
            paused = true;
            out.pause.write(PauseRequest { slot });
        }
        let Some(slot) = slot else {
            continue;
        };
        let active = edge != 0 || frame.iter().any(|a| a.pressed());
        if active
            && roster.last_used(slot) != Some(kind_of(device))
            && roster.set_last_used(slot, kind_of(device))
        {
            out.devices.write(DeviceEvent::LastUsed {
                slot,
                kind: kind_of(device),
            });
        }
        let Some(entry) = merged.get_mut(slot.index()) else {
            continue;
        };
        let entry = entry.get_or_insert(([default(); ACTIONS], 0));
        merge_frames(&mut entry.0, &frame);
        entry.1 |= edge;
    }
    for (i, intent) in intents.slots.iter_mut().enumerate() {
        let ramped = intent.steer_ramped;
        let latched = intent.use_item;
        match merged[i] {
            Some((frame, edge)) if flow.driving() => {
                *intent = levels(&frame, controls, auto && flow.racing());
                intent.use_item = latched || has(edge, DriveAction::UseItem);
            }
            _ => {
                *intent = SlotIntent {
                    use_item: latched,
                    ..default()
                }
            }
        }
        intent.steer_ramped = ramped;
        if let Some((_, edge)) = merged[i]
            && has(edge, DriveAction::Camera)
            && flow.running()
        {
            out.camera.write(CycleCamera {
                slot: PlayerSlot(i as u8),
            });
        }
    }
}

/// Local human racers (autopilot humans carry an `AiDriver`: rc_ai drives them).
type HumanRacers = (With<Racer>, Without<AiDriver>);

/// FixedUpdate `SimSet::Drivers`: `CarControl` of every local human racer.
pub(crate) fn drive_cars(
    mut intents: ResMut<Intents>,
    mut racers: Query<(&LocalPlayer, &mut CarControl), HumanRacers>,
) {
    for (local, mut control) in &mut racers {
        if let Some(intent) = intents.slots.get_mut(local.0.index()) {
            *control = tick(intent);
        }
    }
}

/// `OnEnter(PauseState::Running)`: a press latched before the pause must not fire after it.
pub(crate) fn clear_edges(mut intents: ResMut<Intents>) {
    intents.clear_edges();
}

/// `OnEnter(RacePhase::Setup)`: a new attempt starts from neutral (edges and steering ramp).
pub(crate) fn reset(mut intents: ResMut<Intents>) {
    *intents = Intents::default();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bindings::ActionValue;

    fn frame(pressed: &[DriveAction]) -> ActionFrame {
        let mut f = [ActionValue::default(); ACTIONS];
        for &a in pressed {
            f[a as usize].digital = 1.0;
        }
        f
    }

    #[test]
    fn edges_fire_once_per_press() {
        let mut held = 0;
        let use_item = 1 << DriveAction::UseItem as usize;
        assert_eq!(edges(&frame(&[DriveAction::UseItem]), &mut held), use_item);
        assert_eq!(edges(&frame(&[DriveAction::UseItem]), &mut held), 0, "held");
        assert_eq!(edges(&frame(&[]), &mut held), 0);
        let mut tap = frame(&[]);
        tap[DriveAction::UseItem as usize].tapped = true; // down and up within one frame
        assert_eq!(edges(&tap, &mut held), use_item);
    }

    #[test]
    fn a_latched_edge_survives_a_frame_without_ticks_and_fires_in_one_tick_only() {
        let mut intent = SlotIntent {
            use_item: true,
            ..default()
        };
        // frame 1: no fixed tick (high frame rate) — nothing consumes it.
        assert!(intent.use_item);
        // frame 2: two fixed ticks.
        assert!(tick(&mut intent).use_item);
        assert!(!tick(&mut intent).use_item);
    }

    #[test]
    fn levels_map_actions_and_auto_accelerate_yields_to_the_brake() {
        let controls = ControlSettings::default();
        let i = levels(
            &frame(&[
                DriveAction::Throttle,
                DriveAction::Drift,
                DriveAction::Reset,
            ]),
            &controls,
            false,
        );
        assert_eq!((i.throttle, i.brake), (1.0, 0.0));
        assert!(i.drift && i.reset && !i.boost && !i.use_item);
        let idle = levels(&frame(&[]), &controls, true);
        assert_eq!(idle.throttle, 1.0, "auto-accelerate");
        let braking = levels(&frame(&[DriveAction::Brake]), &controls, true);
        assert_eq!((braking.throttle, braking.brake), (0.0, 1.0));
    }

    #[test]
    fn digital_steering_ramps_in_ticks_and_adds_to_the_stick() {
        let mut intent = SlotIntent {
            steer_target: 1.0,
            ..default()
        };
        let first = tick(&mut intent).steer;
        assert!(first > 0.0 && first < 0.2, "{first}");
        for _ in 0..10 {
            tick(&mut intent);
        }
        assert_eq!(tick(&mut intent).steer, 1.0);
        intent.steer_analog = -0.5;
        assert_eq!(tick(&mut intent).steer, 0.5);
    }
}
