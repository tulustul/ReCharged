//! The HUD systems: push `race.setup`, `viewports` and the 10 Hz `race.hud` when they change,
//! forward race and hit events, and drive the bound shared values every frame.

use bevy::prelude::*;
use bevy_react::animations::{SharedId, SharedValues};
use bevy_react::prelude::*;
use rc_core::prelude::*;

use super::nav::{MinimapFit, bearing_from, map_rotation};
use super::view::race_event;
use super::{HudBindings, HudView, UiHudSetup, UiHudTick, UiRaceEvent, UiViewports};

/// Charge gain (meter per second) that lights the charging glow fully (drift alone peaks at 0.22).
const FULL_GLOW_RATE: f32 = 0.3;

/// Racer state the per-frame visuals read.
type RacerPose = (
    Entity,
    &'static Racer,
    &'static Transform,
    &'static VehicleTelemetry,
    &'static Charge,
    &'static RaceProgress,
    Option<&'static LocalPlayer>,
);

pub(super) fn push_setup(
    view: HudView,
    state: Res<State<AppState>>,
    mut last: Local<Option<UiHudSetup>>,
    events: ReactEvents,
) {
    if *state.get() != AppState::InRace {
        *last = None;
        return;
    }
    if last.is_some() && !state.is_changed() && !view.inputs_changed() {
        return;
    }
    if let Some(setup) = view.setup()
        && last.as_ref() != Some(&setup)
    {
        events.send(&setup);
        *last = Some(setup);
    }
}

pub(super) fn push_viewports(
    view: HudView,
    mut last: Local<Option<UiViewports>>,
    events: ReactEvents,
) {
    let viewports = view.viewports();
    if last.as_ref() != Some(&viewports) {
        events.send(&viewports);
        *last = Some(viewports);
    }
}

pub(super) fn push_tick(view: HudView, mut last: Local<Option<UiHudTick>>, events: ReactEvents) {
    if let Some(tick) = view.tick()
        && last.as_ref() != Some(&tick)
    {
        events.send(&tick);
        *last = Some(tick);
    }
}

pub(super) fn forward_events(
    mut race: MessageReader<RaceEvent>,
    mut hits: MessageReader<HitEvent>,
    racers: Query<(Entity, &Racer, &RaceProgress, Option<&LocalPlayer>)>,
    events: ReactEvents,
) {
    let slot_of = |e: Entity| racers.get(e).ok().and_then(|r| r.3).map(|l| l.0.0);
    let name_of = |e: Entity| racers.get(e).ok().map(|r| r.1.name.clone());
    for event in race.read() {
        let slot = event.racer.and_then(slot_of);
        let fastest = match (event.kind, event.racer) {
            (RaceEventKind::LapCompleted { lap, time, best }, Some(own)) => {
                best && lap >= 2
                    && racers
                        .iter()
                        .filter(|r| r.0 != own)
                        .all(|r| r.2.best_lap.is_none_or(|b| time < b))
            }
            _ => false,
        };
        if let Some(ui) = race_event(event.kind, slot, fastest) {
            events.send(&ui);
        }
    }
    for hit in hits.read() {
        let attacker = hit.attacker.filter(|&a| a != hit.victim);
        if let Some(slot) = slot_of(hit.victim) {
            events.send(&UiRaceEvent::Hit {
                slot,
                by: hit.by.into(),
                attacker: attacker.and_then(name_of),
                blocked: hit.blocked,
            });
        }
        if let Some(slot) = attacker.and_then(slot_of) {
            events.send(&UiRaceEvent::Landed {
                slot,
                by: hit.by.into(),
                victim: name_of(hit.victim).unwrap_or_default(),
                blocked: hit.blocked,
            });
        }
    }
}

/// Writes the bound shared values (compare before write: unchanged values cost nothing).
pub(super) fn drive_values(
    bindings: Res<HudBindings>,
    summary: Option<Res<TrackSummary>>,
    racers: Query<RacerPose>,
    rockets: Query<(&Projectile, &ItemTarget, &Transform)>,
    cameras: Query<(&GameCamera, &GlobalTransform)>,
    mut fit: Local<Option<MinimapFit>>,
    mut values: ResMut<SharedValues>,
) {
    let Some(bound) = bindings.0.as_ref() else {
        return;
    };
    if let Some(summary) = summary.as_ref()
        && (summary.is_changed() || fit.is_none())
    {
        *fit = Some(MinimapFit::of(summary));
    }
    let mut set = |id: SharedId, v: f32| {
        if values.get(id) != Some(v) {
            values.set(id, v);
        }
    };
    for (entity, racer, t, tel, charge, progress, local) in &racers {
        if let (Some(fit), Some(dot)) = (fit.as_ref(), bound.dots.get(racer.index)) {
            let p = fit.project(Vec2::new(t.translation.x, t.translation.z));
            set(dot.x, p.x);
            set(dot.y, p.y);
        }
        let Some(ids) = local.and_then(|l| bound.players.iter().find(|p| p.slot == l.0.0)) else {
            continue;
        };
        set(ids.speed, tel.speed);
        set(ids.charge, charge.0);
        set(ids.glow, (tel.charge_rate / FULL_GLOW_RATE).clamp(0.0, 1.0));
        // The nearest rocket locked on this car, in the car's frame (forward = −Z).
        let nearest = rockets
            .iter()
            .filter(|(p, target, _)| p.kind == ItemKind::SparkRocket && target.0 == Some(entity))
            .map(|(.., r)| t.rotation.inverse() * (r.translation - t.translation))
            .min_by(|a, b| a.length_squared().total_cmp(&b.length_squared()));
        if let Some(local) = nearest {
            set(ids.arrow, local.x.atan2(-local.z).to_degrees());
        }
        set(ids.guide, bearing_from(t, progress.rejoin));
        // Heading-up minimap: the player's view direction (the car's without a camera).
        let view = cameras
            .iter()
            .find(|(c, _)| c.slot == local.map_or(PlayerSlot(0), |l| l.0))
            .map_or(*t.forward(), |(_, g)| *g.forward());
        if let Some(dir) = Vec2::new(view.x, view.z).try_normalize() {
            set(ids.heading, map_rotation(dir));
        }
    }
}
