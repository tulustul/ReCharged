// rc_render WaterMaterial (crates/rc_render/src/materials/water.rs): water ground areas, drawn in
// the transmissive pass (the opaque scene is readable in view_transmission_texture) and absent
// from the depth prepass (the depth under the surface is the bed's):
// - the bed shows through, tinted by base_color and shifted by the ripples;
// - reflections of what is on screen (cars, props, walls) are ray-marched against the depth
//   prepass; where the ray leaves the screen the environment map (and light glints) stand in;
// - foam along the shore: UV_1.x runs from 0 on the outline to 1 a little inside
//   (crates/rc_render/src/track_visuals/water.rs).
// Without the scene copy or the depth prepass the water is a mirror over a black bed.

import bevy_pbr::render::{
    forward_io::{VertexOutput, FragmentOutput},
    mesh_view_bindings::{view, globals},
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    view_transformations::{depth_ndc_to_view_z, position_world_to_view},
};
@if(DEPTH_PREPASS)
import bevy_pbr::render::mesh_view_bindings::depth_prepass_texture;
@if(VIEW_TRANSMISSION_TEXTURE)
import bevy_pbr::render::mesh_view_bindings::{view_transmission_texture, view_transmission_sampler};

// Reflectance at normal incidence (matches the material's `reflectance` 0.79).
const F0: f32 = 0.1;
// Ripples: noise frequency (1/u), slope strength, drift (u/s).
const RIPPLE_FREQ: f32 = 2.2;
const RIPPLE_SLOPE: f32 = 0.05;
const RIPPLE_DRIFT: f32 = 0.25;
// Viewport-uv shift of the bed per unit of ripple slope.
const REFRACT: f32 = 0.03;
// Reflection ray march: steps, reach (u), and how far behind the depth buffer a sample may lie
// and still count as a hit (u, growing with distance).
const SSR_STEPS: i32 = 32;
const SSR_REACH_U: f32 = 25.0;
const SSR_THICKNESS_U: f32 = 0.6;
const FOAM_COLOR: vec3<f32> = vec3<f32>(0.9, 0.92, 0.93);
const FOAM_OPACITY: f32 = 0.6;

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

// Two drifting noise layers.
fn ripple_height(q: vec2<f32>, t: f32) -> f32 {
    let drift = RIPPLE_DRIFT * RIPPLE_FREQ * t;
    return value_noise(q + vec2<f32>(drift, drift * 0.6))
        + 0.5 * value_noise(q * 2.3 + vec2<f32>(-drift * 0.8, drift * 1.1));
}

// Viewport uv (0..1 across this view) → pixel of the view's full-target textures.
fn texel(sub: vec2<f32>) -> vec2<i32> {
    return vec2<i32>(view.viewport.xy + clamp(sub, vec2<f32>(0.0), vec2<f32>(0.9999)) * view.viewport.zw);
}

@if(VIEW_TRANSMISSION_TEXTURE)
fn scene_color(sub: vec2<f32>) -> vec3<f32> {
    let full = vec2<f32>(textureDimensions(view_transmission_texture));
    let uv = (view.viewport.xy + sub * view.viewport.zw) / full;
    return textureSampleLevel(view_transmission_texture, view_transmission_sampler, uv, 0.0).rgb;
}

// Viewport uv and NDC depth of a world point; z < 0 when it is behind the camera.
fn to_screen(p: vec3<f32>) -> vec3<f32> {
    let clip = view.clip_from_world * vec4<f32>(p, 1.0);
    if clip.w <= 0.0 {
        return vec3<f32>(0.0, 0.0, -1.0);
    }
    let ndc = clip.xyz / clip.w;
    return vec3<f32>(ndc.xy * vec2<f32>(0.5, -0.5) + 0.5, ndc.z);
}

// How far (u, view z) the ray point `p` lies behind the visible surface at its pixel.
@if(DEPTH_PREPASS)
fn behind_scene(p: vec3<f32>, sub: vec2<f32>) -> f32 {
    let scene_z = depth_ndc_to_view_z(textureLoad(depth_prepass_texture, texel(sub), 0));
    return scene_z - position_world_to_view(p).z;
}

