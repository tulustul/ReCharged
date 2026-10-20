//! ECS adapters: build the session's [`Track`] in `OnEnter(AppState::Loading)`, spawn its
//! colliders and props (session-scoped), reset dynamic props in `OnEnter(RacePhase::Setup)`,
//! remove the resources in `OnEnter(AppState::Unloading)`.

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use rc_core::prelude::*;

use crate::build::{KILL_FLOOR_DEPTH_U, variant_name};
use crate::mesh::MeshData;
use crate::props::{Primitive, PropDef};
use crate::track::Track;
use crate::{DragPreview, DragSource, LOADING_KEY, PropCatalog, PropSource, TrackDraft};

/// Barrier friction (low, combined with `Min`: cars slide along walls, never stick).
const WALL_FRICTION: f32 = 0.1;
/// Barrier restitution (combined with `Max`: bouncy, GDD §5.1).
const WALL_RESTITUTION: f32 = 0.3;
/// Shell wall and kill floor thickness (u).
const SHELL_THICKNESS_U: f32 = 2.0;
/// Damping of knockable props so they settle instead of rolling forever.
const PROP_LINEAR_DAMPING: f32 = 0.3;
/// Angular damping of knockable props.
const PROP_ANGULAR_DAMPING: f32 = 0.6;

/// Spawn pose of a dynamic prop (restored on every restart).
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct PropSpawn(Transform);

fn groups(layer: u32) -> CollisionGroups {
    CollisionGroups::new(Group::from_bits_truncate(layer), Group::ALL)
}

/// `OnEnter(AppState::Loading)`: build the track of `RaceConfig::variant` (from the [`TrackDraft`]
/// when it is of that track).
pub(crate) fn load_track(
    mut commands: Commands,
    config: Option<Res<RaceConfig>>,
    paths: Option<Res<UserPaths>>,
    draft: Option<Res<TrackDraft>>,
    mut done: MessageWriter<LoadingDone>,
    mut failed: MessageWriter<LoadingFailed>,
) {
    let built = match (config, paths) {
        (Some(config), Some(paths)) => match draft.filter(|d| d.id == config.variant.track) {
            Some(draft) => PropCatalog::load(&paths.assets_dir)
                .and_then(|props| Track::build(&draft.def, &props, &config.variant)),
            None => Track::load(&paths.assets_dir, &config.variant),
        },
        _ => Err("no RaceConfig or UserPaths when loading started".to_owned()),
    };
    let track = match built {
        Ok(track) => track,
        Err(error) => {
            error!("track: {error}");
            failed.write(LoadingFailed {
                key: LOADING_KEY,
                error,
            });
            return;
        }
    };
    let variant = track.variant();
    for lint in track.lints() {
        warn!(
            "track {} ({}): {lint}",
            variant.track,
            variant_name(variant)
        );
    }
    spawn_colliders(&mut commands, &track);
    spawn_props(&mut commands, &track);
    info!(
        "track {} ({}): {:.0} u, {} checkpoints, {} props, hash {:016x}",
        variant.track,
        variant_name(variant),
        track.length(),
        track.checkpoints().len(),
        track.props().len(),
        track.geometry_hash()
    );
    commands.insert_resource(track.summary());
    commands.insert_resource(track);
    done.write(LoadingDone(LOADING_KEY));
}

fn trimesh(parts: &[&MeshData]) -> Option<Collider> {
    let (vertices, triangles) = MeshData::soup(parts);
    if triangles.is_empty() {
        return None;
    }
    Collider::trimesh_with_flags(vertices, triangles, TriMeshFlags::FIX_INTERNAL_EDGES)
        .inspect_err(|e| error!("track collider: {e}"))
        .ok()
}

fn spawn_colliders(commands: &mut Commands, track: &Track) {
    let geo = track.geometry();
    let session = || DespawnOnExit(RaceSession);
    let piece = |kind| TrackPiece { kind };
    for r in &geo.ribbons {
        if let Some(c) = trimesh(&[&r.top, &r.trim, &r.body]) {
            commands.spawn((
                Name::new("track ribbon"),
                piece(TrackPieceKind::Ribbon),
                Surface(r.surface),
                c,
                groups(layers::TRACK),
                Transform::IDENTITY,
                session(),
            ));
        }
    }
    for w in &geo.walls {
        if let Some(c) = trimesh(&[w]) {
            commands.spawn((
                Name::new("track wall"),
                piece(TrackPieceKind::Wall),
                c,
                groups(layers::TRACK),
                Friction {
                    coefficient: WALL_FRICTION,
                    combine_rule: CoefficientCombineRule::Min,
                },
                Restitution {
                    coefficient: WALL_RESTITUTION,
                    combine_rule: CoefficientCombineRule::Max,
                },
                Transform::IDENTITY,
                session(),
            ));
        }
    }
    for g in &geo.ground {
        if let Some(c) = trimesh(&[&g.mesh]) {
            commands.spawn((
                Name::new("track ground"),
                piece(TrackPieceKind::Ground),
                Surface(g.surface),
                c,
                groups(layers::TRACK),
                Transform::IDENTITY,
                session(),
            ));
        }
    }
    // Shell: a kill floor below the kill height and four walls around the bounds.
    let (min, max) = track.bounds();
    let centre = (min + max) * 0.5;
    let half = (max - min) * 0.5;
    let t = SHELL_THICKNESS_U * 0.5;
    let floor_y = track.kill_height() - KILL_FLOOR_DEPTH_U - t;
    let boxes = [
        (
            Vec3::new(centre.x, floor_y, centre.z),
            Vec3::new(half.x + 2.0 * t, t, half.z + 2.0 * t),
        ),
        (
            Vec3::new(min.x - t, centre.y, centre.z),
            Vec3::new(t, half.y, half.z),
        ),
        (
            Vec3::new(max.x + t, centre.y, centre.z),
            Vec3::new(t, half.y, half.z),
        ),
        (
            Vec3::new(centre.x, centre.y, min.z - t),
            Vec3::new(half.x, half.y, t),
        ),
        (
            Vec3::new(centre.x, centre.y, max.z + t),
            Vec3::new(half.x, half.y, t),
        ),
    ];
    for (pos, h) in boxes {
        commands.spawn((
            Name::new("track shell"),
            piece(TrackPieceKind::Shell),
            Collider::cuboid(h.x, h.y, h.z),
            groups(layers::TRACK),
            Transform::from_translation(pos),
            session(),
        ));
    }
}

