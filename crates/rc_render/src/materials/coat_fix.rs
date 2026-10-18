//! Workaround for a Bevy 0.20 clear-coat bug. With a normal prepass (SSAO) and no MSAA, the PBR
//! fragment loads its normal from the prepass (`LOAD_PREPASS_NORMALS`) and skips the block that
//! sets `pbr_input.clearcoat_N`, which stays `vec3(0)`. The coat's NdotV becomes ≈ 0, its Fresnel
//! ≈ 1, and every clear-coated surface (car paint, coated glb props, glass) turns into a mirror
//! of the environment: paints and coated props read washed out or white.
//!
//! The fix patches Bevy's embedded `pbr_fragment.wesl` once at startup: the coat normal is set to
//! the geometric normal (what `KHR_materials_clearcoat` specifies without a coat normal map)
//! before that block, which still overrides it when the prepass is not used.
//! `ponytail:` remove when Bevy sets `clearcoat_N` on the prepass path itself.

use bevy::prelude::*;
use bevy::shader::{Shader, Source};

/// Bevy's embedded PBR fragment library.
const PBR_FRAGMENT: &str = "embedded://bevy_pbr/render/pbr_fragment.wesl";
/// Text the fix is inserted after (once per file).
const ANCHOR: &str = "        pbr_input.specular_occlusion = specular_occlusion;\n";
/// Inserted line (also marks the file as patched).
const FIX: &str =
    "        pbr_input.clearcoat_N = normalize(pbr_input.world_normal); // rc_render coat fix\n";

pub(super) fn build(app: &mut App) {
    app.add_systems(
        Update,
        patch_pbr_fragment.run_if(not(resource_exists::<Patched>)),
    );
}

/// The shader was patched (or cannot be).
#[derive(Resource, Debug)]
struct Patched;

/// Inserts [`FIX`] after [`ANCHOR`]; `None` when the anchor is missing or already patched.
fn patched(source: &str) -> Option<String> {
    if source.contains(FIX) {
        return None;
    }
    let at = source.find(ANCHOR)? + ANCHOR.len();
    Some(format!("{}{}{}", &source[..at], FIX, &source[at..]))
}

fn patch_pbr_fragment(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut shaders: ResMut<Assets<Shader>>,
) {
    let Some(handle) = server.get_handle::<Shader>(PBR_FRAGMENT) else {
        return;
    };
    let Some(shader) = shaders.get(&handle) else {
        return; // still loading
    };
    commands.insert_resource(Patched);
    let Source::Wesl(text) = &shader.source else {
        warn!("coat fix: {PBR_FRAGMENT} is not WESL; clear coats stay as Bevy renders them");
        return;
    };
    let Some(fixed) = patched(text) else {
        if !text.contains(FIX) {
            warn!("coat fix: anchor not found in {PBR_FRAGMENT}; Bevy changed, review the fix");
        }
        return;
    };
    if let Some(mut shader) = shaders.get_mut(&handle) {
        shader.source = Source::Wesl(fixed.into());
        info!("coat fix: clear-coat normal set on the prepass-normal path");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fix_goes_in_once_after_the_anchor() {
        let source = format!("a\n{ANCHOR}b\n");
        let fixed = patched(&source).expect("anchor found");
        assert_eq!(fixed, format!("a\n{ANCHOR}{FIX}b\n"));
        assert!(patched(&fixed).is_none(), "already patched");
        assert!(patched("no anchor here").is_none());
    }
}
