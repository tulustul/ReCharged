// rc_vfx energy shell (crates/rc_vfx/src/assets.rs, BubbleMaterial): Shield Bubble, EMP shells.
// Additive (premultiplied output with alpha 0): a bright fresnel rim, a faint body, slow bands
// rising over the surface and a hit flash. Uses Bevy's default mesh vertex stage.

import bevy_pbr::render::{
    forward_io::VertexOutput,
    mesh_view_bindings::{globals, view},
};

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
    let i = (rim * params.y * shimmer + params.z) * (1.0 + flash * 2.5) + flash * 0.4;
    return vec4<f32>(color.rgb * i, 0.0);
}
