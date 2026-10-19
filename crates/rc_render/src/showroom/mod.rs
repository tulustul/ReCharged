//! The menu showroom (GDD §11, §14): a night-time garage corner at toy scale around
//! `SHOWROOM_FOCUS` — the `MenuBackdrop` car on a slowly turning turntable with a cyan LED ring,
//! a warm key spot, a cool rim light and giant workshop props ([`diorama`]). rc_camera's
//! `MenuCamera` drifts around it. The car itself is a [`CarModel`] (glb, paint, idle lamps).
//!
//! `ShowroomReady` is present exactly while the backdrop car is instanced and painted (the 3D
//! showroom renders): a car swap keeps the previous car until the new one is ready, so it does not
//! flicker. `BackdropFocus::Room` hides the car (screens that cover the turntable: Garage grid,
//! Credits roll, Cup-end podium); the room stays.

mod diorama;

use bevy::prelude::*;
use rc_core::prelude::*;

use crate::cars::{CarModel, CarModelReady};

/// Turntable speed (rad/s): one turn in ≈ 31 s.
const TURN_RATE: f32 = 0.2;
/// Frames between showing a car and signalling `ShowroomReady` (the car is drawn by then).
const READY_AFTER_FRAMES: u8 = 2;

pub(crate) fn build(app: &mut App) {
    app.add_systems(OnEnter(AppState::Menu), diorama::spawn)
        .add_systems(
            Update,
            (turn, choose_car, show_car, signal_ready)
                .chain()
                .in_set(VisualSet)
                .run_if(in_state(AppState::Menu)),
        )
        .add_observer(car_ready)
        .add_systems(OnExit(AppState::Menu), |mut commands: Commands| {
            commands.remove_resource::<ShowroomReady>();
            commands.remove_resource::<ReadyIn>();
        });
}

/// The rotating part of the turntable (cars are its children).
#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct Turntable;

/// Frames left before `ShowroomReady` is inserted.
#[derive(Resource, Debug, Clone, Copy)]
struct ReadyIn(u8);

/// A car on the turntable, in spawn order (the newest is the wanted one).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
struct ShowroomCar {
    serial: u32,
}

fn turn(time: Res<Time>, mut tables: Query<&mut Transform, With<Turntable>>) {
    for mut t in &mut tables {
        t.rotate_y(TURN_RATE * time.delta_secs());
    }
}

/// The car to show: the backdrop's, else (React has not chosen yet) the first catalog car.
fn wanted_car(backdrop: Option<&MenuBackdrop>, catalog: &Catalog) -> Option<CarModel> {
    let chosen = backdrop.and_then(|b| {
        catalog
            .cars
            .iter()
            .find(|c| c.id == b.car)
            .map(|c| (c, b.paint))
    });
    let (car, paint) = chosen.or_else(|| catalog.cars.first().map(|c| (c, PaintId(0))))?;
    let paint = if car.paints.iter().any(|p| p.id == paint) {
        paint
    } else {
        PaintId(0)
    };
    Some(CarModel {
        car: car.id.clone(),
        paint,
    })
}

/// Spawns the wanted car (hidden until ready) when it differs from the newest one.
fn choose_car(
    mut commands: Commands,
    backdrop: Option<Res<MenuBackdrop>>,
    catalog: Option<Res<Catalog>>,
    table: Option<Single<Entity, With<Turntable>>>,
    cars: Query<(Entity, &ShowroomCar, &CarModel, Has<CarModelReady>)>,
    mut serial: Local<u32>,
) {
    let (Some(catalog), Some(table)) = (catalog, table) else {
        return;
    };
    let Some(wanted) = wanted_car(backdrop.as_deref(), &catalog) else {
        return;
    };
    let newest = cars.iter().max_by_key(|(_, s, _, _)| s.serial);
    if newest.is_some_and(|(_, _, model, _)| *model == wanted) {
        return;
    }
    // A car still loading that is no longer wanted is dropped right away.
    for (entity, _, _, ready) in &cars {
        if !ready {
            commands.entity(entity).despawn();
        }
    }
    *serial += 1;
    let car = commands
        .spawn((
            Name::new(format!("Showroom car {}", wanted.car)),
            ShowroomCar { serial: *serial },
            wanted,
            Visibility::Hidden,
        ))
        .id();
    commands.entity(*table).add_child(car);
}

