//! Race HUD channels (T05b, GDD §13): `race.setup` (racers, minimap outline, laps, HUD settings),
//! the batched `race.hud` tick (every local player, ≤ 10 Hz), `race.event` (countdown, laps,
//! finish, hits, hints…), `viewports` (each player's view in UI px) and the per-frame visuals
//! (speed, charge, charge glow, rocket arrow, minimap dots): shared values React creates and binds
//! with `hud.bind`, written here every frame between `AnimationSet::Tick` and
//! `AnimationSet::Apply`. `race.snapshot` carries the same state for (re)mounts.

use std::time::Duration;

use bevy::prelude::*;
use bevy::time::common_conditions::on_real_timer;
use bevy_react::animations::{AnimationSet, SharedId};
use bevy_react::prelude::*;
use rc_core::prelude::*;

use crate::race::{UiMode, UiRacer};
use crate::settings::UiSpeedUnit;

mod systems;
mod view;

pub(crate) use view::HudView;
pub use view::MinimapFit;

/// Cadence of `race.hud`.
const TICK: Duration = Duration::from_millis(100);

mirror_enum!(
    /// [`ItemKind`].
    UiItemKind = ItemKind {
        TurboCell,
        SparkRocket,
        LeechCoil,
        ShockOrb,
        EmpPulse,
        GooBomb,
        OilSlick,
        SpringPad,
        ShieldBubble,
        StormStrike,
    }
);
mirror_enum!(
    /// [`HintKind`].
    UiHint = HintKind { DriftCharge, BoostReady, ItemReady, HoldReset, WrongWay }
);

/// Bevy → React event `race.setup`: what the HUD draws for the whole attempt — racers (by grid
/// index), minimap outline, laps — and the HUD settings; re-sent whenever one of them changes.
#[react_event(name = "race.setup")]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
#[derive(Debug, Clone, PartialEq)]
pub struct UiHudSetup {
    /// Mode.
    pub mode: UiMode,
    /// Laps.
    pub laps: u8,
    /// Items enabled (the item slot shows).
    pub items: bool,
    /// Racing a Time Trial ghost.
    pub ghost: bool,
    /// Racers in grid order; `race.hud` refers to them by index.
    pub racers: Vec<UiRacer>,
    /// Minimap outline: x, y pairs in the 0–1 square (aspect kept, centred).
    pub outline: Vec<f32>,
    /// Speed unit setting.
    pub speed_unit: UiSpeedUnit,
    /// HUD scale setting (0.75–1.25).
    pub hud_scale: f32,
    /// Minimap setting.
    pub minimap: bool,
    /// Reduce flashing: no pulsing warnings or flashing meters.
    pub reduce_flashing: bool,
}

/// One local player's HUD values.
#[derive(serde::Serialize, ts_rs::TS, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct UiPlayerHud {
    /// Slot 0–3.
    pub slot: u8,
    /// Grid index (into the `race.setup` racers).
    pub racer: u8,
    /// Position, 1-based.
    pub position: u8,
    /// Current lap, 1-based (1 on the grid, never above the race's laps).
    pub lap: u8,
    /// Time on the current lap, ms (0 once finished).
    pub lap_ms: u32,
    /// Latest completed lap, ms.
    pub last_lap_ms: Option<u32>,
    /// Best lap of this race, ms.
    pub best_lap_ms: Option<u32>,
    /// Total time once finished, ms.
    pub finish_ms: Option<u32>,
    /// Speed in display km/h.
    pub kmh: u16,
    /// Charge 0–1 (tenths).
    pub charge: f32,
    /// Enough charge to boost.
    pub boost_ready: bool,
    /// Spending charge right now.
    pub spending: bool,
    /// Held item (usable).
    pub item: Option<UiItemKind>,
    /// The item roulette is spinning.
    pub roulette: bool,
    /// Driving the wrong way.
    pub wrong_way: bool,
    /// A Spark Rocket is locked on this car (direction: the `arrow` shared value).
    pub lock_on: bool,
    /// Goo Bomb stuck to this car: fuse seconds left.
    pub goo: Option<f32>,
    /// Shield seconds left.
    pub shield: Option<f32>,
}

/// Bevy → React event `race.hud`: every local player's HUD values, at most 10 Hz, only when
/// something changed.
#[react_event(name = "race.hud")]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
#[derive(Debug, Clone, PartialEq)]
pub struct UiHudTick {
    /// Race clock, ms since GO.
    pub race_ms: u32,
    /// Countdown number showing (3, 2, 1; 0 = none).
    pub countdown: u8,
    /// Soft-lock guard: whole seconds until the race ends.
    pub ends_in: Option<u8>,
    /// Racer grid indices by position.
    pub standings: Vec<u8>,
    /// Local players by slot.
    pub players: Vec<UiPlayerHud>,
}

