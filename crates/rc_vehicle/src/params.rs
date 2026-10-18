//! `VehicleParams` (`assets/data/vehicles/<car>.ron`): loading, validation and the
//! [`VehicleLibrary`] of every car (loaded on `OnEnter(AppState::Loading)`, hot-reloaded in debug
//! builds).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use bevy::prelude::*;
use rc_core::prelude::*;
use serde::Deserialize;

/// Loading-gate key of this crate.
pub const LOADING_KEY: &str = "vehicles";

/// Directory of the vehicle files relative to the assets root.
pub const VEHICLES_DIR: &str = "data/vehicles";

/// Tuning of one car. Units: u, s, u/s, u/s², degrees (angles), Hz; masses are game units
/// (only ratios matter: impulses scale by `(REFERENCE_CAR_MASS / mass)^0.5`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VehicleParams {
    /// Car id (= file stem, catalog id).
    pub id: CarId,
    /// Driven wheels (matches the catalog).
    pub drive: Drivetrain,
    /// Top speed on flat asphalt in u/s (GDD §5.1, by Speed stars).
    pub top_speed: f32,
    /// Seconds from standstill to 99 % of `top_speed` at full throttle (by Accel stars).
    pub accel_time: f32,
    /// Top reverse speed in u/s.
    pub reverse_speed: f32,
    /// Full-brake deceleration in u/s² (friction-limited by the tyres).
    pub brake_decel: f32,
    /// Body envelope (car-dimensions.md).
    pub body: BodyDims,
    /// Distance between the axles in u.
    pub wheelbase: f32,
    /// Front axle.
    pub front: Axle,
    /// Rear axle.
    pub rear: Axle,
    /// Mass (Weight stars).
    pub mass: f32,
    /// Centre of mass above the ground at rest, in u.
    pub com_height: f32,
    /// Suspension of every wheel.
    pub suspension: Suspension,
    /// Tyre friction.
    pub tyres: Tyres,
    /// Steering.
    pub steering: Steering,
    /// Air-control strength 0–1 (lighter cars more).
    pub air_control: f32,
    /// Fraction of horizontal speed lost on a full-strength landing (heavier cars more).
    pub landing_loss: f32,
    /// Downforce at top speed as a fraction of the car's weight.
    pub downforce: f32,
}

/// Body bounding box (excluding wheels and antenna).
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BodyDims {
    /// Length along Z in u.
    pub length: f32,
    /// Width along X in u.
    pub width: f32,
    /// Top of the body above the ground at rest in u.
    pub height: f32,
    /// Lowest chassis point above the ground at rest in u.
    pub clearance: f32,
}

/// One axle's wheels.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Axle {
    /// Distance between the wheel centres in u.
    pub track: f32,
    /// Wheel radius in u.
    pub radius: f32,
    /// Wheel width in u.
    pub width: f32,
}

/// Spring-damper per wheel.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Suspension {
    /// Compression travel above the rest position in u.
    pub bump: f32,
    /// Extension travel below the rest position in u.
    pub droop: f32,
    /// Natural frequency of the sprung mass in Hz (higher = stiffer).
    pub frequency: f32,
    /// Damping ratio (1 = critical; < 1 bounces).
    pub damping: f32,
}

/// Tyre friction coefficients on asphalt (surface, weather and effects multiply them).
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tyres {
    /// Front lateral grip (Handling stars).
    pub grip: f32,
    /// Rear lateral grip (a bit above `grip` = stable understeer at the limit).
    pub rear_grip: f32,
    /// Longitudinal grip (drive and brakes).
    pub traction: f32,
    /// Rear lateral grip multiplier while the handbrake (drift) is held.
    pub drift_grip: f32,
}

/// Steering geometry.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Steering {
    /// Wheel lock at low speed in degrees (less at speed: speed-sensitive steering).
    pub lock_deg: f32,
    /// 0–1: front wheels follow the rear axle's slide direction (catches drifts).
    pub counter_steer: f32,
    /// Body slip angle a handbrake drift settles at with neutral steering, degrees (steering
    /// into the turn widens it, counter-steering narrows it).
    pub drift_angle_deg: f32,
    /// 0–1: strength of the assist that catches slides without the handbrake.
    pub stability: f32,
}