/// One compound collider of a prop's primitives.
pub(crate) fn prop_collider(def: &PropDef) -> Option<Collider> {
    let shapes = def
        .colliders
        .iter()
        .map(|p| {
            let (offset, rot) = p.pose();
            let shape = match *p {
                Primitive::Box { half, .. } => Some(Collider::cuboid(half.x, half.y, half.z)),
                Primitive::Ball { radius, .. } => Some(Collider::ball(radius)),
                Primitive::Cylinder {
                    radius,
                    half_height,
                    ..
                } => Some(Collider::cylinder(half_height, radius)),
                Primitive::Wedge { half, .. } => {
                    Collider::convex_hull(&Primitive::wedge_points(half))
                }
            };
            shape.map(|s| (offset, rot, s))
        })
        .collect::<Option<Vec<_>>>()?;
    Some(Collider::compound(shapes))
}

fn spawn_props(commands: &mut Commands, track: &Track) {
    for p in track.props() {
        let Some(def) = track.prop_def(&p.kind) else {
            continue;
        };
        let Some(collider) = prop_collider(def) else {
            error!("prop {}: degenerate collider", p.kind);
            continue;
        };
        let mut e = commands.spawn((
            Name::new(format!("prop {}", p.kind)),
            Prop {
                kind: p.kind.clone(),
                dynamic: def.dynamic,
            },
            PropSource(p.source),
            p.transform,
            collider,
            groups(layers::PROP),
            DespawnOnExit(RaceSession),
        ));
        if let Some(surface) = def.surface {
            e.insert(Surface(surface));
        }
        if def.dynamic {
            e.insert((
                RigidBody::Dynamic,
                ColliderMassProperties::Mass(def.mass),
                Velocity::zero(),
                Damping {
                    linear_damping: PROP_LINEAR_DAMPING,
                    angular_damping: PROP_ANGULAR_DAMPING,
                },
                Sleeping {
                    sleeping: true,
                    ..default()
                },
                Interpolated,
                PropSpawn(p.transform),
            ));
        }
    }
}

/// `OnEnter(RacePhase::Setup)`: knocked props go back to their spawn poses.
pub(crate) fn reset_props(
    mut props: Query<(Entity, &PropSpawn, &mut Transform, &mut Velocity)>,
    mut teleported: MessageWriter<Teleported>,
) {
    for (entity, spawn, mut transform, mut velocity) in &mut props {
        *transform = spawn.0;
        *velocity = Velocity::zero();
        teleported.write(Teleported { entity });
    }
}

/// The editor's [`DragPreview`] changed: the previewed prop's collider goes to the preview pose,
/// the one previewed before back to where it was. A dynamic body sits out the simulation while
/// previewed (it would fall from a pose in the air) and is teleported (its visual proxy snaps).
pub(crate) fn preview_prop(
    preview: Res<DragPreview>,
    mut props: Query<(Entity, &PropSource, &mut Transform, Has<PropSpawn>), With<Prop>>,
    mut moved: Local<Option<(Entity, Transform)>>,
    mut teleported: MessageWriter<Teleported>,
    mut commands: Commands,
) {
    let target = match preview.0 {
        Some((DragSource::Prop(i), pose)) => Some((i, pose)),
        _ => None,
    };
    let mut place = |entity: Entity, transform: &mut Transform, pose: Transform, dynamic: bool| {
        *transform = pose;
        if dynamic {
            teleported.write(Teleported { entity });
        }
    };
    if let Some((entity, original)) = *moved {
        match props.get_mut(entity) {
            Ok((_, source, mut transform, dynamic))
                if target.is_none_or(|(i, _)| source.0 != i) =>
            {
                place(entity, &mut transform, original, dynamic);
                if dynamic {
                    commands.entity(entity).remove::<RigidBodyDisabled>();
                }
                *moved = None;
            }
            Ok(_) => {}
            Err(_) => *moved = None, // despawned by a session reload
        }
    }
    let Some((i, pose)) = target else {
        *moved = None;
        return;
    };
    for (entity, source, mut transform, dynamic) in &mut props {
        if source.0 != i {
            continue;
        }
        if moved.is_none() {
            *moved = Some((entity, *transform));
            if dynamic {
                commands.entity(entity).insert(RigidBodyDisabled);
            }
        }
        place(entity, &mut transform, pose, dynamic);
    }
}

/// `OnEnter(AppState::Unloading)`: the session's resources go (entities leave with the scope).
pub(crate) fn remove_track(mut commands: Commands) {
    commands.remove_resource::<Track>();
    commands.remove_resource::<TrackSummary>();
}
