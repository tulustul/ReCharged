// rc_render cloud layer (crates/rc_render/src/sky/clouds.rs): a procedural cloud deck seen from
// inside a dome, composited over the atmosphere's sky. Output is premultiplied and
// display-referred (no exposure), like the art bible's emissives.

import bevy_pbr::render::{
    forward_io::VertexOutput,
    mesh_view_bindings::view,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> lit: vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var<uniform> shade: vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var<uniform> sun: vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(3) var<uniform> deck: vec4<f32>;

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

fn fbm(p: vec2<f32>) -> f32 {
    var sum = 0.0;
    var amp = 0.5;
    var q = p;
    // Rotate each octave to hide the lattice.
    let rot = mat2x2<f32>(0.8, 0.6, -0.6, 0.8);
    for (var i = 0; i < 6; i++) {
        sum += amp * value_noise(q);
        q = rot * q * 2.02 + vec2<f32>(5.2, 1.3);
        amp *= 0.5;
    }
    return sum / 0.984;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let dir = normalize(in.world_position.xyz - view.world_position);
    if dir.y <= 0.0 {
        discard;
    }
    // Ray → cloud deck plane `deck.w` above the eye; flattened near the horizon.
    let t = deck.w / max(dir.y, 0.04);
    let p = (view.world_position.xz + dir.xz * t) * deck.z + deck.xy * deck.z;

    let coverage = clamp(lit.a, 0.0, 1.0);
    // Domain warp for billowy shapes.
    let warp = vec2<f32>(fbm(p * 0.5 + vec2<f32>(3.1, 7.7)), fbm(p * 0.5 + vec2<f32>(8.3, 2.8)));
    let n = fbm(p * 3.0 + warp * 1.6);
    let threshold = 1.0 - coverage * 1.05;
    let density = smoothstep(threshold - 0.04, threshold + 0.30, n);
    if density <= 0.002 {
        discard;
    }

    // Fake self-shadowing: denser toward the sun = darker.
    let toward_sun = normalize(vec2<f32>(sun.x, sun.z) + vec2<f32>(1e-4, 0.0));
    let n_sun = fbm(p * 3.0 + warp * 1.6 + toward_sun * 0.08);
    let light = clamp(0.62 + (n - n_sun) * 4.0 - density * 0.25, 0.0, 1.0);
    var color = mix(shade.rgb, lit.rgb, light);
    // Rain clouds: darker, heavier undersides.
    color *= 1.0 - shade.a * (0.35 + 0.45 * density);
    // Silver lining when looking toward the sun.
    let glow = pow(max(dot(dir, normalize(sun.xyz)), 0.0), 12.0) * (1.0 - density) * 0.8;
    color += lit.rgb * glow;
    // Lightning lights the whole deck from inside.
    color += vec3<f32>(0.75, 0.8, 1.0) * sun.w * 3.0 * (0.4 + 0.6 * density);

    // Dissolve into the haze near the horizon.
    let fade = smoothstep(0.015, 0.22, dir.y);
    let alpha = clamp(density * fade * (0.55 + 0.45 * coverage), 0.0, 1.0);
    return vec4<f32>(color * alpha, alpha);
}
