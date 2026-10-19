//! Skid marks: ribbons laid behind sliding wheels (rubber on hard floors, torn grass, ruts in dirt
//! and mud, tracks in snow). All marks share one dynamic mesh rebuilt only when a segment is added
//! or while marks fade; the segment ring is capped by the particle density setting.

use bevy::asset::RenderAssetUsages;
use bevy::light::NotShadowCaster;
use bevy::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use bevy::prelude::*;
use rc_core::prelude::*;

use crate::cars::{CarFx, Poses, SKID_SLIP, ground_point};
use crate::fx::Fx;

/// Segments kept at the High density setting.
pub const MAX_SEGMENTS: usize = 2400;
/// Length of one segment, u.
const SEGMENT_U: f32 = 0.3;
/// A wheel that jumps further than this between frames (respawn) starts a new strip, u.
const MAX_GAP_U: f32 = 2.5;
/// Marks start fading after this many seconds and are gone after `FADE_END_S`.
const FADE_START_S: f32 = 14.0;
const FADE_END_S: f32 = 22.0;
/// Height of marks above the contact patch, u (keeps them out of the ground's depth).
const LIFT_U: f32 = 0.014;
/// How often fading marks refresh their colours, s.
const FADE_REFRESH_S: f32 = 0.25;

/// One quad of a mark: start edge, end edge (left, right), colour, birth time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Segment {
    /// Start edge (left, right).
    pub a: [Vec3; 2],
    /// End edge (left, right).
    pub b: [Vec3; 2],
    /// Linear RGBA.
    pub color: [f32; 4],
    /// Time it was laid, s.
    pub born: f32,
}

/// The capped ring of mark segments.
#[derive(Resource, Debug, Clone)]
pub struct SkidMarks {
    segs: Vec<Segment>,
    head: usize,
    cap: usize,
    dirty: bool,
}

impl Default for SkidMarks {
    fn default() -> Self {
        Self::new(MAX_SEGMENTS)
    }
}

impl SkidMarks {
    /// An empty ring of `cap` segments.
    pub fn new(cap: usize) -> Self {
        Self {
            segs: Vec::with_capacity(cap),
            head: 0,
            cap: cap.max(1),
            dirty: false,
        }
    }

    /// Adds a segment, replacing the oldest when full.
    pub fn push(&mut self, seg: Segment) {
        if self.segs.len() < self.cap {
            self.segs.push(seg);
        } else {
            self.segs[self.head] = seg;
            self.head = (self.head + 1) % self.cap;
        }
        self.dirty = true;
    }

    /// Segments held.
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.segs.len()
    }

    /// Changes the cap (density setting); shrinking drops the oldest marks.
    pub fn set_cap(&mut self, cap: usize) {
        let cap = cap.max(1);
        if cap == self.cap {
            return;
        }
        // Oldest first, then keep the newest `cap`.
        self.segs.rotate_left(self.head);
        if self.segs.len() > cap {
            self.segs.drain(..self.segs.len() - cap);
        }
        self.head = 0;
        self.cap = cap;
        self.dirty = true;
    }

    /// Drops every mark.
    pub fn clear(&mut self) {
        self.segs.clear();
        self.head = 0;
        self.dirty = true;
    }

    /// Opacity multiplier of a mark `age` seconds old.
    pub fn fade(age: f32) -> f32 {
        1.0 - ((age - FADE_START_S) / (FADE_END_S - FADE_START_S)).clamp(0.0, 1.0)
    }

    /// Any mark is between fully visible and gone.
    pub fn fading(&self, now: f32) -> bool {
        self.segs.iter().any(|s| {
            let age = now - s.born;
            age > FADE_START_S && age < FADE_END_S
        })
    }

    /// Fills mesh buffers (cleared first) with the visible segments at time `now`: four vertices
    /// and two triangles per segment, faded by age.
    pub fn build(
        &self,
        now: f32,
        positions: &mut Vec<[f32; 3]>,
        normals: &mut Vec<[f32; 3]>,
        colors: &mut Vec<[f32; 4]>,
        indices: &mut Vec<u32>,
    ) {
        positions.clear();
        normals.clear();
        colors.clear();
        indices.clear();
        for s in &self.segs {
            let fade = Self::fade(now - s.born);
            if fade <= 0.0 {
                continue;
            }
            let base = positions.len() as u32;
            let n = (s.b[0] - s.a[0])
                .cross(s.a[1] - s.a[0])
                .normalize_or(Vec3::Y);
            let n = if n.y < 0.0 { -n } else { n };
            for p in [s.a[0], s.a[1], s.b[1], s.b[0]] {
                positions.push(p.to_array());
                normals.push(n.to_array());
                colors.push([s.color[0], s.color[1], s.color[2], s.color[3] * fade]);
            }
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
    }
}

