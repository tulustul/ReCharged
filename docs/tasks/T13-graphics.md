# T13 — Graphics (`rc_render`) — split into T13a and T13b

Two agents work in `crates/rc_render` at the same time with **module ownership**. T13a owns
`lib.rs` (and registers T13b's sub-plugin `ContentVisualsPlugin` with one line when asked).

**Read first (both):** PRACTICES; ARCHITECTURE §1–4, §3.3 (visual proxies), §5.8 (camera rig & AA
rules), §5.9; GDD §4 (cars), §10 (weather/time), §11, §15 (graphics settings), §17 (visual
direction); `docs/design/art-bible.md`; digests: bevy §0, §5 (assets/glTF — `WorldAssetRoot`),
§6–§10 (meshes, materials & **WESL** shaders, lighting, atmosphere, cameras, post-processing, AA),
§16.

## T13a — rendering core
**Owned:** `crates/rc_render/src/{lib.rs, proxy.rs, quality/**, lighting/**, sky/**, weather_fx.rs,
materials/**, showroom/**}`, `assets/data/environments/*.ron`, `assets/textures/**`,
`assets/shaders/render/**`, `tools/textures/**`, `docs/design/rc_render.md`.
1. **Visual proxies** for every `Interpolated` entity (one-to-one relationship `VisualOf`/`Visual`
   with `linked_spawn`), posed in `InterpolateSet` from `PoseHistory` and the overstep fraction.
2. **Quality**: `Settings.graphics` (+ split-screen tier) → camera components on `GameCamera` /
   `MenuCamera` (HDR, tonemapping, bloom, SSAO, AA with the rules of ARCHITECTURE §5.8 — TAA single
   view only, never MSAA with SSAO —, motion blur, distance fog, volumetric fog only at Ultra in
   single view), shadow map size & cascades per tier; window mode (windowed/borderless), size,
   vsync; applied live on change.
3. **Environments** (`environments/<env>.ron` for the six environments of the art bible + a
   neutral `test` env): sky (atmosphere + sun by `TimeOfDay`) for outdoor; window light, lamps/work
   lights and moonlight for indoor; ambient/environment light probes; per-environment colour
   grading; headlights (shadowless spots) for local players' cars at Night.
4. **Weather visuals** from `WeatherState`: cloudiness (sun/shadow softness, sky tint), fog,
   lightning flashes (reduce-flashing variant), wet ground (darker & glossier materials),
   outside-only weather through windows.
5. **MaterialLibrary**: PBR surface materials for every `SurfaceKind` and environment material list
   (wood, rug/carpet, tile, marble, vinyl, toy plastic, grass, dirt, asphalt, concrete, cardboard,
   stone, rubber, metal) with textures — generated procedurally (`tools/textures/`, numpy/PIL:
   albedo, normal, roughness, tileable) and/or CC0 sources (report URLs + licence); car paint
   materials (clear-coat) from `PaintInfo`.
6. **Showroom**: the menu garage diorama (turntable, lighting, backdrop props) showing
   `MenuBackdrop.car/paint`, built at `rc_core::SHOWROOM_FOCUS` with the car facing −Z (rc_camera's
   `MenuCamera` orbits that point). Camera decoration: add HDR/bloom/AA/etc. to `GameCamera` /
   `MenuCamera` via `On<Add<…>>` + settings changes (rc_camera adds no post-processing; the split
   overlay camera must stay untouched).
7. Registers & completes the `"visuals"` loading key for its preloads.

## T13b — content visuals
**Owned:** `crates/rc_render/src/{content.rs, track_visuals/**, props/**, cars/**, capture/**}`
(exposes `ContentVisualsPlugin`), `docs/design/rc_render_content.md`.
1. **Track visuals** from `TrackGeometry`: styled ribbons (ToyTrack, Asphalt, CarpetRunner; none for
   Invisible), walls/barriers in the environment's style, ground polygons, room shell; materials
   from T13a's `MaterialLibrary`.
2. **Props**: glb scenes (`WorldAssetRoot`) on `Prop` proxies from `props.ron` model paths;
   primitive fallback meshes when a model is missing; decorative-only props.
3. **Cars**: car glb on the racer's proxy, paint applied to the `paint`/`paint_accent` materials,
   wheel spin/steer/suspension from `VehicleTelemetry.wheels` (nodes `wheel_*`), brake/head lights
   (`light_*` materials), ghost translucency (`GhostCar`, respawn blink from `StatusEffects`).
4. **Captures**: `--capture-previews` (track preview images into `assets/ui/tracks/`),
   `--track-tour <track>` (screenshots along the racing line).
5. Registers & completes the `"content"` loading key (track meshes built, props & car scenes loaded).

## Tests (both)
Pure: quality-tier mapping (incl. AA rules), time-of-day sun angles, weather blend maths, mesh
conversion from `MeshData`, wheel transform maths, paint conversion. Visual verification with
screenshots (`--quick-race test_oval|toy_town` at each time of day and weather, 1P and 2P) inspected
and attached to the report.

## Acceptance
Tests, clippy, fmt clean; screenshot set showing a convincing, well-lit scene; frame time within
budget on the dev GPU (report numbers).
