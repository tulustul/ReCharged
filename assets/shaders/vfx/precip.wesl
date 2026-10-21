// rc_vfx precipitation (crates/rc_vfx/src/weather.rs): one mesh of quads per game camera, every
// quad a raindrop streak or a snowflake animated here. Drops live in world space and wrap inside
// a box around the viewing camera, so the camera moves through still-falling rain instead of
// dragging it along; each camera's mesh sits on that camera's own render layer.
//   vertex: position = random seed in [0,1)^3, uv = corner (x −0.5..0.5, y 0..1),
//           uv_b = (rank 0..1: drops above `amount` collapse, size variation 0..1)
// Drops fade where they meet opaque geometry (soft particles) when the view has a depth prepass.

import bevy_pbr::render::{
    mesh_view_bindings::view,
    view_transformations::depth_ndc_to_view_z,
};
import bevy_pbr::prepass::utils as prepass_utils;

// A drop fades out over this distance (u) in front of the scene behind it.
const SOFT_DEPTH: f32 = 0.25;

// xyz: fall velocity (u/s, wind included), w: precipitation clock (s; stops while paused)
@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> motion: vec4<f32>;
// x: box width (u), y: box height (u), z: streak length (u), w: streak width / flake size (u)
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var<uniform> shape: vec4<f32>;
// rgb: colour (linear, lighting folded in), a: opacity
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var<uniform> color: vec4<f32>;
// x: amount 0..1, y: snow (1) or rain (0), z: flake wobble (u)
@group(constants::MATERIAL_BIND_GROUP) @binding(3) var<uniform> params: vec4<f32>;

struct Vertex {
    @location(0) seed: vec3<f32>,
    @location(1) corner: vec2<f32>,
    @location(2) extra: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) corner: vec2<f32>,
    @location(1) fade: f32,
};

@vertex
fn vertex(v: Vertex) -> VertexOutput {
    let size = vec3<f32>(shape.x, shape.y, shape.x);
    let eye = view.world_position.xyz;
    // The box sits a little above the eye: drops under the ground are wasted.
    let centre = eye + vec3<f32>(0.0, shape.y * 0.2, 0.0);
    let t = motion.w;
    let snow = params.y > 0.5;
    var p = v.seed * size + motion.xyz * t * (0.8 + 0.4 * v.extra.y);
    if snow {
        let w = params.z;
        p += vec3<f32>(
            sin(t * 1.3 + v.seed.x * 40.0) * w,
            0.0,
            cos(t * 1.1 + v.seed.z * 40.0) * w,
        );
    }
    let rel = fract((p - centre) / size + 0.5) - 0.5;
    let drop = centre + rel * size;

    // Fade at the box edges (the wrap) and very close to the eye (huge, blurry drops).
    let edge = (1.0 - smoothstep(0.32, 0.5, length(rel.xz))) * (1.0 - smoothstep(0.38, 0.5, abs(rel.y)));
    let near = smoothstep(0.6, 1.8, distance(drop, eye));
    var fade = edge * near;
    if v.extra.x > params.x {
        fade = 0.0;
    }

    var world: vec3<f32>;
    if snow {
        let right = view.world_from_view[0].xyz;
        let up = view.world_from_view[1].xyz;
        let s = shape.w * (0.6 + 0.8 * v.extra.y);
        world = drop + (right * v.corner.x * 2.0 + up * (v.corner.y - 0.5) * 2.0) * s;
    } else {
        let dir = normalize(motion.xyz);
        var side = cross(dir, eye - drop);
        if dot(side, side) < 1e-8 {
            side = view.world_from_view[0].xyz;
        }
        side = normalize(side);
        world = drop + side * v.corner.x * shape.w - dir * v.corner.y * shape.z * (0.7 + 0.6 * v.extra.y);
    }
    if fade <= 0.0 {
        world = drop;
    }

    var out: VertexOutput;
    out.clip = view.clip_from_world * vec4<f32>(world, 1.0);
    out.corner = v.corner;
    out.fade = fade;
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    var a: f32;
    if params.y > 0.5 {
        let q = vec2<f32>(in.corner.x * 2.0, in.corner.y * 2.0 - 1.0);
        a = 1.0 - smoothstep(0.35, 1.0, length(q));
    } else {
        let across = 1.0 - abs(in.corner.x) * 2.0;
        // Brightest at the head, thinning to the tail.
        a = across * (1.0 - in.corner.y * 0.8);
    }
    a = a * color.a * in.fade;
    @if(DEPTH_PREPASS)
    {
        let scene = depth_ndc_to_view_z(prepass_utils::prepass_depth(in.clip, 0u));
        a *= clamp((depth_ndc_to_view_z(in.clip.z) - scene) / SOFT_DEPTH, 0.0, 1.0);
    }
    return vec4<f32>(color.rgb * a, a);
}
