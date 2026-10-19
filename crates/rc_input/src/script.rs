//! `--input-script <file>` (ARCHITECTURE §8, DoD-1): a scripted virtual device. Steps press keys of
//! both keyboard schemes and buttons/sticks of virtual gamepads in `ButtonInput<KeyCode>` /
//! `Gamepad` — the layer real devices feed — right before `InputSet::Collect`, so bindings, the
//! roster (lobby joins), latching and `UiNav` suppression apply exactly as for a player. The host
//! (the binary) loads the file ([`InputScript::load`]), sets [`InputScript::ui_ready`] and
//! [`InputScript::screen`], saves [`ScriptShot`]s and honours [`InputScript::exit_requested`].
//!
//! One step per line, `#` starts a comment:
//! ```text
//! [<when>] <step>
//! when     <secs> after the previous step (wall clock, default 0) | @<tick> once the race clock
//!          (RaceStatus.race_ticks) reaches <tick>
//! wait     ui | menu | loading | race | intro | countdown | racing | finished | paused | running
//!          | screen <name>   (the screen React reports: a menu route such as `title`, `main`,
//!          `trackSelect`, or `loading`, `race`, `pause`, `results`, `standings`, `score`,
//!          `changeTrack`)
//! nav      <device> up|down|left|right|confirm|back|prevTab|nextTab    tap the fixed menu key
//!          (≥ 4 frames after the previous step, so React keeps up at low frame rates)
//! hold     <device> <control> <value>   until changed: steer -1…1, the others 0…1
//! tap      <device> <control>           held for 0.1 s (not steer)
//! release  <device>                     let go of everything on the device
//! shot     <name>                       screenshot request
//! exit                                  ask the host to quit
//! device   keyboardA | keyboardB | gamepad (= gamepad1) | gamepad2 … gamepad4
//! control  throttle brake steer drift item boost lookBack reset camera pause
//! ```
//! A control presses the device's first binding of that action in `Settings.controls` (a key is
//! down from 0.5; pad buttons and sticks take the value). Two taps of one input closer than
//! 0.1 s merge into one press.

use std::path::Path;

use bevy::ecs::system::SystemParam;
use bevy::input::InputSystems;
use bevy::input::gamepad::{Gamepad, GamepadAxis, GamepadButton, GamepadInput};
use bevy::prelude::*;
use rc_core::prelude::*;

use crate::bindings::{ACTIONS, PRESS_THRESHOLD};

/// A tap holds its input this long (s): long enough for fixed ticks to see it (reset taps).
const TAP_SECS: f32 = 0.1;
/// A virtual pad button reads as pressed from this value (Bevy's default press threshold).
const PAD_PRESS: f32 = 0.75;
/// A `nav` step waits at least this many frames after the previous step: React needs a few
/// frames to show the screen a step leads to (a dialog, the pause menu), and at low frame rates
/// (a blanked monitor stalling vsync, software rendering in CI) wall-clock delays alone outrun it.
const NAV_GAP_FRAMES: u32 = 4;

