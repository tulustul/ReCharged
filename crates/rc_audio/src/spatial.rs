//! Our own spatialisation (bevy_kira_audio's supports one listener): every `GameCamera` is a
//! listener; a sound is attenuated by its distance to the **nearest** one and panned only when
//! there is a single listener (one stereo output cannot serve several split-screen viewpoints).

use bevy::prelude::*;
use rc_core::prelude::*;

use crate::Step;
use crate::buses::{SILENCE_DB, gain_db};

/// How far a sound carries, in world units (1 u = 22.5 cm; a car is ≈ 2 u long).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Reach {
    /// Full level up to this distance.
    pub(crate) near: f32,
    /// Silent from this distance.
    pub(crate) far: f32,
}

/// Engines, tyres, impacts.
pub(crate) const REACH_CAR: Reach = Reach {
    near: 8.0,
    far: 90.0,
};
/// Explosions, EMP, lightning strikes.
pub(crate) const REACH_LOUD: Reach = Reach {
    near: 12.0,
    far: 170.0,
};
/// Pickups, beeps, small items.
pub(crate) const REACH_SMALL: Reach = Reach {
    near: 6.0,
    far: 55.0,
};
/// The amplitude fade to silence starts at this fraction of `far`.
const FADE_START: f32 = 0.7;
/// Panning width (1 = hard left/right for sounds at 90°).
const PAN_WIDTH: f32 = 0.8;

/// A listener: a game camera's position and right vector.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Ear {
    pub(crate) pos: Vec3,
    pub(crate) right: Vec3,
}

/// This frame's listeners (empty in menus: sounds then play unattenuated and centred).
#[derive(Resource, Debug, Default)]
pub(crate) struct Ears(pub(crate) Vec<Ear>);

/// Level offset and panning of a sound for the current listeners.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Spot {
    pub(crate) db: f32,
    pub(crate) pan: f32,
}

impl Spot {
    /// Unattenuated and centred.
    pub(crate) const CENTRE: Spot = Spot { db: 0.0, pan: 0.0 };
}

pub(crate) fn build(app: &mut App) {
    app.init_resource::<Ears>()
        .add_systems(Update, gather_ears.in_set(Step::Listen));
}

fn gather_ears(cameras: Query<&GlobalTransform, With<GameCamera>>, mut ears: ResMut<Ears>) {
    ears.0.clear();
    ears.0.extend(cameras.iter().map(|t| Ear {
        pos: t.translation(),
        right: t.right().as_vec3(),
    }));
}

/// dB offset at `distance`: 0 within `near`, −6 dB per doubling beyond, fading to silence at `far`.
pub(crate) fn attenuation_db(distance: f32, reach: Reach) -> f32 {
    if distance >= reach.far {
        return SILENCE_DB;
    }
    let rolloff = if distance <= reach.near {
        0.0
    } else {
        -20.0 * (distance / reach.near).log10()
    };
    let fade_from = FADE_START * reach.far;
    let fade = if distance <= fade_from {
        1.0
    } else {
        1.0 - (distance - fade_from) / (reach.far - fade_from)
    };
    (rolloff + gain_db(fade)).max(SILENCE_DB)
}

/// Pan −1 (left) … 1 (right) of `pos` heard by `ear`.
pub(crate) fn pan_for(ear: &Ear, pos: Vec3) -> f32 {
    let dir = (pos - ear.pos).normalize_or_zero();
    (dir.dot(ear.right) * PAN_WIDTH).clamp(-1.0, 1.0)
}

/// Level and pan of a sound at `pos`.
pub(crate) fn spot(ears: &[Ear], pos: Vec3, reach: Reach) -> Spot {
    let nearest = ears
        .iter()
        .map(|e| (e.pos.distance(pos), e))
        .min_by(|a, b| a.0.total_cmp(&b.0));
    let Some((distance, ear)) = nearest else {
        return Spot::CENTRE;
    };
    Spot {
        db: attenuation_db(distance, reach),
        pan: if ears.len() == 1 {
            pan_for(ear, pos)
        } else {
            0.0
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const R: Reach = Reach {
        near: 10.0,
        far: 100.0,
    };

    fn ear(x: f32) -> Ear {
        Ear {
            pos: Vec3::new(x, 0.0, 0.0),
            right: Vec3::X,
        }
    }

    #[test]
    fn attenuation_rolls_off_6_db_per_doubling_then_fades_to_silence() {
        assert_eq!(attenuation_db(0.0, R), 0.0);
        assert_eq!(attenuation_db(R.near, R), 0.0);
        assert!((attenuation_db(20.0, R) - -6.02).abs() < 0.01);
        assert!((attenuation_db(40.0, R) - -12.04).abs() < 0.01);
        assert_eq!(attenuation_db(R.far, R), SILENCE_DB);
        assert_eq!(attenuation_db(1e6, R), SILENCE_DB);
        let mut prev = 1.0;
        for d in (0..100).map(|i| i as f32) {
            let db = attenuation_db(d, R);
            assert!(db <= prev, "monotonic at {d}");
            prev = db;
        }
    }

    #[test]
    fn pans_toward_the_side_of_the_source() {
        let e = ear(0.0);
        assert!(pan_for(&e, Vec3::new(5.0, 0.0, 0.0)) > 0.7, "right");
        assert!(pan_for(&e, Vec3::new(-5.0, 0.0, 0.0)) < -0.7, "left");
        assert_eq!(pan_for(&e, Vec3::new(0.0, 0.0, -5.0)), 0.0, "ahead");
        assert_eq!(pan_for(&e, e.pos), 0.0, "on the listener");
    }

    #[test]
    fn uses_the_nearest_listener_and_centres_in_split_screen() {
        let pos = Vec3::new(90.0, 0.0, 0.0);
        let one = spot(&[ear(0.0)], pos, R);
        let two = spot(&[ear(0.0), ear(85.0)], pos, R);
        assert!(one.db < -15.0 && one.pan > 0.0);
        assert_eq!(
            two,
            Spot { db: 0.0, pan: 0.0 },
            "5 u from the second camera, centred"
        );
        assert_eq!(spot(&[], pos, R), Spot::CENTRE, "menus: no listeners");
    }
}
