# rc_render — content visuals (T13b design note)

Track, prop and car visuals and the capture tools of `rc_render` (`content.rs`,
`track_visuals/`, `props/`, `cars/`, `capture/`). The rendering core (proxies, quality, lighting,
sky, weather, `MaterialLibrary`, showroom) is T13a's; see `docs/design/rc_render.md`.

## Loading
`ContentVisualsPlugin` registers the `"content"` key. During `Loading` it waits for `Track`,
`ActiveEnvironment` and a `MaterialLibrary` rebuilt for that environment, then builds the track
meshes and dresses the props, queues the car scenes of `RaceConfig` (racers + Time Trial ghost)
and completes the key when every scene is loaded. A car scene that fails (missing or broken glb)
fails the loading with its path; a prop scene that fails only logs (the prop stays invisible).
Scene handles are cached for the app's lifetime, so restarts and next races reuse them.

## Track (`track_visuals/`)
| Part | Material (library) | Shadows |
|---|---|---|
| ribbon top | `surface(run.surface)` | cast |
| ribbon lips & sides, `ToyTrack` | `surface(ToyTrack)` (one moulded plastic) | cast |
| ribbon curbs / sides, `Asphalt`, `CarpetRunner` | roles `trim` / `ribbon_side` | cast |
| barriers | role `barrier` | cast |
| ground polygons | `surface(kind)` | only raised ones (> 0.5 u above the lowest: table tops) |
| room walls (+ corner posts) | role `wall` | cast |
| skirting (0.45 u), window frames, sills, mullions | role `trim` | cast |
| ceiling (at `room.height`) | role `ceiling` | no (skylight model: the key light reaches the room) |
| base floor 0.03 u below y = 0 / outdoor ground 600 u past the bounds | role `floor` | no |

- Meshes come straight from `TrackGeometry` (`MeshData` → `Mesh`, world-unit UVs, tangents
  generated), one root `track visuals` entity scoped `DespawnOnExit(RaceSession)`.
- The room shell: wall inner faces stand exactly on `Track::bounds()` (the collider shell),
  1 u thick. Window openings from `ActiveEnvironment.def.room.windows` are cut by splitting each
  wall into solid pieces (`shell::wall_pieces`, tested); frames sit from 0.35 u inside the inner
  face to 0.2 u into the room, mullions every ≤ 7 u plus a mid transom in the first 0.3 u, so the
  shadowed window light draws pane patterns on the floor. T13a glazes the openings (pane ≥ 0.5 u
  inside the wall).

## Props (`props/`)
- Static props: a session-scoped model entity at the prop's pose (the `Prop` entity is
  rc_track's and has no `Visibility`). Knockable props: the model is a child of the visual proxy.
- glb (`PropDef::model`, checked with `is_file` first, so missing models never log asset
  errors) → `WorldAssetRoot`; otherwise **fallback meshes** of the collider primitives
  (`props/shapes.rs`: chamfered boxes and cylinders, balls, wedges, closed, world-unit UVs). A
  fallback uses its drivable `surface` material, else the toy palette (`plastic_*`): knockables
  cycle colours per instance, static kinds keep one colour per kind.
- **Lamps** (environment artists): rc_render reads its own fields from `props/*.ron` (rc_track
  ignores them) and spawns real lights at Night only:
  ```ron
  "street_lamp": (model: "...", colliders: [...],
      lights: [(offset: (0, 18, 1.5),        // prop space, u
                color: (1.0, 0.8, 0.55),     // sRGB, default warm white
                lumens: 1500,                // real-world lumens (× 19.7 via lamp_lumens)
                range: 40,                   // u
                spot: (dir: (0, -1, 0), angle_deg: 55),  // optional: outer angle from the axis
                shadows: false)]),           // ≤ 1–2 shadowed hero lamps per track
  ```
  Bulb materials of a prop glb named `lamp*` glow at their authored strength at Night and 0.05×
  by day. A test loads every shipped `props/*.ron` with these fields.

## Cars (`cars/`)
- `CarModel { car, paint }` (pub) on any entity instances `CarInfo::model` as children; racer
  and ghost proxies get one as a child (`On<Add<VisualOf>>`), the showroom puts one on its
  turntable. `CarModelReady` marks an instanced, painted model. Inserting an equal value is a
  no-op; a different value re-instances.
- On `WorldInstanceReady` one pass assigns the final material of every mesh: `paint` /
  `paint_accent` ← `MaterialLibrary::car_paint` (shared per car + paint); `light_head` 1× at
  Night, 0.3× by day; `light_brake` per-instance copies at 0.2×, lit to 1× while
  `CarControl.brake > 0.05`; `light_tail` and `glow` as authored (art bible §1.2).
- Wheels (`cars/wheels.rs`, tested): node = rest translation + Y·`offset`, rotation =
  `rot_y(−steer) · rot_x(−spin) · rest` from `VehicleTelemetry.wheels` (rc_vehicle's conventions,
  verified by the vehicle arena example).
- Time Trial ghost (`GhostCar`): every material copied translucent (α 0.35, `Blend`, back faces
  culled), no shadow casting, lamps kept. Respawn ghosting (`StatusEffects.ghost`): the model
  blinks at 6 Hz and ends visible.

## Captures (`capture/`)
Both run on the race started by `--quick-race`, through a capture camera with `QualityCamera`
(renders exactly like a game view, drawn over the game camera so no HUD), 3 s after entering the
race, holding each pose 0.6 s; the app quits after the last PNG.
- `--capture-previews`: `TrackInfo.preview` (e.g. `assets/ui/tracks/toy_town.png`), 1280×720,
  forward layout only, from a three-quarter view on the start line's side (indoors clamped under
  the ceiling and inside the walls). All tracks:
  ```sh
  for t in toy_town test_oval; do cargo run -p recharged -- --user-data-dir target/agent-data/T13b \
      --quick-race $t --time noon --weather clear --capture-previews --exit-after 40; done
  ```
- `--track-tour <track>`: one window screenshot every 25 u along the racing line, eye 1.6 u above
  it, looking 8 u ahead along it (shots in a jump gap move past it) → `<user data>/captures/tour_<track>_<variant>/NNN.png`. Pass the same
  track to `--quick-race`.

## Verify
- `cargo test -p rc_render` — shapes (closed, outward, extents, UVs, posing), shell pieces and
  window cut-outs, mesh conversion, wheel maths, blink, ghost materials, lamp data of every props
  file, capture poses.
- Screenshots: `cargo run -p recharged -- --user-data-dir target/agent-data/T13b --quick-race
  toy_town [--reverse] [--time night] [--players 2] --screenshot-at 8 --screenshot <png>
  --exit-after 10`.

## Known limitations
- Barriers are the collider sweep in one `barrier` material: themed barrier props (book rows,
  tyre walls, foam blocks) would need per-environment geometry (`ponytail`: add a barrier style
  table when an environment asks for it).
- Ghost cars have still wheels (no telemetry); the antenna does not sway yet.
- Prop glbs that fail to parse stay invisible (no fallback swap); a missing file uses the fallback.
- One preview per track (forward layout, the time given on the command line).