/// A screenshot the script asks for (`shot <name>`); the host saves it.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct ScriptShot(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Device {
    KeyboardA,
    KeyboardB,
    /// Virtual gamepad `n + 1`.
    Pad(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Control {
    Action(DriveAction),
    Steer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Cond {
    Ui,
    App(AppState),
    Phase(RacePhase),
    Pause(PauseState),
    /// React shows this screen.
    Screen(String),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum When {
    After(f32),
    Tick(u32),
}

#[derive(Debug, Clone, PartialEq)]
enum Action {
    Wait(Cond),
    Nav(Device, NavAction),
    Hold(Device, Control, f32),
    Tap(Device, DriveAction),
    Release(Device),
    Shot(String),
    Exit,
}

#[derive(Debug, Clone, PartialEq)]
struct Step {
    line: usize,
    text: String,
    when: When,
    action: Action,
}

/// A physical input the script holds down: a key, or a pad button/axis at a value.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Input {
    Key(KeyCode),
    Pad(u8, GamepadInput, f32),
}

/// A parsed `--input-script` and its progress. Inserted by the host; rc_input plays it.
#[derive(Resource, Debug, Clone, Default)]
pub struct InputScript {
    steps: Vec<Step>,
    next: usize,
    /// Wall-clock time the previous step finished.
    last: Option<f32>,
    /// Frames since the previous step.
    frames: u32,
    /// Held control levels per device, indexed by `DriveAction as usize`.
    levels: Vec<(Device, [f32; ACTIONS])>,
    /// Tapped inputs and when they come up.
    taps: Vec<(Device, Input, f32)>,
    /// Keys the script held in the previous frame.
    keys: Vec<KeyCode>,
    /// Virtual gamepads, by number − 1.
    pads: Vec<Entity>,
    /// The host's UI is ready for input (`wait ui`). Written by the host.
    pub ui_ready: bool,
    /// The screen the UI shows (`wait screen <name>`). Written by the host.
    pub screen: String,
    exit: bool,
}

impl InputScript {
    /// Reads and parses a script file; errors name the file and line.
    pub fn load(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Self::parse(&text).map_err(|e| format!("{}:{e}", path.display()))
    }

    /// Parses a script; errors start with the line number.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut steps = Vec::new();
        for (i, raw) in text.lines().enumerate() {
            let line = raw.split('#').next().unwrap_or_default().trim();
            if line.is_empty() {
                continue;
            }
            let (when, action) = parse_step(line).map_err(|e| format!("{}: {e}", i + 1))?;
            steps.push(Step {
                line: i + 1,
                text: line.to_owned(),
                when,
                action,
            });
        }
        Ok(Self { steps, ..default() })
    }

    /// Every step has run.
    pub fn finished(&self) -> bool {
        self.next >= self.steps.len()
    }

    /// The step the script is at (`line N: <step> (screen: <name>)`), if any.
    pub fn pending(&self) -> Option<String> {
        let step = self.steps.get(self.next)?;
        let screen = if self.screen.is_empty() {
            "none reported"
        } else {
            &self.screen
        };
        Some(format!(
            "line {}: {} (screen: {screen})",
            step.line, step.text
        ))
    }

    /// An `exit` step ran.
    pub fn exit_requested(&self) -> bool {
        self.exit
    }

    fn levels_mut(&mut self, device: Device) -> &mut [f32; ACTIONS] {
        let i = match self.levels.iter().position(|l| l.0 == device) {
            Some(i) => i,
            None => {
                self.levels.push((device, [0.0; ACTIONS]));
                self.levels.len() - 1
            }
        };
        &mut self.levels[i].1
    }

    fn execute(&mut self, action: Action, now: f32, controls: &ControlSettings) -> Option<String> {
        match action {
            Action::Wait(_) => {}
            Action::Nav(device, nav) => {
                self.taps
                    .push((device, nav_input(device, nav), now + TAP_SECS))
            }
            Action::Hold(device, Control::Action(action), v) => {
                self.levels_mut(device)[action as usize] = v
            }
            Action::Hold(device, Control::Steer, v) => {
                let levels = self.levels_mut(device);
                levels[DriveAction::SteerLeft as usize] = (-v).max(0.0);
                levels[DriveAction::SteerRight as usize] = v.max(0.0);
            }
            Action::Tap(device, action) => match bound_input(controls, device, action, 1.0) {
                Some(input) => self.taps.push((device, input, now + TAP_SECS)),
                None => warn!("--input-script: {action:?} has no binding on {device:?}"),
            },
            Action::Release(device) => {
                self.levels.retain(|l| l.0 != device);
                self.taps.retain(|t| t.0 != device);
            }
            Action::Shot(name) => return Some(name),
            Action::Exit => self.exit = true,
        }
        None
    }
}

fn lookup<T: Clone>(table: &[(&str, T)], word: &str, what: &str) -> Result<T, String> {
    let names = || table.iter().map(|e| e.0).collect::<Vec<_>>().join(" ");
    let found = table.iter().find(|e| e.0 == word).map(|e| e.1.clone());
    found.ok_or_else(|| format!("unknown {what} `{word}` (one of: {})", names()))
}

const DEVICES: [(&str, Device); 6] = [
    ("keyboardA", Device::KeyboardA),
    ("keyboardB", Device::KeyboardB),
    ("gamepad", Device::Pad(0)),
    ("gamepad2", Device::Pad(1)),
    ("gamepad3", Device::Pad(2)),
    ("gamepad4", Device::Pad(3)),
];

const NAV: [(&str, NavAction); 8] = [
    ("up", NavAction::Up),
    ("down", NavAction::Down),
    ("left", NavAction::Left),
    ("right", NavAction::Right),
    ("confirm", NavAction::Confirm),
    ("back", NavAction::Back),
    ("prevTab", NavAction::PrevTab),
    ("nextTab", NavAction::NextTab),
];

const CONTROLS: [(&str, Control); 10] = [
    ("throttle", Control::Action(DriveAction::Throttle)),
    ("brake", Control::Action(DriveAction::Brake)),
    ("steer", Control::Steer),
    ("drift", Control::Action(DriveAction::Drift)),
    ("item", Control::Action(DriveAction::UseItem)),
    ("boost", Control::Action(DriveAction::Boost)),
    ("lookBack", Control::Action(DriveAction::LookBack)),
    ("reset", Control::Action(DriveAction::Reset)),
    ("camera", Control::Action(DriveAction::Camera)),
    ("pause", Control::Action(DriveAction::Pause)),
];

const CONDS: [(&str, Cond); 10] = [
    ("ui", Cond::Ui),
    ("menu", Cond::App(AppState::Menu)),
    ("loading", Cond::App(AppState::Loading)),
    ("race", Cond::App(AppState::InRace)),
    ("intro", Cond::Phase(RacePhase::Intro)),
    ("countdown", Cond::Phase(RacePhase::Countdown)),
    ("racing", Cond::Phase(RacePhase::Racing)),
    ("finished", Cond::Phase(RacePhase::Finished)),
    ("paused", Cond::Pause(PauseState::Paused)),
    ("running", Cond::Pause(PauseState::Running)),
];

const USAGE: [(&str, &str); 7] = [
    ("wait", "wait <condition> | wait screen <name>"),
    ("nav", "nav <device> <action>"),
    ("hold", "hold <device> <control> <value>"),
    ("tap", "tap <device> <control>"),
    ("release", "release <device>"),
    ("shot", "shot <name>"),
    ("exit", "exit"),
];

fn parse_step(line: &str) -> Result<(When, Action), String> {
    let words: Vec<&str> = line.split_whitespace().collect();
    let (when, rest) = match words[0] {
        w if w.starts_with('@') => {
            let tick = w[1..].parse().map_err(|_| format!("bad race tick `{w}`"))?;
            (When::Tick(tick), &words[1..])
        }
        w => match w.parse::<f32>() {
            Ok(secs) if secs.is_finite() && secs >= 0.0 => (When::After(secs), &words[1..]),
            Ok(_) => return Err(format!("bad delay `{w}`")),
            Err(_) => (When::After(0.0), &words[..]),
        },
    };
    let action = match rest {
        ["wait", "screen", name] => Action::Wait(Cond::Screen((*name).to_owned())),
        ["wait", "screen"] => return Err("usage: wait screen <name>".into()),
        ["wait", c] => Action::Wait(lookup(&CONDS, c, "condition")?),
        ["nav", d, a] => Action::Nav(
            lookup(&DEVICES, d, "device")?,
            lookup(&NAV, a, "nav action")?,
        ),
        ["hold", d, c, v] => {
            let control = lookup(&CONTROLS, c, "control")?;
            let min = if control == Control::Steer { -1.0 } else { 0.0 };
            let value = v.parse::<f32>().ok().filter(|v| (min..=1.0).contains(v));
            let value = value.ok_or_else(|| format!("{c} takes a value {min}…1, got `{v}`"))?;
            Action::Hold(lookup(&DEVICES, d, "device")?, control, value)
        }
        ["tap", d, c] => match lookup(&CONTROLS, c, "control")? {
            Control::Action(action) => Action::Tap(lookup(&DEVICES, d, "device")?, action),
            Control::Steer => return Err("steering cannot be tapped: hold it".into()),
        },
        ["release", d] => Action::Release(lookup(&DEVICES, d, "device")?),
        ["shot", name] => Action::Shot((*name).to_owned()),
        ["exit"] => Action::Exit,
        [] => return Err("a time needs a step".into()),
        [verb, ..] => {
            return Err(match USAGE.iter().find(|u| u.0 == *verb) {
                Some((_, usage)) => format!("usage: {usage}"),
                None => format!("unknown step `{verb}`"),
            });
        }
    };
    Ok((when, action))
}

/// The fixed menu key/button of `nav` (GDD §12.2; scheme B shares Q/E for tabs).
fn nav_input(device: Device, nav: NavAction) -> Input {
    use GamepadButton as B;
    use KeyCode as K;
    let (a, b, pad) = match nav {
        NavAction::Up => (K::KeyW, K::ArrowUp, B::DPadUp),
        NavAction::Down => (K::KeyS, K::ArrowDown, B::DPadDown),
        NavAction::Left => (K::KeyA, K::ArrowLeft, B::DPadLeft),
        NavAction::Right => (K::KeyD, K::ArrowRight, B::DPadRight),
        NavAction::Confirm => (K::Space, K::Enter, B::South),
        NavAction::Back => (K::Escape, K::Backspace, B::East),
        NavAction::PrevTab => (K::KeyQ, K::KeyQ, B::LeftTrigger),
        NavAction::NextTab => (K::KeyE, K::KeyE, B::RightTrigger),
    };
    match device {
        Device::KeyboardA => Input::Key(a),
        Device::KeyboardB => Input::Key(b),
        Device::Pad(n) => Input::Pad(n, pad.into(), 1.0),
    }
}

/// The input that drives `action` on `device` at `value` (`None`: released or unbound).
fn bound_input(
    controls: &ControlSettings,
    device: Device,
    action: DriveAction,
    value: f32,
) -> Option<Input> {
    let scheme = match device {
        Device::KeyboardA => ControlScheme::KeyboardA,
        Device::KeyboardB => ControlScheme::KeyboardB,
        Device::Pad(_) => ControlScheme::Gamepad,
    };
    let bindings = controls.bindings(scheme).get(action);
    bindings
        .iter()
        .find_map(|&binding| match (binding, device) {
            (Binding::Key(key), Device::KeyboardA | Device::KeyboardB) => {
                (value >= PRESS_THRESHOLD).then_some(Input::Key(key))
            }
            (Binding::Button(button), Device::Pad(n)) => {
                (value > 0.0).then_some(Input::Pad(n, button.into(), value))
            }
            (Binding::Axis { axis, positive }, Device::Pad(n)) => {
                let v = if positive { value } else { -value };
                (value > 0.0).then_some(Input::Pad(n, axis.into(), v))
            }
            _ => None,
        })
}

/// The clock and the lifecycle the script waits on.
#[derive(SystemParam)]
pub(crate) struct Moment<'w> {
    time: Res<'w, Time<Real>>,
    app: Option<Res<'w, State<AppState>>>,
    phase: Option<Res<'w, State<RacePhase>>>,
    pause: Option<Res<'w, State<PauseState>>>,
    status: Option<Res<'w, RaceStatus>>,
}

