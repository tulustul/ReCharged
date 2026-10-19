//! What every car effect needs: each car's wheel layout ([`CarShape`], from the model's
//! `<car>.wheels.ron`), per-racer effect state ([`CarFx`]), visual poses ([`Poses`]: the proxy,
//! never the rapier body), distance-to-camera emission LOD ([`Eyes`]) and the car's velocity.

use std::collections::HashMap;
use std::path::Path;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use rc_core::prelude::*;
use serde::Deserialize;

use crate::pool::Emitter;

/// Distance (u) from the nearest camera where emission starts thinning out, and where it reaches
/// its floor of [`FAR_EMISSION`].
const NEAR_U: f32 = 25.0;
const FAR_U: f32 = 90.0;
/// Emission multiplier for far cars.
const FAR_EMISSION: f32 = 0.15;
/// Slip (0–1, `WheelState::slip`) above which hard surfaces smoke and mark.
pub const SKID_SLIP: f32 = 0.35;

/// Car geometry the effects need, from the model's `<car>.wheels.ron` (art contract).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CarShape {
    /// Wheel centres in car space at rest (FL, FR, RL, RR; Bevy axes, front = −Z).
    pub wheels: [Vec3; 4],
    /// Wheel radii, u.
    pub radius: [f32; 4],
    /// Tyre widths, u.
    pub width: [f32; 4],
}

impl Default for CarShape {
    /// A mid-size car (Sparky's layout) for cars without a wheel file.
    fn default() -> Self {
        Self {
            wheels: [
                Vec3::new(-0.55, 0.3, -0.65),
                Vec3::new(0.55, 0.3, -0.65),
                Vec3::new(-0.55, 0.32, 0.65),
                Vec3::new(0.55, 0.32, 0.65),
            ],
            radius: [0.3, 0.3, 0.32, 0.32],
            width: [0.22, 0.22, 0.25, 0.25],
        }
    }
}

impl CarShape {
    /// Contact patch of wheel `i` in car space for a suspension `offset` (u, + = pushed up).
    pub fn contact(&self, i: usize, offset: f32) -> Vec3 {
        self.wheels[i] + Vec3::Y * (offset - self.radius[i])
    }

    /// Exhaust / boost nozzle in car space: centred, just behind the rear tyres.
    pub fn exhaust(&self) -> Vec3 {
        let z = self.wheels[2].z.max(self.wheels[3].z) + self.radius[2] + 0.1;
        Vec3::new(0.0, self.wheels[2].y + 0.06, z)
    }

    /// Roof height in car space (Goo Bomb, name tags).
    pub fn roof(&self) -> f32 {
        self.wheels[2].y + self.radius[2] * 1.6
    }

    /// Half the car's length (shield bubble, respawn ring).
    pub fn half_length(&self) -> f32 {
        let front = self.wheels[0].z.min(self.wheels[1].z) - self.radius[0];
        let rear = self.wheels[2].z.max(self.wheels[3].z) + self.radius[2];
        (rear - front) * 0.5 + 0.15
    }

    /// Parses a `<car>.wheels.ron` file.
    pub fn parse(text: &str) -> Result<Self, String> {
        #[derive(Deserialize)]
        struct Wheel {
            center: (f32, f32, f32),
            radius: f32,
            width: f32,
        }
        #[derive(Deserialize)]
        struct File {
            wheels: Vec<Wheel>,
        }
        let file: File = ron::from_str(text).map_err(|e| e.to_string())?;
        let w: [Wheel; 4] = file
            .wheels
            .try_into()
            .map_err(|v: Vec<Wheel>| format!("expected 4 wheels, found {}", v.len()))?;
        Ok(Self {
            wheels: w
                .each_ref()
                .map(|w| Vec3::new(w.center.0, w.center.1, w.center.2)),
            radius: w.each_ref().map(|w| w.radius),
            width: w.each_ref().map(|w| w.width),
        })
    }
}

/// Geometry of every car of the session (loaded in `OnEnter(Loading)`).
#[derive(Resource, Debug, Default)]
pub struct CarShapes(pub HashMap<CarId, CarShape>);

impl CarShapes {
    /// The shape of `car` (the default layout when unknown).
    pub fn get(&self, car: &CarId) -> CarShape {
        self.0.get(car).copied().unwrap_or_default()
    }
}

/// Per-racer effect state (decorates the racer entity).
#[derive(Component, Debug, Default)]
pub(crate) struct CarFx {
    pub shape: CarShape,
    pub sparks: [Emitter; 2],
    pub debris: [Emitter; 4],
    pub flame: Emitter,
    pub arcs: Emitter,
    /// Smoothed drift charge rate, 0–1.25 of `driving::DRIFT_RATE_FULL`.
    pub charge: f32,
    pub was_boosting: bool,
}

