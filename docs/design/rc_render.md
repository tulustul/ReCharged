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
`car_status.wesl` (status effects on car bodies),
`clouds.wesl` (cloud deck), `window_glass.wesl` (outside light, rain beads/streaks, lightning).

## Lifecycle
- Startup: texture sets queued, `MaterialLibrary` with built-in materials.
- `OnEnter(Menu)`: `showroom` environment (+ set, car). `ShowroomReady` 2 frames after the car is
  instanced, painted and shown; a swap keeps the old car until the new one is ready.
- **Menu warm-up** (`lighting/menu_warmup.rs`, QA Q9/B31; human runs only): once per run, once the
  showroom is ready and no shader asset has arrived for 1 s, hidden stage cameras far below it
  (y = −600) draw what every race draws
  whatever its track — every catalog car (+ a Time Trial ghost), rc_vfx's effect instances, the four
  surface-material variants (texture set or not × clear coat), common prop-material variants (plain,
  clear-coated, blended, masked, double-sided; with tangents with and without a normal map), window
  glass (indoor views) and the cloud deck (outdoor views), under a shadowed point light (cube-map
  shadow pipelines, B28) — once per view variant a race can use: 1, 2 and 4 player views
  (`ViewsOverride`: the split-screen quality tiers), indoors and outdoors (`ForceOutdoor`: Bevy's
  atmosphere is part of every forward pipeline key). Six stages, the most common (1 view, indoor)
  first, each when the previous one has compiled. Two transparent replicas of the split-screen UI
  overlay (`Camera2d`, `Msaa::Off`) compile their sprite and UI pipelines with stage 1 and switch on
  their blits after it, while nothing else compiles: one the overlay's premultiplied blit, one alpha
  blended like a second player's view (B29). Human runs compile on Bevy's task threads, so the menu
  keeps its frame rate as long as
  nothing makes a frame wait for them: Bevy compiles a *blit* pipeline blocking the frame, behind
  everything queued, and a camera's blit depends on its place on the window (the first replaces,
  later ones blend) — so the stage cameras come after every window camera and write nothing to it
  (`CameraOutputMode::Skip`); ordered first they changed the menu camera's blit and froze the splash
  for 1–2.5 s. A shader asset arriving while compile tasks hold Bevy's shader cache stalls the frame
  too, hence the quiet second. Each stage logs `visuals: menu warm-up stage k/6 (…): N pipelines in
  X s, worst frame Y ms` (`RUST_LOG=rc_render=debug`: every menu frame over 25 ms). Automated runs skip it
  (they compile synchronously: a stage would freeze the menu for seconds) — as does leaving the menu
  early; the Loading warm-up then compiles what a race needs. Measured: *Verify*.
- `OnEnter(Loading)`: the track's environment (`TrackInfo.environment`; missing/broken file → `test`
  with an error log), library rebuilt for it, lights/atmosphere/clouds spawned
  (`DespawnOnExit(RaceSession)`). `"visuals"` completes when textures are loaded **and** the Loading
  warm-up is over: a hidden `WarmupCamera` (a `QualityCamera` under the loading screen, looking down
  on the track; the content visuals parent one `CarModel` per racer to it) exists from the first
  frames of Loading but draws only once its view is final — quality, environment map, fog — plus 3
  frames (`SETTLE_FRAMES`: the render world sees a new environment map's images a frame or two late,
  and a view drawn without them compiles a second, useless set of variants without
  `ENVIRONMENT_MAP`), so it compiles while the other loaders still load. It is done once every other
  loader is done, the scene is complete (cars instanced) and for 3 frames the render world has
  neither queued nor created a pipeline (≥ 3 frames, 20 s timeout; frame counts stay low because the
  driver throttles an occluded window to 1 frame/s). While Loading, quality and shadow tiers count
  the race's local players as views (split-screen variants); the key light's cascades reach the
  ground under the high camera (prop shadow pipelines, else compiled when the race camera first
  sees a prop: Aisle Rush, Maple Street); a track with shadowed lamps (the garden porch light at
  night) gets a shadowed point light at the warm-up cars (B28). rc_vfx adds one never-culled
  instance of every effect material (`rc_vfx/src/warmup.rs`), and a 2 px bevy_ui node (background,
  Oklab gradient, box shadow) targets the warm-up camera: bevy_ui compiles per target format, and a
  1P race hosts its HUD on the HDR game camera while the loading camera is LDR. The split overlay
  cannot be replicated here (the loading screen's 2D camera uses 4× MSAA on the same window: mixed
  sample counts are a wgpu validation error), so a split-screen quick race (no menu) still compiles
  its sprite/blit pipelines at the start of the Intro (2 small pipelines, < 30 ms). GO of a session's
  first race logs what the warm-ups missed — `visuals: race start: N pipelines compiled after the
  warm-up, worst frame X ms` — and `RUST_LOG=rc_render=debug` names every pipeline the render world
  creates (label, shaders, set shader defs; fragment-only defs after `| fragment`) and, after GO,
  every frame slower than 25 ms with the number of pipelines it created.
