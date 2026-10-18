# T14 — Visual effects (`rc_vfx`)

**Goal:** juicy, readable, cheap effects for driving, items and weather, correct in split-screen.

**Read first:** PRACTICES; ARCHITECTURE §1–4, §5.10; GDD §5.3 (charge feedback), §6 (items),
§10 (weather visuals), §13 (name tags/lock-on via 3D), §17 (VFX list), §18 (reduce flashing);
digests: bevy §0 (WESL), §7, §9 (render layers), §16 (many small objects — rain verified there).

## Owned paths
`crates/rc_vfx/**`, `assets/shaders/vfx/**`, `assets/textures/vfx/**`, `docs/design/rc_vfx.md`.

## Deliverables
1. **Particle system**: pooled, CPU-simulated, shared meshes/materials (batched), bounded by the
   particle-quality setting; runs only while `PauseState::Running`.
2. **Driving FX** from telemetry & messages: drift sparks (colour **and** size by charge-gain rate),
   wheel debris by surface (dust, grass, mud, water splashes), boost flame + electric arcs, landing
   dust, impact sparks, skid marks (ribbon meshes on the ground, fading, capped), respawn
   materialise, finish confetti.
3. **Item FX**: Volt Cube pickups (glow, spin, respawn pop), rockets (trail, explosion), leech tether
   beam, shock orb, EMP ring, goo bomb (pulsing, beeps visual), oil slick, spring pad, shield
   bubble, storm strike lightning bolt; Golden Battery sparkle.
4. **Weather FX** per `GameCamera` on its own `RenderLayers`: rain streaks / snow around the camera
   (choose snow vs rain from `WeatherState.snowing`, not `kind`; density from `precipitation` × quality), splashes on wet ground, lightning bolt in the sky.
5. **Per-camera 3D overlays**: opponent name tags (slot colours / rival colours) and rocket lock-on
   markers, billboarded per camera.
6. Reduce-flashing variants for lightning, Storm Strike, EMP, explosions, boost flashes.

## Tests
Pure: emitter rate maths, pool recycling, skid-mark ribbon building, colour/size mapping. Visual:
screenshots of each effect group (driving on each surface, every item, rain/snow, 2P precipitation
isolation) inspected and attached.

## Acceptance
Tests, clippy, fmt clean; particle counts within budget (ARCHITECTURE §9); screenshots.
