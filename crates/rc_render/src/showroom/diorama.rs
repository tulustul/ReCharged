//! The showroom set: a garage corner at toy scale (the RC car is 2 u long; props are real size ×
//! 4.44), built from primitives with library materials. The `MenuCamera` looks from the −X/−Z
//! quadrant toward the corner walls on +X and +Z.

use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use rc_core::prelude::*;

use super::Turntable;
use crate::lighting::glass::{WindowGlass, WindowMaterial};
use crate::materials::{MaterialDef, MaterialLibrary, SurfaceMaterial, with_world_uvs};

/// Turntable: mat thickness, base radius/height, top radius/height (u).
const MAT_H: f32 = 0.04;
const BASE_R: f32 = 2.05;
const BASE_H: f32 = 0.16;
const TOP_R: f32 = 1.9;
const TOP_H: f32 = 0.05;
/// Distance of the corner walls from the car (u) and their height.
const WALL_D: f32 = 12.5;
const WALL_H: f32 = 16.0;
/// The moonlit window in the side wall: z range, sill and top (u).
const WINDOW_Z: (f32, f32) = (-10.0, 2.0);
const WINDOW_Y: (f32, f32) = (4.5, 12.5);
/// Brand colours (linear-ish sRGB) for emissives: electric cyan and volt yellow.
const CYAN: [f32; 3] = [0.1, 0.85, 1.0];
const VOLT: [f32; 3] = [0.95, 0.88, 0.06];

/// Builds materials and spawns meshes for the set.
struct Set<'a, 'w, 's> {
    commands: &'a mut Commands<'w, 's>,
    meshes: &'a mut Assets<Mesh>,
    surfaces: &'a mut Assets<SurfaceMaterial>,
    standard: &'a mut Assets<StandardMaterial>,
    glass: &'a mut Assets<WindowMaterial>,
    library: &'a MaterialLibrary,
}

impl Set<'_, '_, '_> {
    fn surface(&mut self, def: MaterialDef) -> Handle<SurfaceMaterial> {
        self.library.make(&def, self.surfaces)
    }

    fn textured(&mut self, set: &str, color: [f32; 3], roughness: f32) -> Handle<SurfaceMaterial> {
        self.surface(MaterialDef {
            set: Some(set.into()),
            color,
            roughness,
            weather: false,
            ..default()
        })
    }

    fn flat(&mut self, color: [f32; 3], roughness: f32, metallic: f32) -> Handle<StandardMaterial> {
        self.standard.add(StandardMaterial {
            base_color: Color::srgb(color[0], color[1], color[2]),
            perceptual_roughness: roughness,
            metallic,
            ..default()
        })
    }

    fn glow(&mut self, color: [f32; 3], strength: f32) -> Handle<StandardMaterial> {
        self.standard.add(StandardMaterial {
            base_color: Color::srgb(color[0] * 0.2, color[1] * 0.2, color[2] * 0.2),
            emissive: LinearRgba::rgb(color[0], color[1], color[2]) * strength,
            ..default()
        })
    }

    fn mesh(&mut self, mesh: impl Into<Mesh>) -> Handle<Mesh> {
        self.meshes.add(with_world_uvs(mesh.into()))
    }

    fn piece<M: Material>(
        &mut self,
        name: &'static str,
        mesh: Handle<Mesh>,
        material: Handle<M>,
        at: Transform,
    ) -> Entity {
        self.commands
            .spawn((
                Name::new(name),
                Mesh3d(mesh),
                MeshMaterial3d(material),
                at,
                DespawnOnExit(AppState::Menu),
            ))
            .id()
    }
}

pub(super) fn spawn(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut surfaces: ResMut<Assets<SurfaceMaterial>>,
    mut standard: ResMut<Assets<StandardMaterial>>,
    mut glass: ResMut<Assets<WindowMaterial>>,
    library: Option<Res<MaterialLibrary>>,
) {
    let Some(library) = library else {
        return;
    };
    let mut set = Set {
        commands: &mut commands,
        meshes: &mut meshes,
        surfaces: &mut surfaces,
        standard: &mut standard,
        glass: &mut glass,
        library: &library,
    };
    room(&mut set);
    turntable(&mut set);
    props(&mut set);
    lights(&mut set);
}

