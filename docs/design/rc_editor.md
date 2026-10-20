# rc_editor — track editor (design note)

A separate executable for creating and editing tracks, test driving them, and baking their indirect
lighting. Status: **P1–P3 done** (open/create, edit everything, undo, test drive, save; every track
editor-owned); P4 (irradiance bake) to do.

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

## Irradiance bake (D3)
**Constraints found in the code:**
- Binding arrays are off on every adapter (upstream-notes U7, the Intel Mesa hang). Without them,
  Bevy uses **only the irradiance volume closest to the view**, so we use **one volume per track**
  covering the drivable bounds.
- Shader priority is lightmap > irradiance volume > environment map for *diffuse*. Inside the
  volume, the env map's diffuse is skipped, but its specular stays. So the bake must include sky
  light, which also gives sky occlusion under tables and shelves.
- The sun and window light are **not mirrored** in the mirror variant. Flipping a volume would put
  the bounce on the wrong side, so mirror gets its own bake. Reverse shares the forward bake (only
  `dir`-filtered features differ).
- That gives **8 volumes per track**: {normal, mirror} × {Morning, Noon, Sunset, Night}.

**Baker** (an editor command plus a headless `rc_editor --bake <track>`), using the game renderer
with no custom ray tracer:
1. Load the variant at the time of day, Clear weather, no cars, dynamic props at rest.
2. Probe grid: track bounds horizontally, from the floor to the highest drivable point + 6 u. 2 u
   spacing by default (the car is 2 u long), with a per-track override.
3. Per probe: render a small (16²) HDR cubemap with exposure fixed and no tonemapping (scene light
   units). Convolve it to the 6 ambient-cube values (cosine-weighted irradiance per axis).
4. Probes inside solid geometry (mostly back faces seen) are marked invalid and filled from valid
   neighbours, so solid props don't leak darkness.
5. Optional second pass with pass 1's volume active gives a second bounce.
6. Pack the result into Bevy's layout: a 3D texture of (Rx, 2Ry, 3Rz), RGB9E5.

**Files:** `assets/data/lighting/<track>/<normal|mirror>_<time>.irv`, a raw header (dims, bounds)
plus RGB9E5 texels. It is read by rc_render into an `Image`, so no ktx2 feature is needed. At 2 u
spacing Aisle Rush is about 104×6×85 probes, ≈ 1.3 MB per volume and ≈ 10 MB per track.

**Runtime (rc_render):**
- If the file exists, spawn the `LightProbe + IrradianceVolume` scaled to its bounds.
- Weather scales `intensity` by the look's sky/key factor (Cloudy, Rain, Storm, Fog).
- Lightning and headlights stay real-time only.
- With no file, lighting is the same as today, so tracks can be baked one at a time.

## Phases
| Phase | Delivers | Done when |
|---|---|---|
| P1 Shell | shared app assembly, `rc_editor` binary, editor bundle, editing session + fly cam, overrides, open/save, F5/Esc | open Toy Town, save with no edits → identical file except comments; drive and return |
| P2 Tools | everything in "Editing scope", undo, lints, `rc_editor new`, `label`, catalog entry | build a new track from scratch and race it in the game |
| P3 Migration (done) | corridor lint; every track moved off `plan.py`; labels | no script writes a track without `--layout`; no lints; CI green |
| P4 Bake | baker, `.irv` loader, weather scaling | Sofa Circuit baked; visible occlusion under furniture; bench within budget on the 3070 **and** runs on the Iris Xe |

## Risks / to verify first
- ~~A session with zero racers~~ — not needed: editing holds `RacePhase::Setup`, where no attempt
  entities exist (`tests/session.rs`).
- ~~Two bundles~~ — not needed: `bevy-react build` has a fixed entry; one bundle serves both.
- **A load failure** (a missing model, not the layout: that is built before it is accepted)
  stops the editor from restarting the session until the document changes; the error shows.
- **Bake time:** ~50 k probes × 6 views. Bevy renders shadow cascades per view, so measure on one
  track before committing to 2 u spacing. The budget is ≤ 10 min per track (8 volumes) on the
  3070.
- **The Iris Xe path:** a single irradiance volume without binding arrays has not been tried there
  (see upstream-notes U7).
- **Repo size:** ≈ 10 MB of volumes per track.
