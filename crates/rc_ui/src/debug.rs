//! `RECHARGED_UI_SCRIPT`: scripted UI steps for automated screenshots and smoke runs (the
//! `--shoot` pattern of the bevy-react examples), e.g.
//! `RECHARGED_UI_SCRIPT="2 nav confirm; 3.5 go settings; 5 pause"`.
//!
//! Steps are `<secs> <action>` separated by `;`, timed from `ui.ready`:
//! - `nav <up|down|left|right|confirm|back|prevTab|nextTab> [slot|-] [device]` → a `ui.nav`
//!   event exactly as rc_input's `UiNav` would arrive (default slot 0, `-` = unassigned; device
//!   `keyboardA` | `keyboardB` | `gamepad` | `mouse`, default `keyboardA`);
//! - `pause` → `GameCommand::Pause`;
//! - anything else → a `debug.act { action }` event React interprets (`go <route>`,
//!   `demo <fixture>`, …; verbs in `ui/src/debug.ts`).

use bevy::prelude::*;
use bevy_react::prelude::*;
use rc_core::prelude::*;
use rc_game::GameCommand;

use crate::input::UiDeviceKind;
use crate::ui::{NavEvent, UiNavAction, UiStatus};

/// Environment variable holding the script.
pub const SCRIPT_ENV: &str = "RECHARGED_UI_SCRIPT";

/// Bevy → React event `debug.act`: a scripted step for React (see the module docs).
#[react_event(name = "debug.act")]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugAct {
    /// `<verb> [args]`.
    pub action: String,
}

/// A parsed script step.
#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    /// Send `ui.nav`.
    Nav(NavEvent),
    /// Pause the race.
    Pause,
    /// Send `debug.act`.
    Act(String),
}

#[derive(Resource, Debug)]
struct Script {
    steps: Vec<(f32, Step)>,
    next: usize,
    start: Option<f32>,
}

/// Parses a script into steps sorted by time.
pub fn parse_script(text: &str) -> Result<Vec<(f32, Step)>, String> {
    let mut steps = Vec::new();
    for entry in text.split(';').map(str::trim).filter(|e| !e.is_empty()) {
        let (at, action) = entry
            .split_once(' ')
            .ok_or_else(|| format!("`{entry}`: expected `<secs> <action>`"))?;
        let at: f32 = at
            .parse()
            .map_err(|_| format!("`{entry}`: `{at}` is not a number of seconds"))?;
        steps.push((
            at,
            parse_step(action.trim()).map_err(|e| format!("`{entry}`: {e}"))?,
        ));
    }
    steps.sort_by(|a, b| a.0.total_cmp(&b.0));
    Ok(steps)
}

fn parse_step(action: &str) -> Result<Step, String> {
    let mut words = action.split_whitespace();
    match words.next() {
        Some("pause") => Ok(Step::Pause),
        Some("nav") => {
            let action = match words.next() {
                Some("up") => UiNavAction::Up,
                Some("down") => UiNavAction::Down,
                Some("left") => UiNavAction::Left,
                Some("right") => UiNavAction::Right,
                Some("confirm") => UiNavAction::Confirm,
                Some("back") => UiNavAction::Back,
                Some("prevTab") => UiNavAction::PrevTab,
                Some("nextTab") => UiNavAction::NextTab,
                other => return Err(format!("unknown nav action {other:?}")),
            };
            let slot = match words.next() {
                None => Some(0),
                Some("-") => None,
                Some(slot) => Some(slot.parse().map_err(|_| format!("bad slot `{slot}`"))?),
            };
            let device = match words.next() {
                None | Some("keyboardA") => UiDeviceKind::KeyboardA,
                Some("keyboardB") => UiDeviceKind::KeyboardB,
                Some("gamepad") => UiDeviceKind::Gamepad,
                Some("mouse") => UiDeviceKind::Mouse,
                Some(other) => return Err(format!("unknown device `{other}`")),
            };
            Ok(Step::Nav(NavEvent {
                action,
                slot,
                device,
                repeat: false,
            }))
        }
        Some(_) => Ok(Step::Act(action.to_owned())),
        None => Err("empty action".into()),
    }
}

pub(crate) fn register(app: &mut App) {
    app.add_react_event::<DebugAct>();
}

/// Installs the script runner when the environment variable is set.
pub(crate) fn build(app: &mut App) {
    let Ok(text) = std::env::var(SCRIPT_ENV) else {
        return;
    };
    match parse_script(&text) {
        Ok(steps) => {
            info!("{SCRIPT_ENV}: {} steps", steps.len());
            app.insert_resource(Script {
                steps,
                next: 0,
                start: None,
            })
            .add_systems(Update, run_script.in_set(UiSet));
        }
        Err(e) => error!("{SCRIPT_ENV}: {e}"),
    }
}

fn run_script(
    time: Res<Time<Real>>,
    status: Res<UiStatus>,
    mut script: ResMut<Script>,
    events: ReactEvents,
    mut commands: MessageWriter<GameCommand>,
) {
    if !status.ready {
        return;
    }
    let now = time.elapsed_secs();
    let start = *script.start.get_or_insert(now);
    while let Some((at, step)) = script.steps.get(script.next) {
        if now - start < *at {
            break;
        }
        info!("{SCRIPT_ENV} @{at}s: {step:?}");
        match step {
            Step::Nav(nav) => events.send(nav),
            Step::Pause => {
                commands.write(GameCommand::Pause);
            }
            Step::Act(action) => events.send(&DebugAct {
                action: action.clone(),
            }),
        }
        script.next += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scripts_parse_into_sorted_steps() {
        let steps = parse_script("3 go settings; 1.5 nav down;  2 nav confirm - gamepad; 4 pause;")
            .expect("valid script");
        let times: Vec<f32> = steps.iter().map(|s| s.0).collect();
        assert_eq!(times, [1.5, 2.0, 3.0, 4.0]);
        assert_eq!(
            steps[0].1,
            Step::Nav(NavEvent {
                action: UiNavAction::Down,
                slot: Some(0),
                device: UiDeviceKind::KeyboardA,
                repeat: false
            })
        );
        let Step::Nav(confirm) = &steps[1].1 else {
            panic!("nav step expected");
        };
        assert_eq!(
            (confirm.slot, confirm.device),
            (None, UiDeviceKind::Gamepad)
        );
        assert_eq!(steps[2].1, Step::Act("go settings".into()));
        assert_eq!(steps[3].1, Step::Pause);
    }

    #[test]
    fn bad_steps_name_the_entry() {
        assert!(parse_script("soon go title").unwrap_err().contains("soon"));
        assert!(
            parse_script("1 nav sideways")
                .unwrap_err()
                .contains("sideways")
        );
        assert!(parse_script("1").is_err());
    }
}