impl Moment<'_> {
    fn holds(&self, cond: &Cond, script: &InputScript) -> bool {
        match cond {
            Cond::Ui => script.ui_ready,
            Cond::App(s) => self.app.as_ref().is_some_and(|a| a.get() == s),
            Cond::Phase(p) => self.phase.as_ref().is_some_and(|a| a.get() == p),
            Cond::Pause(p) => self.pause.as_ref().is_some_and(|a| a.get() == p),
            Cond::Screen(name) => script.screen == *name,
        }
    }

    fn due(&self, when: When, last: f32) -> bool {
        match when {
            When::After(secs) => self.time.elapsed_secs() - last >= secs,
            When::Tick(tick) => self.status.as_ref().is_some_and(|s| s.race_ticks >= tick),
        }
    }
}

pub(crate) fn build(app: &mut App) {
    let scripted = resource_exists::<InputScript>;
    app.add_message::<ScriptShot>()
        .add_systems(Startup, spawn_pads.run_if(scripted))
        .add_systems(
            PreUpdate,
            play.run_if(scripted)
                .after(InputSystems)
                .before(InputSet::Collect),
        );
}

/// The virtual gamepads the script uses, connected from the start.
fn spawn_pads(mut commands: Commands, mut script: ResMut<InputScript>) {
    let used = script.steps.iter().filter_map(|s| match s.action {
        Action::Nav(Device::Pad(n), _)
        | Action::Hold(Device::Pad(n), ..)
        | Action::Tap(Device::Pad(n), _)
        | Action::Release(Device::Pad(n)) => Some(n + 1),
        _ => None,
    });
    let count = used.max().unwrap_or(0);
    script.pads = (1..=count)
        .map(|n| {
            let name = Name::new(format!("script gamepad {n}"));
            commands.spawn((name, Gamepad::default())).id()
        })
        .collect();
}

