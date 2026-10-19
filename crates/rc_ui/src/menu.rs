//! `catalog.*` / `menu.*` channels: content metadata for the menus and the showroom backdrop.

use std::path::Path;

use bevy::asset::RenderAssetUsages;
use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
use bevy::prelude::*;
use bevy_react::prelude::*;
use rc_core::prelude::*;

use crate::content::{
    UiCarClass, UiDifficulty, UiDrive, UiPaintSet, UiTimeOfDay, UiUnlock, UiUnlockRule,
    UiWeatherChoice, UiWeatherKind,
};

mirror_enum!(
    /// [`BackdropFocus`]: showroom camera framing.
    UiBackdropFocus = BackdropFocus { Wide, Car, Room }
);

/// Star ratings 1–5.
#[derive(serde::Serialize, ts_rs::TS, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct UiStars {
    /// Top speed.
    pub speed: u8,
    /// Acceleration.
    pub accel: u8,
    /// Handling.
    pub handling: u8,
    /// Weight.
    pub weight: u8,
}

/// A paint of a car.
#[derive(serde::Serialize, ts_rs::TS, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct UiPaint {
    /// Paint id within the car.
    pub id: u8,
    /// Display name.
    pub name: String,
    /// Base colour, sRGB 0–1.
    pub base: [f32; 3],
    /// Accent colour, sRGB 0–1.
    pub accent: [f32; 3],
    /// Set it belongs to (locked with the set).
    pub set: UiPaintSet,
}

/// A car of the roster.
#[derive(serde::Serialize, ts_rs::TS, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct UiCar {
    /// Id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Class.
    pub class: UiCarClass,
    /// Star ratings.
    pub stars: UiStars,
    /// Driven wheels.
    pub drive: UiDrive,
    /// Thumbnail asset path (transparent PNG), if it exists.
    pub thumbnail: Option<String>,
    /// Share of the thumbnail's height under the car (transparent rows below the wheels; 0
    /// without a thumbnail): Cup end stands the car on its podium block by it.
    pub thumbnail_foot: f32,
    /// Paints.
    pub paints: Vec<UiPaint>,
    /// Unlock rule.
    pub unlock: UiUnlockRule,
}

/// A track (environment), listed in menus.
#[derive(serde::Serialize, ts_rs::TS, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct UiTrack {
    /// Id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Environment id.
    pub environment: String,
    /// Indoor (weather outside only).
    pub indoor: bool,
    /// Difficulty stars 1–3.
    pub difficulty: u8,
    /// Preview image asset path, if it exists.
    pub preview: Option<String>,
    /// Default laps.
    pub default_laps: u8,
    /// Allowed times of day.
    pub times: Vec<UiTimeOfDay>,
    /// Allowed weather presets.
    pub weather: Vec<UiWeatherKind>,
    /// Unlock rule of the forward layout.
    pub unlock: UiUnlockRule,
}

/// One race of a cup.
#[derive(serde::Serialize, ts_rs::TS, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct UiCupRaceInfo {
    /// Index of the track in [`UiCatalog::tracks`].
    pub track: usize,
    /// Reverse layout.
    pub reverse: bool,
    /// Mirrored.
    pub mirror: bool,
    /// Weather.
    pub weather: UiWeatherChoice,
    /// Time of day.
    pub time: UiTimeOfDay,
    /// Laps (resolved).
    pub laps: u8,
}

/// A championship cup.
#[derive(serde::Serialize, ts_rs::TS, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct UiCup {
    /// Id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// AI difficulty.
    pub difficulty: UiDifficulty,
    /// Races in order.
    pub races: Vec<UiCupRaceInfo>,
    /// Unlock rule.
    pub unlock: UiUnlockRule,
}

/// Response of `catalog.get`: everything the menus list (dev tracks excluded).
#[derive(serde::Serialize, ts_rs::TS, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct UiCatalog {
    /// Cars in roster order.
    pub cars: Vec<UiCar>,
    /// Tracks in menu order.
    pub tracks: Vec<UiTrack>,
    /// Cups in menu order.
    pub cups: Vec<UiCup>,
    /// Global unlockables.
    pub unlocks: Vec<UiUnlock>,
}

