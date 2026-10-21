// The light baker's probe pass on the GPU (`bake/gpu.rs`): the same maths as `Scene::probe` in
// `bake.rs`, one workgroup per probe, its threads sharing the rays. Plain WGSL: wgpu compiles it
// directly in the editor, outside Bevy's shader pipeline.

struct Params {
    grid_min: vec3f,
    rays: u32,
    grid_max: vec3f,
    first_probe: u32,
    dims: vec3u,
    // 1: pass 2, `grid` holds pass 1's volume (one more bounce).
    bounce: u32,
    sky_at: u32,
    dirs_at: u32,
    suns_at: u32,
    sun_count: u32,
    lamps_at: u32,
    lamp_count: u32,
}

@group(0) @binding(0) var<uniform> params: Params;
// BVH: 2 per node, (min, a) and (max, count) — a leaf (count > 0) holds triangles a..a+count,
// an inner node has children a and a + 1.
// Integers in `u32` lanes, positions as their bits (an integer in a float lane is a denormal,
// which a GPU may flush to 0).
@group(0) @binding(1) var<storage, read> nodes: array<vec4u>;
// 3 per triangle: (v0, material), (v1 − v0, flags: bit 0 casts shadows), (v2 − v0, 0).
@group(0) @binding(2) var<storage, read> tris: array<vec4u>;
// Materials from 0 (albedo, double-sided), then the sky table, ray directions, suns (2 each:
// toward + shadows, irradiance + in volume) and lamps (4 each: pos + 1/range², candela + shadows,
// spot direction + is spot, (scale, offset)).
@group(0) @binding(3) var<storage, read> data: array<vec4f>;
// Pass 1's volume, 6 faces per probe.
@group(0) @binding(4) var<storage, read> grid: array<vec4f>;
// 6 faces per probe of this dispatch; face 0's w counts rays that met a single-sided back face.
@group(0) @binding(5) var<storage, read_write> out: array<vec4f>;

const PI: f32 = 3.14159265;
const EPSILON_U: f32 = 0.02;
// `RAY_START_U` in bake.rs.
const RAY_START_U: f32 = 1e-3;
const FAR_U: f32 = 2000.0;
const SKY_W: u32 = 64u;
const SKY_H: u32 = 32u;
const THREADS: u32 = 64u;
const STACK: u32 = 64u;
const NONE: u32 = 0xffffffffu;

/// The position in a `u32` row.
fn vec(v: vec4u) -> vec3f {
    return bitcast<vec3f>(v.xyz);
}

struct Hit {
    t: f32,
    tri: u32,
}

/// Entry distance of a ray into a box, or −1.
fn box_entry(o: vec3f, inv: vec3f, lo: vec3f, hi: vec3f, reach: f32) -> f32 {
    let t0 = (lo - o) * inv;
    let t1 = (hi - o) * inv;
    let near = max(max(min(t0.x, t1.x), min(t0.y, t1.y)), max(min(t0.z, t1.z), 0.0));
    let far = min(min(max(t0.x, t1.x), max(t0.y, t1.y)), min(max(t0.z, t1.z), reach));
    return select(-1.0, near, near <= far);
}

/// Distance along the ray to a triangle (either side), or −1.
fn triangle(o: vec3f, d: vec3f, v0: vec3f, e1: vec3f, e2: vec3f) -> f32 {
    let p = cross(d, e2);
    let det = dot(e1, p);
    if (det == 0.0) {
        return -1.0;
    }
    let inv = 1.0 / det;
    let s = o - v0;
    let u = dot(s, p) * inv;
    if (u < 0.0 || u > 1.0) {
        return -1.0;
    }
    let q = cross(s, e1);
    let v = dot(d, q) * inv;
    if (v < 0.0 || u + v > 1.0) {
        return -1.0;
    }
    return dot(e2, q) * inv;
}

