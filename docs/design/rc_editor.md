# rc_editor — track editor (design note, proposal)

A separate executable for creating and editing tracks, test driving them, and baking their indirect
lighting. Status: **P1 done** (open, fly, test drive, save); P2–P4 to do.

```sh
npm --prefix ui run build                 # the editor uses the game's React bundle
cargo run -p rc_editor -- toy_town        # F5 drive / back · Ctrl+S save · RMB look · WASD · Q/E
```
Saves, settings and records of test drives go to `<game user data>/editor` (`--user-data-dir`
overrides). Automation as in the game: `--screenshot <png> [--screenshot-at <s>]`,
`--exit-after <s>`, and `RECHARGED_UI_SCRIPT="8 editor drive; 20 editor drive"` toggles test drives
through the real React channel.

## Decisions
| # | Decision | Consequence |
|---|---|---|
| D1 | **The editor is the source of truth** for track layouts | `tracks/<id>.ron` and `decor/<id>.ron` become editor-owned. The `plan.py` track sections are retired one track at a time (§Migration). Blender scripts keep building prop models and catalogs. |
| D2 | **UI in bevy-react** | The game's bundle: `App` shows `ui/src/editor/` while `editor.status` says editing (the game answers `editor.snapshot` with `null`); test drives show the game's race UI. Channels in rc_ui (`editor.rs`), handled by rc_editor. |
| D3 | **Irradiance volumes only; no lightmaps** (maybe later) | Bake indirect diffuse into one `IrradianceVolume` per track variant × time of day. Direct light stays real-time. |
| D4 | **No geometry or draw-call "compile" step** | The QA bench is well inside budget (1P 185–204 fps, 4P ≈ 105 fps at High, RTX 3070). Revisit only if a measurement says so (e.g. Iris Xe). |

## Architecture
```
crates/recharged      lib.rs: the game app (`recharged::game`) ── main.rs: the game binary
crates/rc_editor      lib.rs: EditorPlugin ── main.rs: `rc_editor <track>` = recharged::game + EditorPlugin
crates/rc_ui          editor.rs: `editor.status` / `editor.snapshot` / `editor.save` / `editor.drive`
ui/src/editor/        EditorLayer (same bundle as the game)
```
- **The game, unchanged, renders the edited track.** On `OnEnter(Menu)` the editor starts a Quick
  Race (no AI) on the track; while editing, `SimSet::Race` (rc_race's phase driver) does not run,
  so the session stays in `RacePhase::Setup`: track, props, lighting, no cars. The editor writes
  the `GameCamera`'s transform (rc_camera holds its pose without a racer). *(P2: an orthographic
  top-down toggle.)* The room shell, props, decor, ground, lighting and weather all
  come from the existing plugins, so nothing is duplicated.
- **In-memory layout.** rc_track builds the `TrackDraft` resource instead of the file when the
  session's track has its id, so test drives and rebuilds use unsaved edits; disk is written only
  on save (temporary file + rename). *(P2: the same for rc_render's decor file once decor is
  editable.)*
- **Rebuild on commit, preview while dragging.** A finished edit (mouse release, field change)
  reloads the session: Unloading → Loading takes about 0.25 s. While dragging, the editor draws
  the centreline and edges with gizmos from rc_track's pure spline code.
- **Test drive:** F5 lifts the hold (Setup → Intro → Countdown → Racing with the player's car);
  F5 again sends `Restart` (→ Setup, no reload) and editing resumes at the same camera position.
  The pause menu works as in the game; "Quit to menu" reopens the editing session. *(P2: pick the
  variant, time and weather; skip the intro.)*
- **Undo:** a stack of `(TrackDef, DecorDef)` snapshots, one per committed edit. The defs are
  small, so no command pattern.

## Editing scope ("all track properties")
| Area | Interaction |
|---|---|
| Control points | select, move (3D handles and top-down), insert or delete; panel: width, bank, surface, walls, offroad margin, optional **`label`** (new field: keeps `plan.py`'s `// 2: T1` notes after the switch) |
| Track settings | style, start, kill height, wall height, bounds |
| Ground polygons | vertex edit, surface |
| Props / decor | place from the `PropCatalog` list (filtered by environment + common), move, yaw, `dir` |
| Gaps, shortcuts, item rows, golden battery | handles on the centreline / in the world + panels, `dir` |
| Catalog entry | `TrackInfo` for the track (name, environment, difficulty, allowed weather and times, hidden) |
| Lints | `Track::lints()` + rc_track analysis, live in a panel. Click to focus the problem. |

**New track:** a dialog asks for id, name, environment and style. It writes a minimal oval (like
`test_oval`), an empty decor file and a `hidden: true` catalog entry.

**Saving** uses pretty RON in the existing field order, with `implicit_some`. Comments are lost on
the first save, which is why `label` exists.

## Migration off `plan.py` (D1)
1. Port `plan.py`'s corridor check (no static obstacle in the racing corridor) to `rc_track::analysis`
   as a lint, so every track gets it, not just generated ones.
2. Per track, when it is next touched: open it in the editor and move the comment labels into
   `label`. Save, then delete the layout and decor writer from that environment's `plan.py`.
   Remove the `GENERATED` header.

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
| P2 Tools | everything in "Editing scope", undo, lints, new-track dialog, `label` | build a new track from scratch and race it in the game |
| P3 Migration | corridor lint; first track moved off `plan.py` | `plan.py` no longer writes that track; CI green |
| P4 Bake | baker, `.irv` loader, weather scaling | Sofa Circuit baked; visible occlusion under furniture; bench within budget on the 3070 **and** runs on the Iris Xe |

## Risks / to verify first
- ~~A session with zero racers~~ — not needed: editing holds `RacePhase::Setup`, where no attempt
  entities exist (`tests/session.rs`).
- ~~Two bundles~~ — not needed: `bevy-react build` has a fixed entry; one bundle serves both.
- **A load failure** stops the editor from restarting the session (it would loop through the
  menu); the error shows in the editor. P2 must build edits with `Track::build` before sending a
  draft, so a bad edit is rejected in the editor instead.
- **Bake time:** ~50 k probes × 6 views. Bevy renders shadow cascades per view, so measure on one
  track before committing to 2 u spacing. The budget is ≤ 10 min per track (8 volumes) on the
  3070.
- **The Iris Xe path:** a single irradiance volume without binding arrays has not been tried there
  (see upstream-notes U7).
- **Repo size:** ≈ 10 MB of volumes per track.
