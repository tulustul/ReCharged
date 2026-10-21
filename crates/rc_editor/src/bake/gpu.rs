//! The probe passes on the GPU: the same tracing as [`Scene::probe`] in a compute shader
//! (`probe.wgsl`), over a BVH built here. Runs on the editor's own wgpu device (Bevy's
//! `RenderDevice`, from the bake thread) in chunks of about [`CHUNK_TARGET`], so the window keeps
//! drawing and no GPU watchdog takes a long dispatch for a hang.
//!
//! No error scopes: in Bevy's wgpu build (no `std` feature) they are global to the device, so one
//! would swallow the renderer's errors (and its own scopes'). Bevy treats an uncaptured error as
//! fatal, so everything that could fail is checked first: the shader with naga, every buffer
//! against the device's limits. A failed check returns `Err` and the CPU bakes instead.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use bevy::prelude::*;
use rc_render::IrradianceGrid;
use wgpu::util::DeviceExt;

use super::{GridSpec, INSIDE_SHARE, Scene, bake_with, directions};

/// Wall time aimed at per dispatch.
const CHUNK_TARGET: Duration = Duration::from_millis(60);
/// Probes in the first dispatch (later ones scale to [`CHUNK_TARGET`]), and the most per dispatch.
const FIRST_CHUNK: u32 = 64;
const MAX_CHUNK: u32 = 16_384;
/// Triangles per BVH leaf at most (unless they cannot be split), and the tree's depth limit
/// (the shader's traversal stack holds 64).
const LEAF_TRIANGLES: usize = 4;
const MAX_DEPTH: u32 = 60;
/// BVH boxes grow this much on every side (u): a ray along a box's face plane (an axis-aligned
/// direction, a probe on round coordinates) would otherwise skip the box, and miss what is in it.
const BOX_PAD_U: f32 = 1e-3;

/// [`super::bake`] on the GPU.
pub fn bake_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    scene: &Scene,
    spec: &GridSpec,
    rays: usize,
    progress: &AtomicUsize,
) -> Result<IrradianceGrid, String> {
    let tracer = Tracer::new(device, queue, scene, rays, spec.count())?;
    bake_with(spec, |grid, bounce| tracer.pass(grid, bounce, progress))
}

/// The probe shader, validated (naga's errors would be a fatal rendering error in Bevy).
fn shader() -> Result<&'static str, String> {
    use wgpu::naga::{front::wgsl, valid};
    let source = include_str!("probe.wgsl");
    let module = wgsl::parse_str(source).map_err(|e| format!("GPU bake: {e}"))?;
    valid::Validator::new(valid::ValidationFlags::all(), valid::Capabilities::empty())
        .validate(&module)
        .map_err(|e| format!("GPU bake: {e:?}"))?;
    Ok(source)
}

/// A node of the BVH: a leaf (`count > 0`) holds triangles `a..a + count` of the reordered list,
/// an inner node has children `a` and `a + 1`.
#[derive(Debug, Clone, Copy)]
struct Node {
    lo: Vec3,
    hi: Vec3,
    a: u32,
    count: u32,
}

