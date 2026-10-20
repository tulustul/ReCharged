# Editor v2 (P5–P10) — design review 1
Verdict: **CHANGES REQUESTED.** P5 (layout only) may start now. P6–P10 wait for the doc
amendments below. The architecture holds, but the incremental-rebuild plan and the live-drag plan
need these specifics or implementers will build them wrong.

Scope: `docs/design/rc_editor.md`: §Editor v2, D5–D10, Architecture, the P5–P10 rows and the new
Risks. I checked them against the code at `2cf223f` + the working tree. I do not re-argue the
product decisions the user already settled.

Checked and fine: re-inserting `Track` replaces the resource entity's component
(`bevy_ecs-0.20.0/src/world/mod.rs:3086-3101`, `InsertMode::Replace`), so `is_added()` stays false
and `gi::spawn_volume` (`lighting/gi.rs:240`) and `spawn_decor` (`props/decor.rs:51`) do not
re-fire. The AI, cars, pickups and the battery spawn per attempt (`rc_race/src/spawn.rs:28`,
`rc_items/src/pickups.rs:24`), so editing never sees them. `dress_props` already runs in `InRace`
(`props/mod.rs:70`), so newly spawned `Prop`s get dressed. Time-look fields that `compute_look`
reads follow `ActiveEnvironment.def` live (`weather_fx.rs:314-334`, env map `room.rs:147`).
The decor mirror rule (negate X, yaw and roll; keep pitch) is the correct reflection of a YXZ
rotation.

## Findings

**1. major: `geometry_hash` is the wrong diff key for track rebuilds.**
- It hashes props (`rc_track/src/analysis.rs:463-475`), so every prop move would respawn all track
  colliders and visuals.
- It leaves out `style`, explicit `bounds`, shortcuts and the start (`analysis.rs:446-478`), so a
  style or bounds edit would rebuild nothing.
- Auto bounds grow with prop positions (`build.rs:265-269`), so a prop edit can legitimately
  change the shell.

*Fix:* rebuild colliders and visuals when *(the `TrackDef` with `props`, `item_rows`,
`golden_battery` and labels cleared) or `track.bounds()`* differs from what was built. Keep
`geometry_hash` for records only.

**2. major: prop indices do not line up, and direction-limited props and decor cannot be
reached.**
- `Layout::new` drops props inactive in the direction (`layout.rs:87-97`).
- `PlacedProp` keeps no source index (`place.rs:79-87`), so `track.props()[i]` ≠
  `doc.def.props[i]`. Toy Town has 13 direction-limited props.
- A `dir: Rev` prop or decor item has no entity in the forward editing session, so it cannot be
  clicked. P5 removes the index chips, so it becomes unreachable, which breaks P5's own "done
  when" row.

*Fix:*
- Carry `source: usize` through `LayoutProp` → `PlacedProp`, and do the same in
  `rc_render::props::decor::load_decor`. `PropSource` / `DecorSource` take that value.
- Items inactive in the editing direction get a marker drawn from the `Doc`, shown dimmed.

**3. major: live drags break the single-writer rule, and prop models are not linked to their
props.**
- A static prop's model is a separate, unlinked `PropModel` entity (`props/mod.rs:145-151`).
  Respawning the `Prop` leaves the old model behind.
- A dynamic prop's model sits under its `Visual` proxy, which `pose_proxies` overwrites every
  frame (`proxy.rs:28-37`). Hits on it walk `ChildOf` up to the proxy, never to the `Prop`.
- The editor writing a rapier body's `Transform` violates ARCHITECTURE §3.3 ("Nobody but
  rc_vehicle (and rapier) writes a rapier body's `Transform`", `ARCHITECTURE.md:221-222`) and
  PRACTICES §4 one-writer (`PRACTICES.md:88`).
- Environment lights carry no tag at all (`lighting/mod.rs:261-295`).

*Fix:*
- The editor writes the live pose into its own draft: `TrackDraft.preview` /
  `EnvironmentDraft.preview: Option<(Source, Transform)>`. rc_track and rc_render each apply it to
  their own tagged entities. Dynamic bodies also get `Teleported`.
- `dress_props` copies `PropSource` onto the `PropModel` root, and rc_render despawns that model
  when its `Prop` goes: store the model entity on the `Prop`, or use an `On<Remove, Prop>`
  observer.
- `EnvLightSource(i)` uses the index into `def.lights` (before the `on_at` filter).
- Update the Architecture "Outputs" list. The editor then writes no game components beyond the
  ones it already writes (`GameCamera` pose, `Overhead` visibility).

