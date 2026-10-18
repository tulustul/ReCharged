// rc_vfx particles and sprites (crates/rc_vfx/src/particles.rs). One quad mesh, one material;
// each instance's look comes from its transform and its MeshTag:
//   transform  translation = centre; mode 0: X axis = size × (cos spin, sin spin, 0);
//              mode 1: Y axis = stretch direction × length, |X| = width; mode 2: the quad's plane
//   tag        bits 0-15 RGB565 (gamma 2), 16-23 opacity (lit) or intensity (emissive),
//              24-27 shape, 28-29 mode (0 billboard, 1 axial, 2 flat), 30 emissive
// Emissive particles are added (premultiplied output with alpha 0); lit ones are shaded by the
// scene's PBR lighting and blended. Shapes are procedural (no textures, no atlas bleeding).

import bevy_pbr::render::{
    mesh_functions::{get_world_from_local, get_tag},
    mesh_view_bindings::view,
    view_transformations::position_world_to_clip,
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    pbr_types::STANDARD_MATERIAL_FLAGS_FOG_ENABLED_BIT,
    forward_io::{VertexOutput, FragmentOutput},
};

// x: intensity of a full level byte.
@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> particle_params: vec4<f32>;

struct ParticleVertex {
    @builtin(instance_index) instance_index: u32,
    @location(2) uv: vec2<f32>,
};

@vertex
fn vertex(v: ParticleVertex) -> VertexOutput {
    let m = get_world_from_local(v.instance_index);
    let tag = get_tag(v.instance_index);
    let mode = (tag >> 28u) & 3u;
    let c = v.uv * 2.0 - 1.0;
    let centre = m[3].xyz;
    let ax = m[0].xyz;
    let ay = m[1].xyz;
    let to_cam = normalize(view.world_position.xyz - centre);
    let right = view.world_from_view[0].xyz;
    let up = view.world_from_view[1].xyz;

    var world: vec3<f32>;
    var normal: vec3<f32>;
    if mode == 0u {
        // Billboard, rotated in the view plane by the spin encoded in the X axis.
        let size = length(ax);
        let cs = ax.xy / max(size, 1e-6);
        let q = vec2<f32>(c.x * cs.x - c.y * cs.y, c.x * cs.y + c.y * cs.x);
        world = centre + (right * q.x + up * q.y) * (0.5 * size);
        normal = to_cam;
    } else if mode == 1u {
        // Axial billboard: long along Y, turned around it to face the camera.
        let len = length(ay);
        let dir = ay / max(len, 1e-6);
        var side = cross(dir, to_cam);
        if dot(side, side) < 1e-8 {
            side = right;
        }
        side = normalize(side);
        world = centre + dir * (c.y * 0.5 * len) + side * (c.x * 0.5 * length(ax));
        normal = to_cam;
    } else {
        // Flat quad in the transform's XY plane.
        world = (m * vec4<f32>(c.x * 0.5, c.y * 0.5, 0.0, 1.0)).xyz;
        normal = normalize(m[2].xyz);
    }

    var out: VertexOutput;
    out.world_position = vec4<f32>(world, 1.0);
    out.position = position_world_to_clip(world);
    out.world_normal = normal;
@if(VERTEX_UVS_A)
    out.uv = c;
@if(VERTEX_OUTPUT_INSTANCE_INDEX)
    out.instance_index = v.instance_index;
    return out;
}