- `PipelineQueue` (pub): pipelines queued/compiling and created, published by the render world
  every frame; the binary waits for an idle queue before a human run exits (B2).
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

Motion blur runs at shutter 0.5 (2 samples) and eases to 1.25 (4 samples) while the view's
player spends charge (`VehicleTelemetry::charge_boost`). Leaving TAA removes `TemporalJitter`/`MipBias` too (they would keep jittering); prepasses this
module added are removed when no effect needs them. The split overlay camera is never touched.
Window: mode/vsync on every settings change (vsync never in automated runs: the binary presents
them unsynced), size only when the setting changes and never with
`--window` (fixed-size screenshot runs).

## Light units and the look
Physical units everywhere: sun/window/moon in lux, lamps in **real-world lumens × 4.44²**
(`lamp_lumens`; falloff is inverse-square in u), EV100 exposure per time of day. Rule of thumb used
for tuning: `EV ≈ log2(0.191 × key lux)` puts an 18 % grey under the key light at ≈ 0.25 linear;
indirect light ≈ 15 % of the key (physical; `test`). The garden and street day looks use ≈ 3× the
physical sky fill (`env_light` 15–32 k) so shade under trees, fences and parked cars stays readable
(≈ 0.3 of the sunlit ground, art review). Screenshots are sRGB: 0.18 linear shows as 118/255.

- **Outdoor:** Bevy atmosphere for the visible sky (`AtmosphereSettings`), sun `lux` is top of
  atmosphere (the atmosphere attenuates/reddens it), `SunDisk` (also the moon at night), star
  skybox at night (the atmosphere composites over it, so stars fade by themselves), cloud dome
  (a 1500 u premultiplied sphere drawn before every other transparent, `depth_bias` −1e6: sorted
  by its centre it came after farther transparents — sky bolts, windows against the sky — and hid
  them under the cloud cover).
  IBL comes from a generated sky cubemap (`sky::sky_radiance`, regenerated when the cloud cover
  changes) scaled by `env_light` (cd/m²) — **not** `AtmosphereEnvironmentMapLight`, whose probe
  bind-group preparation panics every frame in Bevy 0.20 here. Bevy re-filters a
  `GeneratedEnvironmentMapLight` every frame, so it lives on one probe entity and every camera
  shares the filtered maps (`EnvironmentMapLight` copies; split screen used to filter per view).
- **Indoor:** key = sun through windows/skylights (the room shell's ceiling casts no shadows, walls
  do), fill = soft sky bounce, a generated room cubemap (walls/floor/ceiling, window band toward the
  key, lamp spots at night) for IBL, ceiling lamp grid over `Track::bounds()` at the env's
  `room.height`, window glass panes on the wall centre plane (`door: true` openings: no mullions,
  no rain on the pane; frames/openings in `track_visuals/shell.rs`). Extra env `lights` (point or
  spot, `times` filter, real-world lumens) for display spots and practicals. Per time of day:
  `lamps` = room lamp level 0–1 (supermarket night 0.15), `practicals` = props' lamp level
  (default on at Night only); `fixture_materials` names decor fixtures that follow `lamps`.
- **Indoor times of day** (QA Q8, Toy Town and Sofa Circuit): exposure is *not* normalised per time
  — morning sits a little darker than noon with a low golden key and cool (bluish) fill and room
  bounce; noon is the brightest, even look; sunset keeps a warm key but a cool sky fill and neutral
  room bounce (no red-brown wash), ceiling lamps at 0.6; night is ≈ 4 EV darker than noon: a cool
  moonlight key (≈ 60–70 lux, window patches) and a faint cool fill, warm narrow-cone ceiling spots
  (Toy Town 56°) or practical lamps drawing pools, and enough bounce that the floor stays readable.
  Contact sheets: `target/agent-data/T20c/shots/` (track tours, fixed poses).
