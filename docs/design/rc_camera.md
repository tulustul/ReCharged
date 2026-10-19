# rc_camera — cameras & the UI host (design note)

## UI host rig (ARCHITECTURE §5.8)
Exactly one camera carries `UiHost` + `IsDefaultUiCamera`; each is spawned with both markers and
scoped to its state, so hosts never overlap (`tests/rig.rs` asserts it per state).

| State | Cameras | UI host |
|---|---|---|
| Menu | `MenuCamera` (Camera3d, showroom drift) | menu camera |
| Loading | `LoadingCamera` (Camera2d, navy clear) | loading camera |
| InRace, 1 player | one `GameCamera` (full window) | the game camera |
| InRace, 2–4 players | `GameCamera`s with viewports (order 0..n) + `OverlayCamera` (order 10) | overlay |
| Boot / Unloading (1 frame) | none | none: the window clears to `ClearColor` = `BACKDROP` |

`GameCamera`s spawn in `OnEnter(AppState::InRace)` with `DespawnOnExit(RaceSession)`; restarts keep
them (and the host) and `OnEnter(RaceAttempt)` re-binds them to the new racers.
rc_camera adds no post-processing: rc_render decorates the cameras (`On<Add<GameCamera>>` …).

### Split-screen overlay: rig B (verified, ARCHITECTURE §5.8 updated)
```rust
Camera2d, Msaa::Off, OverlayCamera, UiHost, IsDefaultUiCamera,
Camera { order: 10, clear_color: ClearColorConfig::Custom(Color::NONE),
         output_mode: CameraOutputMode::Write {
             blend_state: Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING),
             clear_color: ClearColorConfig::None }, .. }
```
- **Rig A (`ClearColorConfig::None` overlay) fails with HDR player cameras.** The LDR overlay never
  shares a main texture with the HDR players, and a `None` camera always loads last frame's pixels
  (`bevy_render` `texture_attachment.rs`: `(None, _) => LoadOp::Load`; `MsaaWriteback` docs:
  "preserves content across frames"). The UI accumulates: translucent panels turn opaque within a
  few frames, and changed or hidden UI stays on screen.
- Rig B clears the overlay to transparent each frame. bevy_ui blends straight alpha onto that, which
  yields premultiplied colour, so the overlay is blitted with premultiplied blending.
  `backdropFilter` has nothing to sample there, so split-screen panels use translucent fills.
- Evidence (2P top/bottom, `Hdr` + `Bloom::NATURAL` on the player cameras, the real React menu UI,
  every React root hidden at 5 s, frames at 4 s and 7 s): `target/agent-data/T04/shots/`
  `rigA_2p_000/001.png` (UI still visible after hiding, opaque panel) vs `rigB_2p_000/001.png`
  (translucent panel, UI gone after hiding).
- Rig B for every layout, HDR + bloom, UI on top: `rig_1p.png`, `rig_2p.png` (top/bottom),
  `rig_2vp.png` (left/right), `rig_3p.png` (free bottom-right quadrant shows `BACKDROP`),
  `rig_4p.png`. Bloom stays inside each viewport.
- How it was verified: no track data existed yet, so a temporary probe (since removed) built the
  same race rig (`rig::spawn_rig`) in the menu, added `Hdr` + `Bloom` to the game cameras and an
  emissive test scene. Source kept for reference at `target/agent-data/T04/rig_probe.rs`.
- Anti-aliasing stays rc_render's job: no TAA on viewport cameras (squeezed, Bevy digest §0.2).

## Layout & lens (pure, `layout.rs`)
- `split_rects(players, split, physical size)`: 1 = full, 2 = halves (`Horizontal` = top/bottom,
  default; `Vertical` setting = left/right), 3 = TL/TR/BL (BR left to the React minimap panel),
  4 = quadrants. Integer cells tile the window exactly (odd sizes included).
- `PlayerViewports` = cells / scale factor (logical px), sorted by slot. Recomputed when the window
  size, scale factor, split setting or camera set changes.
