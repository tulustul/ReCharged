# rc_render — rendering core (design note, T13a)

Proxies, quality, lighting/sky/weather visuals, materials and the showroom (T13a). Track, prop and
car visuals and captures are described in `docs/design/rc_render_content.md` (written by T13b, same
crate; `lib.rs` registers both).

## Module map
| Module | What |
|---|---|
| `proxy.rs` | one `VisualOf` proxy per `Interpolated` entity (queued after the sim spawn), posed in `InterpolateSet` by `PoseHistory::interpolate(overstep)` |
| `quality/` | `view_quality` / `shadow_tier` (pure), camera decoration, display settings |
| `lighting/` | `ActiveEnvironment` lifecycle, env RON (`env.rs`), key/fill/flash lights, cascades, camera look, generated env maps + room lamps + window glass, headlights, volumetric fog, pipeline warm-up, `"visuals"` key |
| `sky/` | sun direction maths, atmosphere entity, star cubemap, sky radiance model, cloud dome |
| `weather_fx.rs` | the frame's `Look` = env time look × `WeatherState`; surface wetness/snow |
| `materials/` | `MaterialLibrary`, `SurfaceMaterial` (world-space extension), car paints, `with_world_uvs`, Bevy clear-coat fix (`coat_fix.rs`) |
| `showroom/` | menu garage set, turntable, `CarModel` swap without gaps, `ShowroomReady` |

Shaders (WESL, `assets/shaders/render/`): `surface.wesl` (macro variation, wetness/puddles, snow),
`clouds.wesl` (cloud deck), `window_glass.wesl` (outside light, rain beads/streaks, lightning).

## Lifecycle
- Startup: texture sets queued, `MaterialLibrary` with built-in materials.
- `OnEnter(Menu)`: `showroom` environment (+ set, car). `ShowroomReady` 2 frames after the car is
  instanced, painted and shown; a swap keeps the old car until the new one is ready.
- `OnEnter(Loading)`: the track's environment (`TrackInfo.environment`; missing/broken file → `test`
  with an error log), library rebuilt for it, lights/atmosphere/clouds spawned
  (`DespawnOnExit(RaceSession)`). `"visuals"` completes when textures are loaded **and** the pipeline
  warm-up is over: once every other loader is done, a hidden `WarmupCamera` (a `QualityCamera` under
  the loading screen, looking down on the track; the content visuals parent one `CarModel` per
  racer to it) renders ≥ 3 frames, its scene is complete (cars instanced, generated environment
  map attached — both change pipeline keys; finishing earlier left ≈ 2 s of compiles for the
  Intro) and for 3 frames the render world has neither queued nor created a pipeline (20 s
  timeout; frame counts stay low because the driver throttles an occluded window to 1 frame/s).
  Still compiled in the Intro: pickups/effects spawned in `Setup` (transparent PBR variants, ≈ 1 s).
- `OnEnter(Unloading)`: `ActiveEnvironment` removed; `Look` disappears with it.

## Quality (GDD §15, ARCH §5.8, §9)
Individual options are authoritative; the preset only picks detail (SSAO/SMAA level, 4096 shadow
maps at Ultra). Every 3D view gets `Hdr`, `TonyMcMapface`, `Msaa::Off` (SSAO needs it; post AA
instead), then per `view_quality(settings, views)`:

| | 1 view | 2 views | 3–4 views |
|---|---|---|---|
| AA | setting (TAA ok) | TAA → SMAA | TAA → SMAA |
| SSAO | preset level | ≤ Medium | off |
| motion blur | setting | setting | off |
| contact shadows | High+ | off | off |
| volumetric fog | Ultra + setting | off | off |
| sun shadows | 2048 × 4 cascades (Ultra 4096) | 2048 × 3 | Low: 1024 × 2 |

Leaving TAA removes `TemporalJitter`/`MipBias` too (they would keep jittering); prepasses this
module added are removed when no effect needs them. The split overlay camera is never touched.
Window: mode/vsync on every settings change (vsync never in automated runs: the binary presents
them unsynced), size only when the setting changes and never with
`--window` (fixed-size screenshot runs).

## Light units and the look
Physical units everywhere: sun/window/moon in lux, lamps in **real-world lumens × 4.44²**
(`lamp_lumens`; falloff is inverse-square in u), EV100 exposure per time of day. Rule of thumb used
for tuning: `EV ≈ log2(0.191 × key lux)` puts an 18 % grey under the key light at ≈ 0.25 linear;
indirect light ≈ 15 % of the key. Screenshots are sRGB: 0.18 linear shows as 118/255.

- **Outdoor:** Bevy atmosphere for the visible sky (`AtmosphereSettings`), sun `lux` is top of
  atmosphere (the atmosphere attenuates/reddens it), `SunDisk` (also the moon at night), star
  skybox at night (the atmosphere composites over it, so stars fade by themselves), cloud dome.
  IBL comes from a generated sky cubemap (`sky::sky_radiance`, regenerated when the cloud cover
  changes) scaled by `env_light` (cd/m²) — **not** `AtmosphereEnvironmentMapLight`, whose probe
  bind-group preparation panics every frame in Bevy 0.20 here.
