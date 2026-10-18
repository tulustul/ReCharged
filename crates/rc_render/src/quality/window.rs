//! Display settings (GDD §15): window mode, windowed size and vsync, applied live whenever
//! `Settings.graphics` changes (rc_game never writes the window). The size is re-applied only
//! when the setting itself changes, so a window the user resized is not fought every frame, and
//! never with `--window <W>x<H>` (`AutomationConfig::window`: screenshot runs keep their size).
//! Automated runs never apply vsync: the binary presents them without it (a blanked monitor
//! stalls vsync'd presentation to ~1 frame/s).

use bevy::prelude::*;
use bevy::window::{MonitorSelection, PresentMode, PrimaryWindow, WindowMode};
use rc_core::prelude::*;

pub(super) fn build(app: &mut App) {
    app.add_systems(
        Update,
        apply_display
            .run_if(resource_exists_and_changed::<Settings>)
            .in_set(VisualSet),
    );
}

/// Display options last applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Display {
    mode: DisplayMode,
    size: [u32; 2],
    vsync: bool,
}

fn apply_display(
    settings: Res<Settings>,
    automation: Option<Res<AutomationConfig>>,
    window: Option<Single<&mut Window, With<PrimaryWindow>>>,
    mut last: Local<Option<Display>>,
) {
    let Some(mut window) = window else {
        return;
    };
    let g = &settings.graphics;
    let now = Display {
        mode: g.display_mode,
        size: g.window_size,
        vsync: g.vsync,
    };
    if *last == Some(now) {
        return;
    }
    let mode = window_mode(now.mode);
    if window.mode != mode {
        window.mode = mode;
    }
    let automated = automation.as_ref().is_some_and(|a| a.is_automated());
    let present = present_mode(now.vsync);
    if !automated && window.present_mode != present {
        window.present_mode = present;
    }
    let size_changed = last.is_none_or(|l| l.size != now.size || l.mode != now.mode);
    let fixed_size = automation.is_some_and(|a| a.window.is_some());
    if now.mode == DisplayMode::Windowed
        && size_changed
        && !fixed_size
        && now.size[0] > 0
        && now.size[1] > 0
    {
        let (w, h) = (now.size[0] as f32, now.size[1] as f32);
        if (window.width() - w).abs() > 0.5 || (window.height() - h).abs() > 0.5 {
            window.resolution.set(w, h);
        }
    }
    info!(
        "display: {:?} {}×{} vsync {}",
        now.mode, now.size[0], now.size[1], now.vsync
    );
    *last = Some(now);
}

fn window_mode(mode: DisplayMode) -> WindowMode {
    match mode {
        DisplayMode::Windowed => WindowMode::Windowed,
        DisplayMode::Borderless => WindowMode::BorderlessFullscreen(MonitorSelection::Current),
    }
}

fn present_mode(vsync: bool) -> PresentMode {
    if vsync {
        PresentMode::AutoVsync
    } else {
        PresentMode::AutoNoVsync
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_settings_map_to_window_options() {
        assert_eq!(window_mode(DisplayMode::Windowed), WindowMode::Windowed);
        assert!(matches!(
            window_mode(DisplayMode::Borderless),
            WindowMode::BorderlessFullscreen(_)
        ));
        assert_eq!(present_mode(true), PresentMode::AutoVsync);
        assert_eq!(present_mode(false), PresentMode::AutoNoVsync);
    }
}