fn room(set: &mut Set) {
    // Polished garage floor: dark epoxy over concrete, glossy enough to catch the neon and window.
    let floor = set.surface(MaterialDef {
        set: Some("concrete".into()),
        color: [0.36, 0.38, 0.42],
        roughness: 0.6,
        clearcoat: 0.5,
        clearcoat_roughness: 0.18,
        weather: false,
        ..default()
    });
    let mesh = set.mesh(Plane3d::default().mesh().size(220.0, 220.0));
    set.piece("Garage floor", mesh, floor, Transform::default());

    let wall = set.textured("plaster", [0.36, 0.39, 0.46], 1.0);
    let back = set.mesh(Cuboid::new(70.0, WALL_H, 0.6));
    set.piece(
        "Back wall",
        back,
        wall.clone(),
        Transform::from_xyz(0.0, WALL_H / 2.0, WALL_D),
    );
    // Side wall around the window opening.
    let (z0, z1) = WINDOW_Z;
    let (y0, y1) = WINDOW_Y;
    for (len_z, cz, h, cy) in [
        (70.0, 0.0, y0, y0 / 2.0),
        (70.0, 0.0, WALL_H - y1, (WALL_H + y1) / 2.0),
        (z0 + 35.0, (z0 - 35.0) / 2.0, y1 - y0, (y0 + y1) / 2.0),
        (35.0 - z1, (z1 + 35.0) / 2.0, y1 - y0, (y0 + y1) / 2.0),
    ] {
        let piece = set.mesh(Cuboid::new(0.6, h, len_z));
        set.piece(
            "Side wall",
            piece,
            wall.clone(),
            Transform::from_xyz(WALL_D, cy, cz),
        );
    }
    // Window: night sky glass, a white-painted frame and a mullion cross.
    let pane = set.glass.add(WindowMaterial::default());
    let glass = set.meshes.add(Rectangle::new(z1 - z0, y1 - y0));
    let id = set.piece(
        "Window glass",
        glass,
        pane,
        Transform::from_xyz(WALL_D + 0.1, (y0 + y1) / 2.0, (z0 + z1) / 2.0)
            .looking_to(Vec3::X, Vec3::Y),
    );
    set.commands
        .entity(id)
        .insert((WindowGlass::default(), NotShadowCaster));
    let frame = set.flat([0.82, 0.83, 0.85], 0.45, 0.0);
    let (cz, cy) = ((z0 + z1) / 2.0, (y0 + y1) / 2.0);
    for (size, at) in [
        (
            Vec3::new(0.3, y1 - y0, 0.25),
            Vec3::new(WALL_D - 0.2, cy, cz),
        ),
        (
            Vec3::new(0.3, 0.25, z1 - z0),
            Vec3::new(WALL_D - 0.2, cy + 0.5, cz),
        ),
        (
            Vec3::new(1.4, 0.25, z1 - z0 + 0.8),
            Vec3::new(WALL_D - 0.6, y0 - 0.1, cz),
        ),
    ] {
        let bar = set.mesh(Cuboid::from_size(size));
        set.piece(
            "Window frame",
            bar,
            frame.clone(),
            Transform::from_translation(at),
        );
    }

    // Skirting.
    let trim = set.flat([0.08, 0.085, 0.1], 0.5, 0.0);
    let skirt_back = set.mesh(Cuboid::new(70.0, 0.7, 0.2));
    set.piece(
        "Skirting",
        skirt_back,
        trim.clone(),
        Transform::from_xyz(0.0, 0.35, WALL_D - 0.4),
    );
    let skirt_side = set.mesh(Cuboid::new(0.2, 0.7, 70.0));
    set.piece(
        "Skirting",
        skirt_side,
        trim,
        Transform::from_xyz(WALL_D - 0.4, 0.35, 0.0),
    );

    // Rubber mat under the turntable, inside a volt-yellow floor marking.
    let mat = set.surface(MaterialDef {
        set: Some("rubber".into()),
        color: [0.6, 0.6, 0.63],
        tile_u: 1.0,
        weather: false,
        ..default()
    });
    let mat_mesh = set.mesh(Cuboid::new(9.0, MAT_H, 9.0));
    set.piece(
        "Rubber mat",
        mat_mesh,
        mat,
        Transform::from_xyz(0.0, MAT_H / 2.0, 0.0),
    );
    let paint = set.flat(VOLT, 0.55, 0.0);
    for (w, d, x, z) in [
        (11.0, 0.3, 0.0, -5.4),
        (11.0, 0.3, 0.0, 5.4),
        (0.3, 11.0, -5.4, 0.0),
        (0.3, 11.0, 5.4, 0.0),
    ] {
        let line = set.mesh(Cuboid::new(w, 0.01, d));
        set.piece(
            "Floor marking",
            line,
            paint.clone(),
            Transform::from_xyz(x, 0.005, z),
        );
    }
}