impl VehicleParams {
    /// Problems with the values, each naming the field (empty = valid).
    pub fn problems(&self) -> Vec<String> {
        let mut out = Vec::new();
        let mut positive = |name: &str, v: f32| {
            if !(v.is_finite() && v > 0.0) {
                out.push(format!("`{name}` must be > 0 (is {v})"));
            }
        };
        positive("top_speed", self.top_speed);
        positive("accel_time", self.accel_time);
        positive("reverse_speed", self.reverse_speed);
        positive("brake_decel", self.brake_decel);
        positive("body.length", self.body.length);
        positive("body.width", self.body.width);
        positive("body.height", self.body.height);
        positive("wheelbase", self.wheelbase);
        for (axle, a) in [("front", self.front), ("rear", self.rear)] {
            positive(&format!("{axle}.track"), a.track);
            positive(&format!("{axle}.radius"), a.radius);
            positive(&format!("{axle}.width"), a.width);
        }
        positive("mass", self.mass);
        positive("com_height", self.com_height);
        positive("suspension.bump", self.suspension.bump);
        positive("suspension.droop", self.suspension.droop);
        positive("suspension.frequency", self.suspension.frequency);
        positive("suspension.damping", self.suspension.damping);
        positive("tyres.grip", self.tyres.grip);
        positive("tyres.rear_grip", self.tyres.rear_grip);
        positive("tyres.traction", self.tyres.traction);
        positive("tyres.drift_grip", self.tyres.drift_grip);
        positive("steering.lock_deg", self.steering.lock_deg);
        let mut unit = |name: &str, v: f32| {
            if !(0.0..=1.0).contains(&v) {
                out.push(format!("`{name}` must be within 0–1 (is {v})"));
            }
        };
        unit("steering.counter_steer", self.steering.counter_steer);
        unit("steering.stability", self.steering.stability);
        unit("air_control", self.air_control);
        unit("landing_loss", self.landing_loss);
        unit("downforce", self.downforce);
        unit("tyres.drift_grip", self.tyres.drift_grip);
        if !(5.0..=60.0).contains(&self.steering.drift_angle_deg) {
            out.push("`steering.drift_angle_deg` must be within 5–60".into());
        }
        if self.body.clearance < 0.0 || self.body.clearance >= self.body.height {
            out.push("`body.clearance` must be within 0..body.height".into());
        }
        if self.wheelbase >= self.body.length + 0.5 {
            out.push("`wheelbase` does not fit the body length".into());
        }
        if self.collider_bottom() + MIN_COLLIDER_HEIGHT_U > self.body.height {
            out.push(format!(
                "`suspension.bump` leaves no room for the chassis collider below `body.height` \
                 (needs bump + {COLLIDER_BUMP_MARGIN_U} + {MIN_COLLIDER_HEIGHT_U} ≤ height)"
            ));
        }
        out
    }

    /// Bottom of the chassis collider above the ground at rest: the body's clearance, raised
    /// when the suspension travel needs more room so the chassis only scrapes when bottoming out.
    pub fn collider_bottom(&self) -> f32 {
        self.body
            .clearance
            .max(self.suspension.bump + COLLIDER_BUMP_MARGIN_U)
    }

    /// Wheel centres at rest in car space (Bevy axes, origin = ground-contact centre), with
    /// radius and width, in the order FL, FR, RL, RR (car-dimensions.md).
    pub fn wheel_layout(&self) -> [(Vec3, f32, f32); 4] {
        let half = self.wheelbase / 2.0;
        let w = |a: Axle, x: f32, z: f32| {
            (Vec3::new(x * a.track / 2.0, a.radius, z), a.radius, a.width)
        };
        [
            w(self.front, -1.0, -half),
            w(self.front, 1.0, -half),
            w(self.rear, -1.0, half),
            w(self.rear, 1.0, half),
        ]
    }
}

/// Room kept between the full-bump position and the chassis collider, in u.
const COLLIDER_BUMP_MARGIN_U: f32 = 0.06;
/// Smallest chassis collider height, in u.
const MIN_COLLIDER_HEIGHT_U: f32 = 0.15;

/// Every car's params with the file's modification time (for the dev hot reload).
#[derive(Resource, Debug, Clone, Default)]
pub struct VehicleLibrary {
    /// Directory the files were read from.
    pub dir: PathBuf,
    /// Params by car id.
    pub cars: BTreeMap<CarId, VehicleParams>,
    mtimes: BTreeMap<CarId, Option<SystemTime>>,
}

impl VehicleLibrary {
    /// Reads and validates every `*.ron` in `dir` (sorted by name).
    pub fn load(dir: &Path) -> Result<Self, String> {
        let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let mut files: Vec<PathBuf> = entries
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "ron"))
            .collect();
        files.sort();
        let mut lib = Self {
            dir: dir.to_owned(),
            ..default()
        };
        for path in files {
            let params = load_file(&path)?;
            lib.mtimes.insert(params.id.clone(), mtime(&path));
            lib.cars.insert(params.id.clone(), params);
        }
        if lib.cars.is_empty() {
            return Err(format!("{}: no vehicle files", dir.display()));
        }
        Ok(lib)
    }

    /// Params of `car`.
    pub fn get(&self, car: &CarId) -> Option<&VehicleParams> {
        self.cars.get(car)
    }

    /// Re-reads files whose modification time changed; returns the reloaded ids. A broken file
    /// keeps the previous params (the error is returned for logging).
    pub fn reload_changed(&mut self) -> (Vec<CarId>, Vec<String>) {
        let (mut changed, mut errors) = (Vec::new(), Vec::new());
        for (id, old) in &mut self.mtimes {
            let path = self.dir.join(format!("{id}.ron"));
            let now = mtime(&path);
            if now == *old {
                continue;
            }
            *old = now;
            match load_file(&path) {
                Ok(params) if params.id == *id => {
                    self.cars.insert(id.clone(), params);
                    changed.push(id.clone());
                }
                Ok(params) => {
                    errors.push(format!("{}: id \"{}\" changed", path.display(), params.id))
                }
                Err(e) => errors.push(e),
            }
        }
        (changed, errors)
    }
}