/// Marker on effect entities whose `Transform` rc_vfx writes every frame (keeps them out of
/// [`Poses`], so one system can read poses and move visuals).
#[derive(Component, Debug, Clone, Copy, Default)]
pub(crate) struct FxVisual;

impl CarFx {
    /// Fresh effect state for a car of `shape`.
    pub fn new(shape: CarShape) -> Self {
        Self { shape, ..default() }
    }
}

/// Visual pose of a simulation entity: its rc_render proxy's `Transform` when it has one, else
/// its own. Effects never read rapier bodies.
#[allow(clippy::type_complexity)]
#[derive(SystemParam)]
pub(crate) struct Poses<'w, 's> {
    sims: Query<
        'w,
        's,
        (&'static Transform, Option<&'static Visual>),
        (Without<VisualOf>, Without<FxVisual>),
    >,
    proxies: Query<'w, 's, &'static Transform, (With<VisualOf>, Without<FxVisual>)>,
}

impl Poses<'_, '_> {
    /// Where `entity` is drawn this frame.
    pub fn of(&self, entity: Entity) -> Option<Transform> {
        let (own, visual) = self.sims.get(entity).ok()?;
        Some(
            visual
                .and_then(|v| self.proxies.get(v.get()).ok())
                .copied()
                .unwrap_or(*own),
        )
    }
}

/// Positions of the local cameras (emission thins out far from all of them).
#[derive(SystemParam)]
pub(crate) struct Eyes<'w, 's> {
    cams: Query<'w, 's, &'static GlobalTransform, With<GameCamera>>,
}

impl Eyes<'_, '_> {
    /// Emission multiplier for an effect at `pos`: 1 near a camera, [`FAR_EMISSION`] far away.
    pub fn lod(&self, pos: Vec3) -> f32 {
        let d = self
            .cams
            .iter()
            .map(|c| c.translation().distance(pos))
            .fold(f32::INFINITY, f32::min);
        if !d.is_finite() {
            return 1.0;
        }
        let t = ((d - NEAR_U) / (FAR_U - NEAR_U)).clamp(0.0, 1.0);
        1.0 + (FAR_EMISSION - 1.0) * t
    }
}

pub(crate) fn build(app: &mut App) {
    app.init_resource::<CarShapes>()
        .add_observer(decorate_racer)
        .add_systems(OnEnter(AppState::Loading), load_shapes);
}

/// Reads the wheel layout of every car in the race (missing files fall back to a default).
fn load_shapes(
    config: Option<Res<RaceConfig>>,
    paths: Option<Res<UserPaths>>,
    mut shapes: ResMut<CarShapes>,
) {
    let (Some(config), Some(paths)) = (config, paths) else {
        return;
    };
    for setup in &config.racers {
        if !shapes.0.contains_key(&setup.car) {
            let shape = load_shape(&paths.assets_dir, &setup.car);
            shapes.0.insert(setup.car.clone(), shape);
        }
    }
}

fn load_shape(assets: &Path, car: &CarId) -> CarShape {
    let path = assets.join(format!("models/cars/{}.wheels.ron", car.0));
    match std::fs::read_to_string(&path)
        .map_err(|e| e.to_string())
        .and_then(|t| CarShape::parse(&t))
    {
        Ok(shape) => shape,
        Err(e) => {
            warn!(
                "rc_vfx: {}: {e}; using a default wheel layout",
                path.display()
            );
            CarShape::default()
        }
    }
}

fn decorate_racer(
    add: On<Add<Racer>>,
    racers: Query<&Racer>,
    shapes: Res<CarShapes>,
    mut commands: Commands,
) {
    let shape = racers
        .get(add.entity)
        .map(|r| shapes.get(&r.car))
        .unwrap_or_default();
    commands.entity(add.entity).insert(CarFx::new(shape));
}

/// Where wheel `i` of a car drawn at `pose` touches the ground this frame, and the ground normal:
/// the drawn contact patch moved onto the ground plane rc_vehicle's wheel ray hit
/// (`WheelState::contact` / `normal`), so marks and debris sit exactly on ramps and curbs while
/// following the interpolated car. Airborne wheels (or no ground data) keep the drawn patch.
pub(crate) fn ground_point(
    pose: &Transform,
    shape: &CarShape,
    i: usize,
    w: &WheelState,
) -> (Vec3, Vec3) {
    let patch = pose.transform_point(shape.contact(i, w.offset));
    if !w.grounded || w.normal == Vec3::ZERO {
        return (patch, pose.rotation * Vec3::Y);
    }
    (
        patch - w.normal * (patch - w.contact).dot(w.normal),
        w.normal,
    )
}

