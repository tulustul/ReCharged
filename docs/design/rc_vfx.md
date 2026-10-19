# rc_vfx — visual effects (design note, T14)

Juicy, readable, cheap effects for driving, items and weather, correct in split-screen. Code:
`crates/rc_vfx/src/`, shaders `assets/shaders/vfx/{particle,bubble,precip}.wesl`, lab
`crates/rc_vfx/examples/vfx_lab/`. rc_vfx only spawns visual entities (and adds `RenderLayers` to
each `GameCamera`); it never writes simulation state.

## Modules
| Module | Contents |
|---|---|
| `particles.rs`, `pool.rs`, `fx.rs` | particle data & tag packing; emitter maths and the fixed-slot pool; the `Fx` context, slot entities, settings, sprites, zigzag helpers |
| `assets.rs` | shared meshes/materials (particle quad + material, `BubbleMaterial`, item primitives, oil texture) |
| `cars.rs`, `driving.rs`, `car_events.rs`, `skids.rs` | car geometry/poses; sparks, debris, boost; landing, impact, respawn, confetti, stun; skid marks |
| `items.rs`, `projectiles.rs`, `hazards.rs`, `blasts.rs` | cubes & Golden Battery; rocket, orb, leech, EMP wind-up; goo, oil, spring pad, shield; explosions, hits, Storm Strike |
| `weather.rs`, `splashes.rs`, `lightning.rs`, `tags.rs` (+ `tags/markers.rs`) | per-camera precipitation; rain splashes; sky lightning; name tags & lock-on markers (colour-blind marker shapes) |
| `warmup.rs` | one never-culled instance of every effect material in the menu and during Loading (pipeline warm-up) |