/// The nearest triangle within `reach`; with `shadow`, the first one that casts shadows.
fn trace(o: vec3f, d: vec3f, reach: f32, shadow: bool) -> Hit {
    let inv = 1.0 / select(d, vec3f(1e-9), abs(d) < vec3f(1e-9));
    var best = Hit(reach, NONE);
    var stack: array<u32, STACK>;
    var sp = 0u;
    var node = 0u;
    loop {
        let first = nodes[node * 2u].w;
        let count = nodes[node * 2u + 1u].w;
        if (count > 0u) {
            for (var i = first; i < first + count; i++) {
                let t1 = tris[i * 3u + 1u];
                if (shadow && (t1.w & 1u) == 0u) {
                    continue;
                }
                let t = triangle(o, d, vec(tris[i * 3u]), vec(t1), vec(tris[i * 3u + 2u]));
                if (t >= 0.0 && t <= best.t) {
                    best = Hit(t, i);
                    if (shadow) {
                        return best;
                    }
                }
            }
        } else {
            let l = first;
            let r = first + 1u;
            let tl = box_entry(o, inv, vec(nodes[l * 2u]), vec(nodes[l * 2u + 1u]), best.t);
            let tr = box_entry(o, inv, vec(nodes[r * 2u]), vec(nodes[r * 2u + 1u]), best.t);
            if (tl >= 0.0 && tr >= 0.0) {
                let left_first = tl <= tr;
                if (sp < STACK) {
                    stack[sp] = select(l, r, left_first);
                    sp++;
                }
                node = select(r, l, left_first);
                continue;
            }
            if (tl >= 0.0) {
                node = l;
                continue;
            }
            if (tr >= 0.0) {
                node = r;
                continue;
            }
        }
        if (sp == 0u) {
            break;
        }
        sp--;
        node = stack[sp];
    }
    return best;
}

fn blocked(o: vec3f, d: vec3f, reach: f32) -> bool {
    return trace(o, d, reach, true).tri != NONE;
}

fn sky(d: vec3f) -> vec3f {
    var lon = atan2(d.z, d.x);
    if (lon < 0.0) {
        lon += 2.0 * PI;
    }
    let lat = asin(clamp(d.y, -1.0, 1.0));
    let u = min(u32(lon / (2.0 * PI) * f32(SKY_W)), SKY_W - 1u);
    let v = min(u32((lat / PI + 0.5) * f32(SKY_H)), SKY_H - 1u);
    return data[params.sky_at + u + v * SKY_W].xyz;
}

/// Irradiance from the lights on a surface at `p` facing `n` (`Scene::direct`).
fn direct(p: vec3f, n: vec3f) -> vec3f {
    let start = p + n * EPSILON_U;
    var e = vec3f(0.0);
    for (var i = 0u; i < params.sun_count; i++) {
        let toward = data[params.suns_at + i * 2u];
        let c = dot(n, toward.xyz);
        if (c > 0.0 && !(toward.w > 0.5 && blocked(start, toward.xyz, FAR_U))) {
            e += data[params.suns_at + i * 2u + 1u].xyz * c;
        }
    }
    for (var i = 0u; i < params.lamp_count; i++) {
        let at = params.lamps_at + i * 4u;
        let pos = data[at];
        let d = pos.xyz - p;
        let dist2 = max(dot(d, d), 1e-4);
        let k = clamp(1.0 - (dist2 * pos.w) * (dist2 * pos.w), 0.0, 1.0);
        let falloff = k * k;
        let l = d / sqrt(dist2);
        let c = dot(n, l);
        if (falloff <= 0.0 || c <= 0.0) {
            continue;
        }
        var cone = 1.0;
        let spot = data[at + 2u];
        if (spot.w > 0.5) {
            let shape = data[at + 3u];
            let s = clamp(dot(-l, spot.xyz) * shape.x + shape.y, 0.0, 1.0);
            cone = s * s;
        }
        if (cone <= 0.0) {
            continue;
        }
        let candela = data[at + 1u];
        if (candela.w > 0.5 && blocked(start, l, sqrt(dist2) - 2.0 * EPSILON_U)) {
            continue;
        }
        e += candela.xyz * (c * falloff * cone / dist2);
    }
    return e;
}