fn turntable(set: &mut Set) {
    let base_mat = set.surface(MaterialDef {
        set: Some("metal".into()),
        color: [0.22, 0.24, 0.27],
        metallic: 1.0,
        roughness: 0.9,
        weather: false,
        ..default()
    });
    let base = set.mesh(Cylinder::new(BASE_R, BASE_H).mesh().resolution(96));
    set.piece(
        "Turntable base",
        base,
        base_mat,
        Transform::from_xyz(0.0, MAT_H + BASE_H / 2.0, 0.0),
    );
    let ring_mat = set.glow(CYAN, 5.0);
    let ring = set.meshes.add(
        Torus::new(BASE_R - 0.03, BASE_R + 0.03)
            .mesh()
            .minor_resolution(8)
            .major_resolution(128),
    );
    set.piece(
        "LED ring",
        ring,
        ring_mat,
        Transform::from_xyz(0.0, MAT_H + BASE_H * 0.55, 0.0),
    );

    let top_y = MAT_H + BASE_H + TOP_H;
    let top_mat = set.textured("rubber", [0.3, 0.3, 0.33], 1.0);
    let top = set.mesh(Cylinder::new(TOP_R, TOP_H).mesh().resolution(96));
    let disc = set
        .commands
        .spawn((
            Name::new("Turntable top"),
            Mesh3d(top),
            MeshMaterial3d(top_mat),
            Transform::from_xyz(0.0, -TOP_H / 2.0, 0.0),
        ))
        .id();
    set.commands
        .spawn((
            Name::new("Turntable"),
            Turntable,
            Transform::from_translation(SHOWROOM_FOCUS + Vec3::Y * top_y),
            Visibility::default(),
            DespawnOnExit(AppState::Menu),
        ))
        .add_child(disc);
}