/// PreUpdate, before `InputSet::Collect`: runs due steps, then presses what the script holds.
fn play(
    mut script: ResMut<InputScript>,
    moment: Moment,
    settings: Option<Res<Settings>>,
    keys: Option<ResMut<ButtonInput<KeyCode>>>,
    mut pads: Query<&mut Gamepad>,
    mut shots: MessageWriter<ScriptShot>,
    mut held: Local<Vec<Input>>,
) {
    let defaults = ControlSettings::default();
    let controls = settings.as_deref().map_or(&defaults, |s| &s.controls);
    let now = moment.time.elapsed_secs();
    let script = &mut *script;
    script.taps.retain(|t| t.2 > now);
    script.frames = script.frames.saturating_add(1);
    let mut last = *script.last.get_or_insert(now);
    while let Some(step) = script.steps.get(script.next) {
        let waiting = matches!(&step.action, Action::Wait(c) if !moment.holds(c, script));
        let early = matches!(step.action, Action::Nav(..)) && script.frames < NAV_GAP_FRAMES;
        if !moment.due(step.when, last) || waiting || early {
            break;
        }
        info!("--input-script line {}: {}", step.line, step.text);
        let action = step.action.clone();
        if let Some(name) = script.execute(action, now, controls) {
            shots.write(ScriptShot(name));
        }
        script.next += 1;
        script.frames = 0;
        last = now;
    }
    script.last = Some(last);

    held.clear();
    held.extend(script.taps.iter().map(|t| t.1));
    for (device, levels) in &script.levels {
        for (action, &v) in DriveAction::ALL.iter().zip(levels) {
            held.extend(bound_input(controls, *device, *action, v));
        }
    }
    if let Some(mut keys) = keys {
        for &key in &script.keys {
            if !held.contains(&Input::Key(key)) {
                keys.release(key);
            }
        }
        script.keys.clear();
        for input in held.iter() {
            if let Input::Key(key) = *input {
                keys.press(key); // again every frame: a focus loss releases every key
                script.keys.push(key);
            }
        }
    }
    for (n, &entity) in script.pads.iter().enumerate() {
        if let Ok(mut pad) = pads.get_mut(entity) {
            set_pad(&mut pad, n as u8, &held);
        }
    }
}

