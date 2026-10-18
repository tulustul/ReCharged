//! The UI host rig (ARCHITECTURE §5.8): which cameras exist in which state, which one hosts the
//! React UI, and the split-screen viewports.
//!
//! | State | Cameras | UI host |
//! |---|---|---|
//! | Menu | `MenuCamera` (3D showroom) | the menu camera |
//! | Loading | a plain 2D camera | that camera |
//! | InRace, 1 player | one `GameCamera` (full window) | the game camera |
//! | InRace, 2–4 players | `GameCamera`s with viewports + a full-window `OverlayCamera` | the overlay |
//!
//! Every host is spawned together with `UiHost` + `IsDefaultUiCamera` and scoped to its state, so
//! exactly one exists at a time (Boot and Unloading last one frame and have none: the window is
//! cleared to [`BACKDROP`]). Game cameras are session-scoped: a restart keeps them (and the host).

use bevy::camera::{CameraOutputMode, Viewport};
use bevy::prelude::*;
use bevy::render::render_resource::BlendState;
use bevy::ui::IsDefaultUiCamera;
use bevy::window::PrimaryWindow;
use rc_core::prelude::*;

use crate::follow::Rig;
use crate::layout::{split_rects, to_logical};

/// Backdrop colour behind everything until rc_render draws a scene (dark navy, sRGB). Also the
/// window clear colour, so state gaps and the free 3P quadrant match it.
pub const BACKDROP: Color = Color::srgb(0.035, 0.05, 0.1);

/// Render order of the split-screen UI overlay (player cameras use 0..4).
const OVERLAY_ORDER: isize = 10;

/// The plain 2D camera that hosts the loading screen.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct LoadingCamera;

pub(crate) fn build(app: &mut App) {
    app.insert_resource(ClearColor(BACKDROP))
        .init_resource::<PlayerViewports>()
        .add_systems(OnEnter(AppState::Loading), spawn_loading_camera)
        .add_systems(OnEnter(AppState::InRace), spawn_game_cameras);
}

/// The components every UI host carries.
fn ui_host() -> (UiHost, IsDefaultUiCamera) {
    (UiHost, IsDefaultUiCamera)
}

fn spawn_loading_camera(mut commands: Commands) {
    commands.spawn((
        Name::new("Loading camera"),
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(BACKDROP),
            ..default()
        },
        LoadingCamera,
        ui_host(),
        DespawnOnExit(AppState::Loading),
    ));
}

/// Local player slots of the session: the humans of `RaceConfig` (at least one camera, which
/// spectates when there is no human).
pub(crate) fn session_slots(config: Option<&RaceConfig>) -> Vec<PlayerSlot> {
    let mut slots: Vec<PlayerSlot> = config
        .into_iter()
        .flat_map(|c| &c.racers)
        .filter_map(|r| match r.controller {
            Controller::Human { slot, .. } => Some(slot),
            Controller::Ai { .. } => None,
        })
        .take(MAX_LOCAL_PLAYERS)
        .collect();
    slots.sort_by_key(|s| s.0);
    slots.dedup();
    if slots.is_empty() {
        slots.push(PlayerSlot(0));
    }
    slots
}

fn spawn_game_cameras(
    mut commands: Commands,
    config: Option<Res<RaceConfig>>,
    settings: Option<Res<Settings>>,
) {
    let slots = session_slots(config.as_deref());
    let mode = settings.map_or_else(default, |s| s.gameplay.camera_mode);
    let split = slots.len() > 1;
    info!("cameras: {} player view(s)", slots.len());
    for (order, &slot) in slots.iter().enumerate() {
        let mut camera = commands.spawn((
            Name::new(format!("Game camera P{}", slot.0 + 1)),
            Camera3d::default(),
            Camera {
                order: order as isize,
                ..default()
            },
            Projection::Perspective(PerspectiveProjection {
                near: 0.05,
                far: 2000.0,
                ..default()
            }),
            GameCamera { slot },
            Rig::new(mode),
            DespawnOnExit(RaceSession),
        ));
        if !split {
            camera.insert(ui_host());
        }
    }
    if split {
        commands.spawn((overlay_camera(), DespawnOnExit(RaceSession)));
    }
}

