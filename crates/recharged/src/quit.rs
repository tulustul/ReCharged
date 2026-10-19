//! Deferred exit for human runs (B2). Quitting while render pipelines still compile on Bevy's
//! task threads drops the GPU device under them and crashes the NVIDIA driver (exit 139).
//! Automated runs compile synchronously; human runs keep the window open after a close request
//! or an `AppExit` (Quit) until rc_render's [`PipelineQueue`] has been idle for a few
//! frames, at most [`MAX_WAIT_SECS`], then exit with the requested code.

use bevy::app::OnAppExitSystems;
use bevy::prelude::*;
use bevy::window::{ExitSystems, WindowCloseRequested};
use rc_render::PipelineQueue;

/// Longest wait for the pipeline queue, s (wall clock).
const MAX_WAIT_SECS: f32 = 3.0;
/// Consecutive frames with an idle queue before exiting (the render world reports a frame late).
const IDLE_FRAMES: u32 = 2;

/// Install with `WindowPlugin::close_when_requested = false`: this decides when the window goes.
pub fn plugin(app: &mut App) {
    app.add_systems(Last, defer_exit.after(ExitSystems).before(OnAppExitSystems));
}

/// An exit that waits for the pipeline queue.
#[derive(Debug, Default)]
struct Gate {
    /// Requested exit code and when it was first requested (s).
    pending: Option<(AppExit, f32)>,
    idle: u32,
}

impl Gate {
    /// Records this frame's requests; returns the exit to send now, if any.
    fn update(&mut self, request: Option<AppExit>, busy: bool, now: f32) -> Option<AppExit> {
        if let Some(code) = request {
            // An error code wins over success; the wait starts at the first request.
            self.pending = Some(match self.pending.take() {
                Some((old, since)) if old.is_error() => (old, since),
                Some((_, since)) => (code, since),
                None => (code, now),
            });
        }
        let (code, since) = self.pending.as_ref()?;
        self.idle = if busy { 0 } else { self.idle + 1 };
        let waited = now - since;
        (self.idle >= IDLE_FRAMES || waited >= MAX_WAIT_SECS).then(|| {
            if busy {
                warn!("exit: pipelines still compiling after {waited:.1} s; exiting anyway");
            } else {
                info!("exit: pipeline queue idle after {waited:.2} s");
            }
            code.clone()
        })
    }
}

/// Takes every `AppExit` and window close request of the frame and lets one exit through once
/// the pipeline queue is idle (`OnAppExitSystems`, e.g. rc_game's save flush, then run with it).
fn defer_exit(
    mut closes: MessageReader<WindowCloseRequested>,
    mut exits: ResMut<Messages<AppExit>>,
    queue: Res<PipelineQueue>,
    time: Res<Time<Real>>,
    mut gate: Local<Gate>,
) {
    let closing = closes.read().count() > 0;
    let request = exits
        .drain()
        .chain(closing.then_some(AppExit::Success))
        .reduce(|a, b| if a.is_error() { a } else { b });
    if let Some(code) = gate.update(request, queue.waiting() > 0, time.elapsed_secs()) {
        exits.write(code);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_are_held_back_until_the_gate_opens() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<AppExit>()
            .add_message::<WindowCloseRequested>()
            .init_resource::<PipelineQueue>()
            .add_plugins(plugin);
        app.update();
        app.world_mut().write_message(WindowCloseRequested {
            window: Entity::PLACEHOLDER,
        });
        app.update();
        assert_eq!(
            app.should_exit(),
            None,
            "held back: idle for one frame only"
        );
        app.update();
        assert_eq!(app.should_exit(), Some(AppExit::Success));
    }

    #[test]
    fn exit_waits_for_an_idle_queue_or_the_cap() {
        let mut gate = Gate::default();
        assert_eq!(gate.update(None, true, 0.0), None, "nothing requested");
        assert_eq!(
            gate.update(Some(AppExit::Success), true, 1.0),
            None,
            "compiling"
        );
        assert_eq!(gate.update(None, false, 1.1), None, "idle for one frame");
        assert_eq!(gate.update(None, false, 1.2), Some(AppExit::Success));

        let mut gate = Gate::default();
        assert_eq!(gate.update(Some(AppExit::from_code(3)), true, 0.0), None);
        assert_eq!(gate.update(Some(AppExit::Success), true, 1.0), None);
        assert_eq!(
            gate.update(None, true, MAX_WAIT_SECS),
            Some(AppExit::from_code(3)),
            "the cap ends the wait; the error code wins"
        );
    }
}
