// rc_render SurfaceMaterial: StandardMaterial + world-space macro variation, wetness/puddles and
// snow cover (crates/rc_render/src/materials/surface.rs). All noise is in world X/Z (u), so the
// patterns are continuous across meshes and independent of UVs.

import bevy_pbr::render::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
};
@if(PREPASS_PIPELINE)
import bevy_pbr::{
    prepass::io::{VertexOutput, FragmentOutput},
    deferred::functions::deferred_output,
};
@else
import bevy_pbr::render::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
};

// x: albedo variation strength, y: noise frequency (1/u), z: roughness variation strength.
@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> variation: vec4<f32>;
// x: wetness, y: puddle amount, z: snow cover, w: 1 = receives weather.
@group(constants::MATERIAL_BIND_GROUP) @binding(101) var<uniform> weather: vec4<f32>;

const PUDDLE_FREQ: f32 = 0.045;
const SNOW_FREQ: f32 = 0.55;
const SNOW_COLOR: vec3<f32> = vec3<f32>(0.88, 0.9, 0.93);

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

// 4 octaves, normalised to 0..1.
fn fbm(p: vec2<f32>) -> f32 {
    var sum = 0.0;
    var amp = 0.5;
    var q = p;
    for (var i = 0; i < 4; i++) {
        sum += amp * value_noise(q);
        q = q * 2.03 + vec2<f32>(17.1, 9.2);
        amp *= 0.5;
    }
    return sum / 0.9375;
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    let p = in.world_position.xz;

    // Macro variation: large soft blotches in brightness and roughness.
    let m = fbm(p * variation.y) * 2.0 - 1.0;
    let m2 = value_noise(p * variation.y * 2.7 + vec2<f32>(31.7, 11.3)) * 2.0 - 1.0;
    var color = pbr_input.material.base_color.rgb * max(1.0 + variation.x * m, 0.0);
    var rough = pbr_input.material.perceptual_roughness * max(1.0 + variation.z * m2, 0.05);

    let receives = weather.w;
    let level = smoothstep(0.8, 0.97, pbr_input.world_normal.y);

    // Wetness: porous (rough) surfaces darken more; everything turns glossier.
    let wet = clamp(weather.x * receives, 0.0, 1.0);
    color *= mix(1.0, mix(0.85, 0.62, clamp(rough, 0.0, 1.0)), wet);
    rough = mix(rough, rough * 0.35, wet);

    // Puddles on level ground: a flat, dark, mirror-like water film.
    let pn = fbm(p * PUDDLE_FREQ + vec2<f32>(3.7, 1.3));
    let edge = 0.68 - 0.22 * weather.y;
    let puddle = wet * level * step(0.001, weather.y) * smoothstep(edge, edge + 0.06, pn);
    color = mix(color, color * 0.5, puddle);
    rough = mix(rough, 0.03, puddle);
    pbr_input.N = normalize(mix(pbr_input.N, pbr_input.world_normal, puddle));

    // Snow: patches grow from the noise peaks; full cover still leaves ≈ 30 % of the ground
    // showing (readable track surfaces); slopes stay clearer.
    let cover = clamp(weather.z * receives, 0.0, 1.0);
    let sn = fbm(p * SNOW_FREQ + vec2<f32>(9.1, 4.4));
    let threshold = 0.85 - cover * 0.42;
    let slope = smoothstep(0.55, 0.9, pbr_input.world_normal.y);
    let snow = slope * step(0.001, cover) * smoothstep(threshold - 0.1, threshold + 0.05, sn);
    color = mix(color, SNOW_COLOR, snow);
    rough = mix(rough, 0.7, snow);
    pbr_input.material.clearcoat = mix(pbr_input.material.clearcoat, 0.0, snow);
    pbr_input.N = normalize(mix(pbr_input.N, pbr_input.world_normal, snow * 0.85));

    pbr_input.material.base_color = vec4<f32>(color, pbr_input.material.base_color.a);
    pbr_input.material.perceptual_roughness = clamp(rough, 0.02, 1.0);
    pbr_input.material.base_color = alpha_discard(
        pbr_input.material.flags,
        pbr_input.material.alpha_cutoff,
        pbr_input.material.base_color,
    );

    var out: FragmentOutput;
@if(PREPASS_PIPELINE) {
    out = deferred_output(in, pbr_input);
} @else {
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
}
    return out;
}