// What a ray from `origin` along `dir` hits on screen: rgb, a = confidence (0 = missed).
@if(DEPTH_PREPASS && VIEW_TRANSMISSION_TEXTURE)
fn reflect_screen(origin: vec3<f32>, dir: vec3<f32>) -> vec4<f32> {
    var prev = 0.0;
    for (var i = 0; i < SSR_STEPS; i++) {
        let s = (f32(i) + 0.5) / f32(SSR_STEPS);
        let t = SSR_REACH_U * s * s + 0.05;
        let at = to_screen(origin + dir * t);
        if at.z < 0.0 || any(at.xy < vec2<f32>(0.0)) || any(at.xy > vec2<f32>(1.0)) {
            break;
        }
        let behind = behind_scene(origin + dir * t, at.xy);
        if behind > 0.0 {
            if behind > SSR_THICKNESS_U + t * 0.05 {
                // Passed behind something thick: what is there is unknown.
                break;
            }
            // Refine between the last sample in front and this one.
            var lo = prev;
            var hi = t;
            for (var j = 0; j < 4; j++) {
                let mid = 0.5 * (lo + hi);
                let m = to_screen(origin + dir * mid);
                if behind_scene(origin + dir * mid, m.xy) > 0.0 {
                    hi = mid;
                } else {
                    lo = mid;
                }
            }
            let hit = to_screen(origin + dir * hi).xy;
            let edge = min(min(hit.x, 1.0 - hit.x), min(hit.y, 1.0 - hit.y));
            let confidence = smoothstep(0.0, 0.08, edge) * (1.0 - s * s);
            return vec4<f32>(scene_color(hit), confidence);
        }
        prev = t;
    }
    return vec4<f32>(0.0);
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    let p = in.world_position.xyz;

    // Ripples tilt the normal a little.
    let q = p.xz * RIPPLE_FREQ;
    let e = 0.2;
    let t = globals.time;
    let slope = vec2<f32>(
        ripple_height(q + vec2<f32>(e, 0.0), t) - ripple_height(q - vec2<f32>(e, 0.0), t),
        ripple_height(q + vec2<f32>(0.0, e), t) - ripple_height(q - vec2<f32>(0.0, e), t),
    ) * (RIPPLE_SLOPE / (2.0 * e));
    let n = normalize(vec3<f32>(-slope.x, 1.0, -slope.y));
    let v = normalize(view.world_position.xyz - p);
    let fresnel = F0 + (1.0 - F0) * pow(1.0 - max(dot(n, v), 0.0), 5.0);

    // The environment map and light glints: a black, glossy dielectric.
    var mirror = pbr_input;
    mirror.N = n;
    mirror.material.base_color = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    mirror.material.specular_transmission = 0.0;
    var reflection = apply_pbr_lighting(mirror).rgb;

    // What is on screen replaces it where the ray finds it.
    @if(DEPTH_PREPASS && VIEW_TRANSMISSION_TEXTURE)
    {
        let hit = reflect_screen(p + n * 0.02, reflect(-v, n));
        reflection = mix(reflection, hit.rgb * fresnel, hit.a);
    }

    // The bed, shifted by the ripples where the shifted pixel is still behind the water.
    var bed = vec3<f32>(0.0);
    @if(VIEW_TRANSMISSION_TEXTURE)
    {
        let here = (in.position.xy - view.viewport.xy) / view.viewport.zw;
        var sub = here + slope * REFRACT;
        @if(DEPTH_PREPASS)
        if textureLoad(depth_prepass_texture, texel(sub), 0) > in.position.z {
            sub = here;
        }
        bed = scene_color(sub) * pbr_input.material.base_color.rgb;
    }

    // Foam: clumps along the shore, broken up by drifting noise.
    var shore = 1.0;
    @if(VERTEX_UVS_B)
    {
        shore = in.uv_b.x;
    }
    let wobble = value_noise(p.xz * 7.0 + vec2<f32>(t * 0.2, -t * 0.15)) - 0.5;
    let clumps = smoothstep(0.3, 0.6, value_noise(p.xz * 1.6 + vec2<f32>(-t * 0.05, t * 0.04)));
    let foam = FOAM_OPACITY * clumps * (1.0 - smoothstep(0.0, 0.4, shore + wobble * 0.7));
    var froth = pbr_input;
    froth.material.base_color = vec4<f32>(FOAM_COLOR, 1.0);
    froth.material.perceptual_roughness = 0.85;
    froth.material.specular_transmission = 0.0;
    let foam_lit = apply_pbr_lighting(froth).rgb;

    let water = reflection + bed * (1.0 - fresnel);
    // ponytail: fog is applied over the bed and the reflections, which carry their own; fine at
    // puddle distances, separate them if far water looks too hazy.
    var out: FragmentOutput;
    out.color = main_pass_post_lighting_processing(
        pbr_input,
        vec4<f32>(mix(water, foam_lit, foam), 1.0),
    );
    return out;
}
