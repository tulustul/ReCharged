// rc_render CarBodyMaterial: car paint (StandardMaterial) + status effects on the body
// (crates/rc_render/src/cars/status.rs): an energy rim with rising bands (shield, charge boost),
// a hit flare, electric crackle (power loss) and an oil film (grip loss). All zero = plain paint.

import bevy_pbr::render::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
    mesh_view_bindings::globals,
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

// rgb: rim colour (linear HDR), w: band shimmer 0..1.
@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> rim: vec4<f32>;
// x: crackle 0..1, y: oil film 0..1, z: hit flare 0..1.
@group(constants::MATERIAL_BIND_GROUP) @binding(101) var<uniform> status: vec4<f32>;

const FLARE: vec3<f32> = vec3<f32>(6.0, 14.0, 20.0);
const STUN: vec3<f32> = vec3<f32>(12.0, 20.0, 48.0);
// Crackle pattern re-rolls this often (Hz).
const CRACKLE_HZ: f32 = 16.0;

fn hash31(p: vec3<f32>) -> f32 {
    var q = fract(p * 0.1031);
    q += dot(q, q.zyx + 31.32);
    return fract((q.x + q.y) * q.z);
}

fn noise3(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = mix(hash31(i), hash31(i + vec3<f32>(1.0, 0.0, 0.0)), u.x);
    let b = mix(hash31(i + vec3<f32>(0.0, 1.0, 0.0)), hash31(i + vec3<f32>(1.0, 1.0, 0.0)), u.x);
    let c = mix(hash31(i + vec3<f32>(0.0, 0.0, 1.0)), hash31(i + vec3<f32>(1.0, 0.0, 1.0)), u.x);
    let d = mix(hash31(i + vec3<f32>(0.0, 1.0, 1.0)), hash31(i + vec3<f32>(1.0, 1.0, 1.0)), u.x);
    return mix(mix(a, b, u.y), mix(c, d, u.y), u.z);
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    let p = in.world_position.xyz;
    let t = globals.time;
    let facing = clamp(dot(pbr_input.N, pbr_input.V), 0.0, 1.0);
    let fresnel = pow(1.0 - facing, 1.5);
    var glow = vec3<f32>(0.0);

    // Energy rim: bright at grazing angles, bands rising over the body, a faint body glow.
    let bands = 0.5 + 0.5 * sin(p.y * 14.0 - t * 4.0 + sin(p.x * 3.0 + p.z * 2.0) * 1.5);
    let shimmer = mix(1.0, mix(0.35, 1.65, bands), rim.w);
    glow += rim.rgb * shimmer * (fresnel + 0.12 * rim.w);

    // Hit flare: the whole body flashes, strongest at the rim.
    glow += FLARE * status.z * (0.3 + 1.5 * fresnel);

    // Crackle: thin electric veins that jump to a new pattern many times a second.
    if (status.x > 0.001) {
        let step_t = floor(t * CRACKLE_HZ);
        let q = p * 2.2 + vec3<f32>(step_t * 1.7, step_t * 0.9, step_t * 2.3);
        let vein = 1.0 - smoothstep(0.0, 0.06, abs(noise3(q) - 0.5));
        let flicker = mix(0.3, 1.0, step(0.35, hash31(vec3<f32>(step_t, 7.0, 3.0))));
        glow += STUN * vein * flicker * status.x;
    }

    // Oil film: darker, glossier paint with a thin-film rainbow sheen.
    let oil = status.y;
    if (oil > 0.001) {
        let phase = facing * 1.6 + noise3(p * 2.0) * 0.8;
        let film = 0.5 + 0.5 * cos(6.2831 * (phase + vec3<f32>(0.0, 0.33, 0.67)));
        let base = pbr_input.material.base_color;
        pbr_input.material.base_color = vec4<f32>(mix(base.rgb, base.rgb * 0.2 + film * 0.06, oil), base.a);
        pbr_input.material.perceptual_roughness = mix(pbr_input.material.perceptual_roughness, 0.08, oil);
        glow += film * fresnel * 2.0 * oil;
    }

    pbr_input.material.base_color = alpha_discard(
        pbr_input.material.flags,
        pbr_input.material.alpha_cutoff,
        pbr_input.material.base_color,
    );

    var out: FragmentOutput;
@if(PREPASS_PIPELINE) {
    let e = pbr_input.material.emissive;
    pbr_input.material.emissive = vec4<f32>(e.rgb + glow, e.a);
    out = deferred_output(in, pbr_input);
} @else {
    // Added after lighting: as emission, the paint's clear coat would dim it most at grazing
    // angles, exactly where the rim should be brightest.
    out.color = apply_pbr_lighting(pbr_input);
    out.color = vec4<f32>(out.color.rgb + glow * out.color.a, out.color.a);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
}
    return out;
}