/// The open strip of one wheel: where its last segment ended.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Strip {
    last: Option<(Vec3, [Vec3; 2])>,
}

impl Strip {
    /// Extends the strip to `centre` (half-width vector `half`), laying a segment every
    /// [`SEGMENT_U`]; a jump longer than [`MAX_GAP_U`] starts a new strip.
    pub fn advance(
        &mut self,
        centre: Vec3,
        half: Vec3,
        color: [f32; 4],
        now: f32,
        marks: &mut SkidMarks,
    ) {
        let edge = [centre - half, centre + half];
        match self.last {
            Some((prev, prev_edge)) => {
                let d = prev.distance(centre);
                if d > MAX_GAP_U {
                    self.last = Some((centre, edge));
                } else if d >= SEGMENT_U {
                    marks.push(Segment {
                        a: prev_edge,
                        b: edge,
                        color,
                        born: now,
                    });
                    self.last = Some((centre, edge));
                }
            }
            None => self.last = Some((centre, edge)),
        }
    }

    /// The wheel stopped marking.
    pub fn end(&mut self) {
        self.last = None;
    }
}

/// Mark colour (linear RGBA) a wheel leaves, if any: `slip` 0–1, `speed` u/s.
pub fn mark_color(
    surface: SurfaceKind,
    wetness: f32,
    snow: f32,
    slip: f32,
    speed: f32,
) -> Option<[f32; 4]> {
    use SurfaceKind::*;
    if speed < 2.0 {
        return None;
    }
    let strength = |threshold: f32| {
        (slip > threshold).then(|| ((slip - threshold) / 0.3 + 0.45).clamp(0.45, 1.0))
    };
    if snow > 0.4 && surface != Water {
        return Some([0.25, 0.29, 0.35, 0.45]);
    }
    let wet = 1.0 - 0.5 * wetness.clamp(0.0, 1.0);
    let (rgb, alpha, threshold) = match surface {
        // Linear colours (vertex colours are linear): a little darker than each ground.
        Asphalt | Stone | Tile | Wood | ToyTrack | Cardboard => {
            ([0.008, 0.008, 0.009], 0.6, SKID_SLIP)
        }
        Carpet => ([0.02, 0.018, 0.016], 0.3, SKID_SLIP),
        Ice => ([0.6, 0.68, 0.78], 0.25, SKID_SLIP),
        Grass => ([0.012, 0.025, 0.006], 0.65, 0.2),
        Dirt if wetness > 0.35 => ([0.012, 0.008, 0.004], 0.75, 0.15),
        Dirt => ([0.03, 0.018, 0.009], 0.65, 0.2),
        Water | Oil => return None,
    };
    strength(threshold).map(|s| [rgb[0], rgb[1], rgb[2], alpha * s * wet])
}

/// Open strips of a racer's four wheels.
#[derive(Component, Debug, Default)]
pub(crate) struct CarSkids([Strip; 4]);

/// The entity drawing every mark.
#[derive(Resource, Debug, Default)]
struct SkidMesh {
    entity: Option<Entity>,
    mesh: Handle<Mesh>,
    last_build: f32,
}

pub(crate) fn build(app: &mut App) {
    app.init_resource::<SkidMarks>()
        .init_resource::<SkidMesh>()
        .add_observer(|add: On<Add<CarFx>>, mut commands: Commands| {
            commands.entity(add.entity).insert(CarSkids::default());
        })
        .add_systems(OnEnter(RaceSession), spawn_mesh)
        .add_systems(OnEnter(RacePhase::Setup), clear_marks)
        .add_systems(OnEnter(AppState::Unloading), forget_mesh)
        .add_systems(
            Update,
            (lay_marks, rebuild_mesh)
                .chain()
                .in_set(crate::VfxSystems::Emit),
        );
}

fn spawn_mesh(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut state: ResMut<SkidMesh>,
    mut marks: ResMut<SkidMarks>,
) {
    let mesh = meshes.add(empty_mesh());
    let material = materials.add(material());
    let entity = commands
        .spawn((
            Name::new("Skid marks"),
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material),
            Transform::default(),
            Visibility::Hidden,
            NotShadowCaster,
            DespawnOnExit(RaceSession),
        ))
        .id();
    *state = SkidMesh {
        entity: Some(entity),
        mesh,
        last_build: 0.0,
    };
    marks.clear();
}