/// Asset path of a car's menu thumbnail (rendered by the art pipeline).
pub fn car_thumbnail(car: &CarId) -> String {
    format!("ui/cars/{car}.png")
}

/// `path` if that asset exists under `assets_dir` (React must not request missing images).
fn existing(assets_dir: &Path, path: &str) -> Option<String> {
    (!path.is_empty() && assets_dir.join(path).is_file()).then(|| path.to_owned())
}

/// Thumbnail pixels at least this opaque are the car (anti-aliased edges are fainter).
const OPAQUE_ALPHA: f32 = 0.5;

/// Share of a PNG's height below its lowest opaque row: the transparent margin under a car in its
/// thumbnail, which differs per car (QA B37: one constant left some cars floating above their
/// podium block and others sunk into it). 0 when the file cannot be read.
fn thumbnail_foot(file: &Path) -> f32 {
    let Some(image) = std::fs::read(file).ok().and_then(|bytes| {
        Image::from_buffer(
            &bytes,
            ImageType::Extension("png"),
            CompressedImageFormats::NONE,
            true,
            ImageSampler::Default,
            RenderAssetUsages::MAIN_WORLD,
        )
        .ok()
    }) else {
        return 0.0;
    };
    let (width, height) = (image.width(), image.height());
    let opaque = |y: u32| {
        (0..width).any(|x| {
            image
                .get_color_at(x, y)
                .is_ok_and(|c| c.alpha() >= OPAQUE_ALPHA)
        })
    };
    (0..height)
        .rev()
        .find(|&y| opaque(y))
        .map_or(0.0, |lowest| (height - 1 - lowest) as f32 / height as f32)
}

impl UiCatalog {
    /// The menu view of `catalog`; image paths are checked against `assets_dir`.
    pub fn new(catalog: &Catalog, assets_dir: &Path) -> Self {
        let tracks: Vec<&TrackInfo> = catalog.tracks.iter().filter(|t| !t.hidden).collect();
        let track_index = |id: &TrackId| tracks.iter().position(|t| t.id == *id).unwrap_or(0);
        Self {
            cars: catalog
                .cars
                .iter()
                .map(|car| UiCar {
                    id: car.id.0.clone(),
                    name: car.name.clone(),
                    class: car.class.into(),
                    stars: UiStars {
                        speed: car.stars.speed,
                        accel: car.stars.accel,
                        handling: car.stars.handling,
                        weight: car.stars.weight,
                    },
                    drive: car.drive.into(),
                    thumbnail: existing(assets_dir, &car_thumbnail(&car.id)),
                    thumbnail_foot: thumbnail_foot(&assets_dir.join(car_thumbnail(&car.id))),
                    paints: car
                        .paints
                        .iter()
                        .map(|p| UiPaint {
                            id: p.id.0,
                            name: p.name.clone(),
                            base: p.base,
                            accent: p.accent,
                            set: p.set.into(),
                        })
                        .collect(),
                    unlock: (&car.unlock).into(),
                })
                .collect(),
            tracks: tracks
                .iter()
                .map(|t| UiTrack {
                    id: t.id.0.clone(),
                    name: t.name.clone(),
                    environment: t.environment.clone(),
                    indoor: t.indoor,
                    difficulty: t.difficulty,
                    preview: existing(assets_dir, &t.preview),
                    default_laps: t.default_laps,
                    times: t.allowed_times.iter().map(|&x| x.into()).collect(),
                    weather: t.allowed_weather.iter().map(|&x| x.into()).collect(),
                    unlock: (&t.unlock).into(),
                })
                .collect(),
            cups: catalog
                .cups
                .iter()
                .map(|cup| UiCup {
                    id: cup.id.0.clone(),
                    name: cup.name.clone(),
                    difficulty: cup.difficulty.into(),
                    races: cup
                        .races
                        .iter()
                        .map(|race| {
                            let index = track_index(&race.variant.track);
                            UiCupRaceInfo {
                                track: index,
                                reverse: race.variant.reverse,
                                mirror: race.variant.mirror,
                                weather: race.weather.weather.into(),
                                time: race.weather.time.into(),
                                laps: race.laps.unwrap_or_else(|| {
                                    tracks.get(index).map_or(3, |t| t.default_laps)
                                }),
                            }
                        })
                        .collect(),
                    unlock: (&cup.unlock).into(),
                })
                .collect(),
            unlocks: catalog
                .unlocks
                .iter()
                .map(|(item, rule)| UiUnlock {
                    item: (*item).into(),
                    rule: rule.into(),
                })
                .collect(),
        }
    }
}

