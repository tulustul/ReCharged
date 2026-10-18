// rc_render window glass (crates/rc_render/src/lighting/glass.rs): the outside light seen through
// an indoor room's window, with rain beads and streaks running down the glass and lightning.
// Unlit and display-referred (bypasses exposure, like emissives).

import bevy_pbr::render::{
    forward_io::VertexOutput,
    mesh_view_bindings::globals,
};

// rgb: outside light, a: rain amount.
@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> sky: vec4<f32>;
// x: lightning flash.
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var<uniform> extra: vec4<f32>;

fn hash11(x: f32) -> f32 {
    return fract(sin(x * 127.1) * 43758.5453);
}

fn hash21(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

// Static beads: one drop per cell, random size and position; returns coverage 0..1.
fn beads(q: vec2<f32>) -> f32 {
    let cell = floor(q);
    let f = fract(q) - 0.5;
    let h = hash21(cell);
    let centre = vec2<f32>(hash21(cell + 7.3), hash21(cell + 3.1)) - 0.5;
    let r = 0.05 + 0.18 * h * h;
    let d = length((f - centre * 0.6) * vec2<f32>(1.0, 0.8));
    return smoothstep(r, r * 0.6, d) * step(0.35, h);
}

// Running streaks: drops sliding down columns at random speeds, leaving a short trail.
fn streaks(q: vec2<f32>, t: f32) -> f32 {
    let col = floor(q.x);
    let h = hash11(col);
    if h < 0.55 {
        return 0.0;
    }
    let x = fract(q.x) - 0.5 + (hash11(col + 9.0) - 0.5) * 0.5;
    let speed = 0.25 + h * 0.6;
    let y = fract(q.y * 0.18 + t * speed + hash11(col + 4.2));
    let head = smoothstep(0.035, 0.0, length(vec2<f32>(x * 2.5, (y - 0.08) * 6.0)) - 0.02);
    let trail = smoothstep(0.04, 0.0, abs(x)) * smoothstep(0.08, 0.6, y) * (1.0 - y) * 0.6;
    return clamp(head + trail, 0.0, 1.0);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let p = in.world_position.xyz;
    // In-plane coordinates in u (works for windows on any wall).
    let q = vec2<f32>(p.x + p.z, p.y);
    // Brighter toward the top of the opening (sky above, the garden / street below).
    let n = clamp(in.world_normal, vec3<f32>(-1.0), vec3<f32>(1.0));
    var color = sky.rgb * (0.72 + 0.06 * p.y * 0.1 + 0.08 * n.y);

    let rain = clamp(sky.a, 0.0, 1.0);
    if rain > 0.001 {
        let b = beads(q * 1.6) * smoothstep(0.0, 0.4, rain);
        let s = streaks(q * vec2<f32>(2.2, 1.0), globals.time) * smoothstep(0.2, 0.8, rain);
        let water = max(b, s);
        // Drops refract the darker ground and catch a highlight on top.
        color = color * (1.0 - 0.45 * water) + sky.rgb * 0.35 * water * water;
        // Wet, blurred glass overall.
        color *= 1.0 - 0.15 * rain;
    }
    color += vec3<f32>(0.8, 0.85, 1.0) * extra.x * 3.0;
    return vec4<f32>(color, 1.0);
}
