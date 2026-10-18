//! Time Trial ghosts: recording the human's run at [`GHOST_HZ`] and playing a `GhostCar` back.
//!
//! Sample `i` is the pose after race tick `3i + 1` (the first tick after GO is race tick 1), and
//! playback interpolates between samples, so a ghost replays its run tick for tick. The ghost's
//! checkpoint times (for `CheckpointSplit`) come from projecting its samples onto the track.

use bevy::prelude::*;
use rc_core::prelude::*;
use rc_track::Track;

use crate::progress::{Tracker, crossed};

/// Race ticks between two ghost samples.
const SAMPLE_TICKS: u32 = SIM_HZ as u32 / GHOST_HZ;

/// The Time Trial human's recording of this attempt.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub(crate) struct GhostRecorder(pub Vec<GhostSample>);

/// Playback data of the ghost car.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub(crate) struct GhostRun {
    samples: Vec<GhostSample>,
    /// Fractional race ticks at which the ghost validated each checkpoint, in order.
    pub splits: Vec<f32>,
}

impl GhostRun {
    /// Playback of a recorded run on `track`.
    pub(crate) fn new(track: &Track, samples: Vec<GhostSample>) -> Self {
        let splits = split_ticks(track, &samples);
        Self { samples, splits }
    }
}

/// Race tick (fractional) of sample `i`.
fn sample_tick(i: usize) -> f32 {
    (i as u32 * SAMPLE_TICKS + 1) as f32
}

/// The recorded pose at race tick `tick` (before the first sample: the first; after the last:
/// the last).
pub(crate) fn pose_at(samples: &[GhostSample], tick: u32) -> Option<Transform> {
    let pose =
        |s: &GhostSample| Transform::from_translation(s.translation).with_rotation(s.rotation);
    let f = tick.saturating_sub(1) as f32 / SAMPLE_TICKS as f32;
    let i = f as usize;
    match (samples.get(i), samples.get(i + 1)) {
        (Some(a), Some(b)) => {
            let t = f - i as f32;
            Some(
                Transform::from_translation(a.translation.lerp(b.translation, t))
                    .with_rotation(a.rotation.slerp(b.rotation, t)),
            )
        }
        _ => samples.last().map(pose),
    }
}

/// When a recorded run passed each checkpoint (the same order the progress rules validate them,
/// starting with the start line); no gate test — it was a valid run.
pub(crate) fn split_ticks(track: &Track, samples: &[GhostSample]) -> Vec<f32> {
    let cps = track.checkpoints();
    let mut next = 0;
    let mut previous: Option<f32> = None;
    let mut out = Vec::new();
    for (i, sample) in samples.iter().enumerate() {
        let s1 = track.project(sample.translation, previous).s;
        if let Some(s0) = previous {
            let d = track.delta_s(s0, s1);
            while let Some(frac) = crossed(cps[next].s, s0, d, track.length()) {
                out.push(sample_tick(i - 1) + frac * SAMPLE_TICKS as f32);
                next = (next + 1) % cps.len();
            }
        }
        previous = Some(s1);
    }
    out
}

/// `SimSet::Race` (Racing): the Time Trial human's pose every [`SAMPLE_TICKS`].
pub(crate) fn record(
    status: Res<RaceStatus>,
    config: Option<Res<RaceConfig>>,
    mut recorder: ResMut<GhostRecorder>,
    human: Query<(&Transform, &Tracker), With<LocalPlayer>>,
) {
    let time_trial = config.is_some_and(|c| c.mode == GameMode::TimeTrial);
    let now = status.race_ticks;
    let (Ok((transform, tracker)), true) = (human.single(), time_trial) else {
        return;
    };
    if tracker.finish.is_some_and(|f| f < now)
        || !now.saturating_sub(1).is_multiple_of(SAMPLE_TICKS)
    {
        return;
    }
    recorder.0.push(GhostSample {
        translation: transform.translation,
        rotation: transform.rotation,
    });
}

/// `SimSet::Race` (Racing): the ghost car follows its run.
pub(crate) fn play(status: Res<RaceStatus>, mut ghosts: Query<(&GhostRun, &mut Transform)>) {
    for (run, mut transform) in &mut ghosts {
        if let Some(pose) = pose_at(&run.samples, status.race_ticks) {
            *transform = pose;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(x: f32) -> GhostSample {
        GhostSample {
            translation: Vec3::new(x, 0.0, 0.0),
            rotation: Quat::from_rotation_y(x * 0.01),
        }
    }

    #[test]
    fn playback_replays_the_recording_tick_for_tick() {
        let samples: Vec<GhostSample> = (0..5).map(|i| sample(10.0 * i as f32)).collect();
        let at = |tick| pose_at(&samples, tick).expect("pose").translation.x;
        assert_eq!(at(0), 0.0, "before GO: the first sample");
        assert_eq!(at(1), 0.0);
        assert_eq!(at(4), 10.0, "sample 1 = race tick 4");
        assert!((at(5) - 10.0 - 10.0 / 3.0).abs() < 1e-4, "interpolated");
        assert_eq!(at(13), 40.0);
        assert_eq!(at(500), 40.0, "the run is over: stays at the end");
        assert!(pose_at(&[], 3).is_none());
    }

    #[test]
    fn ghost_splits_follow_the_checkpoints() {
        let track = crate::progress::tests::track(false);
        // A run along the centreline at 30 u/s from the pole, one sample every 3 ticks.
        let start = track.project(track.grid()[0].translation, None).s;
        let samples: Vec<GhostSample> = (0..1200)
            .map(|i| {
                let s = start + 30.0 * SIM_DT * (i * SAMPLE_TICKS + 1) as f32;
                GhostSample {
                    translation: track.frame_at(s).point(0.0, 0.3),
                    rotation: Quat::IDENTITY,
                }
            })
            .collect();
        let splits = split_ticks(&track, &samples);
        let n = track.checkpoints().len();
        assert!(splits.len() > 2 * n, "{} splits", splits.len());
        assert!(splits.windows(2).all(|w| w[0] < w[1]), "in order");
        // The start line is 3 u ahead of the pole: 0.1 s = 6 ticks after GO.
        let to_line = track.delta_s(start, 0.0) / 30.0 * SIM_HZ as f32;
        assert!(
            (splits[0] - to_line).abs() < 0.2,
            "{} vs {to_line}",
            splits[0]
        );
        let lap = track.length() / 30.0 * SIM_HZ as f32;
        assert!((splits[n] - splits[0] - lap).abs() < 0.5, "one lap later");
    }
}
