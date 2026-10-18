# rc_input — devices, intents, navigation (design note)

## Flow per frame
`PreUpdate / InputSet::Collect` (chained): `track_devices` (pad connect/disconnect, lobby
open/close) → `capture` (rebind) → `navigate` (`UiNav`, lobby join/leave, takeover) → `collect`
(driving intents, `PauseRequest`, `CycleCamera`, `LastUsed`) → `publish` (`LocalPlayers`, on change
only) → `rumble`. `FixedUpdate / SimSet::Drivers`: `drive_cars` writes `CarControl` of
`LocalPlayer` racers without `AiDriver` (autopilot humans are rc_ai's).

## Devices & players (`players.rs`, `Roster`)
- Single (menus, 1P sessions): slot 0 owns keyboard A, keyboard B and every pad (GDD §12.1).
- Versus: one device per slot.
  - Lobby open (`LobbyState.open`): Confirm on an unassigned device joins the lowest free slot
    below `max_slots` (`DeviceEvent::Joined`). Back on a joined device leaves (`Left`). Opening the
    lobby from single mode starts empty; reopening it after a Versus race keeps the players.
    Closing it empty returns to single mode.
  - A multi-player session without a lobby (`--quick-race --players n`) is auto-assigned in
    `OnEnter(Loading)`: pads first, then keyboard A, then B. A 1-human session is always single.
- Disconnect: in the lobby the slot is freed (`Left`). In a race the slot is orphaned and
  `Disconnected { slot }` is sent (single mode only if the player was on a pad). In menus nothing
  is sent.
- Takeover: Confirm/Start/Enter/Space on an unassigned device claims the lowest orphaned slot and is
  reported as `Joined` (by button press, never by pad id). That press does not navigate.
- `LastUsed { slot, kind }` is sent when a slot's device family changes (any press or stick
  deflection, menus included) and is mirrored into `SlotBinding::last_used` for glyphs.

## Driving intents (`bindings.rs`, `intents.rs`)
- Bindings come from `Settings.controls` per scheme, and an action's value is the max over its
  bindings and the slot's devices. Keys and buttons count as digital, axes as analog. Analog
  triggers keep their travel.
- Steering: stick halves get the dead zone (rescaled, default 0.12), then the sensitivity curve
  `v^(1/s)` (1 = linear; full lock stays reachable). The digital target −1/0/1 is ramped once per
  fixed tick (0.12 s to full lock = 8 ticks). steer = clamp(analog + ramped).
  - Deviation: the brief lists the ramp under PreUpdate. It runs per tick so it counts simulation
    time (deterministic, pauses with the sim).
- Driving input reaches the car only while the race is running in `Countdown`/`Racing` and no text
  field has focus; otherwise the intents are neutral. Auto-accelerate (throttle 1 unless braking)
  only applies in `Racing`, so the countdown never stalls.
- Edges: per device, `pressed && !held_last_frame` or a key/button `just_pressed` (taps shorter
  than a frame). Held bits are tracked in every state, so a button still held when a race resumes
  never makes a new edge. `use_item` is latched only while driving and consumed by the first fixed
  tick (none in a frame → it waits; two in a frame → only the first). Latched edges are dropped on
  `OnEnter(PauseState::Running)`; `OnEnter(RacePhase::Setup)` resets intents completely.
- Pause: the Pause action sends `PauseRequest { slot }` (unassigned devices: `None`) only while
  running in Intro/Countdown/Racing, at most once per frame. On the results screen (`Finished`) and
  while paused the same keys navigate (Esc = Back, Start = Confirm). rc_input never toggles pause.
- Camera action: `CycleCamera { slot }` (rc_core) while the race runs (rc_camera: mode, or the
  spectated racer after finishing).

## Menu navigation (`nav.rs`, GDD §12.2)
- Fixed keys: WASD (A) / arrows (B) move, Space (A) / Enter, NumpadEnter (B) confirm, Backspace (B)
  and Esc back, Q/E tabs. Left Ctrl is scheme A's Back in the lobby only. Pads: D-pad, left stick
  (press 0.5, release 0.3, dominant axis), A confirm, B back, LB/RB tabs, Start confirm.
- Directions repeat after 0.35 s, then every 0.1 s (`repeat: true`, no catch-up bursts after hitches).
- Tagging: `slot` = the device's owner (`None` = unassigned), `device` = family. In the lobby the
  shared keys (Esc, Q, E) are sent with `slot: None`, i.e. "not a player's" (Back from nobody =
  leave the screen). T05: single-player screens should accept nav from any slot, including `None`.
- Suppressed while a bevy-react text field has `InputFocus` (`EditableText`), while a rebind is
  captured (and in the frame it ends), and while `Racing` unpaused. In a running race the Pause
  keys only pause.

## Rebinding (`rebind.rs`)
`RebindRequest` → the next new press of the scheme's family (keyboard key; pad button or an axis
crossing 0.5 that was not already deflected) → `RebindResult { binding, conflict }`. Esc (keyboards)
and Start (pad) cancel (`binding: None`). The conflict is the first other action using the binding:
in the same scheme and, for keyboards, the other keyboard scheme. The press that opened the capture
is never captured. rc_game applies the result to `Settings`.

## Rumble (`rumble.rs`)
Impacts (strength × 0.8/0.4, 180 ms), item hits (0.9/0.5, 320 ms; blocked 0/0.4, 150 ms), landings
after ≥ 0.3 s air (strength × 0.6/0.3, 140 ms) and boost start (0.15/0.5, 220 ms). They go to the
pads of the slot whose car it is, only while that player is on a pad, × `vibration_intensity`, and
are off with `vibration`. All pads stop on pause and at session end.

## Tests
`cargo test -p rc_input`: binding resolution (schemes, analog triggers, stick halves, merging), dead
zone and sensitivity, ramp timing, edges and taps, one-tick consumption, levels and auto-accelerate,
repeat timing and hitches, stick hysteresis, key table, roster (single, lobby join/leave/reopen,
disconnect/takeover, auto-assignment, last used), rebind conflicts and capture, rumble pulses.
Headless `tests/input.rs`: `CarControl` only for local humans without `AiDriver`; the steering ramp
in ticks; item edges with 0 and 2 ticks per frame; pause → Confirm → resume → no item; a latched
edge does not survive a pause; nav tagging, repeat and racing suppression; pause only while
running; camera cycling; lobby join/leave; the keyboard split for a 2P session; rebind capture and
conflicts; pad disconnect and takeover.

## Known limitations
- Mouse use does not set `LastUsed` (pointer input is React's), and there is no `DeviceKind::Mouse`
  glyph switching.
- `--input-script` is implemented in `src/script.rs`: a scripted virtual device feeding `ButtonInput`/`Gamepad` before `InputSet::Collect` (see `docs/design/automation.md`).
- A finished human's car still receives driving input (rc_race may hand it to the AI, Re-Volt style).