/// The marks' mesh before the first mark: one degenerate triangle with the marks' attributes.
pub(crate) fn empty_mesh() -> Mesh {
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0.0f32; 3]; 3])
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0f32, 1.0, 0.0]; 3])
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, vec![[0.0f32; 4]; 3])
    .with_inserted_indices(Indices::U32(vec![0, 1, 2]))
}

/// The marks' material (vertex colours carry colour and fade).
pub(crate) fn material() -> StandardMaterial {
    StandardMaterial {
        base_color: Color::WHITE,
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.92,
        reflectance: 0.3,
        ..default()
    }
}

fn clear_marks(mut marks: ResMut<SkidMarks>, mut skids: Query<&mut CarSkids>) {
    marks.clear();
    for mut s in &mut skids {
        s.0 = default();
    }
}

fn forget_mesh(mut state: ResMut<SkidMesh>, mut marks: ResMut<SkidMarks>) {
    *state = SkidMesh::default();
    marks.clear();
}

fn lay_marks(
    time: Res<Time>,
    fx: Res<Fx>,
    weather: Option<Res<WeatherState>>,
    poses: Poses,
    mut marks: ResMut<SkidMarks>,
    mut cars: Query<(Entity, &VehicleTelemetry, &CarFx, &mut CarSkids)>,
) {
    marks.set_cap((MAX_SEGMENTS as f32 * fx.density) as usize);
    let now = time.elapsed_secs();
    let (wet, snow) = weather.map_or((0.0, 0.0), |w| (w.wetness, w.snow_cover));
    for (entity, t, car, mut skids) in &mut cars {
        let Some(pose) = poses.of(entity) else {
            continue;
        };
        let right = pose.rotation * Vec3::X;
        for (i, w) in t.wheels.iter().enumerate() {
            let color = w
                .surface
                .filter(|_| w.grounded)
                .and_then(|s| mark_color(s, wet, snow, w.slip, t.speed));
            let Some(color) = color else {
                skids.0[i].end();
                continue;
            };
            // On the ground the wheel touches (ramps, curbs), across the tyre within that plane.
            let (at, up) = ground_point(&pose, &car.shape, i, w);
            let centre = at + up * LIFT_U;
            let across = (right - up * right.dot(up)).normalize_or(right);
            let half = across * car.shape.width[i] * 0.45;
            skids.0[i].advance(centre, half, color, now, &mut marks);
        }
    }
}

