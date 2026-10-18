# rc_render — content visuals (T13b design note)

Track, prop and car visuals and the capture tools of `rc_render` (`content.rs`,
`track_visuals/`, `props/`, `cars/`, `capture/`). The rendering core (proxies, quality, lighting,
sky, weather, `MaterialLibrary`, showroom) is T13a's; see `docs/design/rc_render.md`.

## Loading
`ContentVisualsPlugin` registers the `"content"` key. During `Loading` it waits for `Track`,
`ActiveEnvironment` and a `MaterialLibrary` rebuilt for that environment, then builds the track
meshes, dresses the props, spawns the decor, queues the car scenes of `RaceConfig` (racers + Time
Trial ghost) and completes the key when every scene is loaded. A car scene that fails (missing or
broken glb) fails the loading with its path; a prop scene that fails only logs. Scene handles are
cached for the app's lifetime, so restarts and next races reuse them.

## Track (`track_visuals/`)
| Part | Material (library) | Shadows |
|---|---|---|
| ribbon top | `surface(run.surface)` | cast |
| ribbon lips & sides, `ToyTrack` | `surface(ToyTrack)` (one moulded plastic) | cast |
| ribbon curbs / sides, `Asphalt`, `CarpetRunner` | roles `trim` / `ribbon_side` | cast |
| barriers | block style of the environment, else role `barrier` | cast |
| ground polygons | `surface(kind)` | only raised ones (> 0.5 u above the lowest: table tops) |
| room walls (+ corner posts) | role `wall` | cast |
| skirting (0.45 u), window frames, sills, mullions | role `trim` | cast |
| ceiling (at `room.height`) | role `ceiling` | no (skylight model: the key light reaches the room) |
| base floor 0.03 u below y = 0 / outdoor ground 600 u past the bounds | role `floor` | no |

- Meshes come straight from `TrackGeometry` (`MeshData` → `Mesh`, world-unit UVs, tangents
  generated) under one root `track visuals` entity scoped `DespawnOnExit(RaceSession)`.
- **Barriers** (`barriers.rs`): where rc_track has a barrier, a row of chamfered blocks fills
  exactly the collider's box (same frames, inner face at half width + the style's edge, 0.6 u
  thick, ribbon depth to `wall_height`; a test pins this to the collider vertices). Blocks are
  chords of the barrier line: `length` on straights, shorter where it curves (≤ 0.03 u off the
  curve), so they hug hairpins. Styles by environment (code table `block_style`):

  | Environment | Blocks | Materials |
  |---|---|---|
  | `toy_store` | 2 u, 0.06 gaps | `plastic_red/yellow/blue/green` (foam/toy blocks) |
  | `test` | 1.5 u, no gaps | `plastic_red`, `plastic_white` (kart barrier) |
  | `street` | 4 u, 0.05 gaps | role `barrier` (kerb stones) |
  | `garden` | 1.2 u, 0.06 gaps | role `barrier` (brick edging) |
  | others | the collider sweep | role `barrier` |