fn hash21(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn value_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash21(i);
    let b = hash21(i + vec2<f32>(1.0, 0.0));
    let c = hash21(i + vec2<f32>(0.0, 1.0));
    let d = hash21(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// Signed distance to the lightning-bolt icon (a 7-gon, inside < 0).
fn sd_bolt(p: vec2<f32>) -> f32 {
    var v = array<vec2<f32>, 7>(
        vec2<f32>(0.16, 0.92),
        vec2<f32>(-0.44, 0.04),
        vec2<f32>(-0.10, 0.04),
        vec2<f32>(-0.34, -0.92),
        vec2<f32>(0.34, 0.12),
        vec2<f32>(-0.01, 0.12),
        vec2<f32>(0.46, 0.92),
    );
    var d = dot(p - v[0], p - v[0]);
    var s = 1.0;
    var j = 6;
    for (var i = 0; i < 7; i++) {
        let e = v[j] - v[i];
        let w = p - v[i];
        let b = w - e * clamp(dot(w, e) / dot(e, e), 0.0, 1.0);
        d = min(d, dot(b, b));
        let c = vec3<bool>((p.y >= v[i].y), (p.y < v[j].y), (e.x * w.y > e.y * w.x));
        if all(c) || all(!c) {
            s = -s;
        }
        j = i;
    }
    return s * sqrt(d);
}

// Opacity of a shape at quad coordinate p (−1..1). May exceed 1 for hot cores.
fn shape_alpha(shape: u32, p: vec2<f32>) -> f32 {
    let r = length(p);
    let edge = 1.0 - smoothstep(0.8, 1.0, r);
    switch shape {
        case 0u: {
            return exp(-4.5 * r * r) * edge;
        }
        case 1u: {
            return (exp(-18.0 * r * r) * 1.6 + exp(-3.5 * r * r) * 0.45) * edge;
        }
        case 2u: {
            let across = 1.0 - smoothstep(0.0, 1.0, abs(p.x));
            let along = 1.0 - smoothstep(0.4, 1.0, abs(p.y));
            return across * across * along;
        }
        case 3u: {
            let n = value_noise(p * 2.1 + vec2<f32>(7.3, 2.9)) * 0.65
                + value_noise(p * 4.7 + vec2<f32>(1.7, 9.1)) * 0.35;
            let d = r + (n - 0.5) * 0.6;
            return (1.0 - smoothstep(0.2, 0.95, d)) * (1.0 - smoothstep(0.9, 1.0, r));
        }
        case 4u: {
            return 1.0 - smoothstep(0.0, 0.13, abs(r - 0.8));
        }
        case 5u: {
            let rays = exp(-abs(p.x) * 16.0) * exp(-abs(p.y) * 2.4)
                + exp(-abs(p.y) * 16.0) * exp(-abs(p.x) * 2.4);
            return min(1.4, rays + exp(-14.0 * r * r) * 1.2) * edge;
        }
        case 6u: {
            let a = atan2(p.y, p.x);
            let lim = 0.74 + 0.14 * sin(a * 3.0 + 1.3) + 0.08 * sin(a * 5.0 + 0.4);
            return 1.0 - smoothstep(lim - 0.08, lim, r);
        }
        case 7u: {
            let d = max(abs(p.x), abs(p.y));
            return 1.0 - smoothstep(0.9, 1.0, d);
        }
        case 8u: {
            let q = vec2<f32>(p.x * (1.25 + 0.6 * max(p.y, 0.0)), p.y);
            return 1.0 - smoothstep(0.55, 0.95, length(q));
        }
        case 9u: {
            let d = sd_bolt(p * 1.05);
            return (1.0 - smoothstep(-0.015, 0.015, d)) + exp(-max(d, 0.0) * 14.0) * 0.35;
        }
        case 10u: {
            let a = atan2(p.y, p.x);
            let arcs = (1.0 - smoothstep(0.0, 0.06, abs(r - 0.78)))
                * (1.0 - smoothstep(0.42, 0.62, abs(sin(2.0 * a))));
            let ticks = (1.0 - smoothstep(0.03, 0.06, min(abs(p.x), abs(p.y))))
                * step(0.42, r) * step(r, 0.64);
            return max(max(arcs, ticks), exp(-60.0 * r * r));
        }
        default: {
            return 0.0;
        }
    }
}

fn unpack_rgb(tag: u32) -> vec3<f32> {
    let c = vec3<f32>(
        f32(tag & 31u) / 31.0,
        f32((tag >> 5u) & 63u) / 63.0,
        f32((tag >> 11u) & 31u) / 31.0,
    );
    return c * c;
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    let tag = get_tag(in.instance_index);
    let shape = (tag >> 24u) & 15u;
    let mode = (tag >> 28u) & 3u;
    let emissive = ((tag >> 30u) & 1u) == 1u;
    let level = f32((tag >> 16u) & 255u) / 255.0;
    let rgb = unpack_rgb(tag);
    let p = in.uv;
    let a_shape = shape_alpha(shape, p);
    if a_shape < 0.004 {
        discard;
    }

    var pbr_input = pbr_input_from_standard_material(in, is_front);
    var out: FragmentOutput;
    if emissive {
        // Lights in fog stay visible: no fog on glows, only tonemapping when not HDR.
        pbr_input.material.flags = pbr_input.material.flags & ~STANDARD_MATERIAL_FLAGS_FOG_ENABLED_BIT;
        let c = main_pass_post_lighting_processing(
            pbr_input,
            vec4<f32>(rgb * level * particle_params.x, 1.0),
        );
        out.color = vec4<f32>(c.rgb * a_shape, 0.0);
    } else {
        if mode == 0u {
            // Billboards shade like little spheres, so puffs read as volumes.
            let right = view.world_from_view[0].xyz;
            let up = view.world_from_view[1].xyz;
            let to_cam = normalize(view.world_position.xyz - in.world_position.xyz);
            let r2 = min(dot(p, p), 1.0);
            let n = normalize(right * p.x + up * p.y + to_cam * sqrt(1.0 - r2) * 1.2);
            pbr_input.N = n;
            pbr_input.world_normal = n;
        }
        pbr_input.material.base_color = vec4<f32>(rgb, 1.0);
        var c = apply_pbr_lighting(pbr_input);
        c = main_pass_post_lighting_processing(pbr_input, c);
        let a = clamp(level * a_shape, 0.0, 1.0);
        out.color = vec4<f32>(c.rgb * a, a);
    }
    return out;
}