- **Emissives** are not scaled by exposure (`emissive_exposure_weight = 0`), so art-bible values
  read the same in every environment. Clear-coated paints reflect the environment map: keep
  `env_light` consistent with the actual room brightness.
- **Clear-coat fix** (`materials/coat_fix.rs`): Bevy 0.20 leaves `clearcoat_N = vec3(0)` when the
  PBR fragment loads prepass normals (SSAO on, MSAA off — always the case here); the coat's Fresnel
  goes to ≈ 1 and every coated material (paints, `KHR_materials_clearcoat` props) mirrors the
  environment and reads white. The embedded `pbr_fragment.wesl` is patched once at startup to set
  the coat normal to the geometric normal (warns if Bevy's source changes). `ponytail:` remove when
  Bevy fixes it upstream.

## Baked indirect light (`lighting/gi.rs`, tested)
If `assets/data/lighting/<track>/<normal|mirror>_<time>.irv` exists (reverse layouts use the
forward file), the session gets one `LightProbe + IrradianceVolume` over its bounds: inside it,
Bevy takes *diffuse* indirect light from the volume instead of the camera's environment map (whose
specular stays); without a file nothing changes. Values are irradiance / π (cd/m²-like, they
multiply the diffuse colour directly); `Look::env_light / clear env_light` scales them with the
weather. Binding arrays are off (upstream U7), so Bevy uses one volume per view — one per session
is all there is. Verified on the Iris Xe (Mesa): no hang, faces where the shader samples them
(rows `[0, y)` = +axis normals, `[y, 2y)` = −axis — `bevy_pbr`'s module docs say the opposite;
the shader wins). The editor bakes the files; `keep_cpu_copies()` (editor only) keeps rc_render's
meshes, materials and textures on the CPU for it.

## Weather (`weather_fx.rs`, tested)
`overcast = smoothstep(0.15, 1, cloudiness)`: sun × (1 − 0.9·overcast), exposure −1.6·overcast EV,
sky IBL × (1 − 0.35·overcast) + grey flat ambient, greyer fog, saturation −22 %. Every look keeps
white balance and sectional grading in use (a 1e-4 temperature / midtone gamma where it would
grade nothing): Bevy keys the tonemapping pipeline on those two stages, clouds and snow switch them
on, and the race's weather is planned only at Setup — after the Loading warm-up drew the
menu-default clear look — so overcast races compiled a tonemapping pipeline at the Intro (QA R5)
and a Dynamic change from Clear one mid-race. One tonemapping pipeline serves every environment,
time and weather (test). Fog: log-lerp of
visibility from the env's clear value to 25 u at density 1 (`fog^0.75`; the Fog preset, 0.85, sees
≈ 40 u: the next corner, not the far fence), exponential-squared `DistanceFog` (clear near field,
dense at distance) with a key-light glow. Fog also acts as a low cloud deck (overcast from
`max(cloudiness, 0.9·fog)`: veiled sun, soft shadows, grey sky and cloud dome), and its colour turns
from the clear-weather sky haze into a pale, slightly cool grey of the same brightness
(`fog_tone`, 85 % at density 1) — it reads as fog, not haze (QA). Indoors fog is ignored. Lightning: flash light (60 klx outdoors / 9 klx through windows, at most
2.5 × `exposed_lux(EV)` = 1.2π·2^EV, the illuminance the exposure renders as mid grey: a noon
strike as before, a night or sunset one a bright flash instead of a white frame) + clouds/glass;
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
UVs are world units; `tile_u` per set/material; `hue_variation` drifts macro patches toward a
drier tint (lawns). Water: light muddy base under a clear coat (puddles mirror the sky instead of
reading as holes). Environment files override the built-in names
(`floor`, `wall`, `ceiling`, `trim`, `barrier`, `ribbon_side`, generic sets, toy palette) and map
surfaces to them. Car paint: clear coat 1.0 (0.04 solid / 0.06 metallic coat roughness, none on
chrome).

