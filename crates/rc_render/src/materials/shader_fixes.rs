//! Workarounds for Bevy 0.20 shader bugs, patched into Bevy's embedded WESL once at startup
//! (`docs/upstream-notes.md`). Each [`Patch`] replaces one anchor; a missing anchor warns (Bevy
//! changed: review the fix).
//!
//! - **Clear coat (U1).** With a normal prepass (SSAO) and no MSAA, the PBR fragment loads its
//!   normal from the prepass (`LOAD_PREPASS_NORMALS`) and skips the block that sets
//!   `pbr_input.clearcoat_N`, which stays `vec3(0)`. The coat's NdotV becomes ≈ 0, its Fresnel
//!   ≈ 1, and every clear-coated surface (car paint, coated glb props, glass) turns into a mirror
//!   of the environment. The fix sets the coat normal to the geometric normal (what
//!   `KHR_materials_clearcoat` specifies without a coat normal map) before that block, which still
//!   overrides it when the prepass is not used.
//! - **Irradiance volumes (U8).** Once a view has an irradiance volume, `pbr_functions` marks
//!   diffuse indirect light as found for *every* fragment, inside a volume or not, so fragments
//!   outside it get neither the volume's light nor the environment map's diffuse: black ambient
//!   (outdoor roofs and trees past the baked bounds). The fix counts the volume only when it gave
//!   light (`irradiance_volume_light` returns 0 outside every volume).
//!
//! `ponytail:` remove each when Bevy fixes it upstream.

use bevy::prelude::*;
use bevy::shader::{Shader, Source};

/// One text replacement in an embedded shader (`to` also marks the file as patched).
struct Patch {
    name: &'static str,
    shader: &'static str,
    from: &'static str,
    to: &'static str,
}

const PATCHES: [Patch; 2] = [
    Patch {
        name: "coat fix",
        shader: "embedded://bevy_pbr/render/pbr_fragment.wesl",
        from: "        pbr_input.specular_occlusion = specular_occlusion;\n",
        to: "        pbr_input.specular_occlusion = specular_occlusion;
        pbr_input.clearcoat_N = normalize(pbr_input.world_normal); // rc_render coat fix\n",
    },
    Patch {
        name: "irradiance volume fix",
        shader: "embedded://bevy_pbr/render/pbr_functions.wesl",
        from:
            "        indirect_light += irradiance_volume_light * diffuse_color * diffuse_occlusion;
        found_diffuse_indirect = true;\n",
        to: "        indirect_light += irradiance_volume_light * diffuse_color * diffuse_occlusion;
        found_diffuse_indirect = any(irradiance_volume_light != vec3(0.0)); // rc_render fix\n",
    },
];

pub(super) fn build(app: &mut App) {
    app.init_resource::<Pending>()
        .add_systems(Update, patch_shaders.run_if(|p: Res<Pending>| p.0 != 0));
}

/// Bit `i` set: [`PATCHES`]`[i]` still waits for its shader.
#[derive(Resource, Debug)]
struct Pending(u32);

impl Default for Pending {
    fn default() -> Self {
        Self((1 << PATCHES.len()) - 1)
    }
}

/// `source` with the patch applied; `None` when the anchor is missing or already patched.
fn patched(source: &str, patch: &Patch) -> Option<String> {
    if source.contains(patch.to) {
        return None;
    }
    let at = source.find(patch.from)?;
    Some(format!(
        "{}{}{}",
        &source[..at],
        patch.to,
        &source[at + patch.from.len()..]
    ))
}

fn patch_shaders(
    mut pending: ResMut<Pending>,
    server: Res<AssetServer>,
    mut shaders: ResMut<Assets<Shader>>,
) {
    for (i, patch) in PATCHES.iter().enumerate() {
        if pending.0 & (1 << i) == 0 {
            continue;
        }
        let Some(handle) = server.get_handle::<Shader>(patch.shader) else {
            continue;
        };
        let Some(shader) = shaders.get(&handle) else {
            continue; // still loading
        };
        pending.0 &= !(1 << i);
        let Source::Wesl(text) = &shader.source else {
            warn!(
                "{}: {} is not WESL; left as Bevy has it",
                patch.name, patch.shader
            );
            continue;
        };
        let Some(fixed) = patched(text, patch) else {
            if !text.contains(patch.to) {
                warn!(
                    "{}: anchor not found in {}; Bevy changed, review the fix",
                    patch.name, patch.shader
                );
            }
            continue;
        };
        if let Some(mut shader) = shaders.get_mut(&handle) {
            shader.source = Source::Wesl(fixed.into());
            info!("{}: patched {}", patch.name, patch.shader);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_fix_goes_in_once_in_place_of_its_anchor() {
        for patch in &PATCHES {
            let source = format!("a\n{}b\n", patch.from);
            let fixed = patched(&source, patch).expect("anchor found");
            assert_eq!(fixed, format!("a\n{}b\n", patch.to));
            assert!(patched(&fixed, patch).is_none(), "already patched");
            assert!(patched("no anchor here", patch).is_none());
        }
    }
}
