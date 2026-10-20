# rc_editor — track editor (design note)

A separate executable for creating and editing tracks, test driving them, and baking their indirect
lighting. Status: **P1–P4 done** (open/create, edit everything, undo, test drive, save; every track
editor-owned; irradiance bake). The shipped tracks are not baked yet (§Irradiance bake: open).

```sh
npm --prefix ui run build                       # the editor uses the game's React bundle
cargo run -p rc_editor -- toy_town              # edit a track
cargo run -p rc_editor -- new back_lot --env street --name "Back Lot" --style Asphalt
```
| Input | Does |
|---|---|
| left click / left drag | select the nearest handle (or clear) / move it on its horizontal plane; **Alt**: up/down; **Ctrl**: snap to 0.5 u |
| right drag · WASD · Q/E · Shift | look · fly · down/up · faster |
| F5 | test drive / back to editing |
| Ctrl+S · Ctrl+Z · Ctrl+Y (Ctrl+Shift+Z) | save · undo · redo |
| Delete · Insert · Ctrl+D | delete the selection · insert a point/vertex/path point after it · duplicate |
| T · F | top view (over the selection or the view's focus) · fly to the selection |

Saves, settings and records of test drives go to `<game user data>/editor` (`--user-data-dir`
overrides). Automation as in the game: `--screenshot <png> [--screenshot-at <s>]`,
`--exit-after <s>`, and `RECHARGED_UI_SCRIPT` with `editor <command>` (`drive`, `undo`, `topView`,
…) and `editor select <kind> <index>` drives the editor through the real React channel.

## Decisions
| # | Decision | Consequence |
|---|---|---|
| D1 | **The editor is the source of truth** for track layouts | `tracks/<id>.ron` and `decor/<id>.ron` become editor-owned. The `plan.py` track sections are retired one track at a time (§Migration). Blender scripts keep building prop models and catalogs. |
| D2 | **UI in bevy-react** | The game's bundle: `App` shows `ui/src/editor/` while `editor.status` says editing (the game answers `editor.snapshot` with `null`); test drives show the game's race UI. Channels in rc_ui (`editor.rs`), handled by rc_editor. |
| D3 | **Irradiance volumes only; no lightmaps** (maybe later) | Bake indirect diffuse into one `IrradianceVolume` per track variant × time of day. Direct light stays real-time. |
| D4 | **No geometry or draw-call "compile" step** | The QA bench is well inside budget (1P 185–204 fps, 4P ≈ 105 fps at High, RTX 3070). Revisit only if a measurement says so (e.g. Iris Xe). |

## Architecture
```
crates/recharged   lib.rs: the game app (`recharged::game`) ── main.rs: the game binary
crates/rc_editor   lib.rs   EditorPlugin: Editor state, edit/undo pipeline, session control, commands, status
                   doc.rs   Doc (layout + decor + catalog entry): handles, add/insert/delete/duplicate, validate, new_track
                   fields.rs the inspector: one slot list per item serves reading and setting
                   catalog.rs rewrites the track's entry in catalog.ron (the rest of the file untouched)
                   view.rs  EditorViewPlugin: fly camera, picking, dragging, gizmos, focus point, cutaway
                   main.rs  `rc_editor [new] <track>` = recharged::game + both plugins
crates/rc_ui       editor.rs: editor.status / snapshot / command / select / set / options
ui/src/editor/     EditorLayer (toolbar, item list, inspector, notes), controls.tsx (fields)
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
  snapshots; whole documents, they are small), mirrored to `TrackDraft` (rc_track builds and
  rc_render draws it instead of the files) and to the game's `Catalog`, and the session rebuilds
  with `GameCommand::ChangeTrack` (≈ 0.7 s on the Iris Xe; generation counters make an edit during
  a rebuild trigger another one, never get lost).
- **Dragging previews with gizmos** (the control polygon follows the dragged point); the release
  commits one edit.
- **Test drive:** F5 lifts the hold (the intro is skipped with a `UiNav::Confirm`); F5 again sends
  `Restart` (→ Setup, no reload). Reverse/mirror drives (toolbar) reload the session in that
  layout and back to forward afterwards: editing is always forward, unmirrored. Time and weather
  (the track's allowed presets) apply to editing and drives. "Quit to menu" from the pause menu
  reopens the editing session.
- **Saving** writes the track (`TrackDef::to_ron`), decor (`decor_to_ron`) and, if it changed, the
  catalog entry, each through a temporary file + rename. Comments in those files are lost
  (`label` keeps per-point notes); `catalog.ron` keeps every comment outside the entry.

## Editing scope
| Item | Handles | Inspector |
|---|---|---|
| Track (+ catalog entry) | — | style, start, kill/wall height, bounds; name, environment, indoor, difficulty, laps, music, allowed times/weather and unlock (RON text), preview, hidden |
| Control points | yes | pos, width, bank, surface, walls, offroad margin (inherited fields show what they inherit; × unsets), label |
| Jumps (gaps) | red segment | from, to, dir; "Jump from here" on a point adds one |
| Ground areas / vertices | vertices (layer off by default) | surface, height, dir; vertex x/z |
| Item rows, props, Golden Battery, decor | yes (decor layer off by default) | pos, count / kind (all prop kinds) / yaw / model, scale, dir |
| Shortcuts / path points | path points | width, dir; point pos |

Add (toolbar of each list) puts the item at the view's focus — the first fixed collider under the
screen centre (a new shortcut leaves and rejoins the track around it).

**Not done:** lints are text without a position (no click-to-focus); prop kinds are one long list
(no filter by environment); a new decor item copies an existing model (the model path is a text
field); no orthographic view (T is a perspective view from above); dragging has not been tried
with a real mouse by an agent (no input injection on the test machine).

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
```
or **Bake light** (this time, forward) / **Bake all** in the editor's toolbar. Files:
`assets/data/lighting/<track>/<normal|mirror>_<time>.irv` (format and runtime: rc_render
`lighting/gi.rs`, `rc_render.md` §Baked indirect light). Reverse layouts use the forward bake (only
`dir`-filtered features differ); the sun is not mirrored, so mirror gets its own.

**How (CPU ray tracing, not rendered cubemaps).** The plan was to render tiny cubemaps with the
game's renderer; measured against Bevy's cost per camera view (shadow cascades are rendered per
view) that is hours per track on the Iris Xe. Instead the baker traces the session itself on the
CPU (`bake.rs`, `baking.rs`):
- The editor calls `rc_render::keep_cpu_copies()` so meshes, materials and textures stay on the
  CPU. Every opaque `StandardMaterial` mesh in the session (the ceiling too: editing only hides
  it) becomes world triangles in a parry `TriMesh` (≈ 0.8 M for Toy Town); albedo = base colour ×
  the texture's mean colour; double-sided materials are noted.
- Lights as the game has them in clear weather: directional lights (lux × colour; shadow rays for
  the ones that cast shadows — the indoor fill does not), point and spot lights (lumens / 4π,
  Bevy's range fall-off and cone). Rays that leave the scene see `rc_render::environment_radiance`
  (the room or sky the environment map shows, × `env_light`).
- Probes over the track's bounds (indoors floor to ceiling, so tall walls have no seam; outdoors
  to the highest drivable point + 12 u), 2 u apart horizontally (coarser on large tracks, ≤ 12 k
  columns), ≈ 8 layers. 512 rays per probe (Fibonacci set), two passes: direct light + sky, then
  one more bounce through pass 1's volume. Probes that mostly see back faces are inside solid
  geometry and take their neighbours' values. RGB9E5, ≈ 0.7–1 MB per volume.
- Each job reloads the session in its layout and time (clear weather), waits 30 frames, gathers on
  the main thread and traces on a background thread (the window keeps running; edits wait).
- **Measured on the Iris Xe laptop (dev build):** Toy Town 40 k probes, 0.78 M triangles: 8.6 s per
  volume; Backyard Bash 29 k probes: 9–10 s; all 8 volumes of a track ≈ 1.5 min with reloads.

**Look (before/after screenshots, `target/agent-data/gi/`):** indoors the overall level matches
the environment map (units check out) and gains contact occlusion (shelf undersides) and colour
bleeding (the orange track warms the ceiling; lamp pools glow on the ceiling at night). Outdoors
shaded surfaces facing away from the open sky (the garden fence) get **darker**: the environment
map's art-boosted sky fill (≈ 3× physical, so shade reads) used to reach every surface unoccluded.
**Open (art):** accept, or add a per-environment `baked_light_scale`; the shipped tracks are not
baked yet (≈ 40 MB compressed for all seven, no LFS in this repo).

## Phases
| Phase | Delivers | Done when |
|---|---|---|
| P1 Shell | shared app assembly, `rc_editor` binary, editor bundle, editing session + fly cam, overrides, open/save, F5/Esc | open Toy Town, save with no edits → identical file except comments; drive and return |
| P2 Tools | everything in "Editing scope", undo, lints, `rc_editor new`, `label`, catalog entry | build a new track from scratch and race it in the game |
| P3 Migration (done) | corridor lint; every track moved off `plan.py`; labels | no script writes a track without `--layout`; no lints; CI green |
| P4 Bake (done) | baker, `.irv` loader, weather scaling | Toy Town and Backyard Bash baked in a scratch copy; occlusion and bleeding visible; runs on the Iris Xe. Not yet: shipped volumes, a 3070 bench run |

## Risks / to verify first
- ~~A session with zero racers~~ — not needed: editing holds `RacePhase::Setup`, where no attempt
  entities exist (`tests/session.rs`).
- ~~Two bundles~~ — not needed: `bevy-react build` has a fixed entry; one bundle serves both.
- **A load failure** (a missing model, not the layout: that is built before it is accepted)
  stops the editor from restarting the session until the document changes; the error shows.
- ~~Bake time~~ — ≈ 10 s per volume on the Iris Xe (CPU tracing).
- ~~The Iris Xe path~~ — a single volume without binding arrays renders there (P4a test).
- **Repo size:** ≈ 6–8 MB of volumes per track (≈ 75 % after git's compression).