/// Full-window UI camera over the player viewports. It clears to transparent and is blended as
/// premultiplied alpha (bevy_ui blends straight alpha onto the transparent clear), because a
/// `ClearColorConfig::None` overlay never clears its own texture when the player cameras are HDR:
/// the UI then accumulates frame over frame (`docs/design/rc_camera.md`, rig verification).
fn overlay_camera() -> impl Bundle {
    (
        Name::new("Overlay camera"),
        Camera2d,
        Camera {
            order: OVERLAY_ORDER,
            clear_color: ClearColorConfig::Custom(Color::NONE),
            output_mode: CameraOutputMode::Write {
                blend_state: Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                clear_color: ClearColorConfig::None,
            },
            ..default()
        },
        // The UI needs no MSAA; this also keeps the overlay off the players' main textures.
        Msaa::Off,
        OverlayCamera,
        ui_host(),
    )
}

/// What the split layout depends on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LayoutKey {
    size: UVec2,
    scale: f32,
    split: SplitOrientation,
    cameras: usize,
}

/// Viewports of the game cameras from the window size and the 2P split setting, and
/// `PlayerViewports` in logical pixels for the HUD; recomputed when any input changes (resize,
/// setting, new cameras).
pub(crate) fn layout_viewports(
    window: Option<Single<&Window, With<PrimaryWindow>>>,
    settings: Option<Res<Settings>>,
    mut cameras: Query<(&GameCamera, &mut Camera)>,
    added: Query<(), Added<GameCamera>>,
    mut viewports: ResMut<PlayerViewports>,
    mut last: Local<Option<LayoutKey>>,
) {
    let Some(window) = window else {
        return;
    };
    let size = window.physical_size();
    if size.x == 0 || size.y == 0 {
        return; // minimised: keep the last layout
    }
    let key = LayoutKey {
        size,
        scale: window.scale_factor(),
        split: settings.map_or_else(default, |s| s.gameplay.split),
        cameras: cameras.iter().count(),
    };
    if *last == Some(key) && added.is_empty() {
        return;
    }
    *last = Some(key);
    let cells = split_rects(key.cameras, key.split, size);
    let mut slots: Vec<PlayerSlot> = cameras.iter().map(|(c, _)| c.slot).collect();
    slots.sort_by_key(|s| s.0);
    let mut rects = Vec::with_capacity(slots.len());
    for (game, mut camera) in &mut cameras {
        let index = slots.iter().position(|&s| s == game.slot).unwrap_or(0);
        let cell = cells.get(index).copied().unwrap_or_default();
        camera.viewport = (key.cameras > 1).then(|| Viewport {
            physical_position: cell.min,
            physical_size: cell.size(),
            ..default()
        });
        rects.push((game.slot, to_logical(cell, key.scale)));
    }
    rects.sort_by_key(|(slot, _)| slot.0);
    viewports.set_if_neq(PlayerViewports(rects));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(controllers: &[Controller]) -> RaceConfig {
        RaceConfig {
            racers: controllers
                .iter()
                .map(|c| RacerSetup {
                    controller: c.clone(),
                    ..default()
                })
                .collect(),
            ..default()
        }
    }

    fn human(slot: u8) -> Controller {
        Controller::Human {
            slot: PlayerSlot(slot),
            autopilot: false,
        }
    }

    #[test]
    fn one_camera_per_human_and_one_spectator_without_humans() {
        let ai = Controller::Ai {
            rival: "volt".into(),
            difficulty: Difficulty::Pro,
        };
        let slots = session_slots(Some(&config(&[human(1), ai.clone(), human(0)])));
        assert_eq!(slots, [PlayerSlot(0), PlayerSlot(1)]);
        assert_eq!(session_slots(Some(&config(&[ai]))), [PlayerSlot(0)]);
        assert_eq!(session_slots(None), [PlayerSlot(0)]);
    }
}