fn rebuild_mesh(
    time: Res<Time>,
    mut state: ResMut<SkidMesh>,
    mut marks: ResMut<SkidMarks>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut visibility: Query<&mut Visibility>,
) {
    let now = time.elapsed_secs();
    let refresh = now - state.last_build > FADE_REFRESH_S && marks.fading(now);
    if !(marks.dirty || refresh) {
        return;
    }
    let Some(entity) = state.entity else {
        return;
    };
    let Some(mut mesh) = meshes.get_mut(&state.mesh) else {
        return;
    };
    marks.dirty = false;
    state.last_build = now;
    // Reuse the mesh's own buffers: no allocation once they reached their peak size.
    let mut take = |id| match mesh.remove_attribute(id) {
        Some(VertexAttributeValues::Float32x3(v)) => v,
        _ => Vec::new(),
    };
    let (mut positions, mut normals) =
        (take(Mesh::ATTRIBUTE_POSITION), take(Mesh::ATTRIBUTE_NORMAL));
    let mut colors = match mesh.remove_attribute(Mesh::ATTRIBUTE_COLOR) {
        Some(VertexAttributeValues::Float32x4(v)) => v,
        _ => Vec::new(),
    };
    let mut indices = match mesh.remove_indices() {
        Some(Indices::U32(v)) => v,
        _ => Vec::new(),
    };
    marks.build(now, &mut positions, &mut normals, &mut colors, &mut indices);
    let empty = indices.is_empty();
    if empty {
        // A degenerate triangle: Bevy warns about meshes without vertices.
        positions.extend([[0.0; 3]; 3]);
        normals.extend([[0.0, 1.0, 0.0]; 3]);
        colors.extend([[0.0; 4]; 3]);
        indices.extend([0, 1, 2]);
    }
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
    if let Ok(mut v) = visibility.get_mut(entity) {
        v.set_if_neq(if empty {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(born: f32) -> Segment {
        Segment {
            a: [Vec3::ZERO, Vec3::X],
            b: [Vec3::NEG_Z, Vec3::new(1.0, 0.0, -1.0)],
            color: [0.0, 0.0, 0.0, 0.5],
            born,
        }
    }

    #[test]
    fn strips_lay_a_segment_every_step_and_restart_after_jumps() {
        let mut marks = SkidMarks::new(100);
        let mut strip = Strip::default();
        let half = Vec3::X * 0.1;
        for i in 0..=10 {
            strip.advance(
                Vec3::new(0.0, 0.0, -0.1 * i as f32),
                half,
                [0.0; 4],
                0.0,
                &mut marks,
            );
        }
        // 1 u of travel at 0.3 u per segment.
        assert_eq!(marks.len(), 3);
        let s = marks.segs[1];
        assert_eq!(s.a, marks.segs[0].b, "segments of a strip are joined");
        assert!(
            (s.a[1] - s.a[0] - half * 2.0).length() < 1e-6,
            "width = 2 × half"
        );
        // A respawn far away does not draw a quad across the track.
        strip.advance(Vec3::new(50.0, 0.0, 0.0), half, [0.0; 4], 0.0, &mut marks);
        strip.advance(Vec3::new(50.0, 0.0, -0.31), half, [0.0; 4], 0.0, &mut marks);
        assert_eq!(marks.len(), 4);
        assert!(marks.segs[3].a[0].x > 49.0);
        strip.end();
        strip.advance(Vec3::ZERO, half, [0.0; 4], 0.0, &mut marks);
        assert_eq!(marks.len(), 4, "a new strip needs a second point");
    }

    #[test]
    fn ring_is_capped_and_keeps_the_newest() {
        let mut marks = SkidMarks::new(3);
        for i in 0..5 {
            marks.push(seg(i as f32));
        }
        assert_eq!(marks.len(), 3);
        let mut born: Vec<f32> = marks.segs.iter().map(|s| s.born).collect();
        born.sort_by(f32::total_cmp);
        assert_eq!(born, vec![2.0, 3.0, 4.0]);
        marks.set_cap(2);
        let mut born: Vec<f32> = marks.segs.iter().map(|s| s.born).collect();
        born.sort_by(f32::total_cmp);
        assert_eq!(born, vec![3.0, 4.0], "shrinking drops the oldest");
    }

    #[test]
    fn mesh_has_two_triangles_per_visible_segment_and_fades() {
        let mut marks = SkidMarks::new(10);
        marks.push(seg(0.0));
        marks.push(seg(10.0));
        let (mut p, mut n, mut c, mut i) = (vec![], vec![], vec![], vec![]);
        marks.build(10.0, &mut p, &mut n, &mut c, &mut i);
        assert_eq!((p.len(), i.len()), (8, 12));
        assert!(n.iter().all(|n| n[1] > 0.99), "normals face up");
        assert_eq!(i[6..], [4, 5, 6, 4, 6, 7]);
        // 18 s later the first is half faded, the second still full.
        marks.build(18.0, &mut p, &mut n, &mut c, &mut i);
        assert!((c[0][3] - 0.25).abs() < 1e-5 && c[4][3] == 0.5, "{c:?}");
        // After the fade the first one is gone from the mesh.
        marks.build(23.0, &mut p, &mut n, &mut c, &mut i);
        assert_eq!(p.len(), 4);
        assert!(marks.fading(20.0) && !marks.fading(5.0));
    }

    #[test]
    fn surfaces_leave_the_right_marks() {
        use SurfaceKind::*;
        assert!(
            mark_color(Asphalt, 0.0, 0.0, 0.1, 20.0).is_none(),
            "gripping"
        );
        let rubber = mark_color(Asphalt, 0.0, 0.0, 0.8, 20.0).expect("sliding");
        assert!(rubber[0] < 0.05 && rubber[3] > 0.4);
        let wet = mark_color(Asphalt, 1.0, 0.0, 0.8, 20.0).expect("sliding");
        assert!(wet[3] < rubber[3], "wet roads show fainter marks");
        assert!(mark_color(Water, 0.0, 0.0, 1.0, 20.0).is_none());
        assert!(
            mark_color(Grass, 0.0, 0.0, 0.3, 20.0).is_some(),
            "soft ground marks earlier"
        );
        assert!(
            mark_color(Grass, 0.0, 0.0, 0.3, 1.0).is_none(),
            "standing still"
        );
        let snow = mark_color(Asphalt, 0.0, 1.0, 0.0, 10.0).expect("tracks in snow");
        assert!(snow[0] > 0.2, "grey tracks, not black rubber");
    }
}