/// The ground plane under a car (a point and the unit normal): the mean contact of its grounded
/// wheels; `None` when airborne or without ground data.
pub(crate) fn ground_plane(t: &VehicleTelemetry) -> Option<(Vec3, Vec3)> {
    let (n, point, normal) = t
        .wheels
        .iter()
        .filter(|w| w.grounded && w.normal != Vec3::ZERO)
        .fold((0.0, Vec3::ZERO, Vec3::ZERO), |(n, p, m), w| {
            (n + 1.0, p + w.contact, m + w.normal)
        });
    (n > 0.0).then(|| (point / n, normal.normalize_or(Vec3::Y)))
}

/// Car velocity in world space from telemetry (heading + slip angle), u/s.
pub(crate) fn car_velocity(pose: &Transform, t: &VehicleTelemetry) -> Vec3 {
    let fwd = pose.rotation * Vec3::NEG_Z;
    let right = pose.rotation * Vec3::X;
    fwd * t.forward_speed + right * t.speed * t.slip_angle.sin()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wheels_file_parses() {
        let text = r#"(
            car: "x",
            wheels: [
                (name: "wheel_fl", center: (-0.5, 0.3, -0.6), radius: 0.3, width: 0.2),
                (name: "wheel_fr", center: (0.5, 0.3, -0.6), radius: 0.3, width: 0.2),
                (name: "wheel_rl", center: (-0.5, 0.32, 0.7), radius: 0.32, width: 0.25),
                (name: "wheel_rr", center: (0.5, 0.32, 0.7), radius: 0.32, width: 0.25),
            ],
        )"#;
        let shape = CarShape::parse(text).expect("parses");
        assert_eq!(shape.wheels[3], Vec3::new(0.5, 0.32, 0.7));
        assert!(
            (shape.contact(0, 0.0).y).abs() < 1e-6,
            "rest contact on the ground"
        );
        assert!(
            shape.exhaust().z > 0.7,
            "the nozzle is behind the rear axle"
        );
        assert!(CarShape::parse("(wheels: [])").is_err());
    }

    #[test]
    fn every_car_model_has_a_wheel_file() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/models/cars");
        let mut n = 0;
        for entry in std::fs::read_dir(&dir).expect("cars dir") {
            let path = entry.expect("entry").path();
            if path.to_string_lossy().ends_with(".wheels.ron") {
                let text = std::fs::read_to_string(&path).expect("readable");
                CarShape::parse(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
                n += 1;
            }
        }
        assert!(n >= 8, "found {n} wheel files");
    }

    #[test]
    fn contacts_sit_on_the_ground_the_wheel_ray_hit() {
        let shape = CarShape::default();
        // The drawn car is level but its front-left wheel rides up a 30° curb face whose plane
        // passes through (−0.55, 0.1, −0.65).
        let n = Vec3::new(0.5, 0.75f32.sqrt(), 0.0);
        let on_curb = WheelState {
            grounded: true,
            contact: Vec3::new(-0.55, 0.1, -0.65),
            normal: n,
            ..default()
        };
        let pose = Transform::from_xyz(0.0, 0.0, 0.0);
        let (at, up) = ground_point(&pose, &shape, 0, &on_curb);
        assert_eq!(up, n);
        assert!(
            (at - on_curb.contact).dot(n).abs() < 1e-5,
            "on the curb face: {at}"
        );
        let patch = pose.transform_point(shape.contact(0, 0.0));
        assert!(
            (at - patch).cross(n).length() < 1e-5,
            "moved along the normal only"
        );

        let airborne = WheelState::default();
        assert_eq!(ground_point(&pose, &shape, 0, &airborne), (patch, Vec3::Y));
        let t = VehicleTelemetry {
            wheels: [on_curb, airborne, airborne, airborne],
            ..default()
        };
        assert_eq!(ground_plane(&t), Some((on_curb.contact, n)));
        assert_eq!(
            ground_plane(&VehicleTelemetry::default()),
            None,
            "in the air"
        );
    }

    #[test]
    fn velocity_follows_heading_and_slip() {
        let pose = Transform::default();
        let t = VehicleTelemetry {
            speed: 10.0,
            forward_speed: 10.0,
            ..default()
        };
        assert!((car_velocity(&pose, &t) - Vec3::new(0.0, 0.0, -10.0)).length() < 1e-4);
        let drift = VehicleTelemetry {
            speed: 10.0,
            forward_speed: 10.0 * 0.5f32.cos(),
            slip_angle: 0.5,
            ..default()
        };
        assert!(car_velocity(&pose, &drift).x > 4.0, "slip + = moving right");
    }
}
