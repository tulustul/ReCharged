# T04 — Input (`rc_input`) & cameras (`rc_camera`)

**Goal:** every device drives the right player and the menus; every local player gets a camera
in the right viewport; the UI host camera rig works in single view and split-screen.

**Read first:** PRACTICES; ARCHITECTURE §1–4, §5.3, §5.8; GDD §11, §12, §13 (viewport needs);
digests: bevy §0, §9 (cameras, split screen, overlay), §10 (AA), §11 (input, gamepads, rumble),
§12 (windows); bevy-react §7 (input & navigation), §8 (cameras).

## Owned paths
`crates/rc_input/**`, `crates/rc_camera/**`, `docs/design/rc_input.md`, `docs/design/rc_camera.md`.
**rc_core grant:** additive edits in `crates/rc_core/src/input.rs` only.

## Deliverables — rc_input
1. Devices `KeyboardA`, `KeyboardB`, `Gamepad(Entity)`; default bindings (GDD §12.1) in
   `Settings.controls` defaults (coordinate: rc_core `settings` is orchestrator-owned — propose
   changes in your report if the defaults need structure changes).
2. `LocalPlayers`: single-player → slot 0 takes all devices; `LobbyState.open` → join/leave per
   GDD §12.3 (`DeviceEvent::Joined/Left`); `LastUsed` device kind per slot (for glyphs).
3. PreUpdate intents: analog + latched edges, digital steering ramp, dead zone, sensitivity,
   auto-accelerate (never during countdown before GO), reset button level → `CarControl.reset_held`
   (rc_race times the hold). FixedUpdate `SimSet::Drivers`: write `CarControl` for `LocalPlayer`
   racers without `AiDriver`, consume edges exactly once per fixed tick. **Driving edges are latched
   only while `PauseState::Running` and no menu/dialog is open; all latched edges are cleared on
   entering `Running` and on `OnEnter(RacePhase::Setup)`** (review 2, N2).
4. **Menu navigation** → `UiNav` messages with GDD §12.2 rules (stick 0.5/0.3, repeat 0.35 s then
   0.1 s, per-scheme lobby keys, slot tagging, `repeat` flag); suppressed while a bevy-react text
   field has focus (`InputFocus`), during rebind capture, and while `RacePhase::Racing` and not
   paused (D-pad steering must not spam the UI).
5. Pause: Pause action → `PauseRequest` only while `PauseState::Running`. Gamepad disconnect →
   `DeviceEvent::Disconnected { slot }`. Reassignment by button press.
6. Rebinding: `RebindRequest` → capture next press (Esc/Start cancels) → conflict check across both
   keyboard schemes → `RebindResult` (rc_game writes `Settings`).
7. Rumble for local pads from `CarImpact`, `HitEvent`, `CarLanded`, boost start (setting).

## Deliverables — rc_camera
1. **UI host rig first** (ARCHITECTURE §5.8): single view = the full-window 3D camera is
   `IsDefaultUiCamera`; split-screen = `OverlayCamera` (`Camera2d`, top order,
   `ClearColorConfig::None`, `IsDefaultUiCamera`) over HDR viewport cameras. Exactly one UI host at a
   time. **Verify with the real React UI** (the T01 placeholder is enough): screenshots of 1, 2
   (both orientations), 3 and 4 players with HDR + bloom on player cameras and UI on top. If the
   overlay misbehaves with bevy-react, implement the documented fallback and report.
2. Menu showroom camera (`MenuCamera`, slow drift around `MenuBackdrop`), Loading camera.
3. `GameCamera { slot }` per local player — **session-scoped**, re-bound to the slot's racer in
   `OnEnter(RaceAttempt)` (a restart never despawns the UI host); split layout as a pure function
   (1–4 players, 2P horizontal/vertical, 3P leaves the 4th quadrant to the UI); `PlayerViewports`
   in logical px; recompute on window resize.
4. Modes chase / far / hood (cycle), look-back, obstruction pull-in (rapier rays excluding cars),
   FOV Hor+ from the setting with speed/boost widening and the 2P ≤ 100° horizontal clamp, shake
   from `Explosion`/`CarImpact`/`CarLanded` (setting), intro fly-by (from `Track`), finish orbit,
   spectate (cycle racers).
5. Follow the racer's `Visual` proxy when present, else its `Transform` (proxies arrive with
   rc_render later).

## Tests
Pure: binding resolution, intent latching (edges survive 0 or 2 fixed ticks in a frame; **pause →
Confirm → resume → no `use_item`**), ramp,
dead zone, UiNav repeat timing, lobby join/leave, rebind conflicts; viewport layouts for every
player count/orientation and aspect, Hor+ conversion and clamp, spring-follow maths. Headless where
practical (e.g. CarControl written only for the right racers).

## Acceptance
Tests, clippy, fmt clean; split-screen rig screenshots (1–4 players) in the report; a manual run
(`--quick-race test_oval --players 2 --exit-after 20`) shows two viewports following two racers.
