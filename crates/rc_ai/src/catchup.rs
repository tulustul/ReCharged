//! Catch-up (GDD §8.1): AI rivals behind the leading human get up to +6 % top speed, rivals far
//! ahead lose up to −6 %, smoothly (never a visible rubber band). Off in Time Trial, when
//! `RaceConfig::ai_catch_up` is off and when no human races. rc_vehicle applies `CatchUp`.

use bevy::prelude::*;
use rc_core::prelude::*;

use crate::data::{AiTuning, CatchUpTuning};

/// Catch-up of an AI `gap` u of race distance ahead of the leading human (negative = behind).
pub fn catch_up_target(gap: f32, t: &CatchUpTuning) -> f32 {
    let excess = (gap.abs() - t.dead_zone_u).max(0.0);
    let share = (excess / (t.full_at_u - t.dead_zone_u)).min(1.0);
    -gap.signum() * share * t.max
}

/// AI rivals (not autopilot humans).
type Rivals = (With<AiDriver>, Without<LocalPlayer>);

/// `SimSet::Drivers`: every AI rival's `CatchUp`, eased toward its target.
pub(crate) fn update_catch_up(
    config: Option<Res<RaceConfig>>,
    tuning: Option<Res<AiTuning>>,
    humans: Query<&RaceProgress, With<LocalPlayer>>,
    mut rivals: Query<(&RaceProgress, &mut CatchUp), Rivals>,
) {
    let Some(tuning) = tuning else { return };
    let t = &tuning.catch_up;
    let enabled = config.is_some_and(|c| c.ai_catch_up && c.mode != GameMode::TimeTrial);
    let leader = humans
        .iter()
        .map(|p| p.total_distance)
        .max_by(f32::total_cmp)
        .filter(|_| enabled);
    let step = t.rate_per_sec * SIM_DT;
    for (progress, mut catch_up) in &mut rivals {
        let target = match leader {
            Some(lead) if !progress.finished => catch_up_target(progress.total_distance - lead, t),
            _ => 0.0,
        };
        let next = catch_up.0 + (target - catch_up.0).clamp(-step, step);
        if next != catch_up.0 {
            catch_up.0 = next;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tuning() -> CatchUpTuning {
        CatchUpTuning {
            max: 0.06,
            dead_zone_u: 10.0,
            full_at_u: 150.0,
            rate_per_sec: 0.02,
        }
    }

    #[test]
    fn behind_gains_ahead_loses_within_the_cap() {
        let t = tuning();
        assert_eq!(catch_up_target(0.0, &t), 0.0);
        assert_eq!(catch_up_target(-8.0, &t), 0.0, "dead zone");
        assert!(
            (catch_up_target(-80.0, &t) - 0.03).abs() < 1e-6,
            "halfway behind"
        );
        assert!(
            (catch_up_target(80.0, &t) + 0.03).abs() < 1e-6,
            "halfway ahead"
        );
        assert_eq!(catch_up_target(-900.0, &t), 0.06);
        assert_eq!(catch_up_target(900.0, &t), -0.06);
    }
}