/// A bounding volume hierarchy over `tris` (binned surface-area heuristic): its nodes, root
/// first, and the triangle order its leaves index.
fn bvh(tris: &[[Vec3; 3]]) -> (Vec<Node>, Vec<u32>) {
    const BINS: usize = 16;
    let boxes: Vec<(Vec3, Vec3)> = tris
        .iter()
        .map(|[a, b, c]| (a.min(*b).min(*c), a.max(*b).max(*c)))
        .collect();
    let centres: Vec<Vec3> = boxes.iter().map(|(lo, hi)| (*lo + *hi) / 2.0).collect();
    let mut order: Vec<u32> = (0..tris.len() as u32).collect();
    let empty = (Vec3::INFINITY, Vec3::NEG_INFINITY);
    let union = |(l, h): (Vec3, Vec3), (l2, h2): (Vec3, Vec3)| (l.min(l2), h.max(h2));
    let area = |(l, h): (Vec3, Vec3)| {
        let d = (h - l).max(Vec3::ZERO);
        d.x * d.y + d.y * d.z + d.z * d.x
    };
    let mut nodes = vec![Node {
        lo: Vec3::ZERO,
        hi: Vec3::ZERO,
        a: 0,
        count: 0,
    }];
    let mut todo = vec![(0usize, 0usize, tris.len(), 0u32)];
    while let Some((index, start, end, depth)) = todo.pop() {
        let items = &mut order[start..end];
        let (lo, hi) = items
            .iter()
            .fold(empty, |b, &t| union(b, boxes[t as usize]));
        nodes[index] = Node {
            lo,
            hi,
            a: start as u32,
            count: (end - start) as u32,
        };
        let (clo, chi) = items.iter().fold(empty, |(l, h), &t| {
            let c = centres[t as usize];
            (l.min(c), h.max(c))
        });
        let extent = chi - clo;
        let axis = extent.max_position();
        if items.len() <= LEAF_TRIANGLES || depth >= MAX_DEPTH || extent[axis] <= 1e-6 {
            continue;
        }
        let scale = BINS as f32 / extent[axis];
        let bin =
            |t: u32| (((centres[t as usize][axis] - clo[axis]) * scale) as usize).min(BINS - 1);
        let mut bins = [(empty, 0usize); BINS];
        for &t in items.iter() {
            let b = &mut bins[bin(t)];
            *b = (union(b.0, boxes[t as usize]), b.1 + 1);
        }
        // Cost of splitting after bin i: left area × count + right area × count.
        let mut left = [(0.0f32, 0usize); BINS];
        let mut acc = (empty, 0usize);
        for i in 0..BINS - 1 {
            acc = (union(acc.0, bins[i].0), acc.1 + bins[i].1);
            left[i] = (area(acc.0), acc.1);
        }
        let (mut best, mut best_cost) = (0, f32::INFINITY);
        let mut acc = (empty, 0usize);
        for i in (0..BINS - 1).rev() {
            acc = (union(acc.0, bins[i + 1].0), acc.1 + bins[i + 1].1);
            let cost = left[i].0 * left[i].1 as f32 + area(acc.0) * acc.1 as f32;
            if left[i].1 > 0 && acc.1 > 0 && cost < best_cost {
                (best, best_cost) = (i, cost);
            }
        }
        let mut mid = 0;
        if best_cost.is_finite() {
            for i in 0..items.len() {
                if bin(items[i]) <= best {
                    items.swap(i, mid);
                    mid += 1;
                }
            }
        } else {
            // Every centre in one bin: split at the median.
            mid = items.len() / 2;
            items.select_nth_unstable_by(mid, |&a, &b| {
                centres[a as usize][axis].total_cmp(&centres[b as usize][axis])
            });
        }
        let child = nodes.len();
        nodes[index].a = child as u32;
        nodes[index].count = 0;
        nodes.extend([nodes[index]; 2]);
        todo.push((child, start, start + mid, depth + 1));
        todo.push((child + 1, start + mid, end, depth + 1));
    }
    (nodes, order)
}

/// Everything uploaded once per bake.
struct Tracer<'a> {
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    params: wgpu::Buffer,
    nodes: wgpu::Buffer,
    tris: wgpu::Buffer,
    data: wgpu::Buffer,
    out: wgpu::Buffer,
    read: wgpu::Buffer,
    /// Word offsets of the parameters that do not change per dispatch.
    layout: [u32; 6],
    rays: usize,
}

fn bytes<T: Copy, const N: usize>(v: &[[T; N]], word: impl Fn(T) -> u32) -> Vec<u8> {
    v.iter()
        .flatten()
        .flat_map(|&x| word(x).to_le_bytes())
        .collect()
}