fn mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// Reads, parses and validates one vehicle file; errors name the file and field.
pub(crate) fn load_file(path: &Path) -> Result<VehicleParams, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let params: VehicleParams =
        ron::from_str(&text).map_err(|e| format!("{}:{e}", path.display()))?;
    let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned());
    let mut problems = params.problems();
    if stem.as_deref() != Some(params.id.as_str()) {
        problems.push(format!("`id` \"{}\" must match the file name", params.id));
    }
    if problems.is_empty() {
        Ok(params)
    } else {
        Err(format!("{}: {}", path.display(), problems.join("; ")))
    }
}

/// `OnEnter(AppState::Loading)`: (re)load the library and check every racer's car has params.
pub(crate) fn load_library(
    paths: Option<Res<UserPaths>>,
    config: Option<Res<RaceConfig>>,
    mut commands: Commands,
    mut done: MessageWriter<LoadingDone>,
    mut failed: MessageWriter<LoadingFailed>,
) {
    let result = paths
        .ok_or_else(|| "no UserPaths resource".to_owned())
        .and_then(|p| VehicleLibrary::load(&p.assets_dir.join(VEHICLES_DIR)))
        .and_then(|lib| {
            let missing: Vec<&str> = config
                .iter()
                .flat_map(|c| c.racers.iter())
                .map(|r| r.car.as_str())
                .filter(|car| !lib.cars.contains_key(&CarId::new(*car)))
                .collect();
            match missing.is_empty() {
                true => Ok(lib),
                false => Err(format!("no {VEHICLES_DIR}/<car>.ron for {missing:?}")),
            }
        });
    match result {
        Ok(lib) => {
            info!("vehicles: {} cars loaded", lib.cars.len());
            commands.insert_resource(lib);
            done.write(LoadingDone(LOADING_KEY));
        }
        Err(error) => {
            error!("vehicles: {error}");
            failed.write(LoadingFailed {
                key: LOADING_KEY,
                error,
            });
        }
    }
}

/// `OnEnter(AppState::Unloading)`: drop the session's library.
pub(crate) fn unload_library(mut commands: Commands) {
    commands.remove_resource::<VehicleLibrary>();
}

/// Seconds between modification-time polls of the dev hot reload.
#[cfg(debug_assertions)]
const HOT_RELOAD_POLL_SECS: f32 = 1.0;

/// Debug builds: poll the files' modification times (≤ 1 Hz) and reload changed ones.
#[cfg(debug_assertions)]
pub(crate) fn hot_reload(
    time: Res<Time<Real>>,
    mut next_poll: Local<f32>,
    mut library: ResMut<VehicleLibrary>,
) {
    if time.elapsed_secs() < *next_poll {
        return;
    }
    *next_poll = time.elapsed_secs() + HOT_RELOAD_POLL_SECS;
    // Only a real change marks the resource changed (the body rebuild keys off that).
    let (changed, errors) = library.bypass_change_detection().reload_changed();
    for e in errors {
        warn!("vehicles hot reload: {e}");
    }
    if !changed.is_empty() {
        info!("vehicles hot reload: {changed:?}");
        library.set_changed();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> VehicleParams {
        ron::from_str(include_str!("../../../assets/data/vehicles/sparky.ron")).expect("parses")
    }

    #[test]
    fn sample_file_is_valid() {
        let p = sample();
        assert!(p.problems().is_empty(), "{:?}", p.problems());
    }

    #[test]
    fn problems_name_the_field() {
        let mut p = sample();
        p.top_speed = 0.0;
        p.steering.counter_steer = 2.0;
        let problems = p.problems().join("\n");
        assert!(problems.contains("`top_speed`"), "{problems}");
        assert!(problems.contains("`steering.counter_steer`"), "{problems}");
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let text =
            include_str!("../../../assets/data/vehicles/sparky.ron").replacen("mass:", "mas:", 1);
        let err = ron::from_str::<VehicleParams>(&text).expect_err("typo detected");
        assert!(err.to_string().contains("mas"), "{err}");
    }

    #[test]
    fn wheel_layout_follows_the_axles() {
        let p = sample();
        let [fl, fr, rl, rr] = p.wheel_layout();
        assert_eq!(
            fl.0,
            Vec3::new(-p.front.track / 2.0, p.front.radius, -p.wheelbase / 2.0)
        );
        assert_eq!(fr.0.x, -fl.0.x);
        assert_eq!(rl.0.z, p.wheelbase / 2.0);
        assert_eq!(rr.1, p.rear.radius);
    }

    #[test]
    fn collider_bottom_leaves_room_for_bump_travel() {
        let mut p = sample();
        p.body.clearance = 0.05;
        p.suspension.bump = 0.1;
        let expected = 0.1 + COLLIDER_BUMP_MARGIN_U;
        assert!((p.collider_bottom() - expected).abs() < 1e-6);
        p.body.clearance = 0.4;
        assert_eq!(
            p.collider_bottom(),
            0.4,
            "a high body keeps its own clearance"
        );
    }
}