- **Room shell** (`shell.rs`): wall inner faces stand exactly on `Track::bounds()` (the collider
  shell), 1 u thick; ceiling at `ActiveEnvironment.def.room.height`. Window openings from
  `room.windows` are cut by splitting each wall into solid pieces (`wall_pieces`, tested); frames
  sit from 0.35 u inside the inner face to 0.2 u into the room, mullions every ≤ 7 u plus a mid
  transom in the first 0.3 u, so the shadowed window light draws pane patterns on the floor.
  T13a glazes the openings (pane on the wall's centre plane).

## Props (`props/`)
- Static props: a session-scoped model entity at the prop's pose (the `Prop` entity is
  rc_track's and has no `Visibility`). Knockable props: the model is a child of the visual proxy.
- glb (`PropDef::model`, checked with `is_file` first, so missing models never log asset
  errors) → `WorldAssetRoot`; otherwise **fallback meshes** of the collider primitives
  (`shapes.rs`: chamfered boxes and cylinders, balls, wedges; closed, world-unit UVs). A fallback
  uses its drivable `surface` material, else the toy palette (`plastic_*`): knockables cycle
  colours per instance, static kinds keep one colour per kind.
- **Lamps** (environment artists; the street artist's format): rc_render reads `lights` from
  `props/*.ron` (rc_track ignores it) and spawns real lights at Night only:
  ```ron
  "street_lamp": (model: "...", colliders: [...],
      lights: [(offset: (0, 29, -6),         // prop space, u
                color: (1, 0.7, 0.38),       // linear RGB, default warm white
                lumens: 60000,               // real-world lumens (× 19.7 via lamp_lumens)
                range: 70,                   // u
                spot_deg: 75,                // full cone angle (core 0.7×); 0 / omitted = point light
                dir: (0, -1, -0.15),         // spot direction in prop space (default straight down)
                shadows: false)]),           // ≤ 1–2 shadowed hero lamps per track
  ```
  Bulb materials of a prop glb named `lamp*` glow at their authored strength at Night and 0.05×
  by day. Unknown lamp fields are load errors naming the file and kind; a test loads every
  shipped `props/*.ron`.
- **Decor** (`decor.rs`): decorative-only models without colliders (backdrops, furniture beyond
  the barriers) from the optional `assets/data/decor/<track>.ron`:
  `(decor: [(model: "models/...glb", pos: (x, y, z), yaw: 90, scale: 1.0, dir: Both)])` —
  placed like track props (mirror negates X and yaw, `dir` filters by direction).

## Cars (`cars/`)
- `CarModel { car, paint }` (pub) on any entity instances `CarInfo::model` as children; racer
  and ghost proxies get one as a child (`On<Add<VisualOf>>`), the showroom puts one on its
  turntable. `CarModelReady` marks an instanced, painted model. Inserting an equal value is a
  no-op; a different value re-instances.
- On `WorldInstanceReady` one pass assigns the final material of every mesh: `paint` /
  `paint_accent` ← `MaterialLibrary::car_paint` (shared per car + paint); `light_head` 1× at
  Night, 0.3× by day; `light_brake` per-instance copies at 0.2×, lit to 1× while
  `CarControl.brake > 0.05`; `light_tail` and `glow` as authored (art bible §1.2).
- Wheels (`wheels.rs`, tested): node = rest translation + Y·`offset`, rotation =
  `rot_y(−steer) · rot_x(−spin) · rest` from `VehicleTelemetry.wheels` (rc_vehicle's
  conventions). Ghost cars (no telemetry) roll their wheels from their own motion (radius = rest
  height of the wheel centre).
- Antenna (`antenna.rs`, tested): the `antenna` node leans against the model's acceleration on an
  underdamped spring (≈ 1.7 Hz, ≤ 0.45 rad).
- Time Trial ghost (`GhostCar`): every material copied translucent (α 0.35, `Blend`, back faces
  culled), no shadow casting, lamps kept. Respawn ghosting (`StatusEffects.ghost`): the model
  blinks at 6 Hz and ends visible.

## Captures (`capture/`)
Both run on the race started by `--quick-race`, through a capture camera with `QualityCamera`
(renders exactly like a game view) into a 1280×720 image (no HUD, independent of the window).
The camera renders its first pose for 4 s (pipelines compile, assets settle), holds every pose
0.6 s, and the app quits once every PNG is written, logging frame-time p50/p95/max of the run.
- `--capture-previews`: `TrackInfo.preview` (e.g. `assets/ui/tracks/toy_town.png`), forward
  layout only. The camera looks across the track's long axis from the start line's side, as close
  as framing the whole outline allows (bisection on the projected outline); indoors it stays under
  the ceiling and inside the walls and aims lower so the floor fills the frame. All tracks:
  ```sh
  for t in toy_town maple_street test_oval; do cargo run -p recharged -- --user-data-dir \
      target/agent-data/T13b --unlock-all --quick-race $t --time noon --weather clear \
      --capture-previews --exit-after 60; done
  ```
- `--track-tour <track>`: a shot every 25 u along the racing line, eye 1.6 u above it, looking
  8 u ahead along it (shots in a jump gap move past it) →
  `<user data>/captures/tour_<track>_<variant>/NNN.png`. Pass the same track to `--quick-race`.

## Verify
- `cargo test -p rc_render` — shapes (closed, outward, extents, UVs, posing), barrier blocks vs
  colliders, wall runs, shell pieces and window cut-outs, mesh conversion, wheel maths, antenna
  spring, blink, ghost materials, lamp data of every props file, decor variants, capture poses and
  frame stats.
- Screenshots: `cargo run -p recharged -- --user-data-dir target/agent-data/T13b --window
  1280x720 [--unlock-all] --quick-race toy_town [--reverse] [--time night] [--players 2]
  --screenshot-at 8 --screenshot <png> --exit-after 10`; tours as above.

## Known limitations
- The first seconds of a session freeze while Bevy compiles new pipelines (≈ 250 ms per variant,
  serialized; measured 0.8–2.7 s frames); a warm-up during Loading belongs to the rendering core.
- Barrier styles are a code table; book rows (house), tyre walls (workshop) and shelf kick-plates
  (supermarket) use the plain sweep until an environment asks for them (`ponytail`).
- Prop glbs that fail to parse stay invisible (no fallback swap); a missing file uses the fallback.
- One preview per track (forward layout, the time given on the command line).