/// How a ready car shows under the backdrop's framing.
fn car_visibility(backdrop: Option<&MenuBackdrop>) -> Visibility {
    match backdrop.map(|b| b.focus) {
        Some(BackdropFocus::Room) => Visibility::Hidden,
        _ => Visibility::Inherited,
    }
}

/// A car is instanced and painted: show it, retire the older ones, signal the UI.
fn car_ready(
    ready: On<Add<CarModelReady>>,
    backdrop: Option<Res<MenuBackdrop>>,
    mut commands: Commands,
    mut cars: Query<(Entity, &ShowroomCar, &mut Visibility)>,
) {
    let Ok((_, shown, mut visibility)) = cars.get_mut(ready.entity) else {
        return;
    };
    let serial = shown.serial;
    *visibility = car_visibility(backdrop.as_deref());
    for (entity, other, _) in &cars {
        if other.serial < serial {
            commands.entity(entity).despawn();
        }
    }
    commands.insert_resource(ReadyIn(READY_AFTER_FRAMES));
}

/// Hides or shows the ready car when the backdrop's framing changes.
fn show_car(
    backdrop: Option<Res<MenuBackdrop>>,
    mut cars: Query<&mut Visibility, (With<ShowroomCar>, With<CarModelReady>)>,
) {
    let Some(backdrop) = backdrop.filter(|b| b.is_changed()) else {
        return;
    };
    let wanted = car_visibility(Some(&backdrop));
    for mut visibility in &mut cars {
        visibility.set_if_neq(wanted);
    }
}

/// Counts down after a car became visible; once it has been drawn, the UI may rely on it.
fn signal_ready(mut commands: Commands, pending: Option<ResMut<ReadyIn>>) {
    let Some(mut pending) = pending else {
        return;
    };
    if pending.0 == 0 {
        commands.remove_resource::<ReadyIn>();
        commands.insert_resource(ShowroomReady);
    } else {
        pending.0 -= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog() -> Catalog {
        let car = |id: &str, paints: u8| CarInfo {
            id: CarId::new(id),
            paints: (0..paints)
                .map(|i| PaintInfo {
                    id: PaintId(i),
                    ..default()
                })
                .collect(),
            ..default()
        };
        Catalog {
            cars: vec![car("sparky", 4), car("zip", 2)],
            ..default()
        }
    }

    #[test]
    fn the_backdrop_car_is_shown_else_the_first_car() {
        let catalog = catalog();
        let backdrop = MenuBackdrop {
            car: CarId::new("zip"),
            paint: PaintId(1),
            ..default()
        };
        let model = wanted_car(Some(&backdrop), &catalog).expect("a car");
        assert_eq!((model.car.as_str(), model.paint), ("zip", PaintId(1)));
        let model = wanted_car(Some(&MenuBackdrop::default()), &catalog).expect("fallback");
        assert_eq!((model.car.as_str(), model.paint), ("sparky", PaintId(0)));
        assert!(wanted_car(None, &Catalog::default()).is_none());
    }

    #[test]
    fn the_room_framing_hides_the_car() {
        let room = MenuBackdrop {
            focus: BackdropFocus::Room,
            ..default()
        };
        assert_eq!(car_visibility(Some(&room)), Visibility::Hidden);
        assert_eq!(
            car_visibility(Some(&MenuBackdrop::default())),
            Visibility::Inherited
        );
        assert_eq!(car_visibility(None), Visibility::Inherited);
    }

    #[test]
    fn unknown_paints_fall_back_to_the_first() {
        let backdrop = MenuBackdrop {
            car: CarId::new("zip"),
            paint: PaintId(7),
            ..default()
        };
        let model = wanted_car(Some(&backdrop), &catalog()).expect("a car");
        assert_eq!(model.paint, PaintId(0));
    }
}