/// Writes the held values of virtual pad `n`; everything else rests at 0. Mutates (and marks
/// changed) only what differs.
fn set_pad(pad: &mut Mut<Gamepad>, n: u8, held: &[Input]) {
    let value = |input: GamepadInput| -> f32 {
        let values = held.iter().filter_map(|h| match *h {
            Input::Pad(p, i, v) if p == n && i == input => Some(v),
            _ => None,
        });
        values.sum::<f32>().clamp(-1.0, 1.0)
    };
    for button in GamepadButton::all() {
        let v = value(button.into());
        if pad.get(button) != Some(v) {
            pad.analog_mut().set(button, v);
        }
        match (v >= PAD_PRESS, pad.pressed(button)) {
            (true, false) => pad.digital_mut().press(button),
            (false, true) => pad.digital_mut().release(button),
            _ => {}
        }
    }
    for axis in GamepadAxis::all() {
        let v = value(axis.into());
        if pad.get(axis) != Some(v) {
            pad.analog_mut().set(axis, v);
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::input::InputPlugin;
    use bevy::state::app::StatesPlugin;
    use bevy::time::TimeUpdateStrategy;

    use super::*;

    #[test]
    fn scripts_parse_with_times_ticks_and_comments() {
        let script = InputScript::parse(
            "# title\nwait ui\n1.5 nav keyboardA confirm  # go\n@120 hold gamepad2 steer -0.5\n\
             tap keyboardB item\nrelease gamepad\nshot menu\n0.5 exit\nwait screen trackSelect\n",
        )
        .expect("valid");
        let s = &script.steps;
        assert_eq!(s.len(), 8);
        assert_eq!(
            s[7].action,
            Action::Wait(Cond::Screen("trackSelect".into()))
        );
        assert_eq!((s[0].line, &s[0].action), (2, &Action::Wait(Cond::Ui)));
        assert_eq!(s[1].when, When::After(1.5));
        assert_eq!(
            s[1].action,
            Action::Nav(Device::KeyboardA, NavAction::Confirm)
        );
        assert_eq!(s[2].when, When::Tick(120));
        assert_eq!(
            s[2].action,
            Action::Hold(Device::Pad(1), Control::Steer, -0.5)
        );
        assert_eq!(
            s[3].action,
            Action::Tap(Device::KeyboardB, DriveAction::UseItem)
        );
        assert_eq!(
            script.pending().as_deref(),
            Some("line 2: wait ui (screen: none reported)")
        );
    }

    #[test]
    fn bad_steps_name_the_line_and_the_problem() {
        let err = |text: &str| InputScript::parse(text).expect_err(text);
        assert!(err("wait ui\nnav keyboardC up").starts_with("2: unknown device `keyboardC`"));
        assert!(err("hold gamepad throttle 2").contains("0…1"));
        assert!(err("hold gamepad steer").contains("usage: hold"));
        assert!(err("tap gamepad steer").contains("hold it"));
        assert!(err("-1 exit").contains("bad delay"));
        assert!(err("@soon exit").contains("bad race tick"));
        assert!(err("jump").contains("unknown step `jump`"));
        assert!(err("2").contains("needs a step"));
        assert!(err("wait lunch").contains("one of: ui menu"));
        assert!(err("wait screen").contains("usage: wait screen <name>"));
    }

    #[test]
    fn bindings_pick_the_device_family_and_keys_need_half_travel() {
        let c = ControlSettings::default();
        let throttle = DriveAction::Throttle;
        let key = bound_input(&c, Device::KeyboardB, throttle, 1.0);
        assert_eq!(key, Some(Input::Key(KeyCode::ArrowUp)));
        assert_eq!(bound_input(&c, Device::KeyboardA, throttle, 0.4), None);
        let trigger = bound_input(&c, Device::Pad(2), throttle, 0.6);
        assert_eq!(
            trigger,
            Some(Input::Pad(2, GamepadButton::RightTrigger2.into(), 0.6))
        );
        let left = bound_input(&c, Device::Pad(0), DriveAction::SteerLeft, 0.5);
        assert_eq!(
            left,
            Some(Input::Pad(0, GamepadAxis::LeftStickX.into(), -0.5))
        );
    }

    #[derive(Resource, Default)]
    struct Seen(Vec<UiNav>, Vec<ScriptShot>);

    fn record(
        mut seen: ResMut<Seen>,
        mut nav: MessageReader<UiNav>,
        mut shots: MessageReader<ScriptShot>,
    ) {
        seen.0.extend(nav.read().copied());
        seen.1.extend(shots.read().cloned());
    }

    #[test]
    fn wait_screen_holds_the_script_until_the_ui_shows_that_screen() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin, InputPlugin, CorePlugin))
            .add_plugins(crate::GameInputPlugin)
            .insert_resource(TimeUpdateStrategy::FixedTimesteps(1))
            .insert_resource(
                InputScript::parse("wait screen title\nnav keyboardA confirm").expect("valid"),
            )
            .init_resource::<Seen>()
            .add_systems(Last, record);
        app.finish();
        app.cleanup();
        for _ in 0..10 {
            app.update();
        }
        assert!(
            app.world().resource::<Seen>().0.is_empty(),
            "still on the splash"
        );
        let pending = app.world().resource::<InputScript>().pending();
        assert_eq!(
            pending.as_deref(),
            Some("line 1: wait screen title (screen: none reported)")
        );
        app.world_mut().resource_mut::<InputScript>().screen = "title".into();
        for _ in 0..10 {
            app.update();
        }
        assert_eq!(
            app.world().resource::<Seen>().0.len(),
            1,
            "the title got its confirm"
        );
        assert!(app.world().resource::<InputScript>().finished());
    }

    #[test]
    fn steps_feed_rc_input_through_the_device_layer() {
        let script = "nav keyboardA down\n0.2 nav gamepad confirm\n0.2 hold keyboardB throttle 1\n\
                      hold gamepad steer -0.5\n0.2 release gamepad\nshot done\nexit";
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin, InputPlugin, CorePlugin))
            .add_plugins(crate::GameInputPlugin)
            .insert_resource(TimeUpdateStrategy::FixedTimesteps(1))
            .insert_resource(InputScript::parse(script).expect("valid"))
            .init_resource::<Seen>()
            .add_systems(Last, record);
        app.finish();
        app.cleanup();
        let pad = |app: &mut App| -> f32 {
            let mut q = app.world_mut().query::<&Gamepad>();
            q.single(app.world())
                .expect("one virtual pad")
                .left_stick()
                .x
        };
        for _ in 0..30 {
            app.update();
        }
        assert!(
            app.world()
                .resource::<ButtonInput<KeyCode>>()
                .pressed(KeyCode::ArrowUp)
        );
        assert_eq!(pad(&mut app), -0.5);
        for _ in 0..20 {
            app.update();
        }
        assert_eq!(pad(&mut app), 0.0, "released");
        let script = app.world().resource::<InputScript>();
        assert!(script.finished() && script.exit_requested());
        let seen = app.world().resource::<Seen>();
        let navs: Vec<_> = seen.0.iter().map(|n| (n.action, n.device)).collect();
        // Held driving inputs navigate menus like real ones: throttle on keyboard B is the ↑ key
        // (with hold-repeat after 0.35 s), a stick at -0.5 is a step left.
        let (kb_a, kb_b, pad) = (
            DeviceKind::KeyboardA,
            DeviceKind::KeyboardB,
            DeviceKind::Gamepad,
        );
        let expected = [
            (NavAction::Down, kb_a),
            (NavAction::Confirm, pad),
            (NavAction::Up, kb_b),
            (NavAction::Left, pad),
            (NavAction::Up, kb_b),
        ];
        assert_eq!(navs, expected);
        assert!(seen.0.iter().all(|n| n.slot == Some(PlayerSlot(0))));
        assert_eq!(seen.1, [ScriptShot("done".into())]);
    }
}
