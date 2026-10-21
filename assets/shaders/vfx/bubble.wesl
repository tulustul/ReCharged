// rc_vfx energy shell (crates/rc_vfx/src/assets.rs, BubbleMaterial): Shield Bubble, EMP shells.
// Additive (premultiplied output with alpha 0): a bright fresnel rim, a faint body, slow bands
// rising over the surface and a hit flash. Uses Bevy's default mesh vertex stage. With the view's
// depth prepass the body fades where it meets opaque geometry (soft particles) and a bright line
// glows along the cut (the ground, cars inside the shell).

import bevy_pbr::render::{
    forward_io::VertexOutput,
    mesh_view_bindings::{globals, view},
    view_transformations::depth_ndc_to_view_z,
};
import bevy_pbr::prepass::utils as prepass_utils;

// The shell fades out over this distance (u) in front of the scene behind it.
const SOFT_DEPTH: f32 = 0.3;
// The contact line: width (u, an exponential falloff) and brightness relative to the rim.
const CONTACT_U: f32 = 0.06;
const CONTACT_GLOW: f32 = 1.5;

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> color: vec4<f32>;
// x: rim exponent, y: rim intensity, z: body intensity, w: hit flash 0..1
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var<uniform> params: vec4<f32>;

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> @location(0) vec4<f32> {
    let n = normalize(in.world_normal);
    let v = normalize(view.world_position.xyz - in.world_position.xyz);
    let facing = abs(dot(n, v));
    let rim = pow(1.0 - facing, params.x);
    let p = in.world_position.xyz;
    let bands = 0.5 + 0.5 * sin(p.y * 9.0 - globals.time * 2.5 + sin(p.x * 3.0 + p.z * 2.0));
    let shimmer = mix(0.75, 1.25, bands);
    let flash = params.w;
    var i = (rim * params.y * shimmer + params.z) * (1.0 + flash * 2.5) + flash * 0.4;
    @if(DEPTH_PREPASS)
    {
        let scene = depth_ndc_to_view_z(prepass_utils::prepass_depth(in.position, 0u));
        let gap = max(depth_ndc_to_view_z(in.position.z) - scene, 0.0);
        i = i * min(gap / SOFT_DEPTH, 1.0) + params.y * CONTACT_GLOW * exp(-gap / CONTACT_U);
    }
    return vec4<f32>(color.rgb * i, 0.0);
}
