//! Reading the race for the HUD: [`HudView`] builds `race.setup`, `race.hud`, `viewports` and the
//! snapshot; [`MinimapFit`] maps the world onto the minimap; the `RaceEvent` → `race.event` map.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use rc_core::prelude::*;

use super::{
    UiHudSetup, UiHudSnapshot, UiHudTick, UiPlayerHud, UiRaceEvent, UiRect, UiViewport, UiViewports,
};
use crate::race::UiRacer;
use crate::results::ticks_to_ms;

/// Most outline points sent for the minimap.
const OUTLINE_POINTS: usize = 240;
/// Minimap margin inside its 0–1 square.
pub(super) const MAP_PAD: f32 = 0.06;

/// Maps world X/Z onto the minimap's 0–1 square: aspect kept, centred, padded by [`MAP_PAD`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MinimapFit {
    origin: Vec2,
    scale: f32,
    offset: Vec2,
}

impl MinimapFit {
    /// The fit of a track outline (world X/Z).
    pub fn new(outline: &[Vec2]) -> Self {
        let Some(&first) = outline.first() else {
            return Self {
                origin: Vec2::ZERO,
                scale: 0.0,
                offset: Vec2::splat(0.5),
            };
        };
        let (min, max) = outline
            .iter()
            .fold((first, first), |(lo, hi), &p| (lo.min(p), hi.max(p)));
        let scale = (1.0 - 2.0 * MAP_PAD) / (max - min).max_element().max(1e-3);
        Self {
            origin: min,
            scale,
            offset: (Vec2::ONE - (max - min) * scale) / 2.0,
        }
    }

    /// World X/Z → minimap 0–1.
    pub fn project(&self, p: Vec2) -> Vec2 {
        (p - self.origin) * self.scale + self.offset
    }
}

/// The outline as flat minimap x, y pairs (at most [`OUTLINE_POINTS`] points).
pub(super) fn outline_points(outline: &[Vec2]) -> Vec<f32> {
    let fit = MinimapFit::new(outline);
    let step = outline.len().div_ceil(OUTLINE_POINTS).max(1);
    let round = |v: f32| (v * 10_000.0).round() / 10_000.0;
    outline
        .iter()
        .step_by(step)
        .flat_map(|&p| {
            let q = fit.project(p);
            [round(q.x), round(q.y)]
        })
        .collect()
}

fn tenths(v: f32) -> f32 {
    (v * 10.0).round() / 10.0
}

/// A racer event for the HUD; `slot` = the racer's local player, if it has one.
pub(super) fn race_event(
    kind: RaceEventKind,
    slot: Option<u8>,
    fastest: bool,
) -> Option<UiRaceEvent> {
    use RaceEventKind as K;
    Some(match (kind, slot) {
        (K::CountdownTick(n), _) => UiRaceEvent::Countdown { n },
        (K::Go, _) => UiRaceEvent::Go,
        (K::RaceEndsIn(secs), _) => UiRaceEvent::EndsIn { secs },
        (_, None) => return None,
        (K::PerfectStart, Some(slot)) => UiRaceEvent::PerfectStart { slot },
        (K::Stall, Some(slot)) => UiRaceEvent::Stall { slot },
        (K::CheckpointSplit(secs), Some(slot)) => UiRaceEvent::Split {
            slot,
            secs: (secs * 100.0).round() / 100.0,
        },
        (K::LapCompleted { lap, time, best }, Some(slot)) => UiRaceEvent::Lap {
            slot,
            lap,
            ms: ticks_to_ms(time),
            best,
            fastest,
        },
        (K::FinalLap, Some(slot)) => UiRaceEvent::FinalLap { slot },
        (K::Finished { rank, time }, Some(slot)) => UiRaceEvent::Finished {
            slot,
            rank,
            ms: ticks_to_ms(time),
        },
        (K::GoldenBattery, Some(slot)) => UiRaceEvent::GoldenBattery { slot },
        (K::Hint(hint), Some(slot)) => UiRaceEvent::Hint {
            slot,
            hint: hint.into(),
        },
        // `race.hud` carries the warning while it lasts.
        (K::WrongWay(_), Some(_)) => return None,
        // T19b: show "Missed checkpoint — go back" (B15).
        (K::MissedCheckpoint, Some(_)) => return None,
    })
}

type RacerData = (
    Entity,
    &'static Racer,
    &'static RaceProgress,
    &'static VehicleTelemetry,
    &'static Charge,
    &'static CarControl,
    &'static ItemSlot,
    &'static StatusEffects,
    Option<&'static LocalPlayer>,
);

type ItemData = (
    Option<&'static Projectile>,
    Option<&'static Hazard>,
    &'static ItemTarget,
    Option<&'static ItemTimer>,
);