/// Bevy → React event `race.event`: a race happening for the HUD. `slot` = the local player it
/// concerns; countdown, GO and the end-of-race countdown are for everyone.
#[react_event(name = "race.event")]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(tag = "kind", rename_all = "camelCase")]
#[derive(Debug, Clone, PartialEq)]
pub enum UiRaceEvent {
    /// Countdown number (3, 2, 1).
    Countdown {
        /// The number.
        n: u8,
    },
    /// GO.
    Go,
    /// Throttle timed just before GO: launch boost.
    PerfectStart {
        /// Slot 0–3.
        slot: u8,
    },
    /// Throttle held too early: wheel-spin stall.
    Stall {
        /// Slot 0–3.
        slot: u8,
    },
    /// Time Trial split vs the ghost/best at a checkpoint.
    Split {
        /// Slot 0–3.
        slot: u8,
        /// Seconds, + = slower.
        secs: f32,
    },
    /// A lap was completed.
    Lap {
        /// Slot 0–3.
        slot: u8,
        /// The completed lap, 1-based.
        lap: u8,
        /// Lap time, ms.
        ms: u32,
        /// Personal best of this race.
        best: bool,
        /// Fastest lap of anyone so far (from lap 2).
        fastest: bool,
    },
    /// The final lap started.
    FinalLap {
        /// Slot 0–3.
        slot: u8,
    },
    /// The player finished.
    Finished {
        /// Slot 0–3.
        slot: u8,
        /// Position.
        rank: u8,
        /// Total time, ms.
        ms: u32,
    },
    /// The Golden Battery was collected.
    GoldenBattery {
        /// Slot 0–3.
        slot: u8,
    },
    /// The soft-lock guard's last ten seconds.
    EndsIn {
        /// Seconds left.
        secs: u8,
    },
    /// An onboarding hint (once each, setting).
    Hint {
        /// Slot 0–3.
        slot: u8,
        /// Which.
        hint: UiHint,
    },
    /// The player was hit by an item.
    Hit {
        /// Slot 0–3.
        slot: u8,
        /// The item.
        by: UiItemKind,
        /// Who fired it (none: own item or unknown).
        attacker: Option<String>,
        /// The shield blocked it.
        blocked: bool,
    },
    /// The player's item hit someone.
    Landed {
        /// Slot 0–3.
        slot: u8,
        /// The item.
        by: UiItemKind,
        /// Who was hit.
        victim: String,
        /// Their shield blocked it.
        blocked: bool,
    },
}

/// A rectangle in UI px.
#[derive(serde::Serialize, ts_rs::TS, Debug, Clone, Copy, PartialEq)]
pub struct UiRect {
    /// Left.
    pub x: f32,
    /// Top.
    pub y: f32,
    /// Width.
    pub w: f32,
    /// Height.
    pub h: f32,
}

/// One local player's view.
#[derive(serde::Serialize, ts_rs::TS, Debug, Clone, Copy, PartialEq)]
pub struct UiViewport {
    /// Slot 0–3.
    pub slot: u8,
    /// The view in UI px.
    pub rect: UiRect,
}

/// Bevy → React event `viewports`: each local player's view in UI px (window logical px ÷
/// `UiScale`) and, in a 3-player split, the free quadrant for the shared minimap and standings.
#[react_event(name = "viewports")]
#[derive(Debug, Clone, PartialEq)]
pub struct UiViewports {
    /// Views by slot (empty outside a race).
    pub players: Vec<UiViewport>,
    /// The free quadrant (3 players).
    pub panel: Option<UiRect>,
}

/// The HUD part of `race.snapshot`.
#[derive(serde::Serialize, ts_rs::TS, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct UiHudSnapshot {
    /// As `race.setup`.
    pub setup: UiHudSetup,
    /// As `race.hud` (`null` before the attempt runs).
    pub tick: Option<UiHudTick>,
    /// As `viewports`.
    pub viewports: UiViewports,
}

/// Shared value ids of one player's per-frame visuals.
#[derive(serde::Deserialize, ts_rs::TS, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct UiPlayerValues {
    /// Slot 0–3.
    pub slot: u8,
    /// Speed, u/s.
    pub speed: SharedId,
    /// Charge 0–1.
    pub charge: SharedId,
    /// Charging glow 0–1 (charge gain rate).
    pub glow: SharedId,
    /// Direction of a rocket locked on this car: degrees clockwise from straight ahead.
    pub arrow: SharedId,
}

/// A minimap dot: position in the minimap's 0–1 square.
#[derive(serde::Deserialize, ts_rs::TS, Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiDotValues {
    /// X.
    pub x: SharedId,
    /// Y.
    pub y: SharedId,
}

/// React → Bevy message `hud.bind`: shared values to drive every frame (players by slot, dots by
/// grid index); replaces any earlier binding.
#[react_message(name = "hud.bind")]
#[derive(Debug, Clone)]
pub struct BindHud {
    /// Per-player visuals.
    pub players: Vec<UiPlayerValues>,
    /// Minimap dots by grid index.
    pub dots: Vec<UiDotValues>,
}

/// React → Bevy message `hud.unbind`: stop driving the HUD's shared values (unmounted).
#[react_message(name = "hud.unbind")]
pub struct UnbindHud;

/// What React bound (`hud.bind`).
#[derive(Resource, Default)]
struct HudBindings(Option<BindHud>);

pub(crate) fn register(app: &mut App) {
    app.init_resource::<HudBindings>()
        .add_react_event::<UiHudSetup>()
        .add_react_event::<UiHudTick>()
        .add_react_event::<UiRaceEvent>()
        .add_react_event::<UiViewports>()
        .add_react_handler(|bind: On<BindHud>, mut bindings: ResMut<HudBindings>| {
            bindings.0 = Some(bind.event().clone());
        })
        .add_react_handler(|_: On<UnbindHud>, mut bindings: ResMut<HudBindings>| {
            bindings.0 = None;
        });
}

/// The HUD systems (the running app only; the exporter just registers channels).
pub(crate) fn build(app: &mut App) {
    let in_race = in_state(AppState::InRace);
    app.add_systems(
        Update,
        (
            (
                systems::push_setup,
                systems::push_viewports,
                systems::forward_events,
            ),
            systems::push_tick
                .run_if(in_race.clone())
                .run_if(on_real_timer(TICK)),
            systems::drive_values
                .run_if(in_race)
                .after(AnimationSet::Tick)
                .before(AnimationSet::Apply),
        )
            .in_set(UiSet),
    );
}
