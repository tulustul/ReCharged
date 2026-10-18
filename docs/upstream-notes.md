# Upstream notes

Issues found while building ReCharged in libraries we depend on, with our workarounds. Nothing
here has been reported upstream yet; that is the project owner's call.

## bevy-react 0.8.1
| # | Issue | Workaround in ReCharged | Suggested fix |
|---|---|---|---|
| R1 | An animated `opacity` on a `<text>` draws a dark box behind the glyphs. | Animate the text colour's alpha instead. | Apply opacity to the glyph colours (or composite text layers without a backdrop). |
| R2 | The radial-gradient `shape` keyword form works at runtime but is missing from the TS typings. | A cast at the call site. | Add the `shape` keyword variants to the gradient types. |
| R3 | `boxShadow: []` draws one default black shadow: `BoxShadowList` is `#[serde(untagged)]`, so an empty array deserializes as a single default shadow instead of none. (It also drew our empty item slot as a black box.) | Pass `undefined` instead of `[]`. | Treat an empty list as "no shadow". |
| R4 | The `init` template's `npm run typecheck` fails on a standalone install: its TS sources import `react-reconciler`, but `@types/react-reconciler` exists only in the bevy-react monorepo. The template also has no `@types/node`, so test files can't be type-checked. | `ui/src/types/react-reconciler.d.ts`; tests excluded from `tsc` (node still runs them). | Add `@types/react-reconciler` and `@types/node` to the template's devDependencies. |
| R5 | npm 12 blocks the esbuild/@swc install scripts. | None needed — the builds work anyway. | Informational. |
| R6 | Docs: for split screen, an overlay `Camera2d` with `ClearColorConfig::None` over HDR viewport cameras is never cleared (`LoadOp::Load`), so React UI accumulates (translucent panels turn opaque within a few frames, removed UI stays on screen). | Rig B: the overlay clears to `Color::NONE` and is written with `CameraOutputMode::Write { blend_state: PREMULTIPLIED_ALPHA_BLENDING, clear_color: None }`, `Msaa::Off` (see `docs/tech/bevy-react-notes.md` §8.3, `docs/design/rc_camera.md`). | Recommend rig B in the split-screen docs/examples. |

## Bevy 0.20
| # | Issue | Workaround in ReCharged |
|---|---|---|
| U1 | **Clear coat with a normal prepass and no MSAA** (e.g. with SSAO): the PBR fragment loads its normal from the prepass and skips the block that sets `pbr_input.clearcoat_N`, which stays `vec3(0)` → the coat's Fresnel ≈ 1 and every clear-coated surface turns into an environment mirror. | `crates/rc_render/src/materials/coat_fix.rs` patches the embedded `pbr_fragment.wesl` at startup (coat normal = geometric normal). |
| U2 | **`AtmosphereEnvironmentMapLight`**: the probe's bind-group preparation panics every frame in our setup (`atmosphere_transforms.binding().unwrap()`). | Outdoor IBL from a generated sky cubemap (`rc_render` `sky::sky_radiance`); the visible sky still uses `Atmosphere`. |
| U3 | **TAA with `Camera::viewport`**: the frame renders squeezed — the TAA pass sets the viewport but samples full-texture UVs (`bevy_anti_alias/src/taa/mod.rs:211`). | TAA only in single view; SMAA/FXAA in split screen. |
| U4 | **Quitting while pipelines still compile** (async pipeline compilation) segfaults at teardown on Linux/NVIDIA (exit 139) — possibly wgpu or the driver rather than Bevy. | Synchronous pipeline compilation in automated runs; human exits are deferred until the pipeline queue is idle (backlog B2). |
