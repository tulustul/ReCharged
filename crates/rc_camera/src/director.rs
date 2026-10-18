//! What each game camera looks at (GDD §3.2, §11): its slot's racer (re-bound on every attempt,
//! so a restart keeps the camera), the intro clock, the finish orbit, spectating other racers and
//! cycling the camera mode.

use bevy::prelude::*;
use rc_core::prelude::*;

use crate::follow::{Rig, View};

/// Seconds since the intro fly-by started.
#[derive(Resource, Debug, Default)]
pub(crate) struct IntroClock(pub f32);

pub(crate) fn build(app: &mut App) {
    app.init_resource::<IntroClock>()
        .add_systems(OnEnter(RaceAttempt), rebind)
        .add_systems(OnEnter(RacePhase::Intro), |mut c: ResMut<IntroClock>| {
            c.0 = 0.0
        });
}

/// `OnEnter(RaceAttempt)`: the attempt's racers are new entities; forget the old ones.
fn rebind(mut rigs: Query<&mut Rig>) {
    for mut rig in &mut rigs {
        rig.target = None;
        rig.view = View::Follow;
        rig.trauma = 0.0;
        rig.snap = true;
    }
}

/// Advances the fly-by clock while the intro runs (it freezes with the pause).
pub(crate) fn tick_intro(
    time: Res<Time>,
    phase: Option<Res<State<RacePhase>>>,
    pause: Option<Res<State<PauseState>>>,
    mut clock: ResMut<IntroClock>,
) {
    let running = pause.is_some_and(|p| *p.get() == PauseState::Running);
    if running && phase.is_some_and(|p| *p.get() == RacePhase::Intro) {
        clock.0 += time.delta_secs();
    }
}

/// Binds each camera to its slot's racer; a slot without one spectates the leader. Teleports of
/// the followed racer snap the camera.
pub(crate) fn bind_targets(
    mut rigs: Query<(&GameCamera, &mut Rig)>,
    racers: Query<(Entity, &Racer, Option<&LocalPlayer>)>,
    standings: Option<Res<Standings>>,
    mut teleported: MessageReader<Teleported>,
) {
    for (camera, mut rig) in &mut rigs {
        if let View::Spectate(other) = rig.view
            && !racers.contains(other)
        {
            rig.view = View::Follow;
            rig.snap = true;
        }
        if rig.target.is_some_and(|e| racers.contains(e)) {
            continue;
        }
        let own = racers
            .iter()
            .find(|(.., local)| local.is_some_and(|l| l.0 == camera.slot))
            .map(|(e, ..)| e);
        let leader = || {
            standings
                .as_ref()
                .and_then(|s| s.0.first().copied())
                .or_else(|| {
                    racers
                        .iter()
                        .min_by_key(|(_, r, _)| r.index)
                        .map(|(e, ..)| e)
                })
        };
        let target = own.or_else(leader);
        if target != rig.target {
            rig.target = target;
            rig.snap = true;
        }
    }
    for Teleported { entity } in teleported.read() {
        for (_, mut rig) in &mut rigs {
            if rig.followed() == Some(*entity) {
                rig.snap = true;
            }
        }
    }
}

/// Chase → far chase → hood → chase.
pub(crate) fn next_mode(mode: CameraMode) -> CameraMode {
    match mode {
        CameraMode::Chase => CameraMode::FarChase,
        CameraMode::FarChase => CameraMode::Hood,
        CameraMode::Hood => CameraMode::Chase,
    }
}

/// After finishing: orbit → spectate each other racer in grid order → orbit.
pub(crate) fn next_view(view: View, own: Entity, racers_by_index: &[Entity]) -> View {
    let position = |e: Entity| racers_by_index.iter().position(|&r| r == e);
    let from = match view {
        View::Spectate(other) => position(other),
        View::Follow | View::Orbit => position(own),
    };
    let Some(from) = from else {
        return View::Orbit;
    };
    let next = racers_by_index[(from + 1) % racers_by_index.len()];
    if next == own {
        View::Orbit
    } else {
        View::Spectate(next)
    }
}

/// The Camera action: cycles the mode, or the spectated racer once the slot's racer finished.
pub(crate) fn cycle_views(
    mut presses: MessageReader<CycleCamera>,
    mut rigs: Query<(&GameCamera, &mut Rig)>,
    racers: Query<(Entity, &Racer, &RaceProgress)>,
) {
    for press in presses.read() {
        for (camera, mut rig) in &mut rigs {
            if camera.slot != press.slot {
                continue;
            }
            let finished = rig
                .target
                .and_then(|e| racers.get(e).ok())
                .is_some_and(|(.., p)| p.finished);
            match (finished, rig.target) {
                (true, Some(own)) => {
                    let mut order: Vec<(usize, Entity)> =
                        racers.iter().map(|(e, r, _)| (r.index, e)).collect();
                    order.sort_unstable_by_key(|&(index, _)| index);
                    let entities: Vec<Entity> = order.into_iter().map(|(_, e)| e).collect();
                    rig.view = next_view(rig.view, own, &entities);
                }
                _ => rig.mode = next_mode(rig.mode),
            }
            rig.snap = true;
        }
    }
}

/// A slot's racer that finished gets the orbit camera (once; spectating keeps going after).
pub(crate) fn finish_orbit(mut rigs: Query<&mut Rig>, progress: Query<&RaceProgress>) {
    for mut rig in &mut rigs {
        let finished = rig
            .target
            .and_then(|e| progress.get(e).ok())
            .is_some_and(|p| p.finished);
        if finished && rig.view == View::Follow {
            rig.view = View::Orbit;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_cycle_through_all_three() {
        let mut m = CameraMode::Chase;
        for expected in [CameraMode::FarChase, CameraMode::Hood, CameraMode::Chase] {
            m = next_mode(m);
            assert_eq!(m, expected);
        }
    }

    #[test]
    fn spectating_visits_every_other_racer_then_returns_to_the_orbit() {
        let mut world = World::new();
        let racers: Vec<Entity> = (0..4).map(|_| world.spawn_empty().id()).collect();
        let own = racers[1];
        let mut view = View::Orbit;
        let mut seen = Vec::new();
        for _ in 0..3 {
            view = next_view(view, own, &racers);
            let View::Spectate(e) = view else {
                panic!("expected spectate, got {view:?}");
            };
            seen.push(e);
        }
        assert_eq!(seen, [racers[2], racers[3], racers[0]]);
        assert_eq!(next_view(view, own, &racers), View::Orbit);
        assert_eq!(
            next_view(View::Orbit, own, &[own]),
            View::Orbit,
            "alone: stays"
        );
        assert_eq!(next_view(View::Orbit, own, &[]), View::Orbit);
    }
}