/// What the HUD shows, read from the race (the systems and `race.snapshot` share it).
#[derive(SystemParam)]
pub(crate) struct HudView<'w, 's> {
    config: Option<Res<'w, RaceConfig>>,
    status: Option<Res<'w, RaceStatus>>,
    standings: Option<Res<'w, Standings>>,
    summary: Option<Res<'w, TrackSummary>>,
    settings: Option<Res<'w, Settings>>,
    viewports: Option<Res<'w, PlayerViewports>>,
    scale: Option<Res<'w, UiScale>>,
    racers: Query<'w, 's, RacerData>,
    items: Query<'w, 's, ItemData>,
}

impl HudView<'_, '_> {
    /// `race.setup` (needs a `RaceConfig`).
    pub(crate) fn setup(&self) -> Option<UiHudSetup> {
        let config = self.config.as_deref()?;
        let defaults = Settings::default();
        let settings = self.settings.as_deref().unwrap_or(&defaults);
        Some(UiHudSetup {
            mode: (&config.mode).into(),
            laps: config.laps,
            items: config.items,
            ghost: config.ghost.is_some(),
            racers: UiRacer::of(config),
            outline: self
                .summary
                .as_deref()
                .map(|s| outline_points(&s.outline_2d))
                .unwrap_or_default(),
            speed_unit: settings.gameplay.speed_unit.into(),
            hud_scale: settings.gameplay.hud_scale,
            minimap: settings.gameplay.minimap,
            reduce_flashing: settings.accessibility.reduce_flashing,
        })
    }

    /// `race.hud` (needs a `RaceStatus`, i.e. an attempt).
    pub(crate) fn tick(&self) -> Option<UiHudTick> {
        let status = self.status.as_deref()?;
        let laps = status.total_laps.max(1);
        let standings = self
            .standings
            .as_deref()
            .map(|s| {
                s.0.iter()
                    .filter_map(|&e| self.racers.get(e).ok().map(|r| r.1.index as u8))
                    .collect()
            })
            .unwrap_or_default();
        let mut players: Vec<UiPlayerHud> = self
            .racers
            .iter()
            .filter_map(
                |(entity, racer, progress, tel, charge, control, slot, effects, local)| {
                    let local = local?;
                    let done: u32 = progress.lap_times.iter().sum();
                    let finished = progress.finished;
                    let targets = |e: Entity| self.items.iter().filter(move |i| i.2.0 == Some(e));
                    Some(UiPlayerHud {
                        slot: local.0.0,
                        racer: racer.index as u8,
                        position: progress.rank.max(1),
                        lap: progress.lap.clamp(1, laps),
                        lap_ms: if finished {
                            0
                        } else {
                            ticks_to_ms(status.race_ticks.saturating_sub(done))
                        },
                        last_lap_ms: progress.lap_times.last().map(|&t| ticks_to_ms(t)),
                        best_lap_ms: progress.best_lap.map(ticks_to_ms),
                        finish_ms: finished.then(|| ticks_to_ms(done)),
                        kmh: (tel.speed * SPEED_DISPLAY_KMH).round() as u16,
                        charge: tenths(charge.0),
                        boost_ready: charge.0 >= BOOST_MIN_CHARGE,
                        spending: control.boost && tel.boosting && charge.0 > 0.0,
                        item: slot.item.map(Into::into),
                        roulette: slot.item.is_none() && slot.roulette > 0.0,
                        wrong_way: progress.wrong_way,
                        lock_on: targets(entity)
                            .any(|i| i.0.is_some_and(|p| p.kind == ItemKind::SparkRocket)),
                        goo: targets(entity)
                            .find(|i| i.1.is_some_and(|h| h.kind == HazardKind::GooBomb))
                            .map(|i| tenths(i.3.map_or(0.0, |t| t.0))),
                        shield: effects.shield.map(tenths),
                    })
                },
            )
            .collect();
        players.sort_by_key(|p| p.slot);
        Some(UiHudTick {
            race_ms: ticks_to_ms(status.race_ticks),
            countdown: status.countdown,
            ends_in: status.ends_in.map(|s| s.ceil().clamp(0.0, 255.0) as u8),
            standings,
            players,
        })
    }

    /// `viewports`.
    pub(crate) fn viewports(&self) -> UiViewports {
        let scale = self.scale.as_deref().map_or(1.0, |s| s.0).max(1e-3);
        let players: Vec<UiViewport> = self
            .viewports
            .as_deref()
            .map(|v| {
                v.0.iter()
                    .map(|(slot, r)| UiViewport {
                        slot: slot.0,
                        rect: UiRect {
                            x: r.min.x / scale,
                            y: r.min.y / scale,
                            w: r.width() / scale,
                            h: r.height() / scale,
                        },
                    })
                    .collect()
            })
            .unwrap_or_default();
        let panel = (players.len() == 3).then(|| UiRect {
            x: players[1].rect.x,
            y: players[2].rect.y,
            w: players[1].rect.w,
            h: players[2].rect.h,
        });
        UiViewports { players, panel }
    }

