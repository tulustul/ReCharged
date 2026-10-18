//! Car bodies: built on `Added<Racer>` from the car's params (rebuilt on hot reload);
//! ghosting switches car–car collision off.

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use rc_core::prelude::*;

use crate::model::CarSpec;
use crate::params::{VehicleLibrary, VehicleParams};
use crate::state::CarState;

/// Rounded edges of the chassis collider (slides off walls instead of catching), u.
const CHASSIS_BORDER_U: f32 = 0.06;
/// Chassis friction (combine rule `Min`: walls and cars never feel sticky).
const CHASSIS_FRICTION: f32 = 0.25;
/// Chassis restitution (combine rule `Max`: bouncy off walls).
const CHASSIS_RESTITUTION: f32 = 0.3;
/// Angular damping of the body (tumbles slow down over ~1.5 s).
const ANGULAR_DAMPING: f32 = 0.5;
/// Smallest contact velocity change (u/s per tick) that reports a contact force event.
pub(crate) const IMPACT_MIN_DV_UPS: f32 = 2.0;

fn group(bits: u32) -> Group {
    Group::from_bits_truncate(bits)
}

/// Collision groups of a car; ghosted cars keep their membership (pickups, items still see
/// them) but drop other cars from their filter, so no car–car contact is possible.
pub(crate) fn car_groups(ghosted: bool) -> CollisionGroups {
    use layers::*;
    let all = TRACK | PROP | CAR | PROJECTILE | PICKUP | GHOST_CAR;
    match ghosted {
        false => CollisionGroups::new(group(CAR), group(all & !GHOST_CAR)),
        true => CollisionGroups::new(group(CAR | GHOST_CAR), group(all & !(CAR | GHOST_CAR))),
    }
}

/// Collider and mass of a car: one rounded box inside the body envelope (raised so the
/// suspension can travel), mass properties with the centre of mass from the params.
fn chassis(spec: &CarSpec) -> (Collider, ColliderMassProperties) {
    let p: &VehicleParams = &spec.params;
    let (centre, half) = spec.chassis_box();
    let inner = half - Vec3::splat(CHASSIS_BORDER_U);
    let shape = Collider::round_cuboid(inner.x, inner.y, inner.z, CHASSIS_BORDER_U);
    let mass = MassProperties {
        local_center_of_mass: spec.com,
        mass: p.mass,
        principal_inertia_local_frame: Quat::IDENTITY,
        principal_inertia: spec.inertia,
    };
    (
        Collider::compound(vec![(centre, Quat::IDENTITY, shape)]),
        ColliderMassProperties::MassProperties(mass),
    )
}

/// Adds the rapier body and the private state to new racers.
pub(crate) fn spawn_bodies(
    mut commands: Commands,
    library: Option<Res<VehicleLibrary>>,
    racers: Query<(Entity, &Racer), Added<Racer>>,
) {
    for (entity, racer) in &racers {
        let Some(params) = library.as_ref().and_then(|l| l.get(&racer.car)) else {
            error!(
                "vehicles: no params for car \"{}\"; racer has no body",
                racer.car
            );
            continue;
        };
        let spec = CarSpec::new(params);
        let (collider, mass) = chassis(&spec);
        let threshold = ContactForceEventThreshold(spec.mass() * IMPACT_MIN_DV_UPS / SIM_DT);
        commands.entity(entity).insert((
            (RigidBody::Dynamic, collider, mass, Velocity::zero()),
            (ExternalForce::default(), ExternalImpulse::default()),
            (Ccd::enabled(), Sleeping::disabled()),
            Damping {
                linear_damping: 0.0,
                angular_damping: ANGULAR_DAMPING,
            },
            Friction {
                coefficient: CHASSIS_FRICTION,
                combine_rule: CoefficientCombineRule::Min,
            },
            Restitution {
                coefficient: CHASSIS_RESTITUTION,
                combine_rule: CoefficientCombineRule::Max,
            },
            car_groups(false),
            ActiveEvents::COLLISION_EVENTS | ActiveEvents::CONTACT_FORCE_EVENTS,
            threshold,
            CollidingEntities::default(),
            CarState::new(&spec),
            spec,
        ));
    }
}

/// Dev hot reload: new params → new spec, collider and mass for every car of a changed file.
pub(crate) fn rebuild_bodies(
    library: Res<VehicleLibrary>,
    mut commands: Commands,
    cars: Query<(Entity, &Racer, &CarSpec)>,
) {
    for (entity, racer, old) in &cars {
        let Some(params) = library.get(&racer.car).filter(|p| **p != old.params) else {
            continue;
        };
        let spec = CarSpec::new(params);
        let (collider, mass) = chassis(&spec);
        commands.entity(entity).insert((collider, mass, spec));
    }
}

/// Ghosted racers (`StatusEffects::ghosted`) pass through other cars.
pub(crate) fn sync_ghosting(
    mut cars: Query<(&StatusEffects, &mut CarState, &mut CollisionGroups), With<CarSpec>>,
) {
    for (effects, mut state, mut groups) in &mut cars {
        if effects.ghosted() != state.ghosted {
            state.ghosted = effects.ghosted();
            *groups = car_groups(state.ghosted);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn interacts(a: CollisionGroups, b: CollisionGroups) -> bool {
        a.memberships.intersects(b.filters) && b.memberships.intersects(a.filters)
    }

    #[test]
    fn ghosted_cars_skip_only_cars() {
        let (car, ghost) = (car_groups(false), car_groups(true));
        let track = CollisionGroups::new(group(layers::TRACK), Group::ALL);
        let pickup = CollisionGroups::new(group(layers::PICKUP), group(layers::CAR));
        assert!(interacts(car, car));
        assert!(!interacts(car, ghost) && !interacts(ghost, ghost));
        assert!(interacts(ghost, track) && interacts(ghost, pickup));
        assert!(interacts(car, track) && interacts(car, pickup));
    }

    #[test]
    fn chassis_fits_inside_the_body_envelope() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        for car in ["bolt", "dumpster", "zip"] {
            let path = dir
                .join(crate::params::VEHICLES_DIR)
                .join(format!("{car}.ron"));
            let p = crate::params::load_file(&path).expect("loads");
            let spec = CarSpec::new(&p);
            let (collider, _) = chassis(&spec);
            let shape = collider.as_compound().expect("compound");
            let (centre, _, part) = shape.shapes().next().expect("one part");
            let ColliderView::RoundCuboid(cuboid) = part else {
                panic!("{car}: chassis is not a round cuboid");
            };
            let half = cuboid.inner_shape().half_extents() + Vec3::splat(cuboid.border_radius());
            assert!(centre.y - half.y >= p.body.clearance - 1e-4, "{car} bottom");
            assert!(centre.y + half.y <= p.body.height + 1e-4, "{car} top");
            assert!(
                half.x * 2.0 <= p.body.width && half.z * 2.0 <= p.body.length,
                "{car}"
            );
        }
    }
}