- **Indoor:** key = sun through windows/skylights (the room shell's ceiling casts no shadows, walls
  do), fill = soft sky bounce, a generated room cubemap (walls/floor/ceiling, window band toward the
  key, lamp spots at night) for IBL, ceiling lamp grid over `Track::bounds()` at the env's
  `room.height`, window glass panes on the wall centre plane (`door: true` openings: no mullions,
  no rain on the pane; frames/openings in `track_visuals/shell.rs`). Extra env `lights` (point or
  spot, `times` filter, real-world lumens) for display spots and practicals.
- **Emissives** are not scaled by exposure (`emissive_exposure_weight = 0`), so art-bible values
  read the same in every environment. Clear-coated paints reflect the environment map: keep
  `env_light` consistent with the actual room brightness.
- **Clear-coat fix** (`materials/coat_fix.rs`): Bevy 0.20 leaves `clearcoat_N = vec3(0)` when the
  PBR fragment loads prepass normals (SSAO on, MSAA off — always the case here); the coat's Fresnel
  goes to ≈ 1 and every coated material (paints, `KHR_materials_clearcoat` props) mirrors the
  environment and reads white. The embedded `pbr_fragment.wesl` is patched once at startup to set
  the coat normal to the geometric normal (warns if Bevy's source changes). `ponytail:` remove when
  Bevy fixes it upstream.

## Weather (`weather_fx.rs`, tested)
`overcast = smoothstep(0.15, 1, cloudiness)`: sun × (1 − 0.9·overcast), exposure −1.6·overcast EV,
sky IBL × (1 − 0.35·overcast) + grey flat ambient, greyer fog, saturation −22 %. Fog: log-lerp of
visibility from the env's clear value to 40 u (`fog^0.75`), exponential-squared `DistanceFog` with
a key-light glow. Lightning: flash light (60 klx outdoors / 9 klx through windows) + clouds/glass;
*Reduce flashing*: × 0.25 through a 0.6 s low-pass. Surfaces: wetness → darker (porous more),
roughness × 0.35; puddles (noise mask on level ground) flat mirror film; snow cover grows from
noise peaks, full cover still shows ≈ 30 % of the ground, slopes stay clearer. Indoors
(`outside_only`) only the window light and glass react.

## Materials
17 procedural tileable sets (`tools/textures/gen_textures.py`, numpy only; deterministic): albedo
RGBA8 sRGB, normal RG8 (OpenGL convention, Bevy reconstructs Z), roughness RG8 (R = roughness,
G = 255 so `metallic` passes through), full mip chains (linear-light albedo mips, Toksvig roughness
widening) written as KTX2 + Zstandard by our own writer (stdlib `compression.zstd`; validated with
`ktx validate`). 32 MB total, 1024² for hero ground, 512² for walls/props. Loaded with repeat
addressing and 16× anisotropy; missing sets fall back to flat colours instead of invisible meshes.
UVs are world units; `tile_u` per set/material. Environment files override the built-in names
(`floor`, `wall`, `ceiling`, `trim`, `barrier`, `ribbon_side`, generic sets, toy palette) and map
surfaces to them. Car paint: clear coat 1.0 (0.04 solid / 0.06 metallic coat roughness, none on
chrome).

## Showroom
Night garage corner at toy scale (walls 12.5 u away on +X/+Z, where the menu camera looks): polished
epoxy floor, rubber mat with a volt-yellow marking, turntable with a cyan LED ring (emissive 5 +
point light), warm key spot with shadows, cool rim spot, moonlit window in the side wall (the env's
key light through a real opening), pegboard with tools, red tool chest, giant tyre, shelf with paint
cans, cyan/volt neon strips, a warm work lamp. EV 8.

## Verify
- `cargo test -p rc_render` (quality mapping incl. AA rules, shadow tiers, sun angles, sky model,
  cubemap orientation, half floats, weather maths, flash filter, room maps, lamp grid, window glass
  placement, materials, paint, env files, proxies, warm-up, coat fix, barrier materials).
- Screenshots: `recharged --user-data-dir target/agent-data/<id> --window 1600x900 --quick-race
  test_oval|toy_town --time <t> --weather <w> --screenshot-at 12 --screenshot <png> --exit-after 14`;
  the menu without `--quick-race`.
- Frame times: the binary's `--bench <track>_<n>p` (JSON; default settings, vsync off).
- Textures: `python3 tools/textures/gen_textures.py [--only a,b] [--preview DIR]`.

## Known limitations
- Rain on glass needs window openings from the track visuals; outside-only weather otherwise shows
  only in the window light.
- Lamp grids are uniform (no per-track lamp layouts yet); prop lamps belong to the prop visuals.
- Soft shadows: no PCSS (feature off); overcast softness comes from lower key/higher fill.
- Textures are uncompressed on the GPU (RGBA8/RG8, ~250 MB VRAM for all sets). `ponytail:` BCn
  encoding when a compressor is available.
- `--window` is needed for consistent screenshots on tiling window managers.