fn props(set: &mut Set) {
    // Pegboard with hanging tools on the back wall.
    let board = set.textured("plywood", [0.55, 0.43, 0.32], 1.0);
    let board_mesh = set.mesh(Cuboid::new(18.0, 8.0, 0.25));
    set.piece(
        "Pegboard",
        board_mesh,
        board,
        Transform::from_xyz(-2.0, 7.0, WALL_D - 0.45),
    );
    let steel = set.flat([0.8, 0.81, 0.83], 0.28, 1.0);
    let handle_red = set.flat([0.72, 0.1, 0.08], 0.35, 0.0);
    let handle_yellow = set.flat([0.92, 0.7, 0.05], 0.35, 0.0);
    for (i, x) in [-8.5f32, -6.0, -3.5].iter().enumerate() {
        let len = 3.6 - i as f32 * 0.6;
        let bar = set.mesh(Cuboid::new(0.32, len, 0.12));
        set.piece(
            "Wrench",
            bar,
            steel.clone(),
            Transform::from_xyz(*x, 7.4, WALL_D - 0.7),
        );
        let head = set.meshes.add(Torus::new(0.22, 0.42));
        set.piece(
            "Wrench head",
            head,
            steel.clone(),
            Transform::from_xyz(*x, 7.4 + len / 2.0 + 0.3, WALL_D - 0.7)
                .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
        );
    }
    for (x, mat) in [
        (0.5f32, handle_red.clone()),
        (2.0, handle_yellow.clone()),
        (3.5, handle_red),
    ] {
        let shaft = set.meshes.add(Cylinder::new(0.07, 2.4));
        set.piece(
            "Screwdriver",
            shaft,
            steel.clone(),
            Transform::from_xyz(x, 6.0, WALL_D - 0.7),
        );
        let grip = set.meshes.add(Capsule3d::new(0.22, 1.1));
        set.piece(
            "Screwdriver grip",
            grip,
            mat,
            Transform::from_xyz(x, 7.7, WALL_D - 0.7),
        );
    }

    // Red tool chest in the corner.
    let chest = set.surface(MaterialDef {
        set: Some("metal".into()),
        color: [0.78, 0.14, 0.1],
        roughness: 0.5,
        clearcoat: 0.6,
        weather: false,
        ..default()
    });
    let chest_mesh = set.mesh(Cuboid::new(5.0, 6.0, 3.0));
    set.piece(
        "Tool chest",
        chest_mesh,
        chest,
        Transform::from_xyz(WALL_D - 5.0, 3.0, WALL_D - 2.2),
    );
    let gap = set.flat([0.05, 0.05, 0.055], 0.6, 0.0);
    for i in 0..5 {
        let y = 1.0 + i as f32 * 1.05;
        let seam = set.mesh(Cuboid::new(4.7, 0.06, 0.05));
        set.piece(
            "Drawer seam",
            seam,
            gap.clone(),
            Transform::from_xyz(WALL_D - 5.0, y, WALL_D - 3.72),
        );
        let pull = set.mesh(Cuboid::new(2.4, 0.14, 0.16));
        set.piece(
            "Drawer pull",
            pull,
            steel.clone(),
            Transform::from_xyz(WALL_D - 5.0, y + 0.45, WALL_D - 3.78),
        );
    }

    // A real car tyre leaning on the side wall (giant next to the RC car).
    let rubber = set.textured("rubber", [0.35, 0.35, 0.37], 1.0);
    let tyre = set.meshes.add(
        Torus::new(1.25, 2.85)
            .mesh()
            .minor_resolution(24)
            .major_resolution(64),
    );
    set.piece(
        "Tyre",
        tyre,
        rubber,
        Transform::from_xyz(WALL_D - 1.9, 2.75, 7.5)
            .with_rotation(Quat::from_rotation_z(1.42) * Quat::from_rotation_x(0.05)),
    );
    let rim = set.flat([0.6, 0.62, 0.65], 0.3, 1.0);
    let rim_mesh = set
        .meshes
        .add(Cylinder::new(1.6, 1.5).mesh().resolution(48));
    set.piece(
        "Wheel rim",
        rim_mesh,
        rim,
        Transform::from_xyz(WALL_D - 1.9, 2.75, 7.5)
            .with_rotation(Quat::from_rotation_z(1.42) * Quat::from_rotation_x(0.05)),
    );

    // Shelf with paint cans and a cardboard box on the side wall.
    let shelf = set.textured("wood_planks", [0.8, 0.7, 0.58], 1.0);
    let plank = set.mesh(Cuboid::new(2.2, 0.25, 12.0));
    set.piece(
        "Shelf",
        plank,
        shelf,
        Transform::from_xyz(WALL_D - 1.4, 7.5, 16.0),
    );
    for (i, color) in [[0.85, 0.2, 0.15], [0.15, 0.4, 0.85], [0.95, 0.75, 0.1]]
        .iter()
        .enumerate()
    {
        let can_mat = set.flat(*color, 0.35, 0.0);
        let can = set
            .meshes
            .add(Cylinder::new(0.6, 1.4).mesh().resolution(32));
        let z = 11.5 + i as f32 * 1.5;
        set.piece(
            "Paint can",
            can,
            can_mat,
            Transform::from_xyz(WALL_D - 1.4, 8.33, z),
        );
        let lid = set
            .meshes
            .add(Cylinder::new(0.62, 0.08).mesh().resolution(32));
        set.piece(
            "Can lid",
            lid,
            steel.clone(),
            Transform::from_xyz(WALL_D - 1.4, 9.07, z),
        );
    }
    let carton = set.textured("cardboard", [1.0, 1.0, 1.0], 1.0);
    let box_mesh = set.mesh(Cuboid::new(2.0, 1.8, 3.2));
    set.piece(
        "Cardboard box",
        box_mesh,
        carton,
        Transform::from_xyz(WALL_D - 1.5, 8.53, 19.0),
    );

    // Battery charger with status LEDs beside the turntable.
    let charger = set.flat([0.06, 0.065, 0.075], 0.45, 0.0);
    let charger_mesh = set.mesh(Cuboid::new(1.6, 0.7, 1.1));
    set.piece(
        "Charger",
        charger_mesh,
        charger,
        Transform::from_xyz(3.6, MAT_H + 0.35, 2.6).with_rotation(Quat::from_rotation_y(-0.5)),
    );
    let led_on = set.glow(VOLT, 6.0);
    let led_cyan = set.glow(CYAN, 6.0);
    for (i, led) in [led_on.clone(), led_on, led_cyan].into_iter().enumerate() {
        let dot = set.meshes.add(Sphere::new(0.07));
        let local = Vec3::new(-0.4 + i as f32 * 0.3, 0.36, -0.56);
        let at =
            Transform::from_xyz(3.6, MAT_H + 0.35, 2.6).with_rotation(Quat::from_rotation_y(-0.5));
        set.piece(
            "Charger LED",
            dot,
            led,
            Transform::from_translation(at.transform_point(local)),
        );
    }

    // Neon strips: cyan on the back wall, volt on the side wall.
    let cyan_tube = set.glow(CYAN, 7.0);
    let strip = set.meshes.add(Cylinder::new(0.09, 26.0));
    let tube = set.piece(
        "Neon strip",
        strip,
        cyan_tube,
        Transform::from_xyz(-2.0, 12.4, WALL_D - 0.5)
            .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
    );
    set.commands.entity(tube).insert(NotShadowCaster);
    let volt_tube = set.glow(VOLT, 5.0);
    let strip = set.meshes.add(Cylinder::new(0.07, 16.0));
    let tube = set.piece(
        "Neon strip",
        strip,
        volt_tube,
        Transform::from_xyz(WALL_D - 0.5, 13.5, 16.0)
            .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
    );
    set.commands.entity(tube).insert(NotShadowCaster);
}