    /// The HUD part of `race.snapshot` (inside a race session).
    pub(crate) fn snapshot(&self) -> Option<UiHudSnapshot> {
        Some(UiHudSnapshot {
            setup: self.setup()?,
            tick: self.tick(),
            viewports: self.viewports(),
        })
    }

    pub(super) fn inputs_changed(&self) -> bool {
        self.config.as_ref().is_some_and(|r| r.is_changed())
            || self.summary.as_ref().is_some_and(|r| r.is_changed())
            || self.settings.as_ref().is_some_and(|r| r.is_changed())
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::SystemState;

    use super::*;
    use crate::hud::UiRaceEvent;

    #[test]
    fn the_tick_reads_each_local_player() {
        let mut world = World::new();
        world.insert_resource(RaceStatus {
            race_ticks: 600,
            total_laps: 3,
            ..default()
        });
        let me = world
            .spawn((
                Racer {
                    index: 2,
                    ..default()
                },
                LocalPlayer(PlayerSlot(1)),
                RaceProgress {
                    lap: 2,
                    lap_times: vec![300],
                    rank: 4,
                    ..default()
                },
                ItemSlot {
                    item: None,
                    roulette: 0.5,
                },
                Charge(0.2),
            ))
            .id();
        let ai = world.spawn(Racer::default()).id();
        world.insert_resource(Standings(vec![ai, me]));
        world.spawn((
            Projectile {
                kind: ItemKind::SparkRocket,
                owner: ai,
            },
            ItemTarget(Some(me)),
        ));
        world.spawn((
            Hazard {
                kind: HazardKind::GooBomb,
                owner: ai,
            },
            ItemTarget(Some(me)),
            ItemTimer(2.34),
        ));
        let mut state = SystemState::<HudView>::new(&mut world);
        let view = state.get(&world).expect("the params are valid");
        let tick = view.tick().expect("an attempt runs");
        assert_eq!(tick.standings, vec![0, 2]);
        assert_eq!(tick.players.len(), 1, "AI racers have no HUD");
        let p = &tick.players[0];
        assert_eq!((p.slot, p.racer, p.position, p.lap), (1, 2, 4, 2));
        assert_eq!(p.lap_ms, 5_000, "race time minus completed laps");
        assert_eq!(p.last_lap_ms, Some(5_000));
        assert!(p.roulette && p.lock_on && p.boost_ready && p.finish_ms.is_none());
        assert_eq!(p.goo, Some(2.3));
    }

    #[test]
    fn the_minimap_keeps_the_aspect_and_centres_the_track() {
        let fit = MinimapFit::new(&[
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 5.0),
            Vec2::new(0.0, 5.0),
        ]);
        let near = |a: Vec2, b: Vec2| a.distance(b) < 1e-5;
        assert!(near(fit.project(Vec2::new(5.0, 2.5)), Vec2::splat(0.5)));
        assert!(near(fit.project(Vec2::ZERO), Vec2::new(MAP_PAD, 0.28)));
        assert!(near(
            fit.project(Vec2::new(10.0, 5.0)),
            Vec2::new(1.0 - MAP_PAD, 0.72)
        ));
        assert_eq!(
            MinimapFit::new(&[]).project(Vec2::splat(7.0)),
            Vec2::splat(0.5)
        );
        assert_eq!(outline_points(&[Vec2::ZERO; 1000]).len(), 2 * 200);
    }

    #[test]
    fn race_events_reach_only_local_players_except_global_ones() {
        assert_eq!(
            race_event(RaceEventKind::Go, None, false),
            Some(UiRaceEvent::Go)
        );
        assert_eq!(
            race_event(RaceEventKind::RaceEndsIn(7), None, false),
            Some(UiRaceEvent::EndsIn { secs: 7 })
        );
        assert_eq!(race_event(RaceEventKind::FinalLap, None, false), None);
        assert_eq!(
            race_event(RaceEventKind::WrongWay(true), Some(0), false),
            None
        );
        assert_eq!(
            race_event(
                RaceEventKind::LapCompleted {
                    lap: 2,
                    time: 1800,
                    best: true
                },
                Some(1),
                true
            ),
            Some(UiRaceEvent::Lap {
                slot: 1,
                lap: 2,
                ms: 30_000,
                best: true,
                fastest: true
            })
        );
        assert_eq!(
            race_event(RaceEventKind::CheckpointSplit(-0.4249), Some(0), false),
            Some(UiRaceEvent::Split {
                slot: 0,
                secs: -0.42
            })
        );
    }
}
