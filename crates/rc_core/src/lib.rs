//! # rc_core — the contract layer of ReCharged (L0)
//!
//! Crates talk to each other only through the types defined here (ARCHITECTURE §1.2, §4): app
//! states and the session lifecycle, system sets, shared components, resources and messages,
//! catalog and settings data, the keyed RNG, path resolution and interpolation snapshots.
//! Everything is re-exported at the crate root (`use rc_core::*;` or [`prelude`]).
//!
//! - **Responsibilities:** register the states ([`state`]), configure and chain the system sets
//!   ([`schedule`]), register every message, set the fixed rate to [`SIM_HZ`], and snapshot
//!   [`PoseHistory`] of [`Interpolated`] entities in `FixedLast` ([`interp`]).
//! - **Inputs:** `Transform` of `Interpolated` entities, [`Teleported`] messages.
//! - **Outputs:** [`PoseHistory`] (sole writer); `State`s, sets and message queues for everyone.
//! - **Must never:** contain gameplay logic, read input devices, load assets, or depend on another
//!   workspace crate. Types are data only (plus small pure helpers); fields grow additively.
//!
//! Module = area; a task brief may grant additive edits in one module.

pub mod camera;
pub mod catalog;
pub mod consts;
pub mod flow;
pub mod input;
pub mod interp;
pub mod items;
pub mod paths;
pub mod race;
pub mod rng;
pub mod schedule;
pub mod settings;
pub mod state;
pub mod track;
pub mod ui;
pub mod vehicle;
pub mod weather;

pub use camera::*;
pub use catalog::*;
pub use consts::*;
pub use flow::*;
pub use input::*;
pub use interp::*;
pub use items::*;
pub use paths::UserPaths;
pub use race::*;
pub use rng::RngStream;
pub use schedule::*;
pub use settings::*;
pub use state::*;
pub use track::*;
pub use ui::*;
pub use vehicle::*;
pub use weather::*;

use bevy::prelude::*;

/// Everything a subsystem crate usually needs: `use rc_core::prelude::*;`.
pub mod prelude {
    pub use crate::{
        CorePlugin, camera::*, catalog::*, consts::*, flow::*, input::*, interp::*, items::*,
        paths::UserPaths, race::*, rng, rng::RngStream, schedule::*, settings::*, state::*,
        track::*, ui::*, vehicle::*, weather::*,
    };
}

/// Registers the shared runtime model. Add it after `DefaultPlugins` (or `MinimalPlugins` +
/// `StatesPlugin` in headless apps) and before every ReCharged plugin.
pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        state::register_states(app);
        schedule::configure_sets(app);
        app.insert_resource(Time::<Fixed>::from_hz(SIM_HZ))
            .init_resource::<LoadingTasks>()
            .add_systems(FixedLast, interp::snapshot_poses.in_set(CoreSet::Snapshot));
        register_messages(app);
    }
}

fn register_messages(app: &mut App) {
    app.add_message::<Teleported>()
        // flow
        .add_message::<LoadingDone>()
        .add_message::<LoadingFailed>()
        // vehicle
        .add_message::<CarImpact>()
        .add_message::<CarLanded>()
        .add_message::<RespawnCar>()
        .add_message::<CarRespawned>()
        .add_message::<ApplyImpulse>()
        .add_message::<DrainCharge>()
        .add_message::<GrantCharge>()
        // items
        .add_message::<ApplyEffect>()
        .add_message::<ItemEvent>()
        .add_message::<HitEvent>()
        .add_message::<Explosion>()
        // race & weather
        .add_message::<RaceEvent>()
        .add_message::<WeatherEvent>()
        // input & ui
        .add_message::<UiNav>()
        .add_message::<RebindRequest>()
        .add_message::<RebindResult>()
        .add_message::<DeviceEvent>()
        .add_message::<PauseRequest>()
        .add_message::<UiSound>();
}