fn lights(set: &mut Set) {
    let scope = DespawnOnExit(AppState::Menu);
    let car = SHOWROOM_FOCUS + Vec3::Y * 0.5;
    // Warm key spot from above front-left (the work lamp), soft shadows on the turntable.
    set.commands.spawn((
        Name::new("Showroom key"),
        SpotLight {
            color: Color::srgb(1.0, 0.87, 0.72),
            intensity: 5.5e6,
            range: 60.0,
            radius: 0.6,
            outer_angle: 0.34,
            inner_angle: 0.16,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(-5.0, 12.0, -6.0).looking_at(car, Vec3::Y),
        scope.clone(),
    ));
    // Cool rim light from behind right: the silhouette reads against the dark corner.
    set.commands.spawn((
        Name::new("Showroom rim"),
        SpotLight {
            color: Color::srgb(0.55, 0.78, 1.0),
            intensity: 2.2e6,
            range: 60.0,
            outer_angle: 0.5,
            inner_angle: 0.3,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_xyz(7.0, 7.0, 10.0).looking_at(car, Vec3::Y),
        scope.clone(),
    ));
    // Cyan glow of the LED ring on the floor and the car's underside.
    set.commands.spawn((
        Name::new("LED ring light"),
        PointLight {
            color: Color::srgb(CYAN[0], CYAN[1], CYAN[2]),
            intensity: 60_000.0,
            range: 7.0,
            radius: 1.8,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_xyz(0.0, 0.5, 0.0),
        scope.clone(),
    ));
    // A warm work lamp over the bench corner: the tool chest and pegboard read in colour, and the
    // set gets its warm/cool contrast against the moonlit window.
    set.commands.spawn((
        Name::new("Work lamp"),
        PointLight {
            color: Color::srgb(1.0, 0.78, 0.52),
            intensity: 350_000.0,
            range: 22.0,
            radius: 0.4,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_xyz(3.0, 10.0, WALL_D - 3.0),
        scope.clone(),
    ));
    // Neon strips wash the walls.
    for (at, color, lumens) in [
        (Vec3::new(-2.0, 11.5, WALL_D - 2.0), CYAN, 450_000.0),
        (Vec3::new(WALL_D - 2.0, 12.5, 16.0), VOLT, 300_000.0),
    ] {
        set.commands.spawn((
            Name::new("Neon wash"),
            PointLight {
                color: Color::srgb(color[0], color[1], color[2]),
                intensity: lumens,
                range: 30.0,
                radius: 0.5,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_translation(at),
            scope.clone(),
        ));
    }
}