impl<'a> Tracer<'a> {
    fn new(
        device: &'a wgpu::Device,
        queue: &'a wgpu::Queue,
        scene: &Scene,
        rays: usize,
        probes: usize,
    ) -> Result<Self, String> {
        let vertices = scene.mesh.vertices();
        let tris: Vec<[Vec3; 3]> = scene
            .mesh
            .indices()
            .iter()
            .map(|t| t.map(|i| vertices[i as usize]))
            .collect();
        let (nodes, order) = bvh(&tris);
        let row = |v: Vec3, w: u32| {
            let [x, y, z] = v.to_array().map(f32::to_bits);
            [x, y, z, w]
        };
        let node_words: Vec<[u32; 4]> = nodes
            .iter()
            .flat_map(|n| [row(n.lo - BOX_PAD_U, n.a), row(n.hi + BOX_PAD_U, n.count)])
            .collect();
        let tri_words: Vec<[u32; 4]> = order
            .iter()
            .flat_map(|&t| {
                let [a, b, c] = tris[t as usize];
                [
                    row(a, scene.material_of[t as usize]),
                    row(b - a, u32::from(scene.casts[t as usize])),
                    row(c - a, 0),
                ]
            })
            .collect();
        let flag = |b: bool| if b { 1.0 } else { 0.0 };
        let mut data: Vec<[f32; 4]> = scene
            .albedo
            .iter()
            .zip(&scene.double_sided)
            .map(|(a, &d)| a.extend(flag(d)).to_array())
            .collect();
        let sky_at = data.len() as u32;
        data.extend(scene.sky.iter().map(|s| s.extend(0.0).to_array()));
        let dirs_at = data.len() as u32;
        data.extend(directions(rays).iter().map(|d| d.extend(0.0).to_array()));
        let suns_at = data.len() as u32;
        for sun in &scene.suns {
            data.push(sun.toward.extend(flag(sun.shadows)).to_array());
            data.push(sun.irradiance.extend(flag(sun.in_volume)).to_array());
        }
        let lamps_at = data.len() as u32;
        for lamp in &scene.lamps {
            let (dir, scale, offset) = lamp.spot.unwrap_or((Vec3::ZERO, 0.0, 0.0));
            data.push(lamp.pos.extend(lamp.inverse_range_squared).to_array());
            data.push(lamp.candela.extend(flag(lamp.shadows)).to_array());
            data.push(dir.extend(flag(lamp.spot.is_some())).to_array());
            data.push([scale, offset, 0.0, 0.0]);
        }
        let out_size = u64::from(MAX_CHUNK) * 6 * 16;
        let limits = device.limits();
        let limit = limits
            .max_storage_buffer_binding_size
            .min(limits.max_buffer_size);
        let largest = [node_words.len(), tri_words.len(), data.len(), probes * 6]
            .into_iter()
            .max()
            .unwrap_or(0) as u64
            * 16;
        if largest.max(out_size) > limit {
            return Err(format!(
                "GPU bake: a {} MB buffer, the device binds {} MB at most",
                largest >> 20,
                limit >> 20
            ));
        }
        let storage = |label, contents: &[u8]| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents,
                usage: wgpu::BufferUsages::STORAGE,
            })
        };
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bake probes"),
            source: wgpu::ShaderSource::Wgsl(shader()?.into()),
        });
        Ok(Self {
            device,
            queue,
            pipeline: device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("bake probes"),
                layout: None,
                module: &module,
                entry_point: Some("main"),
                compilation_options: default(),
                cache: None,
            }),
            params: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("bake params"),
                size: 80,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            nodes: storage("bake bvh", &bytes(&node_words, |w| w)),
            tris: storage("bake triangles", &bytes(&tri_words, |w| w)),
            data: storage("bake data", &bytes(&data, f32::to_bits)),
            out: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("bake out"),
                size: out_size,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            }),
            read: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("bake read"),
                size: out_size,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            layout: [
                sky_at,
                dirs_at,
                suns_at,
                scene.suns.len() as u32,
                lamps_at,
                scene.lamps.len() as u32,
            ],
            rays,
        })
    }

    /// One pass over every probe (`bounce`: pass 1's volume); `None` for probes inside geometry.
    fn pass(
        &self,
        grid: &IrradianceGrid,
        bounce: Option<&IrradianceGrid>,
        progress: &AtomicUsize,
    ) -> Result<Vec<Option<[Vec3; 6]>>, String> {
        let cubes: Vec<[f32; 4]> = bounce.map_or(vec![[0.0; 4]], |g| {
            g.cubes
                .iter()
                .flatten()
                .map(|v| v.extend(0.0).to_array())
                .collect()
        });
        let volume = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("bake volume"),
                contents: &bytes(&cubes, f32::to_bits),
                usage: wgpu::BufferUsages::STORAGE,
            });
        let buffers = [
            &self.params,
            &self.nodes,
            &self.tris,
            &self.data,
            &volume,
            &self.out,
        ];
        let entries: Vec<wgpu::BindGroupEntry> = buffers
            .iter()
            .enumerate()
            .map(|(i, b)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: b.as_entire_binding(),
            })
            .collect();
        let group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bake"),
            layout: &self.pipeline.get_bind_group_layout(0),
            entries: &entries,
        });
        let count = grid.cubes.len() as u32;
        let mut cubes = Vec::with_capacity(count as usize);
        let (mut first, mut chunk) = (0u32, FIRST_CHUNK);
        while first < count {
            let n = chunk.min(count - first);
            let started = Instant::now();
            let floats = |v: Vec3| v.to_array().map(f32::to_bits);
            let mut params = Vec::with_capacity(20);
            params.extend(floats(grid.min));
            params.push(self.rays as u32);
            params.extend(floats(grid.max));
            params.push(first);
            params.extend(grid.dims.to_array());
            params.push(u32::from(bounce.is_some()));
            params.extend(self.layout);
            let params: Vec<u8> = params.iter().flat_map(|w| w.to_le_bytes()).collect();
            self.queue.write_buffer(&self.params, 0, &params);
            let mut encoder = self.device.create_command_encoder(&default());
            {
                let mut pass = encoder.begin_compute_pass(&default());
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &group, &[]);
                pass.dispatch_workgroups(n, 1, 1);
            }
            let size = u64::from(n) * 6 * 16;
            encoder.copy_buffer_to_buffer(&self.out, 0, &self.read, 0, size);
            self.queue.submit([encoder.finish()]);
            let (done, wait) = mpsc::channel();
            self.read
                .slice(..size)
                .map_async(wgpu::MapMode::Read, move |r| drop(done.send(r)));
            self.device
                .poll(wgpu::PollType::wait_indefinitely())
                .map_err(|e| format!("GPU bake: {e}"))?;
            wait.recv()
                .map_err(|e| e.to_string())?
                .map_err(|e| format!("GPU bake: {e}"))?;
            {
                let view = self
                    .read
                    .slice(..size)
                    .get_mapped_range()
                    .map_err(|e| format!("GPU bake: {e}"))?;
                let f: Vec<f32> = view
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|&b| f32::from_le_bytes(b))
                    .collect();
                for probe in f.as_chunks::<24>().0 {
                    let behind = probe[3];
                    let cube: [Vec3; 6] =
                        std::array::from_fn(|i| Vec3::from_slice(&probe[i * 4..i * 4 + 3]));
                    cubes.push((behind <= INSIDE_SHARE * self.rays as f32).then_some(cube));
                }
            }
            self.read.unmap();
            progress.fetch_add(n as usize, Ordering::Relaxed);
            first += n;
            let took = started.elapsed().as_secs_f32().max(1e-4);
            let scaled = n as f32 * CHUNK_TARGET.as_secs_f32() / took;
            chunk = (scaled as u32).clamp(FIRST_CHUNK, MAX_CHUNK.min(chunk * 4));
        }
        Ok(cubes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shader_validates() {
        shader().unwrap();
    }

    #[test]
    fn the_bvh_holds_every_triangle_once_inside_its_boxes() {
        let tris: Vec<[Vec3; 3]> = (0..500)
            .map(|i| {
                let p = Vec3::new((i % 10) as f32, (i / 10 % 5) as f32 * 3.0, (i / 50) as f32);
                [
                    p,
                    p + Vec3::X * 0.5,
                    p + Vec3::Z * 0.5 + Vec3::Y * (i % 3) as f32,
                ]
            })
            .collect();
        let (nodes, order) = bvh(&tris);
        let mut seen = vec![false; tris.len()];
        let mut todo = vec![0usize];
        while let Some(i) = todo.pop() {
            let n = nodes[i];
            if n.count > 0 {
                assert!(n.count as usize <= LEAF_TRIANGLES);
                for &t in &order[n.a as usize..(n.a + n.count) as usize] {
                    assert!(!seen[t as usize], "triangle {t} twice");
                    seen[t as usize] = true;
                    for v in tris[t as usize] {
                        assert!(v.cmpge(n.lo).all() && v.cmple(n.hi).all());
                    }
                }
            } else {
                for c in [n.a as usize, n.a as usize + 1] {
                    assert!(nodes[c].lo.cmpge(n.lo).all() && nodes[c].hi.cmple(n.hi).all());
                    todo.push(c);
                }
            }
        }
        assert!(seen.iter().all(|&s| s), "every triangle in a leaf");
    }

    /// A floor, a single-sided deck (probes inside it), a ceiling that casts no shadows, a sun,
    /// the fill, a shadowed spot, a point lamp and a graded sky.
    fn scene() -> Scene {
        use super::super::{Lamp, SceneBuilder, Sun};
        let mut b = SceneBuilder::default();
        let ground = b.material(Vec3::new(0.6, 0.5, 0.4), false);
        let deck = b.material(Vec3::new(0.2, 0.4, 0.8), false);
        let ceiling = b.material(Vec3::splat(0.8), true);
        let quad = |b: &mut SceneBuilder, p: Vec3, u: Vec3, v: Vec3, m, casts| {
            b.triangle([p, p + u, p + u + v], m, casts);
            b.triangle([p, p + u + v, p + v], m, casts);
        };
        quad(
            &mut b,
            Vec3::new(-20.0, 0.0, -20.0),
            Vec3::Z * 40.0,
            Vec3::X * 40.0,
            ground,
            true,
        );
        let (lo, hi) = (Vec3::new(-2.5, 1.0, -2.5), Vec3::new(2.5, 1.5, 2.5));
        let d = hi - lo;
        let (x, y, z) = (Vec3::X * d.x, Vec3::Y * d.y, Vec3::Z * d.z);
        for (p, u, v) in [
            (Vec3::new(lo.x, hi.y, lo.z), z, x),
            (lo, x, z),
            (Vec3::new(hi.x, lo.y, lo.z), y, z),
            (lo, z, y),
            (Vec3::new(lo.x, lo.y, hi.z), x, y),
            (lo, y, x),
        ] {
            quad(&mut b, p, u, v, deck, true);
        }
        quad(
            &mut b,
            Vec3::new(-8.0, 6.0, -8.0),
            Vec3::X * 10.0,
            Vec3::Z * 10.0,
            ceiling,
            false,
        );
        b.build(
            vec![
                Sun {
                    toward: Vec3::new(0.3, 1.0, 0.2).normalize(),
                    irradiance: Vec3::new(900.0, 850.0, 800.0),
                    shadows: true,
                    in_volume: false,
                },
                Sun {
                    toward: Vec3::new(-0.2, 1.0, -0.4).normalize(),
                    irradiance: Vec3::splat(120.0),
                    shadows: false,
                    in_volume: true,
                },
            ],
            vec![
                Lamp {
                    pos: Vec3::new(4.0, 4.0, 4.0),
                    candela: Vec3::splat(300.0),
                    inverse_range_squared: 1.0 / 400.0,
                    spot: Some((Vec3::NEG_Y, 2.0, -0.5)),
                    shadows: true,
                },
                Lamp {
                    pos: Vec3::new(-4.0, 2.0, 0.0),
                    candela: Vec3::new(80.0, 40.0, 10.0),
                    inverse_range_squared: 1.0 / 100.0,
                    spot: None,
                    shadows: false,
                },
            ],
            |dir| Vec3::new(5.0, 7.0, 12.0) * (0.5 + 0.5 * dir.y.max(0.0)),
        )
        .unwrap()
    }

    /// GPU and CPU volumes of `scene`, compared (same rays, same maths: only float rounding may
    /// differ); `None` without a GPU.
    fn compare(scene: &Scene, spec: &GridSpec) -> Option<f32> {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let device = bevy::tasks::block_on(instance.request_adapter(&default()))
            .ok()
            .and_then(|a| bevy::tasks::block_on(a.request_device(&default())).ok());
        let Some((device, queue)) = device else {
            eprintln!("no GPU: skipped");
            return None;
        };
        let progress = AtomicUsize::new(0);
        let gpu = bake_gpu(&device, &queue, scene, spec, 256, &progress).unwrap();
        assert_eq!(progress.load(Ordering::Relaxed), 2 * spec.count());
        let cpu = super::super::bake(scene, spec, 256, &AtomicUsize::new(0));
        let mut worst = 0.0f32;
        for (g, c) in gpu.cubes.iter().zip(&cpu.cubes) {
            for (g, c) in g.iter().zip(c) {
                worst = worst.max((*g - *c).abs().max_element() / (c.max_element() + 1.0));
            }
        }
        Some(worst)
    }

    #[test]
    fn the_gpu_bakes_what_the_cpu_bakes() {
        let spec = GridSpec::new(
            Vec3::new(-6.0, -1.0, -6.0),
            Vec3::new(6.0, 5.0, 6.0),
            2.0,
            1.5,
        );
        if let Some(worst) = compare(&scene(), &spec) {
            assert!(worst < 1e-3, "GPU and CPU differ by {:.3} %", worst * 100.0);
        }
    }

    /// Ray 0 points up with z = 0 exactly; from probes on z = 1 it runs along the face of the
    /// shelf's BVH box (the shelf ends at z = 1), which must not be skipped.
    #[test]
    fn a_ray_along_a_box_face_still_hits_what_is_inside() {
        use super::super::SceneBuilder;
        let mut b = SceneBuilder::default();
        let m = b.material(Vec3::splat(0.5), true);
        for (x, z) in [(-6.0, -4.0), (100.0, 0.0), (200.0, 0.0)] {
            let p = Vec3::new(x, 3.5, z);
            let (u, v) = (Vec3::X * 12.0, Vec3::Z * 5.0);
            b.triangle([p, p + u, p + u + v], m, true);
            b.triangle([p, p + u + v, p + v], m, true);
        }
        let scene = b
            .build(Vec::new(), Vec::new(), |_| Vec3::splat(10.0))
            .unwrap();
        let spec = GridSpec::new(
            Vec3::new(-2.0, 2.0, 0.0),
            Vec3::new(2.0, 3.0, 2.0),
            2.0,
            1.0,
        );
        assert_eq!(
            (spec.min.z + spec.max.z) / 2.0,
            1.0,
            "the probes sit on z = 1"
        );
        if let Some(worst) = compare(&scene, &spec) {
            assert!(worst < 1e-3, "GPU and CPU differ by {:.3} %", worst * 100.0);
        }
    }
}