- FOV: `Settings.graphics.fov_deg` is the horizontal FOV at 16:9. Hor+ keeps the corresponding
  vertical FOV for every aspect. Only 2P top/bottom views (32:9) clamp the horizontal FOV to 100°.
  Speed widens it by up to 8° at 36 u/s (5★ top speed) plus 4° while boosting, capped at +12° and
  eased over 0.35 s.

## Following (`follow.rs`, `pose.rs`, `director.rs`)
- Subject pose: the `Visual` proxy's `Transform` when rc_render made one, else
  `PoseHistory::interpolate(overstep)` (smooth even without proxies), else `Transform`.
- Chase (4.6 u back, 1.7 u up) / far (7.5 u, 2.8 u) / hood (car space `(0, 0.6, −0.4)`), cycled by
  `CycleCamera` (chase → far → hood). Heading lags through a critically damped spring (0.22 / 0.3 s).
  The followed point lags 0.03 s in x/z, which keeps the car planted, and 0.15 s in y, which soaks up
  bumps. Look-ahead = 0.12 s of speed, at most 4 u.
- Look-back (`CarControl::look_back`): snaps to the front of the car looking back, or a reversed
  hood view.
- Obstruction: a ray from the look-at point to the eye, `QueryFilter::only_fixed()`, so cars and other
  dynamic bodies never block. The boom pulls in at once (0.3 u margin, at least 1.2 u) and regrows
  over 0.35 s. When even the shortest boom would end inside an obstacle — the look-back eye in front
  of a car stopped against a ramp, a deck face or a wall (B23) — the boom swings up over the car in
  its vertical plane (by 0.6, then 1.2 rad, never steeper than ≈ 85°: straight down flips the view)
  to the first direction with 1.2 u of room (else the roomiest tried) and looks down at the car.
  The swing snaps, like the pull-in (`ponytail:`). Checked in a window (Backyard Bash, car parked
  nose-first against the deck face beside the ramp, look back held; `--input-script` with
  race-tick timing): with the swing disabled the look-back view is the deck's boards filling the
  screen, with it the camera looks down at the car from above the deck edge.
- Shake: trauma from `CarImpact` (×0.6 strength), `CarLanded` (≥ 0.3 s air, ×0.35), `Explosion`
  (0.9 at the centre, 0 at 3 × radius). Angle = trauma² × (2°, 2°, 1.2°), decays 1.6/s. Off with
  `camera_shake`.
- Intro fly-by (first attempt, `RacePhase::Intro`, 3 s): a descending sweep that ends exactly on the
  chase pose. Finish: an orbit around the slot's finished racer. `CycleCamera` then spectates each
  other racer in grid order and returns to the orbit. A slot without a racer spectates the leader.
- Teleports (`Teleported`) and re-binding snap the camera instead of easing. While paused the
  cameras freeze (dt = 0) but still react to resizes.

## Menu showroom
`SHOWROOM_FOCUS` (rc_core, origin) is where rc_render's turntable car stands. The camera drifts
±26° around a three-quarter front view: wide (9.5 u) or, for `BackdropFocus::Car`, close (4.4 u),
eased over 0.6 s.

## Tests
`cargo test -p rc_camera`: layouts for every count, orientation and aspect (tiling, 3P free
quadrant), logical rects, Hor+ and the 2P clamp, spring convergence and frame-rate independence,
chase / look-back / hood / far framing, obstruction pull-in, the boom swinging up over the car when
even the shortest boom is blocked (and keeping its direction otherwise), fly-by end pose, orbit,
widening, shake, spectate order, mode cycle, showroom framing. Headless `tests/rig.rs`: one host per
state, 3P overlay + viewports + `PlayerViewports`, 2P orientation setting and window resize, cameras
kept across restart and re-bound, camera-mode cycling.

## Known limitations
- The fly-by circles the car; flying along the `Track` spline waits for T02's query API (`ponytail`).
- One hood offset for every car. Per-car offsets need model metadata from rc_render (`ponytail`).
- Tuning values are named consts rather than a data file; move them to `assets/data/camera.ron` if
  playtests ask for live tuning.