## Schedule
Every system sits in `VfxSystems::Emit → VfxSystems::Simulate`, inside rc_core's `VisualSet`
(after `InterpolateSet` posed the visual proxies and `CameraSet` moved the cameras), gated on
`PauseState::Running`: a paused race freezes particles, skid marks, precipitation (its clock is
rc_vfx's own, not `globals.time`) and overlays in place. Effects follow the **visual proxy** of a
sim entity (`Visual` → proxy `Transform`, falling back to the entity's own `Transform`), never a
rapier body. Session entities (particle slots, skid mesh, precipitation, name atlas) are
`DespawnOnExit(RaceSession)`; item visuals, tags and markers `DespawnOnExit(RaceAttempt)`;
`OnEnter(RacePhase::Setup)` clears particles and skid marks, `OnEnter(Unloading)` forgets slots.

## Particles (`particles.rs`, `fx.rs`, `particle.wesl`)
- **One mesh, one material, many slot entities.** `MAX_PARTICLES` (4000, ARCHITECTURE §9) slot
  entities share a quad and one `ExtendedMaterial<StandardMaterial, ParticleExtension>` with
  premultiplied alpha, so Bevy batches them (also in the sorted transparent phase). A slot's
  `Transform` carries centre/size/orientation, its `MeshTag` the look: RGB565 colour (gamma 2),
  an 8-bit opacity (lit) or intensity / 40 (emissive), shape, facing mode, emissive bit
  (`pack_tag`). Emissive particles output colour with alpha 0 (additive, HDR → bloom); lit ones go
  through Bevy's PBR lighting (billboards shade as small spheres, so dust reads as volume and
  darkens in shade and at night) and blend.
- **Facings:** camera billboard (spin in the view plane), axial (stretched along velocity or a
  fixed axis — sparks, droplets, bolt segments, beams) and flat (ground rings, confetti). The
  shader builds the quad from each view's camera vectors, so every split-screen view sees
  particles facing it. Positions only span the unit cube, so frustum culling holds.
- **Shapes are procedural** (no textures, no atlas bleeding): dot, hot core, streak, noisy puff,
  ring, star, chip, square, drop, lightning-bolt icon (polygon SDF), lock-on reticle.
- **Pool:** fixed slots + free list; at the budget the next emission replaces a live particle
  round-robin, so new events always show. Budget = 4000 × density (Low 0.35 / Medium 0.65 /
  High 1.0; 3–4 views drop one step), emission rates × the same factor, and driving emitters
  thin out with distance to the nearest camera (100 % within 25 u → 15 % at 90 u).
- Randomness is a local SplitMix64 (`FxRng`): presentation-only, it must not consume rc_core's
  gameplay RNG streams.
- **Soft particles:** billboards (facing mode 0) fade out where they meet opaque geometry, over
  half their size in front of the scene depth (`soft_fade` in `particle.wesl`, from the view's
  depth prepass), so big puffs no longer cut a hard line into the ground. Axial and flat
  particles keep their edges (sparks, ground rings lie on the ground by design). Views without
  a depth prepass (no SSAO, TAA or motion blur: Low quality) skip it.

## Driving (`cars.rs`, `car_events.rs`, `skids.rs`)
Wheel contact patches come from each model's `models/cars/<car>.wheels.ron` (art contract; loaded
for the race's cars in `OnEnter(Loading)`, default layout + warning if missing) plus
`WheelState::offset`, on the drawn (interpolated) car, then moved onto the ground plane
rc_vehicle's wheel ray hit (`WheelState::contact` / `normal`, `cars::ground_point`): skid marks
(laid within that plane), sparks and debris sit exactly on ramps and curbs. The car's velocity
comes from telemetry (`forward_speed`, `speed × sin(slip)`).

| Effect | Trigger | Look |
|---|---|---|
| Drift sparks | `drifting` and `charge_rate > 0` (sparks teach the mechanic) | streaks + hot spot at the rear contacts; colour amber → electric cyan → violet and width 0.035 → 0.1 u with `charge_rate / 0.22` (smoothed); rate 70–270/s per wheel |
| Debris | grounded wheel, by surface & weather (`debris_for`) | dust + pebbles (dirt), grass chips (grass), mud blobs (wet dirt), spray + white sheets (water, wet hard floors), powder (snow cover), tyre smoke (hard floors, only while sliding) |
| Boost | `boosting` | electric plasma jet + flame (Turbo Cell: warm palette), crawling arcs, start flash + ring |
| Landing | `CarLanded` ≥ 0.12 | radial puffs in the ground's colour; clean landing adds a cyan charge ring |
| Impact | `CarImpact` ≥ 0.12 | sparks around the normal, flash for strong hits |
| Respawn | `CarRespawned` | `ToTrack`: scan ring rising through the car, beam, converging sparkles; flip: sparkles |
| Finish | `RaceEvent::Finished` of a local player | 160–240 tumbling confetti, gold sparkles for 1st |
| Stun | `StatusEffects::power_mul() < 0.05` (EMP, Storm Strike) | blue arcs crawling over the body |
| Skid marks | slip > 0.35 on hard floors, > 0.15–0.2 on grass/dirt, always in snow | dark rubber, torn grass, ruts, grey snow tracks; segments every 0.3 u; fade 14 → 22 s; ring of 2400 × density segments; one dynamic mesh rebuilt only when a segment is added or while marks fade (buffers reused) |

## Items (`items.rs`, `hazards.rs`, `blasts.rs`)
Visuals are separate entities bound to their item by `Follows` (despawned when the item is gone)
and drawn at `ItemRadius` wherever a reach matters — **what you see is what triggers**.

| Item | Visual |
|---|---|
| Volt Cube (`Pickup`, `ItemTimer`) | translucent glowing cube spinning on a diamond tilt, bolt icon billboard, halo; taken (timer > 0): burst + ring, hidden; back at 0: elastic pop |
| Golden Battery | gold battery (warm emissive so it stays golden without reflections), halo, drifting sparkles; `RaceEvent::GoldenBattery`: golden burst |
| Spark Rocket | rocket model oriented along its motion, exhaust glow, smoke + sparks per 0.18 u of travel; `Explosion`: flash, fireball, sparks, smoke, debris, ground ring sized by the blast radius |
| Lock-on | reticle over `ItemTarget` of a rocket, snapping in then pulsing, on every camera's layer but the victim's (the victim has its HUD arrow) |
| Shock Orb | plasma core, halo, crackling arcs, faint field shell at its knock radius |
| Leech Coil | two wobbling acid-green strands owner → target, charge motes flowing to the owner; sparks orbit the coil while it searches |
| EMP | wind-up shell swelling around the user + converging sparks; pulse: two rings racing to the pulse radius, an expanding rim-only shell, radial arcs |
| Goo Bomb | glossy slime blob on its carrier, red light blinking 1 s → 0.12 s as the fuse runs out, drips; last 2 s: red ring of the blast radius on the ground; blast: slime splatter across the radius |
| Oil Slick | dark puddle with thin-film sheen (generated texture) at its radius; spreads in 0.4 s, shrinks over its last 1.5 s |
| Spring Pad | rubber base, chrome coils, yellow plate, glowing ring at its radius; bounces (damped spring) + sparks when it launches someone |
| Shield Bubble | fresnel energy shell (`bubble.wesl`) around the car; flashes on blocked hits, blinks in the last 2 s, bursts into shards when it ends |
| Storm Strike | `HitEvent { by: StormStrike }`: jagged bolt (midpoint displacement) from 26 u above, branches, glow, flash, sparks |
| Hits | orb / leech / EMP zaps, oil splashes, blue deflection sparks on blocked hits |

## Weather (`weather.rs`, `precip.wesl`)
- `On<Add<GameCamera>>` gives the camera `RenderLayers [0, 1 + slot]` (`player_layer`) and spawns
  its precipitation mesh on its own layer: 14 000 quads animated in the vertex shader, wrapped
  in a box around the viewing camera (world-anchored, so the camera drives through falling rain),
  faded at the box edges and near the eye. Drops above `amount` collapse.
- Snow vs rain from `WeatherState::snowing`; `amount = precipitation × density × split` (1 view
  1.0, 2 views 0.7, 3–4 views 0.5); fall 40 u/s (rain) / 4 u/s (snow) plus the wind, so a storm
  slants ≈ 20°; colour scaled by time of day and cloud cover so night rain does not glow. Indoor
  tracks (`outside_only`) show nothing here: rc_render draws weather through the windows.
- Splashes (`splashes.rs`; ring + droplets) on wet ground 1.5–13 u ahead of each local player's
  car while it rains, each on the ground under it (B30): within 3 u of the car on the plane its
  wheels touch (`cars::ground_plane`: mean contact point and normal of the grounded wheels — also
  ramps and other prop surfaces); farther out on the track surface closest to that plane — the
  banked ribbon (`Track::project` + `frame_at`, outside jump gaps) or a ground polygon — if one lies
  within 0.6 u of it, else no splash (a gap, a ramp's edge, under a bridge). None while the car is
  airborne or on ground steeper than 60°.
- Sky lightning per camera on `WeatherEvent::Lightning`: a bolt ribbon mesh 150–230 u ahead of
  that camera, on its layer, flickering through a strike and two re-strikes (`strike_brightness`;
  the sky flash itself is rc_render's). The main channel (1.8 u wide) falls from just under the top
  edge of that camera's view (`bolt_height`: pitch + half the vertical FOV − 3.4°, 45–160 u above the
  ground point) to the horizon: its lower part is usually hidden behind houses, fences and trees,
  the upper part shows over them, with three branches forking downward and two crawlers forking
  almost horizontally along the cloud base from the top (`sky_bolt_paths`, ≤ 55 u), so a storm reads
  as lightning in motion, not only as heavier rain (QA). Until T20c no bolt was ever seen: rc_render's
  cloud dome sorted after it among the transparents and covered it (the dome is now drawn first).
  Burst captures (every 0.1 s): `target/agent-data/T20c/burst/`, sheet `shots/lightning_fix.png`.

## Overlays (`tags.rs`)
Names are rendered once per attempt into a 512 × 512 atlas by an offscreen `Camera2d` (`Text2d`
rows in the game's display font with a drop shadow, layer 30). Each camera gets a tag over every
racer except its own, on its own layer: a screen-aligned quad showing the racer's row, tinted
with the racer colour (lifted toward white), sized ≈ constant on screen (0.05 u per u, 0.3–1.6 u:
the cap shrinks far tags, the floor only applies while fading), fading out between 6 u and 3 u
(each camera's tag has its own material copy; alpha in 1/16 steps, so the material changes only
now and then) and hidden beyond 75 u — a car alongside is no longer covered by its name. Tags are
occluded by the world like any 3D object.
- *Colour-blind markers* (setting, toggled live): each row also shows the racer's rc_core
  `RacerMarker` left of the name — a shape sprite for rivals (● ▲ ■ ◆ ★ ✚ ⬟ ⬢ by grid index, from a
  strip rasterized at session start with 4×4 supersampling; the game font has no such glyphs) with
  the text's drop shadow, the player number for humans (split-screen names already are P1–P4, then
  nothing is added). The HUD's minimap and standings draw the same shapes (`ui/src/hud/markers.ts`).
  Sprites share the text's sprite pipeline: no new pipeline to warm up.

## Pipeline warm-up (`warmup.rs`)
Bevy compiles a render pipeline the first time a material/mesh combination is drawn; compiles in
the Intro froze it for ≈ 1 s (pickup glass, sprites, name tags), later ones hitched mid-race
(skid marks, shields). `OnEnter(Loading)` and, in human runs, the menu (once the showroom is
ready and no shader asset has arrived for 1 s, as rc_render's menu warm-up waits) spawn one
`NoFrustumCulling` instance of every pair the effects draw — particle quad, item parts (shadowless and
shadow-casting), Volt Cube glass, oil (same pipeline as the name tags), skid-mark mesh and
material, bubble shell, lightning bolt, precipitation (amount 0) — scoped to that state (in the
menu at (0, −600, 0), rc_render's menu warm-up scene, out of the showroom's view). Every 3D camera
draws them: rc_render's menu warm-up cameras compile them for every view variant once per run,
its Loading warm-up camera whatever the race still needs (`docs/design/rc_render.md`). New effect
materials or mesh layouts belong in this list.

## Reduce flashing (GDD §10.2, §18)
`Settings.accessibility.reduce_flashing` → `Fx::calm`: sky lightning becomes a slow soft glow
low over the horizon (no bolt); the Storm Strike bolt is replaced by a soft column of light that
swells and fades, without flash or sparks; explosions swap the flash for a slowly swelling soft
glow and dim, fade in and slow down their fireball and sparks; EMP rings fade in at a third of
the intensity with a faint shell and fewer arcs; boost start flashes are skipped; impact,
shield, clean-landing and pickup flashes become soft glows; the lock-on reticle and the goo
blast ring stop pulsing.

## Tests
`cargo test -p rc_vfx` (40): emitter rate maths (fractions carried, frame-rate independence),
pool recycling (reuse, round-robin replacement, budget shrink), tag packing round trip and
clamping, fade/size curves, velocity stretch, density maths, RNG cone; wheel files (every car
model has one), spark colour/size mapping, debris and mark tables, car velocity, ground contacts
on the wheel ray's plane (curb face, airborne fallback, mean plane), splashes on that plane and
far ones snapped to Toy Town's banked, raised ribbon (none 3 u above it); skid ribbon
building (joined segments, restart after jumps, cap keeps the newest, 2 triangles per segment,
fading); goo blink period; bubble size; respawn pop; bolt paths; precipitation look (snow by
`snowing`, density × setting × views, indoor → none, wind slant, night dimming), precipitation
mesh, lightning flicker, layers; atlas rows/UVs, tag size and near fade, marker shapes; *Reduce flashing* dims the rocket,
goo, EMP and Storm Strike blasts to ≤ 40 % of their peak. Headless ECS: a drifting racer emits
only while `PauseState::Running` and a restart clears every particle; eight drifting cars at Low
never exceed the budget; each camera sees only its own precipitation.

## How to verify
- Lab scenes (stand-in entities and messages; `--slow 0.25` for transient flashes, `--calm` for
  *Reduce flashing*, `--split` for two views, `--wet` for wet ground, `--night`; the exit line
  prints fps and the peak particle count):
  `cargo run -p rc_vfx --example vfx_lab -- --scene
  <shapes|drive|sparks|items|blasts|events|weather> [--chase] [--snow] --screenshot <png>
  --screenshot-at 2,3 --exit-after 4`. The lab's views have a depth prepass like the game's;
  `--hard` leaves it out to compare soft particles (`--scene events`: landing puffs).
- Warm-up: `RUST_LOG=rc_render=debug` names every pipeline compiled; after the warm-up a race's
  start logs `race start: 0 pipelines compiled after the warm-up` (rc_render).
- Game: `cargo run -p recharged -- --user-data-dir target/agent-data/T14 --quick-race test_oval
  --autopilot [--weather rain|snow|storm] [--players 2] --window 1280x720 --screenshot <png>
  --screenshot-at 40 --screenshot-every 2 --exit-after 70`.

## Known limitations
- Far splashes know only the track's ribbon and ground polygons (no physics queries): none appear
  on prop surfaces (ramps, table tops without a ground polygon) more than 3 u from the car.
- Name tags are occluded by geometry.
- Soft particles need the view's depth prepass: at Low quality (no SSAO, TAA or motion blur)
  puffs keep a hard edge where they meet the ground.
- The lab, not a race, demonstrates effects a quick race does not produce on demand (every item,
  confetti, respawns).