/// React → Bevy request `catalog.get`: the [`UiCatalog`].
#[react_request(name = "catalog.get", response = UiCatalog)]
pub struct GetCatalog;

/// React → Bevy message `menu.backdrop`: what the 3D showroom behind the menus shows.
#[react_message(name = "menu.backdrop")]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct SetBackdrop {
    /// Car on the turntable.
    pub car: String,
    /// Its paint id.
    pub paint: u8,
    /// Camera framing.
    pub focus: UiBackdropFocus,
}

pub(crate) fn register(app: &mut App) {
    app.add_react_request_handler(
        |request: On<Request<GetCatalog>>,
         catalog: Option<Res<Catalog>>,
         paths: Option<Res<UserPaths>>| {
            let Some(catalog) = catalog else {
                return request.respond_err("catalog not loaded");
            };
            let assets = paths.map_or_else(|| "assets".into(), |p| p.assets_dir.clone());
            request.respond(UiCatalog::new(&catalog, &assets));
        },
    )
    .add_react_handler(
        |backdrop: On<SetBackdrop>, current: Option<ResMut<MenuBackdrop>>| {
            let Some(mut current) = current else { return };
            let msg = backdrop.event();
            let next = MenuBackdrop {
                car: CarId::new(msg.car.as_str()),
                paint: PaintId(msg.paint),
                focus: msg.focus.into(),
            };
            current.set_if_neq(next);
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog() -> Catalog {
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        Catalog::load(&assets).expect("the shipped catalog loads")
    }

    #[test]
    fn menus_never_list_dev_tracks_and_cup_races_point_at_listed_tracks() {
        let catalog = catalog();
        let ui = UiCatalog::new(&catalog, Path::new("/nonexistent"));
        assert!(ui.tracks.iter().all(|t| t.id != "test_oval"));
        assert_eq!(
            ui.tracks.len(),
            catalog.tracks.iter().filter(|t| !t.hidden).count()
        );
        for cup in &catalog.cups {
            let ui_cup = ui
                .cups
                .iter()
                .find(|c| c.id == cup.id.0)
                .expect("cup listed");
            for (race, ui_race) in cup.races.iter().zip(&ui_cup.races) {
                assert_eq!(ui.tracks[ui_race.track].id, race.variant.track.0);
                assert!((1..=9).contains(&ui_race.laps));
            }
        }
    }

    #[test]
    fn missing_images_are_not_offered_to_react() {
        let ui = UiCatalog::new(&catalog(), Path::new("/nonexistent"));
        assert!(ui.cars.iter().all(|c| c.thumbnail.is_none()));
        assert!(ui.cars.iter().all(|c| c.thumbnail_foot == 0.0));
        assert!(ui.tracks.iter().all(|t| t.preview.is_none()));
    }

    /// QA B37: the margin under each car is measured from its own thumbnail (the shipped ones
    /// range from ≈ 10 % for the tall Dumpster to ≈ 18 %).
    #[test]
    fn each_thumbnail_reports_the_transparent_margin_under_its_car() {
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let ui = UiCatalog::new(&catalog(), &assets);
        for car in &ui.cars {
            assert!(car.thumbnail.is_some(), "{} has a thumbnail", car.id);
            assert!(
                (0.05..0.25).contains(&car.thumbnail_foot),
                "{}: {}",
                car.id,
                car.thumbnail_foot
            );
        }
        let feet: Vec<f32> = ui.cars.iter().map(|c| c.thumbnail_foot).collect();
        let (lo, hi) = feet
            .iter()
            .fold((1.0f32, 0.0f32), |(lo, hi), &f| (lo.min(f), hi.max(f)));
        assert!(hi - lo > 0.05, "they differ per car: {feet:?}");
    }
}