/// Pass 1's volume at `p` facing `n`, as Bevy samples it (`bake::sample`).
fn bounce(p: vec3f, n: vec3f) -> vec3f {
    let dims = params.dims;
    let cell = (params.grid_max - params.grid_min) / vec3f(dims);
    let f = clamp((p - params.grid_min) / cell - 0.5, vec3f(0.0), vec3f(dims - 1u));
    let base = min(vec3u(floor(f)), dims - 1u);
    let t = f - vec3f(base);
    let nn = n * n;
    let faces = vec3u(select(1u, 0u, n.x >= 0.0), select(3u, 2u, n.y >= 0.0), select(5u, 4u, n.z >= 0.0));
    var sum = vec3f(0.0);
    for (var c = 0u; c < 8u; c++) {
        let o = vec3u(c & 1u, (c >> 1u) & 1u, (c >> 2u) & 1u);
        let q = min(base + o, dims - 1u);
        let w = select(1.0 - t, t, o == vec3u(1u));
        let at = (q.x + dims.x * (q.y + dims.y * q.z)) * 6u;
        let v = grid[at + faces.x].xyz * nn.x + grid[at + faces.y].xyz * nn.y + grid[at + faces.z].xyz * nn.z;
        sum += v * (w.x * w.y * w.z);
    }
    return sum;
}

fn face(i: u32) -> vec3f {
    var faces = array<vec3f, 6>(
        vec3f(1.0, 0.0, 0.0), vec3f(-1.0, 0.0, 0.0),
        vec3f(0.0, 1.0, 0.0), vec3f(0.0, -1.0, 0.0),
        vec3f(0.0, 0.0, 1.0), vec3f(0.0, 0.0, -1.0),
    );
    return faces[i];
}

var<workgroup> sums: array<array<vec3f, 6>, THREADS>;
var<workgroup> backs: array<u32, THREADS>;

@compute @workgroup_size(64)
fn main(@builtin(workgroup_id) group: vec3u, @builtin(local_invocation_index) lane: u32) {
    let dims = params.dims;
    let probe = params.first_probe + group.x;
    let cell = (params.grid_max - params.grid_min) / vec3f(dims);
    let index = vec3u(probe % dims.x, (probe / dims.x) % dims.y, probe / (dims.x * dims.y));
    let p = params.grid_min + (vec3f(index) + 0.5) * cell;

    var sum: array<vec3f, 6>;
    var back = 0u;
    for (var r = lane; r < params.rays; r += THREADS) {
        let dir = data[params.dirs_at + r].xyz;
        let origin = p + dir * RAY_START_U;
        let hit = trace(origin, dir, FAR_U, false);
        var radiance: vec3f;
        if (hit.tri == NONE) {
            radiance = sky(dir);
        } else {
            let material = data[tris[hit.tri * 3u].w];
            var n = normalize(cross(vec(tris[hit.tri * 3u + 1u]), vec(tris[hit.tri * 3u + 2u])));
            if (dot(n, dir) > 0.0) {
                n = -n;
                if (material.w < 0.5) {
                    back++;
                }
            }
            let at = origin + dir * hit.t;
            radiance = material.xyz * direct(at, n) / PI;
            if (params.bounce != 0u) {
                radiance += material.xyz * bounce(at + n * EPSILON_U, n);
            }
        }
        for (var f = 0u; f < 6u; f++) {
            sum[f] += radiance * max(dot(face(f), dir), 0.0);
        }
    }
    sums[lane] = sum;
    backs[lane] = back;
    workgroupBarrier();

    if (lane < 6u) {
        var total = vec3f(0.0);
        var behind = 0u;
        for (var i = 0u; i < THREADS; i++) {
            total += sums[i][lane];
            behind += backs[i];
        }
        // ∫ L cos dω / π ≈ Σ L cos × (4π / N) / π.
        var cube = total * (4.0 / f32(params.rays));
        // The fill's own light, where the probe sees it.
        let normal = face(lane);
        for (var i = 0u; i < params.sun_count; i++) {
            let toward = data[params.suns_at + i * 2u].xyz;
            let light = data[params.suns_at + i * 2u + 1u];
            if (light.w > 0.5 && !blocked(p + toward * RAY_START_U, toward, FAR_U)) {
                cube += light.xyz * (max(dot(normal, toward), 0.0) / PI);
            }
        }
        out[group.x * 6u + lane] = vec4f(cube, select(0.0, f32(behind), lane == 0u));
    }
}
