// rc_vfx sky lightning (crates/rc_vfx/src/lightning.rs, BoltMaterial): an unlit additive
// `StandardMaterial` ribbon that fades where it meets opaque geometry (soft particles) when the
// view has a depth prepass, so a bolt's foot sinks into the hills instead of being cut off.

import bevy_pbr::render::{
    view_transformations::depth_ndc_to_view_z,
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::main_pass_post_lighting_processing,
    forward_io::{VertexOutput, FragmentOutput},
};
import bevy_pbr::prepass::utils as prepass_utils;

// A bolt fades out over this distance (u) in front of the scene behind it.
const SOFT_DEPTH: f32 = 4.0;

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    let pbr_input = pbr_input_from_standard_material(in, is_front);
    // Already premultiplied for additive blending (alpha 0).
    let c = main_pass_post_lighting_processing(pbr_input, pbr_input.material.base_color);
    var a = 1.0;
    @if(DEPTH_PREPASS)
    {
        let scene = depth_ndc_to_view_z(prepass_utils::prepass_depth(in.position, 0u));
        a *= clamp((depth_ndc_to_view_z(in.position.z) - scene) / SOFT_DEPTH, 0.0, 1.0);
    }
    var out: FragmentOutput;
    out.color = vec4<f32>(c.rgb * a, c.a);
    return out;
}