**4. major: control-point preview builds must not replace `Track`.**
- If they do, rc_track's diff (finding 1) respawns colliders on every preview, which contradicts
  "Colliders wait for the release".
- `push_status` re-reads lints on every preview (`lib.rs:711`).
- A late async result can overwrite the committed `Track`.

*Fix:*
- Previews go into a separate editor-written resource (`TrackPreview(Track, generation)`). Only
  rc_render's visuals read it, falling back to `Track`.
- Drop results older than the last commit. On a build error, keep the last good preview and show
  the error; release then rejects the edit and snaps back.
- Remove the preview on commit, and insert `TrackSummary` together with `Track` (see
  `spawn.rs:80-81`).

**5. major: several things are built once per session, and the in-place path does not list
them.**

| What | Where it is built or read | Effect on an in-place edit |
|---|---|---|
| Room lamps and window glass | sized from `bounds` once, behind `RoomBuilt` (`room.rs:43,64,185-189`) | Bounds and lamp-grid edits (the Lighting tab edits the grid) never move them |
| Prop lamp lights | spawned only if `practicals > 0` at dress time, intensity fixed (`props/mod.rs:190-194`) | `practicals` edits do nothing live |
| Bulb and fixture dimming | applied at instancing (`lamps.rs:199`) | `fixture_materials` edits do nothing live |
| `shadow_distance` | re-applied only on a tier change (`lighting/mod.rs:390`) | edits do nothing live |
| Sky / atmosphere | spawned once | edits do nothing live |
| Catalog `environment` | material library and barrier style (`track_visuals/mod.rs:67,99`), ambience (`rc_audio/src/ambience.rs:63`) | read at session start only |
| Catalog `indoor`, `music` | `rc_audio/src/bank.rs:191-194` | read at session start only |
| `TrackSummary` | minimap (`hud/systems.rs:126`), weather race length (`rc_weather/src/lib.rs:67`) | stale unless re-inserted |
| `check_geometry` (records and ghost) | runs only `OnEnter(InRace)` (`rc_game/src/results.rs:97`) | not re-checked after edits |

*Fix (lazy):*
- Re-insert `TrackSummary` with `Track`.
- Tag the room's entities and rebuild them when bounds or `room` change (remove `RoomBuilt` and
  despawn by tag).
- Edits to `practicals`, `fixture_materials`, `shadow_distance`, `sky` and the catalog's
  `environment` / `indoor` / `music` trigger the same reload as a time change.

*Flag to the user:* this adds rare cases to D6's reload list. Also say that the editor's own
test-drive records may compare times across geometries (harmless: they live in `editor/`).

**6. major: environment lights are never mirrored.**
- `EnvSpawner::spawn` uses `light.pos` / `target` as they are (`lighting/mod.rs:261-295`), while
  `Layout::mirror` flips everything else.
- Toy Town's display spots aim at hero props (`assets/maps/toy_town/environment.ron:8-12`). In
  mirror they light empty floor, and the mirror bake bakes it that way.
- The doc says nothing about mirror for edited lights.

*Fix:*
- Decide and write it in §Data changes. Recommended: point/spot lights mirror with the layout
  (negate `pos.x` and `target.x` when `variant.mirror`). The sun stays unmirrored, as §Irradiance
  bake already says.
- Add a check to the bake tests or the mirror tests.
- Lights not on at the previewed time still get a marker from the `Doc`, shown dimmed.

**7. major: dragging from the library into the 3D view cannot work as written.** bevy-react keeps
`PointerCapture.dragging` true for the whole press gesture
(`bevy_react_core-0.8.1/src/reconcile/pointer.rs:259-268`). It also keeps `over_ui` true while
the pressed node is `Pressed`, even with the cursor outside it (`reconcile/picking.rs:139-148`).
So Bevy never "picks the drag up once the pointer leaves captured UI", and "released over the UI"
cannot be read from `PointerCapture`.

*Fix:*
- After `editor.place`, a placement mode ignores `PointerCapture`. The ghost follows
  `Window::cursor_position`, and placement ends on `ButtonInput<MouseButton>` Left
  `just_released`.
- "Over the UI" = the cursor is inside the left or right pane rect: query the pane roots'
  `ComputedNode`, or have React report the pane widths.
- The same mode also serves click-to-arm (click an entry, then click in the view) and an
  automation verb `editor place <id>`, so agents can test it without input injection.

**8. major: G/R/S/L clash with the fly keys.**
- `fly` moves the camera on WASD/QE whenever Ctrl is up, with no right button needed
  (`view.rs:336-342`). Pressing S switches to scale and also flies backwards.
