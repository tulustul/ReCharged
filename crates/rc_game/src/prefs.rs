//! Settings edits (GDD §15): `ApplySettings`, `ResetBindings` and rc_input's `RebindResult`.
//! rc_game is the only writer of `Settings`; values are clamped to their ranges; saving is
//! throttled to one write per second (`save`).

use bevy::prelude::*;
use rc_core::prelude::*;

use crate::save::SaveState;
use crate::{GameCommand, Step};

/// Smallest windowed size in logical pixels.
const MIN_WINDOW: [u32; 2] = [640, 360];
/// Largest windowed size in logical pixels.
const MAX_WINDOW: u32 = 16_384;

pub(crate) fn build(app: &mut App) {
    app.add_systems(Update, settings_commands.in_set(Step::Transitions));
}

fn settings_commands(
    mut requests: MessageReader<GameCommand>,
    mut rebinds: MessageReader<RebindResult>,
    settings: Option<ResMut<Settings>>,
    save: Option<ResMut<SaveState>>,
) {
    let (Some(mut settings), Some(mut save)) = (settings, save) else {
        return;
    };
    for command in requests.read() {
        match command {
            GameCommand::ApplySettings(new) => {
                let mut new = (**new).clone();
                sanitize(&mut new);
                save.settings_dirty |= settings.set_if_neq(new);
            }
            GameCommand::ResetBindings(scheme) => {
                *settings.controls.bindings_mut(*scheme) = default_bindings(*scheme);
                save.settings_dirty = true;
            }
            _ => {}
        }
    }
    for result in rebinds.read() {
        if apply_rebind(&mut settings.controls, result) {
            save.settings_dirty = true;
        }
    }
}

/// The default bindings of a scheme (GDD §12.1).
pub(crate) fn default_bindings(scheme: ControlScheme) -> ActionBindings {
    match scheme {
        ControlScheme::KeyboardA => ActionBindings::keyboard_a(),
        ControlScheme::KeyboardB => ActionBindings::keyboard_b(),
        ControlScheme::Gamepad => ActionBindings::gamepad(),
    }
}

/// A captured binding becomes the action's primary binding (its other bindings stay) and is
/// taken away from the action it conflicted with. `false` for a cancelled capture.
pub(crate) fn apply_rebind(controls: &mut ControlSettings, result: &RebindResult) -> bool {
    let Some(binding) = result.binding else {
        return false;
    };
    if let Some((scheme, action)) = result.conflict {
        controls
            .bindings_mut(scheme)
            .get_mut(action)
            .retain(|b| *b != binding);
    }
    let list = controls.bindings_mut(result.scheme).get_mut(result.action);
    list.retain(|b| *b != binding);
    match list.first_mut() {
        Some(primary) => *primary = binding,
        None => list.push(binding),
    }
    true
}

fn clamp(value: &mut f32, min: f32, max: f32, default: f32) {
    *value = if value.is_nan() {
        default
    } else {
        value.clamp(min, max)
    };
}

/// Clamps every value to its range (GDD §15); NaN takes the default.
pub(crate) fn sanitize(s: &mut Settings) {
    let d = Settings::default();
    clamp(&mut s.gameplay.hud_scale, 0.75, 1.25, d.gameplay.hud_scale);
    clamp(&mut s.graphics.fov_deg, 60.0, 100.0, d.graphics.fov_deg);
    for (size, min) in s.graphics.window_size.iter_mut().zip(MIN_WINDOW) {
        *size = (*size).clamp(min, MAX_WINDOW);
    }
    let a = &mut s.audio;
    for (v, dv) in [
        (&mut a.master, d.audio.master),
        (&mut a.music, d.audio.music),
        (&mut a.sfx, d.audio.sfx),
        (&mut a.engines, d.audio.engines),
        (&mut a.ui, d.audio.ui),
        (&mut a.ambience, d.audio.ambience),
    ] {
        clamp(v, 0.0, 1.0, dv);
    }
    let c = &mut s.controls;
    clamp(&mut c.steer_dead_zone, 0.0, 0.5, d.controls.steer_dead_zone);
    clamp(
        &mut c.steer_sensitivity,
        0.1,
        5.0,
        d.controls.steer_sensitivity,
    );
    clamp(
        &mut c.vibration_intensity,
        0.0,
        1.0,
        d.controls.vibration_intensity,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> Binding {
        Binding::Key(code)
    }

    #[test]
    fn rebinding_replaces_the_primary_and_steals_from_the_conflict() {
        let mut c = ControlSettings::default();
        let result = RebindResult {
            scheme: ControlScheme::KeyboardB,
            action: DriveAction::LookBack,
            binding: Some(key(KeyCode::KeyE)),
            conflict: Some((ControlScheme::KeyboardA, DriveAction::UseItem)),
        };
        assert!(apply_rebind(&mut c, &result));
        assert_eq!(
            c.keyboard_b.look_back,
            [key(KeyCode::KeyE), key(KeyCode::Delete)]
        );
        assert!(
            c.keyboard_a.use_item.is_empty(),
            "taken from the conflicting action"
        );

        let again = RebindResult {
            binding: Some(key(KeyCode::Delete)),
            conflict: None,
            ..result
        };
        assert!(apply_rebind(&mut c, &again));
        assert_eq!(
            c.keyboard_b.look_back,
            [key(KeyCode::Delete)],
            "no duplicates"
        );

        let empty = RebindResult {
            scheme: ControlScheme::KeyboardA,
            action: DriveAction::UseItem,
            binding: Some(key(KeyCode::KeyF)),
            conflict: None,
        };
        assert!(apply_rebind(&mut c, &empty));
        assert_eq!(c.keyboard_a.use_item, [key(KeyCode::KeyF)]);

        let cancelled = RebindResult {
            binding: None,
            ..empty
        };
        let before = c.clone();
        assert!(!apply_rebind(&mut c, &cancelled));
        assert_eq!(c, before);
    }

    #[test]
    fn settings_are_clamped_to_their_ranges() {
        let mut s = Settings::default();
        s.gameplay.hud_scale = 3.0;
        s.graphics.fov_deg = f32::NAN;
        s.graphics.window_size = [10, 100_000];
        s.audio.music = -1.0;
        s.controls.steer_dead_zone = 0.9;
        sanitize(&mut s);
        assert_eq!(s.gameplay.hud_scale, 1.25);
        assert_eq!(s.graphics.fov_deg, Settings::default().graphics.fov_deg);
        assert_eq!(s.graphics.window_size, [640, MAX_WINDOW]);
        assert_eq!((s.audio.music, s.controls.steer_dead_zone), (0.0, 0.5));
        let mut d = Settings::default();
        sanitize(&mut d);
        assert_eq!(d, Settings::default(), "defaults are valid");
    }
}