## Showroom
Night garage corner at toy scale (walls 12.5 u away on +X/+Z, where the menu camera looks): polished
epoxy floor, rubber mat with a volt-yellow marking, turntable with a cyan LED ring (emissive 5 +
point light), warm key spot with shadows, cool rim spot, moonlit window in the side wall (the env's
key light through a real opening), pegboard with tools, red tool chest, giant tyre, shelf with paint
cans, cyan/volt neon strips, a warm work lamp. EV 8. `MenuBackdrop::focus == Room` (Garage grid,
Credits roll, Cup-end podium, and the card screens it would show through: Cup select, Track select,
Versus lobby) hides the turntable car — the room stays and `ShowroomReady` with it; the camera keeps
the wide shot.

## Verify
- `cargo test -p rc_render` (quality mapping incl. AA rules, shadow tiers, sun angles, sky model,
  cubemap orientation, half floats, weather maths, one tonemapping key for every look, flash
  filter, room maps, lamp grid, window glass placement, materials, paint, env files, lamp/fixture
  levels, proxies, warm-up, coat fix, barrier materials, tour clearance, cutaway previews).
- Screenshots: `recharged --user-data-dir target/agent-data/<id> --window 1600x900 --quick-race
  test_oval|toy_town --time <t> --weather <w> --screenshot-at 12 --screenshot <png> --exit-after 14`;
  the menu without `--quick-race`.
- Frame times: the binary's `--bench <track>_<n>p` (JSON; default settings, vsync off).
- Race-start hitch: the `visuals: race start: …` log line of any `--quick-race` run. T19b (toy_town
  1P, dev build, 1280×720, no other game running): before the effects/UI warm-up 6 pipelines
  compiled after the warm-up and the worst frame took 763 / 808 ms; after, 0 pipelines and 27 / 25
  ms; Loading → InRace 3.8–4.3 s → 4.6–4.8 s (the compiles moved under the loading screen).
- Loading time and late compiles (T20c; 1280×720, nothing else running; `log_scan.py` loads =
  Loading → Intro). Automated runs compile synchronously and skip the menu warm-up, so a
  `--quick-race` first load is the worst case: release 3.7–4.8 s (before 4.3–5.2 s), dev 4.1–5.9 s
  (Toy Town / Backyard Bash, 1P / 4P). Through the menu once the warm-up is done (a build that runs
  it in automated runs too): 0.22–0.77 s in 1P, 2P and 4P (before 2.7–4.2 s), GO logs 0 late
  pipelines (before 2–3 in split-screen). Repeat loads 0.2–0.35 s, as before. Human runs (no
  flags, `RUST_LOG=rc_render=debug`): the warm-up takes 11–13 s from the splash, worst frame
  ≤ 43 ms. Night Backyard Bash: nothing compiles after GO (before: 3 cube-map shadow pipelines
  4 s in). `--bench`, dev, 1080p: max frame 7–15 ms in all 12 QA scenarios (QA: 26–33 ms in 9).
- Weather (T22b, QA R5): non-clear `tools/scripts/qa/weather_time.sh` cells with
  `RUST_LOG=rc_render=debug` (Backyard cloudy/rain/fog, Maple storm/snow, Toy Town storm night,
  Wrench storm, Sofa rain sunset) log `race start: 0 pipelines` (T21: 1, the white-balance
  tonemapping variant); clear cells compile the same single variant.
- Textures: `python3 tools/textures/gen_textures.py [--only a,b] [--preview DIR]`.

## Known limitations
- The menu warm-up needs ≈ 12 s after the splash: a race started sooner (or a `--quick-race`)
  compiles the rest under its loading screen (≈ 4–5 s), and a split-screen quick race compiles the
  overlay's sprite/blit pipelines at the start of the Intro.
- Rain on glass needs window openings from the track visuals; outside-only weather otherwise shows
  only in the window light.
- Lamp grids are uniform (no per-track lamp layouts yet); prop lamps belong to the prop visuals.
- Soft shadows: no PCSS (feature off); overcast softness comes from lower key/higher fill.
- Textures are uncompressed on the GPU (RGBA8/RG8, ~250 MB VRAM for all sets). `ponytail:` BCn
  encoding when a compressor is available.
- `--window` is needed for consistent screenshots on tiling window managers.
- `preprocess_fix.rs` works around a Bevy 0.20 leak (`docs/upstream-notes.md` U5): bevy_pbr never
  prunes the GPU-preprocessing bind groups of views that went away; we retain them against the live
  `ExtractedView`s every frame. Remove it once Bevy prunes them itself.
