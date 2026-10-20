# rc_editor — track editor (design note)

A separate executable for creating and editing tracks, test driving them, and baking their indirect
lighting. Status: **P1–P4 done** (open/create, edit everything, undo, test drive, save; every track
editor-owned; irradiance bake). **v2 (P5–P10) designed; P5 (layout), P6 (select & gizmo),
P7 (incremental rebuilds), P8 (asset library) and P9 (lighting) done** (§Editor v2): settings
(P10) to come.
Only Toy Town is baked so far (§Irradiance bake).

```sh
npm --prefix ui run build                       # the editor uses the game's React bundle
cargo run -p rc_editor -- toy_town              # edit a track
cargo run -p rc_editor -- new back_lot --env street --name "Back Lot" --style Asphalt
```
Inputs as of P6 (§Editor v2 → Interaction has the rest of v2). Props and decor are picked by
their models; item rows, the battery and reverse-only props and decor (dimmed) have markers in
every tab; the layout and its markers (control points, jump mid-points, ground vertices, shortcut
path points) show only in the Track tab:

| Input | Does |
|---|---|
| hover | tints the prop or decor model under the pointer (markers turn white) |
| left press | gizmo handle > marker/model (select, or clear on nothing) > **Ctrl** on the road in the Track tab: insert a control point there (on the nearest segment) |
| left drag on the gizmo | moves / rotates / scales the selection, live; **Ctrl**: snap 0.5 u · 15° · 0.1 |
| G · R · C · L | gizmo move · rotate · scale · world/local axes (scale is always local); Ctrl up |
| Esc | cancel a live drag, else deselect |
| right drag · WASD · Q/E · Shift | look · fly · down/up · faster |
| F5 | test drive / back to editing |
| Ctrl+S · Ctrl+Z · Ctrl+Y (Ctrl+Shift+Z) | save · undo · redo |
| Delete · Insert · Ctrl+D | delete the selection · insert a point/vertex/path point after it · duplicate |
| T · F · P | top view (over the selection or the view's focus) · fly to the selection · probes on/off |

While a drag is live, Ctrl+Z/Y, Delete, F5 and a tab switch cancel it first.

Saves, settings and records of test drives go to `<game user data>/editor` (`--user-data-dir`
overrides; v2 reads settings from the game instead, D10). Automation as in the game:
`--screenshot <png> [--screenshot-at <s>]`, `--exit-after <s>`, and `RECHARGED_UI_SCRIPT` with
`editor <command>` (`drive`, `undo`, `topView`, `cancel`, …), `editor select <kind> <index>`,
`editor tab <assets|track|lighting>`, `editor gizmo <move|rotate|scale>` and
`editor move|rotate|scale x y z [hold]` (a gizmo drag of the selection by a translation, a
(pitch, yaw, roll) turn in degrees or per-axis factors; `hold` leaves it live and uncommitted, so
the preview can be screenshotted), `editor place <prop id | decor path | itemRow | battery>
[x y z]` (with a position: placed there; without: placement mode armed; P9 assets: `point`,
`spot`, `sun`, `fill`), `editor swap <asset>`, `editor search <text>` (the Assets tab's search),
`editor select sun|fill|look <time>`, `editor set look <time> <key> <value…>` / `editor set sel
<key> <value…>` (one number, three, or on/off) and `editor option time|weather|reverse|mirror
<value>` (the toolbar's options) drives the editor through the real React channel.

## Decisions
| # | Decision | Consequence |
|---|---|---|
| D1 | **The editor is the source of truth** for track layouts | `tracks/<id>.ron` and `decor/<id>.ron` become editor-owned. The `plan.py` track sections are retired one track at a time (§Migration). Blender scripts keep building prop models and catalogs. |
| D2 | **UI in bevy-react** | The game's bundle: `App` shows `ui/src/editor/` while `editor.status` says editing (the game answers `editor.snapshot` with `null`); test drives show the game's race UI. Channels in rc_ui (`editor.rs`), handled by rc_editor. |
| D3 | **Irradiance volumes only; no lightmaps** (maybe later) | Bake indirect diffuse into one `IrradianceVolume` per track variant × time of day. Direct light stays real-time. |
| D4 | **No geometry or draw-call "compile" step** | The QA bench is well inside budget (1P 185–204 fps, 4P ≈ 105 fps at High, RTX 3070). Revisit only if a measurement says so (e.g. Iris Xe). |
| D5 | **v2 is a refactor, not a rewrite** | Kept: `Doc`, the edit/undo pipeline, `fields` slots, save, catalog rewrite, bake, test drive, automation. Replaced: `view.rs` (picking, handles, gizmos), the React layout, the rebuild path. |
| D6 | **Edits never reload the session** | Game crates rebuild what changed in place, keyed by diff (§Incremental rebuilds). A full reload only for time, weather, mirror, reverse, test drives and a short list of rare once-per-session settings (§Incremental rebuilds → Still a full reload). |
| D7 | **Own transform gizmo** (Bevy `Gizmos` + screen-space picking) | `transform-gizmo-bevy` 0.11 targets Bevy 0.19; three modes do not justify a fork to track. |
| D8 | **`environment.ron` joins the `Doc`** | Lights, sun/fill and time looks get undo and Ctrl+S; rc_render reads an `EnvironmentDraft` when present, like rc_track reads `TrackDraft`. |
| D9 | **Props stay upright** (pos + yaw); **decor gets full rotation and per-axis scale** | Prop colliders and mirror layouts assume upright props; decor has no physics. |
| D10 | **The editor uses the game's settings, read-only** | `<game user data>/settings.ron` is read with a non-mutating loader, never written (saves and records stay in `editor/`). No settings UI in the editor. |

## Architecture
```
crates/recharged   lib.rs: the game app (`recharged::game`) ── main.rs: the game binary
crates/rc_editor   lib.rs   EditorPlugin: Editor state, edit/undo pipeline, session control, commands, status
                   doc.rs   Doc (layout + decor + environment + catalog entry): handles, add/insert/delete/duplicate/swap, validate, new_track
                   fields.rs the right pane: one slot list per item serves reading and setting
                   catalog.rs rewrites the track's entry in catalog.ron (the rest of the file untouched)
                   library.rs v2: the asset library (prop kinds, decor models, lights, gameplay items)
                   lights.rs P9: lights, sun, fill and time looks in the document (poses, markers, slots)
                   save.rs  Ctrl+S: track, decor, environment and catalog files (tmp + rename)
                   view.rs  EditorViewPlugin: fly camera, mesh picking, markers, focus point, cutaway
                   tint.rs  hover/selection tint (per-mesh material clones)
                   gizmo.rs the transform gizmo (move / rotate / scale, world / local, snap), DragPreview
                   live.rs  P7: commit routing (reload list), the in-place commit, TrackPreview builds
                   main.rs  `rc_editor [new] <track>` = recharged::game + both plugins
crates/rc_ui       editor.rs: editor.status / snapshot / command / select / set / options (+ v2: library, tab, gizmo, place, swap)
ui/src/editor/     EditorLayer (toolbar, left tabs, right pane), controls.tsx (fields), Library.tsx
```
- **The game renders the edited track.** From the menu the editor starts a Quick Race (no AI) on
  the track; while editing, `SimSet::Race` (rc_race's phase driver) does not run, so the session
  stays in `RacePhase::Setup`: track, props, lighting, no cars. The editor writes the
  `GameCamera`'s transform (rc_camera holds its pose without a racer) and hides rc_render's
  `Overhead` entities (ceiling, lamp fixtures; they cast no shadows) so rooms are seen from above.
- **Every edit is built before it is accepted.** An edit runs on a copy of the `Doc`;
  `Track::build` must pass in all four variants (≈ 90 ms on the Iris Xe for Toy Town) and a
  changed catalog entry must leave `catalog.ron` valid (`Catalog::parse` of the rewritten text).
  A rejected edit changes nothing and shows its error. Accepted: pushed on the undo stack (200
  snapshots; whole documents, they are small), mirrored to `TrackDraft` / `EnvironmentDraft` and
  to the game's `Catalog`. Up to P4 the session then rebuilt with `GameCommand::ChangeTrack`
  (≈ 0.7 s); v2 updates it in place (§Incremental rebuilds). After every edit, undo
  or redo the editor compares the reload-list fields (§Incremental rebuilds → Still a full
  reload) of the old and new `Doc`; only a difference bumps `wanted` and sends `ChangeTrack`
  (the generation counters keep an edit during a reload from getting lost). Everything else
  updates in place.
- **Test drive:** F5 lifts the hold (the intro is skipped with a `UiNav::Confirm`); F5 again sends
  `Restart` (→ Setup, no reload). Reverse/mirror drives (toolbar) reload the session in that
  layout and back to forward afterwards: editing is always forward, unmirrored. Time and weather
  (the track's allowed presets) apply to editing and drives. "Quit to menu" from the pause menu
  reopens the editing session.
- **Saving** writes the track (`TrackDef::to_ron`), decor (`decor_to_ron`), environment (v2) and,
  if it changed, the catalog entry, each through a temporary file + rename. Comments in those
  files are lost (`label` keeps per-point notes); `catalog.ron` keeps every comment outside the
  entry. v2 makes the environment file editor-owned too: its comments go on the first save.

## Editor v2 (P5–P10)
Decided in a design interview with the user (2026-10-20), amended after design reviews 1 and 2
(`docs/reviews/editor-v2-review-{1,2}.md`; approved with these changes). Everything here replaces the P1–P4 UI and view; the
document, validation, save and bake work as described above.

### Layout
```
┌ toolbar: name · saved/unsaved · undo redo save · time weather mirror reverse · bake stale · F5 ┐
├ left pane (tabs) ──┬──────────────── 3D view ───────────────────────┬ right pane ───────────┤
│ Assets │ Track │   │                                                │ the selection's       │
│ Lighting           │                                                │ fields and actions;   │
│                    │                                                │ empty with nothing    │
│                    │                                                │ selected              │
└────────────────────┴────────────────────────────────────────────────┴───────────────────────┘
```
- **Removed:** the Items window (accordion of index chips), the Notes strip and the P4 layer
  toggles (`toggleLayer`, `Editor.layers`, the `editor toggleLayer` automation verb): the Track
  tab decides which handles show. A rejected edit's error shows as a toast under the toolbar;
  lints list at the bottom of the Track tab, clicking one selects the item it names where it
  names one.
- **Assets tab:** the library (§Library).
- **Track tab:** *Track settings* at the top (style, start, kill/wall height, bounds; the catalog
  entry: name, environment, indoor, difficulty, laps, music, allowed times/weather, unlock,
  preview, hidden), then *Add* buttons for jump, ground area, shortcut, then lints. While this tab
  is open the layout handles are shown and editable: control points (and the spline through
  them), jumps, ground-area vertices, shortcut path points, the start. In the other tabs they
  are hidden and the road and ground are inert surfaces (not selectable).
- **Lighting tab:** for the time of day being previewed (named at the top): the time look
  (exposure, ambient, env light, fog, grading, bloom, lamp and practical levels, clouds, window
  glow), the room's lamp grid (indoor maps), Probes on/off, Bake light / Bake all, and which
  volumes are stale.
- **Right pane:** the selected object's fields (`fields::slots`, as today) and its actions
  (Duplicate, Delete, Fly to; control points: Insert after, Jump from here; library objects:
  Swap → a picker of assets of the same type). Nothing selected → empty.
- **Unreachable items cannot exist:** a prop, decor item or light that has no entity in the
  editing session (`dir: Rev` items — editing is forward — and lights not on at the previewed
  time) gets a dimmed marker drawn from the `Doc`, selectable like any other.

### Interaction
| Input | Does |
|---|---|
| hover | tints the object under the pointer |
| left press | gizmo handle > marker/mesh (select, or clear on nothing) > Ctrl+press on the road in the Track tab (insert a control point on the nearest segment) |
| left drag on the gizmo | moves / rotates / scales the selection, live |
| **G · R · C** | gizmo mode: move · rotate · scale (C, not S: S flies) |
| **L** | gizmo space for move/rotate: world (default) · local; scale is always local |
| hold **Ctrl** during a drag | snap: 0.5 u · 15° · 0.1 scale |
| F · T · P · Esc | frame the selection · top view · probes on/off · cancel a live drag or placement, else deselect |
| Delete · Ctrl+D · Insert | delete · duplicate · insert after (control points, vertices, path points) |
| Ctrl+S · Ctrl+Z · Ctrl+Y · F5 | save · undo · redo · test drive |
| right drag · WASD · Q/E · Shift | fly camera, unchanged (no letter above collides with W A S D Q E) |

Letter keys act only with Ctrl up. While a gizmo drag or a placement is live, Ctrl+Z/Y, Delete,
F5 and tab switches cancel it first (the item snaps back), then act.

- **Picking:** on demand with Bevy's `MeshRayCast` system param, only while editing and only on
  pointer move or press (no `MeshPickingPlugin`: it would ray-cast every frame, test drives
  included, against ≈ 0.83 M triangles, and feed the `HoverMap` bevy-react reads). Ignored while
  bevy-react's `PointerCapture` is set. The nearest visible mesh wins; it selects only if it
  has a source-tagged ancestor (§Incremental rebuilds; walk up `ChildOf`), so a deck or wall in
  front of a prop hides it. Road and ground carry no tag, so they
  are never selected; Ctrl+press on the road uses the rapier ray as `focus_point` does. Mesh
  ray casts need the CPU copies `keep_cpu_copies()` already keeps.
- **Markers:** objects with no mesh — point and spot lights, sun, fill, item rows, the Golden
  Battery, the Track tab's handles, and the dimmed unreachable items above — get a small
  screen-sized icon drawn with `Gizmos`, picked in screen space like the P4 handles; a mesh wins
  over a marker only when the marker is farther away.
- **Hover / selection tint:** each mesh of the hovered and of the selected object gets a
  per-entity clone of its material (`StandardMaterial` or `SurfaceMaterial`) with an emissive
  boost (two colours: hover, selected); unhover/deselect restores the original handle and drops
  the clone. Re-applied on `WorldInstanceReady` when the instance belongs to the hovered or
  selected object (a dropped item is selected before its glb is in; `dim_bulbs` swaps lamp
  materials then too). The selection is cleared before a bake. No outline pass.
- **Gizmo** (`gizmo.rs`, D7): drawn only on the selection. Move = three arrows + three plane
  squares; rotate = three rings; scale = three cube-tipped axes + a centre cube (uniform).
  Handles are picked in screen space (nearest within a few px). Each kind shows only what its
  data stores:

  | Kind | Move | Rotate | Scale |
  |---|---|---|---|
  | Prop | yes | yaw ring only | — |
  | Decor | yes | all rings | per axis (local) + uniform |
  | Point light | yes | — | — |
  | Spot light | yes | aims `target` | — |
  | Sun, fill | — | elevation + azimuth | — |
  | Control point, vertex, path point, item row, battery | yes | — | — |

  Width and bank of a control point, a ground area's height and the like stay in the right pane.
- **Undo:** one step per gizmo drag (committed on release), per field set, per drop, per swap.

### Library
`library.rs` builds the list once at start; `editor.library` (request) hands it to React.

| Category | Entries | Source |
|---|---|---|
| Props · *environment* | the 206 prop kinds, grouped by their `assets/data/props/<env>.ron` file | `PropCatalog` + the file each kind came from |
| Decor · *environment* | every `.glb` under `assets/models/props/<env>/` | directory scan (replaces the free-text model field) |
| Lights | Point light, Spot light, Sun, Fill | fixed |
| Gameplay | Item row, Golden Battery | fixed |

- Text names (prettified ids) with a search box; no thumbnails. The groups of the map's own
  environment are expanded first, the rest collapsed.
- **Placement mode.** bevy-react keeps `PointerCapture.dragging` and `over_ui` set for the whole
  press that started in the pane, so Bevy cannot "pick the drag up" through it. Instead,
  pressing an entry sends `editor.place { asset }` and puts the editor in a placement mode that
  ignores `PointerCapture`: a ghost (the asset's glb scene, or its marker) follows the hit of the
  ray through `Window::cursor_position` on any fixed collider (falling back to y = 0), upright.
  Left `just_released` (`ButtonInput<MouseButton>`) with the cursor outside the left and right
  UI rects (left pane, right pane, toolbar, toast: their roots' `ComputedNode`s) adds the item there and selects it (one edit);
  released inside a pane → stays armed, so a click on an entry then a click in the view places
  too; Esc, right click or the entry's own button again cancels; clicking another entry re-arms
  with it; the search box and scrolling leave it armed. While armed, the placing click does not
  also run select/clear. The ghost loads its model through rc_render's `load_scene` (made
  public) so its load settings match the real spawn. Automation verb: `editor place <asset> x y z`.
- **Sun and fill are per time of day** and exist only in the time look: the sun always (placing
  Sun selects it and aims it from the drop point toward the map's centre), the fill optionally
  (placing Fill adds it if missing; Delete removes it; deleting the sun is refused). Their
  markers float above the map along their direction. Edits change the previewed time's look only.
- **Swap:** the selected object becomes another asset of the same type (prop ↔ prop,
  decor ↔ decor, point ↔ spot light), keeping its transform; from the right pane's picker or a
  library entry's "Replace selected". Point → spot aims the new `target` 5 u straight down;
  spot → point drops it. Gameplay items and sun/fill do not swap.
- Items: `EditorItem` gains `Light(i)` (an index into `EnvironmentDef::lights`, not into the
  spawned lights), `Sun`, `Fill`; `EditorAdd` gains the library ids.

### Data changes
- **Decor** (D9): `yaw: f32, scale: f32` → `rot: (f32, f32, f32)` (pitch, yaw, roll in degrees,
  applied YXZ) and `scale: (f32, f32, f32)`. `DecorItem` is `deny_unknown_fields`, so the old
  form needs a hand-written `Deserialize`: `scale` as `f32 | (f32, f32, f32)`, `yaw` → `rot.1`.
  A save writes `rot` / `scale` only when they differ from the default (files stay short).
  Mirror layouts negate X, yaw and roll, keep pitch (the mesh is never mirrored, as for props).
  `every_decor_file_survives_a_save` (rc_track `decor.rs`) gains an old-form case.
- **Environment lights mirror with the layout** (a game fix, not only the editor): `EnvSpawner`
  negates `pos.x` and `target.x` when `variant.mirror` (today they are spawned as written, so
  Toy Town's display spots light empty floor in mirror and the mirror bake bakes that). The sun
  and fill stay unmirrored, as §Irradiance bake says. No existing volume needs a re-bake (the only
  mirror volumes, Aisle Rush's, have no point/spot lights). A test spawns a spot in mirror and checks the flipped position and aim.
- **Environment**: unchanged format; `EnvironmentDef` gets `to_ron` (tmp + rename like the
  others). Point/spot lights are `EnvLightDef` entries (pos, `target`, angle, colour, lumens,
  range, `times`); sun/fill are `TimeLook::sun` / `fill`.
- **Prop catalog**: no format change; `PropCatalog` remembers each kind's source file (its
  category).
- **Placed props keep their source:** `LayoutProp` / `PlacedProp` gain `source: usize` (the index
  into `TrackDef::props`; `Layout::new` drops direction-limited props, so placed indices are not
  document indices); `rc_render`'s `load_decor` keeps the decor index the same way.

### Incremental rebuilds
Today rc_track (`load_track`) and rc_render (track visuals, props, decor, environment lights,
room) build everything on `OnEnter(Loading)`. v2 makes each of them **also react to a changed
draft while `InRace`**, rebuilding only what differs, with the same spawn code as a load — so a
fresh load and an incremental update cannot drift apart (the tests below prove it). The game
binary has no drafts, so it is unaffected.

- **Source tags** (public components): `PropSource(i)` on the `Prop` collider entity (rc_track)
  and, copied by `dress_props`, on its `PropModel` root (rc_render; today the static model is a
  separate, unlinked entity and a dynamic one sits under its `Visual` proxy — rc_render stores
  the model entity on the prop and despawns it with an `On<Remove, Prop>` observer);
  `DecorSource(i)`; `EnvLightSource(i)` (index into `def.lights`, before the `times` filter);
  `TrackPiece` on track colliders; a `RoomPiece` tag on room lamps and window glass. `i` is
  always the document index.
- **Live drags (no build, single writer kept).** The editor never writes game entities' components
  (ARCHITECTURE §3.3: rc_vehicle, rapier and rc_track's prop placement/reset write a rapier
  body's `Transform`). While the gizmo drags a prop, decor item or light, it writes the live pose
  into **separate preview resources, not the drafts** (a draft changed every frame would re-run
  every draft diff and, for the environment, regenerate the 128² environment map on the CPU each
  frame): `DragPreview(Option<(Source, Transform)>)` owned by rc_track (props: colliders, and
  rc_render's models follow the same resource; decor), and `EnvPreview` owned by rc_render
  (`Light(i, Transform)` or `Sun { elevation, azimuth }` / `Fill { … }`, applied after
  `apply_lights`, which re-poses sun and fill every frame). rc_track (colliders; dynamic bodies
  also get `Teleported`) and rc_render (models, lights) move their own entities to it. Release
  commits the edit and clears the preview; the environment map is regenerated only then, and
  drafts are copied into `ActiveEnvironment` / diffed only when they actually differ. The
  editor's only writes to game state stay the `GameCamera` pose, `Overhead` visibility, the
  drafts and these preview resources.
- **Control-point drags.** At most one `Track::build` of the current variant in flight on the
  `AsyncComputeTaskPool` (latest position wins). Results go into a separate `TrackPreview {
  track, generation }` (defined in rc_track, written by the editor), **not** `Track`; only rc_render's track visuals read it
  (falling back to `Track`). Results older than the last commit are dropped. A failed preview
  build keeps the last good preview and shows its error as the toast; releasing then rejects the
  edit and the point snaps back. The commit removes `TrackPreview`. Colliders, lints and
  everything else wait for the release.
- **On release / any committed edit** (drags, fields, drops, swaps, undo, redo): validate all four
  variants (as today), mirror to the drafts, then insert the current variant's `Track` **and its
  `TrackSummary`** (the minimap and rc_weather read it). Re-inserting `Track` does not re-trigger
  `is_added` (the volume and decor spawns stay put). Systems diff against what they built:
  - **Track key** = the `TrackDef` with `props`, `item_rows`, `golden_battery` and labels cleared,
    plus `track.bounds()` (auto bounds grow with prop positions). Changed → respawn track
    colliders (rc_track) and track visuals (rc_render; the shell with them) and the `RoomPiece`s
    (`RoomBuilt` goes; they are despawned by tag). Not `geometry_hash`: it hashes props and
    leaves out style, bounds, shortcuts and the start; it stays for records.
  - **Props:** per source index, placement or kind changed → respawn that prop's collider and
    model; added/removed → spawn/despawn by source.
  - **Decor:** the same, from `TrackDraft.decor`.
  - **Environment:** changed lights → respawn environment lights; changed lamp grid → respawn the
    `RoomPiece`s; other time-look fields already follow `ActiveEnvironment.def` live
    (`compute_look`), so they are re-applied by updating it. Materials are not editable, so the
    material library never rebuilds.
  - The irradiance volume stays (only the stale flag changes); item pickups, cars and the battery
    spawn per attempt and are untouched while editing.
- **Still a full reload** (the user's call: rare edits, not worth live paths): changing the
  previewed time, weather, mirror or reverse; test drives; and edits to `practicals`,
  `fixture_materials`, `shadow_distance`, `sky`, or the catalog's `environment`, `indoor`,
  `music` (built once per session today). A catalog edit that removes the previewed time or
  weather from the allowed list falls back to the first allowed value and reloads.
- **Records:** `check_geometry` runs only on entering `InRace`, so editor test drives may compare
  times across geometries. Harmless: editor records live in `editor/`.
- **Bake stale:** any committed edit marks every volume of the track stale (in memory, not saved);
  a finished bake clears its own. The toolbar and the Lighting tab show it.
- **Equivalence tests** (rc_testkit is headless — no window, GPU or rc_render — so two halves):
  - (a) rc_track, headless ECS: open a track, apply a sequence of edits (move a control point,
    move/add/delete/swap a prop, a `dir: Rev` prop) through the incremental path, snapshot the
    `Track` and the colliders by `TrackPiece` / `PropSource` with their transforms; reload from
    the same draft; require equal snapshots.
  - (b) rc_render: its spawn code for decor, prop models and environment lights works from a pure
    plan (`Vec<(Source, model or light, Transform)>`, plus the track key) used by both the load
    and the diff path. A unit test applies edits incrementally to a plan and requires it to equal
    the fresh plan.

### Settings (D10)
The editor keeps `<game user data>/editor` for its saves and records but takes its `Settings`
from the game's `<game user data>/settings.ron` through a new read-only
`rc_game::read_settings(path)` (`parse_versioned` + `sanitize`; no backup or rename: the normal
loader renames a corrupt or newer-version file aside, which must not happen to the player's
file). The editor's `main` inserts the result before Boot; `boot_saves` already skips when
`Settings` exists. Missing or unreadable → defaults, logged. With `--user-data-dir <dir>` (automation),
everything stays in `<dir>` as today (no `editor/` subdirectory) and the settings are read the
same read-only way from `<dir>/settings.ron`. A test drive's pause-menu `ApplySettings` applies for the run; rc_game's throttled
write goes to the editor's data dir (`editor/settings.ron`, which the editor never reads;
under `--user-data-dir`, `<dir>/settings.ron` — an automation dir, harmless). Graphics quality therefore matches what the player configured.

### P6 as built
- **Source tags:** `rc_track::PropSource(i)` on `Prop` colliders and `PlacedProp::source` /
  `LayoutProp::source` (the document index; `Layout::new` drops direction-limited props); `rc_render::DecorSource(i)` on
  decor models (`load_decor` keeps the index). *Deviation:* a static prop's model is now a
  **child of its `Prop`** (with `Visibility` added to the collider entity) instead of a stored
  entity + `On<Remove, Prop>` observer: it despawns with the prop and follows its collider, so a
  preview moves it for free. Both model roots copy `PropSource`.
- **Live preview:** `rc_track::DragPreview(Option<(DragSource, Transform)>)` (`DragSource::{Prop,
  Decor}`; the design's `Source`). rc_track moves the collider (a dynamic one also gets
  `RigidBodyDisabled` while previewed, so a prop held in the air does not fall, and `Teleported`);
  rc_render moves decor; both put the last one back when the preview moves on or clears. The
  editor clears it once the reloaded session is in `InRace` (a committed drag never flashes back;
  checked by logging the restore against the reload). The control polygon and markers are drawn
  from a copy of the document with the live pose, as P4 did.
- **Gizmo:** a mode the selected item lacks (rotate on a point, scale on a prop) shows the move
  gizmo. Size is 0.2 × its distance from the camera. Commits round positions and scales to
  0.001 and angles to 0.01°; angles are rewritten only when the rotation changed (a move keeps a
  file's `yaw: 270`). Drags of the gizmo own the mouse until release, over the UI too.
- **Tint:** emissive is not scaled by the exposure in Bevy (`emissive_exposure_weight` 0), so the
  boosts are plain values (hover (0.02, 0.08, 0.12), selected (0.12, 0.09, 0)).
- **Picking:** markers are screen-sized spheres (dimmed: alpha 0.45, not when selected), picked
  within 12 px. The pick filter accepts only tagged meshes, so a hit passes through the road (a
  prop under a deck is pickable through it), as designed.
- **Not verified with a real mouse** (no input injection): hover, handle picking and dragging,
  Ctrl+press insertion, Esc. The maths has unit tests (axis drag, ring angle, scale, nearest
  segment); the preview and commit paths were screenshotted through `editor move|rotate|scale`.

### P7 as built
- **Routing** (`live.rs`): every accepted edit, undo or redo bumps `commits`; a changed catalog
  `environment` / `indoor` / `music`, or a previewed time/weather the entry no longer allows
  (falls back to the first allowed one), bumps `wanted` (reload) instead. Otherwise the forward
  `Track` that validation already built (undo/redo build it once) is `pending`, and `live::apply`
  (editor chain, before `FlowSet`) inserts it with its `TrackSummary`, removes `TrackPreview` and
  clears `DragPreview` in one frame. A session that is not the edited one (loading, test
  driving) reloads instead, so no edit is lost to a load already under way.
- **rc_track** (`rebuild.rs`, after `FlowSet`): `Track::key()` (`TrackKey`: the def without props,
  rows, battery and labels + variant + bounds) against the built one → respawn `TrackPiece`s;
  props diffed by `rc_track::placement_changes` (also rc_render's decor diff): another kind →
  respawn, same kind and another pose → **moved in place** (*deviation*: the design says
  respawn; moving keeps the entity, so its glb never blinks for the frames instancing takes; a
  dynamic one gets `PropSpawn`, zero velocity and `Teleported`). The cleared `DragPreview`'s
  restore runs first in the same chain; a test fails if that order flips.
- **rc_render:** track meshes keyed by `MeshKey` = `TrackKey` + the way-finding marks
  (*deviation*: chevron boards and crosses step aside for props, so a prop edit can move them;
  computing them to compare costs ≈ 0.6 ms on Toy Town). They show `TrackPreview` when present,
  else `Track`. Room lamps and window glass are tagged `RoomPiece` and rebuilt when the bounds'
  floor plan (x/z) changes (`RoomBuilt` now holds that key instead of being a once-guard). Decor
  follows `TrackDraft` with the same plan/diff code as its load (a load = the changes from
  nothing). **Prop models are not in a plan of their own** (*deviation*): since P6 they are
  children of their `Prop` (static) or of its visual proxy (dynamic), so rc_track's diff moves,
  despawns and respawns them and `dress_props` dresses new props — the load's code; test (a)
  covers them through the prop entities.
- **Previews** use quick tangents (UV gradient per triangle, same vertex layout so no new
  pipelines; ≥ 95 % agree with mikktspace on a ribbon): mikktspace made each preview rebuild
  64 ms on the main thread (≈ 12 updates/s); quick ones take 7–9 ms. The meshes are rebuilt with
  mikktspace once the session's `Track` shows again (commit or cancel). `fit_racing_line` was not
  the bottleneck (the whole `Track::build` is ≈ 20 ms, off the main thread), so there is no
  `BuildOptions` flag.
- **Bake stale:** `Editor.stale` holds (mirror, time) pairs marked by every commit (all allowed
  times, both layouts) and cleared by that volume's finished bake; `EditorStatus.staleBake` lists
  those whose file exists. The toolbar shows "Bake out of date", the Lighting tab names them.
- **Measured** (this machine, Toy Town, dev build): a prop move commits in ≈ 80 ms (validation
  of the four variants 77 ms, collider diff 0.1 ms, mesh key check 0.6 ms), `AppState` stays
  `InRace`; a control-point move ≈ 165 ms (validation 78 ms, track colliders 17–19 ms, track
  meshes 67 ms), against the ≈ 0.7 s reload (P4's figure). A scripted 2 s point drag (60 poses) got 59 previews
  (≈ 30/s, the script's pace; build 19.5 ms async + 7–9 ms meshes). Not yet measured on the Iris
  Xe. Screenshots: `target/agent-data/p7/`.
- **Tests:** rc_track `tests/incremental.rs` (edits incl. a `dir: Rev` prop applied in place vs a
  fresh load of the same draft: `Track`, `TrackSummary`, colliders by `TrackPiece` / `PropSource`;
  knockables compared on a 1 cm grid, as they settle while a session loads; plus the
  preview-restore order), rc_render `incremental_plans_equal_fresh_ones` (mesh key + decor
  plan), `placement_changes` and quick-tangent unit tests, the editor session test (an edit, a
  rejected one and undo stay in place; `indoor` reloads).

### P8 as built
- **Library** (`library.rs`, `editor.library` → `EditorLibrary`, built once in `EditorPlugin::load`):
  `PropCatalog::files` (kind → file stem, filled by `merge`; no format change) groups the 206
  kinds; a scan of `assets/models/props/<env>/*.glb` (one level) gives the decor; Gameplay holds
  Item row and Golden Battery (disabled while one exists). Groups of the track's own environment
  (`environment == env` or `env_…`: `toy_store` → `toy`) come first and open, then Gameplay
  (open), `common`, the rest (closed). Names are prettified ids ("Toy block cube"); the search
  matches every word and opens every group. **Lights are left out until P9.**
- **Assets** are `EditorAsset::{Prop { id }, Decor { model }, ItemRow, Battery}`; `Doc::place` /
  `Doc::swap` / `Doc::asset` live in `library.rs`. `EditorAdd` keeps only Ground, Shortcut, Gap
  (the Track tab's); the P5 Add buttons for prop, decor, item row and battery are gone (all four
  are library entries). The inspector lost the prop `kind` and decor `model` fields (and
  `Choices::kinds`): `EditorSelection.asset` feeds a **Swap…** picker (search + list of the same
  type) at the top of the right pane, and every library entry of the selection's type shows
  **Replace**. Swaps keep pose, scale and `dir`; prop ↔ prop, decor ↔ decor only.
- **Placement** (`placement.rs`): an entry's `onPointerDown` (left button) sends `editor.place
  { asset, at: null }`; `Editor.placing` (shown as `EditorStatus.placing`, the entry turns volt)
  is a pure `step(armed, Entry | Cancel | Release { over_ui })`. *Deviation:* "over the UI" is
  not the panes' `ComputedNode` rects but any `ReactInteraction` node in the mouse's `HoverMap`
  (review 2's alternative): every pane, the toolbar and the toast (given `hoverStyle={{}}` for
  this) count, with no rect bookkeeping. A left `just_released` off the UI places through
  `Editor::place` (one edit, in place, selected); Esc, a right click, a tab switch, undo/redo,
  delete, F5 and a bake disarm. While armed `pointer` returns before the gizmo and picking. The
  **ghost** loads the glb through `rc_render::load_scene` (now public) and stands upright at the
  rapier hit of the cursor ray (`view::surface`, shared with the focus point; else y = 0); while
  the cursor is over the UI or outside the window it waits at the view's focus, so the armed
  asset is previewed before the click. A yellow drop marker is drawn at the point (the only
  marker for item rows and the battery). `editor.place` with `at` places at once (automation).
- **Tint** (investigated): it was applied, after in-place moves and after re-instancing too
  (`tint_sign_after_inplace.png`, `swap_done.png`); it was only weak — +0.12 emissive barely moves
  a sunlit bright surface (the bear's muzzle, a lit blue sign). Now the clone also mixes its
  base colour toward the tint (selected: 50 % toward amber, + (0.16, 0.1, 0) glow; hover: 30 %
  toward cyan, + (0.03, 0.1, 0.14)), so bright and dark materials both read. A yellow model
  selected stays yellow-ish (amber on yellow); hover remains cyan.
- **Not verified with a real mouse** (no input injection): the press-to-arm, drag-release and
  click-click paths, right-click cancel and the `HoverMap` UI test; the state logic has a unit
  test, `editor.place`/swap a session test, and the ghost/placement/swap were screenshotted via
  the verbs. Screenshots: `target/agent-data/p8/`.

### P9 as built
- **Document:** `Doc.env` (`EnvironmentDef`, now `Serialize`; `to_ron` = a depth-limited pretty
  print: top-level and time-look fields one per line, a light, material, sun or window list on one
  line, maps sorted, absent options and empty lists left out). Save writes `environment.ron` only
  when it differs from the last save (*deviation*: an untouched file keeps its comments).
  `EnvironmentDef::validate` also checks the lamp grid (spacing ≥ 1, range, lumens, spot angle).
  An environment-only edit skips `Track::build` (it validates the environment: µs, not 80 ms);
  `live::apply` now commits whenever `applied != commits` (the `Track` is optional) and clears
  `EnvPreview` with `DragPreview`, in the frame the drafts change.
- **rc_render** (`lighting/lights.rs`): `EnvironmentDraft { id, def }` is what `OnEnter(Loading)`
  activates when it is of the track (reloads and bakes keep unsaved lighting); in `InRace` a
  changed draft is copied into `ActiveEnvironment` only when it differs, and the lights follow by
  the plan/diff of decor (`plan(def, time, mirror)` → `placement_changes`; kind = cone, colour,
  lumens, range). `EnvLightSource(i)` on every light. `EnvPreview(Option<EnvDrag>)`:
  `Light(i, Transform)` moves that light (restored when it moves on), `Sun`/`Fill { elevation,
  azimuth }` re-pose the key/fill after `apply_lights`. `RoomBuilt` keys on the floor plan and the
  lamp grid. **Mirror fix:** the plan negates `pos.x` and `target.x` in mirror layouts (game too);
  sun and fill are not mirrored.
- **Items** (*deviation*): `Sun`, `Fill` carry their time (`{ time }`), and the Lighting tab's
  fields are a fourth item, `Look { time }`; the previewed time's are shown, and a selected sun or
  fill follows a time change. So the `Doc` needs no time, the right pane names the time ("Sun ·
  Noon") and `editor.set` (now with an optional `item`: `null` = the selection) sets the look
  without selecting it. `EditorStatus.look` carries the Lighting tab's fields. `Light { index }`
  indexes `def.lights`; lights off at the previewed time get dimmed markers.
- **Gizmo:** `Turn::Aim` (spots: move; rotate = two rings, elevation across the beam and heading
  about +Y; the target swings around the light at its distance, a move carries it along) and
  `Turn::Sky` (sun, fill: the same two rings in every mode, never a move; a turn sets
  elevation/azimuth, rounded to 0.01°). Markers: lights orange, sun yellow, fill blue; spots draw
  a line to their target, sun and fill one toward the map's middle (the control points' box, at
  its top; markers float 0.8 × its half diagonal, ≥ 20 u, out along their direction).
- **Library:** a *Lights* group after Gameplay (open): Point light, Spot light (`EditorAsset::Light
  { spot }`), Sun, Fill. A point hangs 3 u above the drop, a spot 8 u aimed at it (always on).
  Sun selects the previewed time's sun and turns its azimuth to the drop point's side of the map,
  keeping its elevation (*interpretation*: aiming "from the drop point" at the floor would put it
  on the horizon). Fill adds one if missing (70°, 15 % of the sun's lux, sky blue, no shadows).
  Swap point ↔ spot (Replace / the picker): a new spot aims 5 u straight down, a point drops it.
  Deleting the sun is refused; the right pane has no Delete for it.
- **Lighting tab:** "Time of day: …", the look's fields (exposure, ambient (+ colour), env light,
  fog colour/visibility, grading's seven, bloom, lamps, practicals, cloud colours, window glow)
  and, indoors, the lamp grid (`lamp_spacing` … `lamp_fixture`), then probes and bake. Room
  colours are not exposed.
- **Reload list** (`live::reloads`): + any time's `practicals`, `fixture_materials`,
  `shadow_distance`, `sky`, and the `lamps` level when there are fixture materials (their emission
  is set at instancing; without any, the room lamps follow live).
- **Measured:** an exposure or sun edit validates in ≈ 6 µs (no `Track::build`) and commits in
  place the next frame (the env map is regenerated once, on the commit). Screenshots `target/agent-data/p9/`: `spot_before/after`,
  `sun_selected/turned` (52°/120° → 25°/45°, shadows move), `look_before/after` (exposure 9.7 →
  8.7), `point_placed`, `lamp_grid`, `mirror_dollhouse` / `mirror_castle` (the game's
  `--track-tour toy_town --mirror --time night`: the display spot lights the mirrored dollhouse).
- **Done-when** (on a scratch copy of the assets): add a spot, aim it, turn noon's sun, set its
  exposure, save (the file has them; reopened, they are there), undo all four, save: the
  environment reads back equal to the original (`EnvironmentDef` ==), track and decor equal
  apart from comments and number formatting.
- **Tests:** rc_render `every_environment_file_survives_a_save`,
  `mirror_layouts_mirror_the_environment_lights`, `planned_lights_keep_their_document_index`,
  `incremental_light_plans_equal_fresh_ones` (equivalence test (b) for lights); rc_editor
  `ring_drags_set_the_suns_elevation_and_azimuth`, `the_fill_is_added_and_deleted_the_sun_never_deleted`,
  `light_items_index_the_document_and_lights_off_now_are_dimmed`, `placing_and_swapping_lights`,
  `a_spot_moves_with_its_target_and_turns_it_around_itself`, `only_reload_list_fields_reload`,
  the save test (environment), the session test `lighting_edits_apply_in_place_and_undo`.
- **Not verified with a real mouse** (as P6/P8): ring drags of the sun and spots by hand; the maths
  is tested and the verbs were screenshotted.

## Editing scope (P4)
| Item | Handles | Inspector |
|---|---|---|
| Track (+ catalog entry) | — | style, start, kill/wall height, bounds; name, environment, indoor, difficulty, laps, music, allowed times/weather and unlock (RON text), preview, hidden |
| Control points | yes | pos, width, bank, surface, walls, offroad margin (inherited fields show what they inherit; × unsets), label |
| Jumps (gaps) | red segment, handle at its middle (does not move) | from, to, dir; "Jump from here" on a point adds one |
| Ground areas / vertices | vertices; a vertex's "Whole area" selects the area | surface, height, dir; vertex x/z |
| Item rows, props, Golden Battery, decor | yes | pos, count / kind (all prop kinds) / yaw / model, rot, scale (P6: per axis), dir |
| Shortcuts / path points | path points; a path point's "Whole shortcut" selects the shortcut | width, dir; point pos |

Add (Assets tab: prop, decor, item row, battery; Track tab: jump, ground area, shortcut) puts the
item at the view's focus — the first fixed collider under the
screen centre (a new shortcut leaves and rejoins the track around it).

**Not done:** lints are text without a position (no click-to-focus); prop kinds are one long list
(no filter by environment); a new decor item copies an existing model (the model path is a text
field); no orthographic view (T is a perspective view from above). P6 replaced handle dragging
with the gizmo (§P6 as built).

## Migration off `plan.py` (D1, done)
- **Corridor lint** (`rc_track::analysis`, every track): no static prop without a drivable
  `surface` may reach into the racing width between 0.25 u (bumps: a cable) and 1.5 u (above the
  car) over the driving surface. It replaces `plan.py`'s supermarket-only check and found, on
  the shipped tracks: a mirrored Sofa Circuit doorway blocked by 2 u of wall (meshes are never
  mirrored: new prop flag `mirror_turn` turns such a prop around in mirror layouts), a Maple
  Street sign 0.13 u into the road (moved), and three deliberate drive-under/over spots (bench,
  sandbox jump, parked SUV), now marked with the new placement flag `on_line`.
- **Labels:** the per-point comments of every track (`// 7: freezer aisle`) became `label`s, so an
  editor save keeps them.
- **Scripts:** `plan.py` / `layout.py` / `garden/decor.py` write the track and decor files only
  with `--layout` (which drops every editor edit); they still build the prop catalogs, kerb arcs
  and model inputs. The files' headers and the environment notes say they are editor-owned.

## Irradiance bake (D3, P4)
```sh
cargo run -p rc_editor -- toy_town --bake      # every allowed time × {normal, mirror}, then quit
cargo run -p rc_editor -- toy_town --bake-time noon   # one volume (normal layout), then quit
```
**Probes: on/off** (Lighting tab, **P**) switches the session's volume — and the fill light it replaces —
to compare the lighting with and without it. Scripted comparisons: `--window 1600x900` and the
UI-script verb `editor lookAt x y z tx ty tz` (camera pose), then `editor toggleProbes`.
or **Bake light** (this time, forward) / **Bake all** in the Lighting tab. Files:
`assets/maps/<track>/<normal|mirror>_<time>.irv` (format and runtime: rc_render
`lighting/gi.rs`, `rc_render.md` §Baked indirect light). Reverse layouts use the forward bake (only
`dir`-filtered features differ); the sun is not mirrored, so mirror gets its own.

**How (CPU ray tracing, not rendered cubemaps).** The plan was to render tiny cubemaps with the
game's renderer; measured against Bevy's cost per camera view (shadow cascades are rendered per
view) that is hours per track on the Iris Xe. Instead the baker traces the session itself on the
CPU (`bake.rs`, `baking.rs`):
- The editor calls `rc_render::keep_cpu_copies()` so meshes, materials and textures stay on the
  CPU. Every opaque mesh in the session — `StandardMaterial` (props, decor) and `SurfaceMaterial`
  (ground, ribbons, walls, the room shell; its `StandardMaterial` base), the ceiling too (editing
  only hides it) — becomes world triangles in a parry `TriMesh` (≈ 0.83 M for Toy Town); albedo =
  base colour × the texture's mean colour; double-sided materials are noted.
- Lights as the game has them in clear weather: the key (always shadowed), point and spot lights
  (lumens / 4π, Bevy's range fall-off and cone). Shadow rays see only meshes that cast shadows in
  the game: a room's ceiling does not (the key reaches the room through it like a skylight).
  **The fill** (shadowless sky light) is off in the game where a volume exists and is baked into
  it instead: added to each probe where the probe sees its direction, so open floor keeps the
  fill's light and the floor under a deck or a table loses it; its bounce counts too. Rays that
  leave the scene see `rc_render::environment_radiance` (the room or sky the environment map
  shows, × `env_light`). Baked light is off while baking, so the fill is gathered.
- Probes over the track's bounds (indoors floor to ceiling, so tall walls have no seam; outdoors
  to the highest drivable point + 12 u), 2 u apart horizontally (coarser on large tracks, ≤ 12 k
  columns), ≈ 8 layers. 512 rays per probe (Fibonacci set), two passes: direct light + sky, then
  one more bounce through pass 1's volume. Probes that mostly see back faces are inside solid
  geometry and take, per face, the darkest valid neighbour (a probe inside a 0.25 u deck must not
  borrow the sunlit top's light for the underside). RGB9E5, ≈ 0.7–1 MB per volume.
- Each job reloads the session in its layout and time (clear weather), waits 30 frames, gathers on
  the main thread and traces on a background thread (the window keeps running; edits wait).
- **Measured on the Iris Xe laptop (dev build):** Toy Town 40 k probes, 0.83 M triangles: 21–36 s
  per volume, all 8 ≈ 4 min with reloads.

**Look (Toy Town normal/noon, the one baked volume; probes on/off in `target/agent-data/probes/`):**
with the fill baked in, the floor under the elevated deck and under the low helix ribbon goes
much darker (≈ 150–230 vs ≈ 600 on open floor, before direct sun) — the occlusion the shadowless
fill hid. Under a wide deck that reads close to black at noon exposure: physically about right,
an art call. Other times of day and the mirror layout are not baked.

## Phases
| Phase | Delivers | Done when |
|---|---|---|
| P1 Shell | shared app assembly, `rc_editor` binary, editor bundle, editing session + fly cam, overrides, open/save, F5/Esc | open Toy Town, save with no edits → identical file except comments; drive and return |
| P2 Tools | everything in "Editing scope", undo, lints, `rc_editor new`, `label`, catalog entry | build a new track from scratch and race it in the game |
| P3 Migration (done) | corridor lint; every track moved off `plan.py`; labels | no script writes a track without `--layout`; no lints; CI green |
| P4 Bake (done) | baker, `.irv` loader, weather scaling | Toy Town baked and shipped; occlusion and bleeding visible; runs on the Iris Xe. Not yet: the other tracks, a 3070 bench run |
| P5 Layout (done) | toolbar, left tabs (Assets placeholder, Track, Lighting = today's bake buttons), right pane; Items window and Notes removed; error toast; lints in the Track tab | everything P4 could edit is still reachable; no item is only reachable through an index chip |
| P6 Select & gizmo (done) | live drag previews (`DragPreview`, `EnvPreview` once P9 adds lights) applied by rc_track/rc_render, `MeshRayCast` picking, source tags + `PlacedProp::source`, hover/selection tint, markers (incl. dimmed unreachable items), `gizmo.rs` (G/R/C, L, Ctrl snap), Track-tab-only layout handles, Ctrl+press insert, decor rotation/scale data change. Commits still reload (P4 path) | select any object, including `dir: Rev` ones, by clicking it; move/rotate/scale decor, move/yaw props, move points; old decor files still load; a prop or decor drag is real-time (the release still reloads) |
| P7 Incremental (done) | `TrackPreview` for point drags, track key + per-source diffs, `RoomPiece` rebuild, `TrackSummary` re-insert, no `ChangeTrack` per edit, both equivalence tests | both tests pass; a point drag updates the road visibly while dragging on the Iris Xe; no session reload on any edit outside the reload list |
| P8 Library (done) | `library.rs`, `editor.library`, categories + search, placement mode with ghost (+ `editor place` verb), swap | build a lap of Toy Town's decor from the library without typing a model path |
| P9 Lighting (done) | `environment.ron` in the `Doc` (+ `to_ron`, save, undo), `EnvironmentDraft`, light objects + sun/fill markers, Lighting tab's time look, bake-stale flags; mirrored environment lights (game fix) | add a spot light, aim it, change noon's sun and exposure, undo it all, save, reopen: identical |
| P10 Settings | `rc_game::read_settings`, inserted before Boot | the editor renders at the player's preset; the file is untouched after an editor run |

## Risks / to verify first
- ~~A session with zero racers~~ — not needed: editing holds `RacePhase::Setup`, where no attempt
  entities exist (`tests/session.rs`).
- ~~Two bundles~~ — not needed: `bevy-react build` has a fixed entry; one bundle serves both.
- **A load failure** (a missing model, not the layout: that is built before it is accepted)
  stops the editor from restarting the session until the document changes; the error shows.
- ~~Bake time~~ — ≈ 10 s per volume on the Iris Xe (CPU tracing).
- ~~The Iris Xe path~~ — a single volume without binding arrays renders there (P4a test).
- **Repo size:** ≈ 6–8 MB of volumes per track (≈ 75 % after git's compression).
- **Point-drag rate (P7, verify first):** one `Track::build` is ≈ 22 ms on the Iris Xe plus mesh
  upload. If the road lags the mouse visibly, skip `fit_racing_line` (the SOR solve) in preview
  builds (a `BuildOptions` flag); the release build runs it. Colliders never rebuild per frame.
- **Diffed rebuilds touch game crates** (rc_track, rc_render): "also rebuild on a changed draft",
  behind the same spawn functions; the game binary (no drafts) is unaffected. The equivalence
  tests are the guard.
- **Placement and gizmo drags with a real mouse:** agents have no input injection; the
  `editor place` and `editor select` verbs cover the logic, a human pass covers the feel (P6, P8).