- The table row "right drag · WASD … unchanged" keeps the clash.

*Fix:*
- WASD/QE fly only while the right button is held. G/R/S/L/F/T act only while it is not held and
  Ctrl is up (Ctrl+S save and Ctrl+D duplicate stay as they are).
- Left-press order: gizmo handle > marker/mesh > Ctrl+click insert on the road. Ctrl at the press
  on the road = insert; Ctrl held during a drag = snap.
- While a gizmo drag or a placement is live, Ctrl+Z/Y, Delete, F5 and tab switches cancel it
  first. Esc cancels a drag or placement before it deselects.
- Add P (probes) to the table.

**9. major: the equivalence test cannot run where the doc puts it.**
- rc_testkit is headless (no window, GPU or rc_render; `rc_testkit/src/lib.rs:4`). The editor's
  session test runs without `GraphicsPlugin` (`rc_editor/tests/session.rs:28-38`).
- Decor, prop models, environment lights and track visuals all live in rc_render, which needs the
  `AssetServer`, glTF and the `MaterialLibrary`.

*Fix:*
- (a) A headless ECS test of rc_track: `Track`, plus colliders by `TrackPiece` and `PropSource`
  with their transforms.
- (b) rc_render's spawn code takes a pure plan (`Vec<(Source, model/light, Transform)>` for decor,
  prop models and env lights, plus the track-mesh key from finding 1) that both the load path and
  the diff path use. A unit test checks incremental plan == fresh plan.

**10. major: D10 as written would modify the player's settings file.**
- rc_game's `save::load` renames a corrupt or newer-version `settings.ron` aside
  (`rc_game/src/save.rs:103-123`).
- `boot_saves` reads from `data_dir`, which the editor points at `<game>/editor`
  (`rc_editor/src/main.rs:197-200`).

*Fix:*
- Add a read-only `rc_game::read_settings(path)` (`parse_versioned` + `sanitize`, no backup). The
  editor's `main` inserts the resulting `Settings` before Boot; `boot_saves` already skips when
  `Settings` exists (`flow.rs:132`).
- State what `--user-data-dir` does to the settings source.
- State that a pause-menu `ApplySettings` still writes `editor/settings.ron`, which is never read,
  or turn that write off.

**11. minor: use on-demand `MeshRayCast` instead of `MeshPickingPlugin`.**
- The plugin raycasts every pointer every frame against every mesh, test drives included (Toy
  Town ≈ 0.83 M triangles). It also feeds the `HoverMap` that bevy-react reads.

*Fix:*
- Use the `MeshRayCast` system param only while editing, on cursor move or click. Its filter
  accepts entities with a source-tagged ancestor; in the Track tab nothing on the road is tagged
  (road and ground stay inert).
- Ctrl+click on the road uses the rapier ray, as `focus_point` does (`view.rs:180-197`).
- Note that picking depends on `keep_cpu_copies()` (`main.rs:231`).

**12. minor: the tint has to cover more cases than the doc says.**
- Fallback props and the track use `SurfaceMaterial` (`props/mod.rs:172-185`), not only
  `StandardMaterial`.
- A dropped item is selected before its glb is instanced.
- `dim_bulbs` swaps lamp materials on `WorldInstanceReady` (`lamps.rs:199`).
- "At most two clones" is really per object: one per mesh of the hovered and the selected object.

*Fix:*
- Tint both material types.
- Re-apply on `WorldInstanceReady` when the instance belongs to the hovered or selected object.
- Deselect before baking.

**13. minor: decor data details an implementer will hit.**
- `deny_unknown_fields` plus the old form needs a custom deserialize: `scale` as `f32 | (f32,
  f32, f32)`, and `yaw` maps to `rot.1`.
- Write `rot` and `scale` only when they differ from the default, so files stay short.
- Extend `every_decor_file_survives_a_save` (`decor.rs:107`) with an old-form case.
- Per-axis scale is stored in the item's local axes, so the scale gizmo always works in local
  space, whatever L says.

**14. minor: leftover implicit decisions.** Write these down in one line each:
- Swapping point → spot needs a default target (e.g. 5 u below); spot → point drops the target.
- What a preview build failure shows while dragging (finding 4).
- Which `EditorItem`s the Lighting tab selects: `Light(i)` indexes `def.lights`, not the spawned
  list.
- What happens when the catalog edit removes the previewed time or weather from the allowed list:
  fall back to the first allowed value and reload.
- That the P4 layer toggles (`toggleLayer`, `Editor.layers`) are retired in favour of "Track tab
  shows handles", including the automation verb.
