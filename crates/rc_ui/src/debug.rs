//! `RECHARGED_UI_SCRIPT`: scripted React fixtures for automated UI screenshots (the `--shoot`
//! pattern of the bevy-react examples), e.g. `RECHARGED_UI_SCRIPT="2 go settings; 3.5 demo hud"`.
//!
//! Steps are `<secs> <action>` separated by `;`, timed from `ui.ready`; each is sent as a
//! `debug.act { action }` event React interprets (`go <route>`, `demo <fixture>`, `tab <id>`;
//! verbs in `ui/src/debug.ts`). Navigation and pausing go through the real input path:
//! `--input-script` (`docs/design/automation.md`).

use bevy::prelude::*;
use bevy_react::prelude::*;
use rc_core::prelude::*;

use crate::ui::UiStatus;

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

#[derive(Resource, Debug)]
struct Script {
    steps: Vec<(f32, String)>,
    next: usize,
    start: Option<f32>,
}

/// Parses a script into `(secs, action)` steps sorted by time.
pub fn parse_script(text: &str) -> Result<Vec<(f32, String)>, String> {
    let mut steps = Vec::new();
    for entry in text.split(';').map(str::trim).filter(|e| !e.is_empty()) {
        let (at, action) = entry
            .split_once(' ')
            .ok_or_else(|| format!("`{entry}`: expected `<secs> <action>`"))?;
        let at: f32 = at
            .parse()
            .map_err(|_| format!("`{entry}`: `{at}` is not a number of seconds"))?;
        let action = action.trim();
        if matches!(action.split_whitespace().next(), Some("nav" | "pause")) {
            return Err(format!("`{entry}`: script navigation with --input-script"));
        }
        steps.push((at, action.to_owned()));
    }
    steps.sort_by(|a, b| a.0.total_cmp(&b.0));
    Ok(steps)
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
) {
    if !status.ready {
        return;
    }
    let now = time.elapsed_secs();
    let start = *script.start.get_or_insert(now);
    while let Some((at, action)) = script.steps.get(script.next) {
        if now - start < *at {
            break;
        }
        info!("{SCRIPT_ENV} @{at}s: {action}");
        events.send(&DebugAct {
            action: action.clone(),
        });
        script.next += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scripts_parse_into_sorted_steps() {
        let steps = parse_script("3 go settings; 1.5 demo hud 2 racing;  2 tab audio;")
            .expect("valid script");
        let times: Vec<f32> = steps.iter().map(|s| s.0).collect();
        assert_eq!(times, [1.5, 2.0, 3.0]);
        assert_eq!(steps[0].1, "demo hud 2 racing");
        assert_eq!(steps[2].1, "go settings");
    }

    #[test]
    fn bad_steps_name_the_entry() {
        assert!(parse_script("soon go title").unwrap_err().contains("soon"));
        assert!(parse_script("1").is_err());
        let nav = parse_script("1 nav confirm").unwrap_err();
        assert!(nav.contains("--input-script"), "{nav}");
        assert!(parse_script("4 pause").is_err());
    }
}
