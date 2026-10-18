# bevy-react 0.8.1 — usage notes for ReCharged

Verified 2026-10-18 against bevy-react 0.8.1 (crate `bevy-react` → `bevy_react`, npm `bevy-react`) on Bevy 0.20: library docs (`/home/tul/Projects/bevy-react/docs/dist/**/*.md`, mirrored at https://tulustul.github.io/bevy-react/), source (`crates/`, `js/`) and examples (`examples/{cyberpunk,coldwake,civilization,synth,minimal,demos}`). File refs like `cyberpunk/ui/src/App.tsx:88` are relative to `/home/tul/Projects/bevy-react/examples/`; `crates/…`/`js/…`/`docs/…` relative to `/home/tul/Projects/bevy-react/`. **UNVERIFIED** = reasoned from source/docs but not confirmed by an example or a run. The TS/TSX snippets in §2–§8 were typechecked (tsc 5.6.3, `strict`) against the real `bevy-react` 0.8.1 types with a simulated generated `bevy.ts`; the Rust snippets were not compiled (names checked against source).

## 0. TL;DR

- **Model**: React runs in an embedded V8 thread (no DOM, no Node, no `fetch`/`localStorage`). Each commit becomes an op batch Bevy applies in `Update` (`ReactApplySet`) to plain `bevy_ui` entities. Props are diffed; unknown props/styles warn, never crash. Everything React does lands **≥ 1 frame** after the input that caused it.
- **Channels**: `#[react_message]` (React→Bevy, fire-and-forget), `#[react_request(response = T)]` (React→Bevy→Promise), `#[react_event]` (Bevy→React broadcast, **not buffered**). One shared `register_bindings(app)` for app + exporter; regenerate and commit `ui/src/bevy.ts`.
- **HUD (1–4 players)**: per-frame visuals (needles, bars, minimap dots, per-viewport nameplates) = shared values created in React, ids sent to Rust once, written by Rust with `SharedValues::set` `.before(AnimationSet::Apply)` → zero React renders. Numbers/text (speed, lap, position, timer) = one batched `react_event` for all players at ~10 Hz into an external store; edge events (lap done, hit, finish) as separate events; `race.snapshot` request (or a `sync` message) on mount. Shared values cannot drive text.
- **Gamepad menus = ours to build**: the library delivers raw `gamepadInput` batches (and window-wide `keyDown`) but has no focus navigation. Build one input translator (edge detection, stick thresholds, hold-repeat — pattern in the demos' `gamepadDemo`) feeding an intent stack; screens keep a focused index in React state; mouse hover sets the same index. Its pad ids can't be mapped to Bevy `Gamepad` entities, so for per-player screens (car select, who paused) send player-indexed intents from Rust instead. Pads have no text entry → on-screen keyboard for the profile name.
- **Input isolation**: React can't consume keys/pad input; Bevy still sees everything. Gate gameplay in Rust by state (`AppState::Racing` + `Paused` set via messages, `not(ui_typing)` from `InputFocus`). Mouse: gate on the `PointerCapture` resource.
- **Cameras**: bevy-react spawns none and renders into bevy_ui's default UI camera (unmarked → the highest-`order` window camera, i.e. one player's viewport!). For split-screen use a full-window `Camera2d` marked `IsDefaultUiCamera` with the highest `order` over per-player `Camera3d`s with viewports, and lay the HUD out in a React grid matching the viewports. Each Bevy camera with a clear color wipes the whole shared main texture, so pick a rig (§8.3): LDR players where only player 0 clears + overlay `ClearColorConfig::None` (frosted glass works), or HDR players + transparent overlay with premultiplied blend (no glass). UNVERIFIED by running — prototype first. `<anchor>` follows only the UI camera → per-viewport nameplates via shared values.
- **Paths**: the bundle path is relative to the **process cwd**; assets are relative to `BEVY_ASSET_ROOT`/`CARGO_MANIFEST_DIR`/exe dir. With the planned `crates/recharged` binary: `-p recharged` in `bevy:generate`, an explicit asset root, a compile-time dev bundle path and an exe-relative release path (§1.4); ship `vendor.js` + `app.js` side by side with `hot_reload(false)`.
- **Hot reload**: keeps hook state; re-runs every effect and all module top-level code → cleanups everywhere, singletons on `globalThis`.

## 1. Setup

### 1.1 Rust
```toml
# Cargo.toml of the crate that adds the plugins
[dependencies]
bevy = "0.20"                                  # app: Bevy default features (bevy_gilrs = gamepads, x11/wayland for cursors)
bevy-react = "0.8.1"                           # imported as `bevy_react`
serde = { version = "1", features = ["derive"] } # only for nested payload types you derive yourself
ts-rs = "10"                                   # same major as bevy-react's (10)
```
Features (all default except `svg_text`): `svg`, `anchor`, `canvas`, `portal`, `surface`, `devtools`, `custom_cursor`, `svg_text`. Disabled feature → element mounts as plain node + `featureMissing` warning. Library crates that only define bindings/elements/filters: `bevy-react = { version = "0.8.1", default-features = false }`. A shipping build can drop devtools code entirely with `default-features = false, features = ["svg", "anchor", "canvas", "portal", "custom_cursor"]` (devtools never *runs* in release anyway; `DevtoolsConfig` only exists with the feature → `#[cfg]` any use).

```rust
use bevy::prelude::*;
use bevy_react::prelude::*;   // ReactPlugins, ReactUiPlugin, ReactAppExt, ReactEvents, ReactNodes, ReactNode,
                              // ReactInteraction, Request, react_message/react_event/react_request/react_filter/react_morph_filter,
                              // SvgPlugin, AnchorPlugin, CanvasPlugin, PortalPlugin, SurfacePlugin
// NOT in prelude (use bevy_react::…): Responder, PointerCapture, PointerCaptureSet, ReactApplySet, OpApplyStats,
// RenderTargets, RenderTargetSpec, RenderTarget, RenderMode, Resolution, PortalCamera, DevtoolsConfig,
// PrecompileFilters, FilterSelection, animations::{SharedValues, SharedId, AnimationSet}, surface::{Surfaces, SurfaceSpec, SurfacePointer}

App::new()
    .add_plugins(DefaultPlugins)
    .add_plugins(ReactPlugins.set(
        ReactUiPlugin::new(ui_bundle())                    // path to app.js; vendor.js must sit next to it
            .hot_reload(cfg!(debug_assertions))            // default true in EVERY build → turn off for release
            .default_font("fonts/Body-Variable.ttf")       // asset paths (AssetServer)
            .font("Display", "fonts/Display-Bold.ttf")     // style={{ fontFamily: "Display" }}
            .font("Mono", "fonts/Mono.ttf")
            .cursor("default", "ui/cursor.png", (4, 2)),   // name, asset path, hotspot px; "default" replaces the arrow app-wide
            // .devtools(DevtoolsConfig { toggle_key: KeyCode::F12, ..default() })   // #[cfg(feature = "devtools")]
            // .precompile_filters(PrecompileFilters { builtins: FilterSelection::All, ..default() })
    ))
    .add_plugins(rc_ui::UiBindingsPlugin)                 // calls rc_ui::register_bindings(app)
    .run();
```
- `ReactPlugins` = `ReactUiPlugin` + one plugin per compiled feature; add it **after** `DefaultPlugins` (it doesn't include them). `.disable::<CanvasPlugin>()` to drop one.
- **bevy-react spawns no camera.** bevy_ui needs one (§8).
- Builder methods (exact, `crates/core/src/plugin.rs`): `new(impl Into<PathBuf>)`, `default()` (= `ReactUiPlugin::DEFAULT_BUNDLE` = `"ui/dist/app.js"`), `hot_reload(bool)`, `default_font(path)`, `font(name, path)`, `cursor(name, path, (u16, u16))` [`custom_cursor`], `devtools(DevtoolsConfig)` [`devtools`], `precompile_filters(PrecompileFilters)`.
- Missing `app.js` or `vendor.js` → **panic at startup** printing the absolute path it looked for.

### 1.2 npm / UI package
```sh
npx bevy-react init ui      # flags: --name <pkg>, --install, --force, --local <path>
cd ui && npm install && npm run watch
```
Scaffold (`js/templates/ui`): `package.json`, `tsconfig.json` (`"jsx": "react-jsx"`, `"jsxImportSource": "bevy-react"`, `"moduleResolution": "bundler"`, `"strict": true`, `"include": ["src"]`, alias `@/*`), `.gitignore` (`node_modules/`, `dist/`), `src/index.tsx` (`mount(<App />)` — **never resolves, don't await**), `src/App.tsx`, `src/bevy.ts` (untyped placeholder until generated).

| Script | Command | Output |
|---|---|---|
| `build` | `bevy-react build` | `dist/vendor.js` (React + reconciler + runtime, loaded once) + `dist/app.js` (our code, re-run on hot reload) |
| `watch` | `bevy-react build --watch` | vendor once, then rebuilds `app.js` on change (**restart after upgrading React/bevy-react**) |
| `build:prod` | `bevy-react build --prod` | `NODE_ENV=production`, no Fast Refresh, no devtools JS half |
| `typecheck` | `tsc --noEmit -p tsconfig.json` | esbuild does NOT type-check |
| `bevy:generate` | `cargo run --manifest-path ../Cargo.toml -- --export-bindings src/bevy.ts` | regenerates `src/bevy.ts` |

- Template deps: `bevy-react` (same version as the crate — keep them equal), `react` 19.2.7, `react-reconciler` 0.33.0; dev: `esbuild` 0.24.0, `@swc/core` 1.10.1, `react-refresh` 0.18.0, `typescript` 5.6.3, `@types/react` 19.2.17.
- Bundles: esbuild, `format: "iife"`, `platform: "neutral"`, `target: "es2022"`, `sourcemap: "inline"` (both modes), entry fixed to `src/index.tsx`, outputs fixed to `dist/`. Custom pipelines: `import { buildVendor, buildApp, watchApp } from "bevy-react/build-lib"`.
- JS runtime = bare V8 (deno_core): **no DOM, no Node APIs, no `fetch`, no `localStorage`**. Provided globals: `setTimeout/setInterval/clear*` (wall clock, NOT paused by `Time<Virtual>`), `queueMicrotask`, `performance.now()`, `console.*` (→ Bevy log). npm packages that touch `window`/`document`/Node built-ins won't run. Persist settings/profiles via Rust (request/message + file IO).
- Import the typed surface from `./bevy`, not the untyped `emit/request/addEventListener` from `"bevy-react"`.

### 1.3 Working directory & paths (the sharp part)
| Thing | Resolved against |
|---|---|
| `ReactUiPlugin::new(path)` bundle | **process cwd** (plain `std::fs` path; absolute paths fine) |
| fonts, cursors, `<image src>`, `backgroundImage.src`, filter shaders | Bevy `AssetServer`: `BEVY_ASSET_ROOT` env → `CARGO_MANIFEST_DIR` (set by `cargo run` = the **bin crate's** dir) → executable's dir; then `AssetPlugin.file_path` (default `"assets"`) |
| devtools layout file `.bevy-react-devtools.json` | cwd (gitignore it) |

### 1.4 ReCharged layout (as planned in `docs/ARCHITECTURE.md` §2)
```
bevy-volt/
  Cargo.toml          # virtual workspace
  crates/rc_ui/       # all #[react_*] types + pub fn register_bindings(app) + HUD push systems
  crates/recharged/   # BIN: main.rs assembles plugins, handles `--export-bindings`
  ui/                 # npm package from `npx bevy-react init ui`; src/bevy.ts generated + committed; dist/ gitignored
  assets/             # fonts/, ui/, … — what bevy-react's font/cursor/image paths resolve against
```
What that layout needs (template defaults assume `ui/` sits inside the Cargo package):
- `ui/package.json`: `"bevy:generate": "cargo run --manifest-path ../Cargo.toml -p recharged -- --export-bindings src/bevy.ts"` (npm runs scripts with cwd = `ui/`, so the output path is `src/bevy.ts`).
- Bundle: `cargo run -p recharged` from the repo root keeps cwd = repo root, so `ReactUiPlugin::default()` (`ui/dist/app.js`) works; to be cwd-independent in dev use the compile-time path below.
- Assets: under `cargo run`, Bevy's base is `CARGO_MANIFEST_DIR` = `crates/recharged/` → set `AssetPlugin { file_path: <absolute repo assets dir>, .. }` (an absolute `file_path` replaces the base; ARCHITECTURE.md's `rc_core::paths` resolver is the place) or set the env var for `cargo run` in `.cargo/config.toml`: `[env]` `BEVY_ASSET_ROOT = { value = ".", relative = true }` (relative to the dir containing `.cargo/`, i.e. the repo root).
- (A root package — `[workspace]` + `[package]` in the root `Cargo.toml` — would make all three defaults line up with zero config, if the layout is ever revisited.)

Bundle path helper (dev: cwd-independent; release: next to the exe):
```rust
fn ui_bundle() -> std::path::PathBuf {
    if cfg!(debug_assertions) {
        // cyberpunk does PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("ui/dist/app.js") (main.rs:82)
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../ui/dist/app.js")
    } else {
        std::env::current_exe().expect("exe path").parent().unwrap().join("ui/app.js") // ship dist/{vendor,app}.js here
    }
}
```
Release checklist: `npm run build:prod` → copy `ui/dist/vendor.js` + `ui/dist/app.js` beside the exe (`ui/`), copy `assets/`, `hot_reload(false)`, `cargo build --release` (devtools inert). A debug binary + prod bundle or release binary + dev bundle shows no devtools panel.

## 2. Typed channels (React ⇄ Bevy)

Three kinds, all declared as Rust types; `bevy.ts` is generated from them.

| Kind | Direction | Rust macro | Register | React side |
|---|---|---|---|---|
| message | React → Bevy, fire-and-forget | `#[react_message(name = "a.b")]` | `app.add_react_handler(observer)` (or `add_react_message::<T>()` + your own `add_observer`) | `bevy.a.b(payload)` / `emit("a.b", payload)` |
| request | React → Bevy → Promise | `#[react_request(name = "a.b", response = T)]` | `app.add_react_request_handler(observer)` (or `add_react_request::<T>()`) | `await bevy.a.b(payload?)` / `request("a.b", payload)` |
| event | Bevy → React broadcast | `#[react_event(name = "a.b")]` | `app.add_react_event::<T>()` (typing only; sending works without it) | `bevy.on("a.b", cb)` → returns unsubscribe |

Macro facts (from `crates/macros/src/lib.rs`):
- Only args: `name = "..."` (all three) and `response = <any Rust type>` (request, **required**; `Vec<CarInfo>`, `Option<u32>`, `u64` all parse).
- Default name = struct ident with first letter lowercased (`StartRace` → `"startRace"`). Dots nest the proxy: `"race.start"` → `bevy.race.start(...)`. Event names with dots are just keys for `bevy.on`.
- The macro **adds** `Deserialize` (message/request) or `Serialize` (event) + `ts_rs::TS` and the trait impl (`Event` with `GlobalTrigger` for messages; `ReactRequest`; `ReactEvent`). Do NOT derive serde/TS on the annotated struct yourself. Your crate needs no serde/ts-rs dep for the annotated struct (routed via `bevy_react::__private`).
- **Nested types you define** (fields, enums, response structs) must derive `serde::Serialize`/`Deserialize` + `ts_rs::TS` themselves → add `serde = { version = "1", features = ["derive"] }` and `ts-rs = "10"` (bevy-react uses ts-rs 10; keep the major version equal).
- camelCase: put BOTH `#[serde(rename_all = "camelCase")]` and `#[ts(rename_all = "camelCase")]` on every struct/enum (docs: they must agree; without them snake_case crosses as-is). ts-rs 10's default `serde-compat` also reads the serde attribute — synth's `Kind` enum has only `#[serde(rename_all = "lowercase")]` and still generates lowercase TS (`synth/synth/engine.rs:38-40` → `synth/ui/src/bevy.ts:151`) — so the `#[ts]` twin is belt-and-braces.
- Type mapping (ts-rs): `u8..u32/i8..i32/f32/f64` → `number`; `usize` → `number`; `u64/i64` → typed `bigint` but **arrive as JS number** (avoid; use `u32`/`f64`); `Option<T>` → `T | null`; `Vec<T>` → `Array<T>`; `HashMap<String, f32>` → `{ [key in string]?: number }`; `[f32; 2]` → tuple; newtype → alias of inner; unit struct → `null`; enum → union in serde's representation (externally tagged by default; prefer `#[serde(tag = "kind")]` + matching rename for clean TS discriminated unions).
- Unit-struct **message** still takes one arg: `bevy.race.quit(null)`. Unit-struct **request** = zero-arg method: `bevy.garage.cars()`.
- Reserved/forbidden names: top-level `emit`, `request`, `on`, `addEventListener`, `removeEventListener` (export panics); a name used as both method and namespace (`"race"` and `"race.start"`) panics; a message and a request with the same name panics; built-ins `gamepad.rumble`, `gamepad.stopRumble`, `gamepad.getAll`, `window.size`, events `resize`, `keyDown`, `keyUp`, `gamepadConnected`, `gamepadDisconnected`, `gamepadInput` are reserved (an app binding with one of those names is left out of `bevy.ts`).
- Registering a second type under a taken name warns and replaces the first.

### 2.1 One registration function (used by the app AND the exporter)

```rust
// crates/rc_ui/src/lib.rs  (the crate that owns React bindings)
use bevy::prelude::*;
use bevy_react::prelude::*;

pub struct UiBindingsPlugin;
impl Plugin for UiBindingsPlugin {
    fn build(&self, app: &mut App) { register_bindings(app); /* systems, resources */ }
}

/// Called by UiBindingsPlugin AND by `--export-bindings`. Every add_react_* call lives here.
pub fn register_bindings(app: &mut App) {
    app.add_react_handler(on_start_race)
        .add_react_request_handler(list_cars)
        .add_react_event::<LapCompleted>();
    // + add_react_filter::<T>(), add_react_morph_filter::<T>(), add_react_element(&EL), add_react_style(..)
}
```

```rust
// crates/recharged/src/main.rs — export entry point (no window, no JS runtime)
fn main() {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() == Some("--export-bindings") {
        let path = args.next().expect("--export-bindings needs a path");
        let mut app = App::new();
        bevy_react::ReactPlugins::register_bindings(&mut app); // JSX types of compiled-in feature elements
        rc_ui::register_bindings(&mut app);                     // our channels/filters/elements
        app.export_react_typescript(&path).expect("write bevy.ts"); // creates parent dirs
        return;
    }
    // ... normal App::new()...run()
}
```
Output is deterministic (sorted). Commit `ui/src/bevy.ts`; CI: regenerate + `git diff --exit-code -- ui/src/bevy.ts`. Exclude `**/bevy.ts` from prettier.

### 2.2 Message round trip (React → Bevy)

```rust
#[react_message(name = "race.start")]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct StartRace {
    pub track_id: String,
    pub laps: u8,
    pub players: Vec<PlayerSetup>,
}

#[derive(serde::Deserialize, ts_rs::TS, Clone, Debug)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct PlayerSetup {
    pub gamepad: Option<u32>,   // TS: number | null
    pub car_id: String,
    pub color: CarColor,
}

#[derive(serde::Deserialize, ts_rs::TS, Clone, Copy, Debug)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum CarColor { Red, Blue, Green, Yellow } // TS: "red" | "blue" | "green" | "yellow"

fn on_start_race(on: On<StartRace>, mut next: ResMut<NextState<AppState>>, mut setup: ResMut<RaceSetup>) {
    let msg = on.event();                // &StartRace
    setup.0 = Some((msg.track_id.clone(), msg.laps, msg.players.clone()));
    next.set(AppState::Loading);
}
```
```tsx
import { bevy } from "./bevy";
bevy.race.start({ trackId: "toytown", laps: 3, players: [{ gamepad: 0, carId: "bandit", color: "red" }] });
```
- Delivered next frame in `PreUpdate`, in emit order; observers run there (your `Update` systems see effects the same frame).
- Messages arrive **before** that frame's UI ops are applied (`ReactApplySet`, in `Update`): a message sent in the same commit that mounts a named node arrives before the node exists → stash in a resource, act in a system `.after(ReactApplySet)`.
- No reply/ack. Unknown name → warning in the Bevy log; bad payload → error log; both dropped. Need failure feedback → use a request.

### 2.3 Request round trip (React → Bevy → React)

```rust
#[react_request(name = "garage.cars", response = Vec<CarInfo>)]
pub struct ListCars;                        // unit payload → bevy.garage.cars()

#[derive(serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct CarInfo { pub id: String, pub name: String, pub top_speed: f32, pub unlocked: bool }

fn list_cars(req: On<Request<ListCars>>, db: Option<Res<CarDb>>) {
    match db {
        Some(db) => req.respond(db.cars.iter().map(CarInfo::from).collect::<Vec<_>>()),
        None => req.respond_err("car db not loaded"),   // rejects the promise with Error(message)
    }
}
// payload: req.payload() -> &ListCars
```
```tsx
const [cars, setCars] = useState<CarInfo[]>([]);
useEffect(() => {
  let alive = true;
  bevy.garage.cars().then((c) => alive && setCars(c), (e) => console.error(e));
  return () => { alive = false; };          // requests can't be cancelled
}, []);
```
- **Every request must be answered** or the promise hangs forever. A param that fails validation (`Single<..>` with no match, missing `Res`) means the observer never runs → hang. Use `Query`/`Option<Res<_>>` and `respond_err`.
- Deferred answer: `let r: Responder<Vec<CarInfo>> = req.responder();` (`bevy_react::Responder<T::Response>`, `Clone + Send + Sync`) → store in a resource/task, call `r.respond(..)` / `r.respond_err(..)` later. First answer wins (later ones warn). Dropping all clones unanswered = hang.
- Auto-rejections: unregistered name (`no handler registered for request "..."`), malformed payload (`malformed request "..."`), response serialize failure, `respond_err`.
- Latency ≥ 1 frame (collected in `PreUpdate`). No timeout — race it with a `setTimeout` promise if needed.

### 2.4 Event round trip (Bevy → React)

```rust
#[react_event(name = "race.lapCompleted")]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct LapCompleted { pub player: u8, pub lap: u8, pub lap_time_ms: u32, pub best: bool }

fn report_laps(mut crossed: MessageReader<LapCrossed>, events: ReactEvents) {
    for c in crossed.read() {
        events.send(&LapCompleted { player: c.player, lap: c.lap, lap_time_ms: c.ms, best: c.best });
    }
}
// register_bindings: app.add_react_event::<LapCompleted>();  (typing only)
```
```tsx
useEffect(() => bevy.on("race.lapCompleted", (e) => showLapToast(e.player, e.lapTimeMs, e.best)), []);
```
- `ReactEvents` is a system param usable in any system/observer/schedule. Serialize failure → error log, not sent.
- **Not buffered**: no listener at arrival time = dropped (incl. everything sent before React mounted). Pair state with a request that returns the current value and call it on mount (the built-ins do: `resize` + `bevy.window.size()`, `gamepadConnected` + `bevy.gamepad.getAll()`).
- Startup senders can wait for the first applied UI batch: `.run_if(|s: Res<bevy_react::OpApplyStats>| s.applied_count > 0)`.
- Listeners are global (not owned by components) — always return the unsubscribe from `useEffect`. Exceptions in one listener are logged; others still run. All listener state updates for one event commit as one React render.
- All Bevy→React traffic (events, UI events, request replies, animation completions) shares one ordered channel.

## 3. Pushing game state to React (HUD data flow)

Cost model: every Bevy→React event = JSON + (usually) a React render + an op batch applied a frame or more later. Shared values cost **zero bridge traffic per frame** (Rust writes `SharedValues`, Bevy applies bindings). `<anchor>` follows one camera only.

| Data | Rate | Mechanism |
|---|---|---|
| Needles, bars, rpm/boost meters, minimap dots, per-viewport nameplate positions, damage flashes | every frame | **shared values created in React, ids sent to Rust, Rust `SharedValues::set` each frame** |
| Speed number, lap/position, current lap time (tenths), weapon slot | 10 Hz | one batched `react_event` for all players (`race.hud`) |
| Lap completed, overtakes, item pickup, hit, race finished | on change | individual `react_event`s (toasts/animations) |
| Initial HUD state after mount / hot reload | once | request `race.snapshot` (events are not buffered) |
| Menus: car/track lists, profile, settings | on demand | requests (`garage.cars`), messages for commands (`race.start`, `settings.video`) |

### 3.1 Per-frame values: React owns the handles, Rust drives them (pattern from `examples/synth`)
```tsx
// ui/src/hud/PlayerHud.tsx
import { useEffect } from "react";
import { interpolate, useSharedValue } from "bevy-react";
import { bevy } from "../bevy";

export function PlayerHud({ player }: { player: number }) {
  const speed = useSharedValue(0);      // km/h
  const boost = useSharedValue(0);      // 0..1
  const mapX = useSharedValue(0), mapY = useSharedValue(0);   // minimap px of this player's car
  useEffect(() => {
    bevy.hud.bind({ player, speed: speed.id, boost: boost.id, mapX: mapX.id, mapY: mapY.id });
    return () => bevy.hud.unbind({ player });
  }, [player, speed, boost, mapX, mapY]);
  return (
    <node style={{ width: "100%", height: "100%" }}>
      {/* needle: -120°..120° over 0..200 km/h, no React re-render */}
      <node style={{ ...needle, transform: { rotate: { animated: interpolate(speed, [0, 200], [-120, 120]) } } }} />
      <node style={{ ...bar, transform: { scaleX: { animated: boost } } }} />
      <node style={{ ...dot, transform: { translateX: { animated: mapX }, translateY: { animated: mapY } } }} />
    </node>
  );
}
```
```rust
use bevy_react::animations::{AnimationSet, SharedId, SharedValues};

#[react_message(name = "hud.bind")]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct BindHud { pub player: u8, pub speed: u32, pub boost: u32, pub map_x: u32, pub map_y: u32 }

#[react_message(name = "hud.unbind")]
pub struct UnbindHud { pub player: u8 }

#[derive(Resource, Default)]
struct HudIds(std::collections::HashMap<u8, [SharedId; 4]>);   // SharedId = u32

fn on_bind(on: On<BindHud>, mut ids: ResMut<HudIds>) {
    let b = on.event();
    ids.0.insert(b.player, [b.speed, b.boost, b.map_x, b.map_y]);
}
fn on_unbind(on: On<UnbindHud>, mut ids: ResMut<HudIds>) { ids.0.remove(&on.event().player); }

fn drive_hud(cars: Query<(&Player, &Telemetry, &GlobalTransform)>, map: Res<Minimap>,
             ids: Res<HudIds>, mut values: ResMut<SharedValues>) {
    for (p, tel, xf) in &cars {
        let Some([speed, boost, mx, my]) = ids.0.get(&p.0).copied() else { continue };
        let uv = map.project(xf.translation());                  // → minimap px
        for (id, v) in [(speed, tel.kmh), (boost, tel.boost), (mx, uv.x), (my, uv.y)] {
            if values.get(id) != Some(v) { values.set(id, v); }   // compare-before-write (synth does this)
        }
    }
}
// register_bindings: app.add_react_handler(on_bind).add_react_handler(on_unbind);
app.init_resource::<HudIds>()
   .add_systems(Update, drive_hud.after(AnimationSet::Tick).before(AnimationSet::Apply));
```
- Same ordering as synth (`examples/synth/synth/mod.rs:119-122`: `.after(AnimationSet::Tick).before(AnimationSet::Apply)`; `show_transport` at `:173-196`).
- A shared value lives as long as the React component that created it; on unmount it is released → the cleanup `unbind` matters. A full reload clears all values; the remount's effect re-binds.
- One value can drive many nodes (e.g. each car's `mapX/mapY` created once at the race-HUD root, passed to all four players' minimaps).
- Prefer `transform`/`opacity`/colors: layout-bound values (`width`, `left`, …) relayout every frame they change. For a bar: `scaleX` on a fixed-width node (+ left-aligned via a wrapper or `translateX`), not `width`.
- Bound `transform` fields: once ANY field is bound, the static fields of that `transform` stay identity → bind all you need or split onto wrapper nodes.
- `SharedValues::set` on an id React already released re-creates a dead entry (harmless, leaks a few bytes) — still unbind properly.

### 3.2 Low-rate state: one batched tick + a store
```rust
use bevy::time::common_conditions::on_timer;

#[react_event(name = "race.hud")]
pub struct HudTick { pub players: Vec<PlayerHudState> }

#[derive(serde::Serialize, ts_rs::TS, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct PlayerHudState { pub player: u8, pub kmh: u16, pub lap: u8, pub laps: u8, pub position: u8, pub lap_ms: u32 }

fn send_hud_tick(q: Query<(&Player, &Telemetry, &RaceProgress)>, events: ReactEvents) {
    let players = q.iter().map(|(p, t, r)| PlayerHudState { /* … */ }).collect();
    events.send(&HudTick { players });
}
app.add_systems(Update, send_hud_tick.run_if(in_state(AppState::Racing)).run_if(on_timer(std::time::Duration::from_millis(100))));
// register_bindings: app.add_react_event::<HudTick>();

#[react_request(name = "race.snapshot", response = HudTick)]     // seed on mount
pub struct RaceSnapshot;
```
```ts
// ui/src/hud/store.ts — external store that survives hot reload (cyberpunk settings/store.ts pattern)
import { useSyncExternalStore } from "react";
import { on, bevy, type PlayerHudState } from "../bevy";
type S = { players: PlayerHudState[]; listeners: Set<() => void> };
const g = globalThis as { __hud?: S };
const s: S = (g.__hud ??= { players: [], listeners: new Set() });
const set = (players: PlayerHudState[]) => { s.players = players; s.listeners.forEach((l) => l()); };
export function startHudFeed() {             // call from the race HUD's useEffect; returns cleanup
  bevy.race.snapshot().then((t) => set(t.players), () => {});
  return on("race.hud", (t) => set(t.players));
}
const subscribe = (l: () => void) => { s.listeners.add(l); return () => { s.listeners.delete(l); }; };
export const usePlayerHud = (p: number) =>
  useSyncExternalStore(subscribe, () => s.players.find((x) => x.player === p));  // objects change identity each tick
```
- Render cost per tick: diffed props → only changed text/styles cross; ~4 text updates per player at 10 Hz is negligible (`performance.md`: `updateText1` on a 1000-node tree ≈ 0.5 ms JS + ~2 ms Bevy incl. relayout). Keep components small so a tick re-renders leaves, not the whole HUD.
- Lap timer: at 10 Hz show tenths; when an exact time matters (lap completed), send it in the `race.lapCompleted` event.
- Smooth a 10 Hz value with a style `transition` (e.g. a position badge sliding with `transition: { transform }`) instead of sending faster.

### 3.3 Per-viewport world overlays (opponent nameplates, offscreen arrows)
`<anchor>` projects through the single `IsDefaultUiCamera` camera only → in split-screen compute positions in Rust per player camera and drive shared values:
```rust
// per player camera. world_to_viewport returns LOGICAL px of the render target (window),
// viewport offset INCLUDED (bevy_camera/src/camera.rs:540-567: `… * rect.size() + rect.min`)
match camera.world_to_viewport(cam_xf, target_world_pos) {
    Ok(p) => {
        let local = p - camera.logical_viewport_rect().map_or(Vec2::ZERO, |r| r.min); // → relative to the player's HUD cell
        values.set(ids.tag_x, local.x); values.set(ids.tag_y, local.y); values.set(ids.tag_alpha, 1.0);
    }
    Err(_) => values.set(ids.tag_alpha, 0.0),   // behind camera / past far plane → hide via an opacity binding
}
```
(Or skip the subtraction and put all tags in one full-window absolutely-positioned layer.) If UI px ≠ window logical px (you use `UiScale`), divide by it; with cyberpunk's scale-factor override UI px == window logical px.

### 3.4 Rust-owned text inside a React slot (UNVERIFIED pattern, use only if 10 Hz text is not enough)
Docs (`communication/named-nodes.md`, "Children you spawn"): entities you parent under a React node are despawned with it, but are orphaned whenever React changes that node's children. So a React leaf `<node name="p1.lapTimer" style={{ width: 180, height: 40 }} />` (no children, never changes) can host a Bevy `Text` child spawned and updated every frame by a Rust system found via `ReactNodes::get("p1.lapTimer")` `.after(ReactApplySet)`. Fonts must then be loaded by Rust (`AssetServer::load` of the same path). No example does this.

## 4. Elements

JSX intrinsics are lowercase and come from `jsxImportSource: "bevy-react"`. No `div`/`span`/DOM. Feature elements (`<svg>`, `<canvas>`, `<portal>`, `<surface>`, `<anchor>`) only type-check after `bevy.ts` is generated.

| Element | What | Key props |
|---|---|---|
| `<node>` | flex/grid container (`bevy_ui::Node`) | `style`, state styles, pointer/wheel/scroll/focus props, `name`, `sharedTag` |
| `<button>` | `<node>` + `focusPolicy: "block"` default + always owns clicks + `bevy::ui_widgets::Button` | `disabled` |
| `<text>` | text block; nested `<text>` = span; `<image>` inside = inline image | string/number children |
| `<editableText>` | native text input | `value`, `onChange(string)`, `maxLength`, `multiline`, `autofocus`, `selectionStart/End`, `onSelect`, `readOnly`, `disabled`, `ariaLabel` |
| `<image>` | texture asset or `.svg` file | `src` (asset path), `tint`, `flipX/Y`, `imageMode`, `sourceRect`, `atlas`, `visualBox` |
| `<root>` | separate window-filling UI tree (modals/toasts) | `style`, `name` only |
| `<portal>` | shows a named render target (camera → texture) | `target` (required) |
| `<surface>` | renders a subtree into a texture for a 3D mesh | `target` (required) |
| `<anchor>` | pins a subtree to a 3D entity's projected position | `entity` (required, `Entity::to_bits()`), `offset: [x,y,z]`, `scale` |
| `<svg>` + `<rect> <circle> <ellipse> <line> <polyline> <polygon> <path> <g>` | CPU-rasterized vector | `viewBox`; shapes take `fill`, `stroke`, `strokeWidth`, `transform`, `transition`, pointer handlers |
| `<canvas>` | retained CPU path drawing | `draw={(ctx)=>…}` or `ref` → `BevyCanvasElement.getContext()`, `onResize` |

Shared rules:
- Top-level elements of the app are children of a window-filling **column that centers them and gaps 16px**. Render ONE root `<node style={{ width: "100%", height: "100%" }}>` to own the window.
- **Always wrap text in `<text>`**: a bare string renders white 20px in Bevy's built-in font.
- **No style inheritance**, not even for text spans: restate `fontFamily`/`fontSize`/`color` on every span (`fontSize: "1em"` inherits size). Share style objects in JS.
- Unknown prop/style → warning (`unknownProp`, `propIgnored`, `styleIgnored`, …) in log + devtools; never a crash. A custom element attribute that fails to deserialize **fails the whole commit** (TypeError).

### 4.1 `<button>` vs `<node onClick>`
- `<button>`: blocks pointer (no camera drag/scene pick under it), owns clicks even without `onClick`, has `ReactInteraction`, Enter/Space click it **when it has focus** (needs `tabIndex`).
- `<node onClick>`: pointer passes through to the 3D scene behind; not keyboard-activatable. Use `<button>` for every menu control.
- `disabled`: no `onClick` (pointer or key), not focusable, no `pressStyle`; `hoverStyle` still applies; no `disabledStyle` — style it yourself.

### 4.2 `<image>` and textures
```tsx
<image src="ui/icons/nitro.png" style={{ width: 48 }} />                 // asset path, AssetServer-relative
<image src="ui/frame.png" imageMode={{ type: "sliced", border: 24 }} style={{ width: 320, height: 180 }} />
<image src="ui/digits.png" atlas={{ tileWidth: 32, tileHeight: 48, columns: 10, rows: 1, index: d }} />
<image src="ui/icons/gear.svg" style={{ width: 64 }} />                  // SVG file, crisp at any size
<node style={{ width: 190, height: 130, backgroundImage: { src: { texture: "trackThumb.toytown" } } }} />
```
- `"auto"` mode: intrinsic size = texture size; measures 0×0 until loaded → give explicit sizes to avoid reflow.
- No `object-fit` (constrain one axis to keep aspect). Large textures drawn small alias → `imageRendering: "trilinear"`; pixel art → `"nearest"`.
- `<image src>` takes asset paths only — it can't show a registered texture. App-generated/loaded textures: `RenderTargets::register(name, handle)` (static) → `backgroundImage: { src: { texture: name } }` (demos `main.rs:189-233` registers `"checker"`). Live camera output → `<portal>`.

### 4.3 `<root>` (overlays, modals, toasts)
Default style `{ width: "100vw", height: "100vh", flexDirection: "column", globalZIndex: 1 }`. Mount = open, unmount = close. Root itself is click-through; make a blocking backdrop child:
```tsx
{open && (
  <root name="pauseMenu" style={{ globalZIndex: 10 }}>
    <node style={{ flexGrow: 1, alignItems: "center", justifyContent: "center",
                   backgroundColor: "#000000aa", focusPolicy: "block" }}>
      <PauseMenu />
    </node>
  </root>
)}
```
- Renders on the default UI camera only (no camera/window target attribute). Two roots with equal `globalZIndex` stack by mount history → give simultaneous overlays distinct values.
- Layer styles (`filter`, `backdropFilter`, `morphFilter`, `transform3d`, `cache`) on the `<root>` itself do nothing; put them on a full-size child. `<root>` is its own UI root for `sharedTag` pairing.
- Tab order: main tree first, then each `<root>`, then anchors. Lighter alternative for a single pinned element: `positionType: "fixed"` + `globalZIndex`.

### 4.4 `<portal>` — garage turntable preview (Rust side)
```rust
use bevy::camera::visibility::RenderLayers;   // Bevy 0.20 path (as in the examples); not in bevy::prelude
use bevy_react::{PortalCamera, RenderMode, RenderTargetSpec, RenderTargets, Resolution};

const GARAGE_LAYER: usize = 7;

fn spawn_garage_preview(mut commands: Commands, mut targets: ResMut<RenderTargets>, mut images: ResMut<Assets<Image>>) {
    let rt = targets.create(&mut images, "garage.preview", RenderTargetSpec {
        size: Resolution::Auto,          // texture follows the <portal>'s laid-out size (×16, ≤2048/side)
        mode: RenderMode::Live,          // renders only while a portal showing it is visible
        ..default()                      // format: Rgba8UnormSrgb; use Rgba16Float for HDR/bloom
    });
    commands.spawn((
        Camera3d::default(),
        Camera { clear_color: ClearColorConfig::Custom(Color::NONE), ..default() }, // transparent → portal backgroundColor shows
        rt.camera_target(),                       // the Bevy RenderTarget component pointing at the texture
        PortalCamera("garage.preview".into()),    // lets bevy-react switch it off when no portal is visible
        RenderLayers::layer(GARAGE_LAYER),        // own mini-scene: car model + lights on this layer
        Transform::from_xyz(0.0, 1.2, 4.0).looking_at(Vec3::new(0.0, 0.4, 0.0), Vec3::Y),
    ));
}
// turntable: rotate the car entity's Transform in Update; nothing to tell React.
```
```tsx
<portal target="garage.preview" style={{ width: 640, height: 400, backgroundColor: "#0b0f1a" }} />
```
- `targets.remove(name)` → portals go transparent (despawn the camera yourself); `set_mode(name, RenderMode::Snapshot)` freezes it; `invalidate(name)` re-renders a snapshot; `get(name)` → handle.
- `Snapshot` mode = render once (track thumbnails, save slots), then free.
- A live portal inside a composited layer (ancestor with `filter`, `backdropFilter`, `transform3d`, `morphFilter`, `cache`, or group `opacity`) **freezes** → put `cache: "never"` on the node that creates the layer.
- `Resolution::Auto` assumes one portal per target; several portals on one target → `Resolution::Fixed(UVec2::new(w, h))`.
- `<portal>` has no intrinsic size; pointer events hit the box, never the camera's scene. `<anchor>` overlays don't appear inside portals.
- Mark your main/overlay UI camera `IsDefaultUiCamera` whenever portal/surface cameras exist (anchors fall back to "the first camera found" otherwise).

### 4.5 `<anchor>` (world-anchored labels)
```tsx
<anchor entity={carEntityBits} offset={[0, 1.5, 0]} scale={{ min: 0.5, max: 1.5, factor: 1, baseDistance: 15 }}>
  <text style={tagStyle}>P2</text>
</anchor>
```
- `entity` = `Entity::to_bits()` from Rust (request/event; typed `number | bigint`, crosses as JS number, exact < 2^53).
- Projects through **the `IsDefaultUiCamera` camera (else the first camera found)** — ONE camera. In split-screen with an overlay UI camera, anchors are useless for per-viewport labels → compute screen positions per viewport in Rust and drive shared values (§3.3).
- Hidden while entity missing / behind camera / beyond far plane; off-screen-side points stay positioned off-screen. No occlusion.
- Overrides `positionType`, `left`, `top`, and `translate*`/`scale*` of its `transform` (rotate works). Anchors live in their own UI root at `globalZIndex: -1` (below app UI) — set `globalZIndex` to raise.
- Move camera/entities in `Update` (or `PostUpdate` `.before(bevy_react::anchor::position_anchored_nodes)`) → never a frame behind.

### 4.6 `<svg>` (icons, gauges, minimap outline)
```tsx
<svg viewBox="0 0 200 200" style={{ width: 220, height: 220 }}>
  <path d={trackOutlineD} fill="none" stroke="#ffffff88" strokeWidth={6} strokeLinejoin="round" />
</svg>
```
- Whole `<svg>` re-rasterizes on the CPU on ANY shape change (incl. every frame of an animated/transitioned attribute) and on resize. Static outline = fine; **don't animate shapes per frame** (car dots → separate absolutely-positioned `<node>`s with animated `transform`).
- Subset: no `<text>`, gradients, clip/mask, `<defs>/<use>`, dashes. `points` is a flat `number[]`. Shapes take no `style`/`hoverStyle`.
- Animated numeric attrs: `r={{ animated: sv, seed: 12 }}`; `transition={{ y: { stiffness: 160, damping: 13 } }}`.

### 4.7 `<canvas>` (alternative for track outline)
```tsx
const draw = useCallback((ctx: CanvasContext) => {
  ctx.strokeStyle = "#ffffff88"; ctx.lineWidth = 4; ctx.beginPath();
  pts.forEach(([x, y], i) => (i ? ctx.lineTo(x, y) : ctx.moveTo(x, y)));
  ctx.closePath(); ctx.stroke();
}, [pts]);
<canvas draw={draw} style={{ width: 220, height: 220 }} />
```
- `draw` re-records whenever its identity changes → `useCallback`. Coordinates = logical px of the node; painter doesn't get the size (use `onResize`).
- Only path API (`moveTo/lineTo/quadraticCurveTo/bezierCurveTo/arc/rect/closePath/fill/stroke/clearRect/clear`, `fillStyle/strokeStyle/lineWidth`). No text/transforms/images/dashes. Fill starts **white** (web: black).
- CPU raster + full texture re-upload per batch; resize clears.

### 4.8 `<editableText>` (profile name)
```tsx
const [name, setName] = useState(profile.name);
<editableText value={name} onChange={setName} maxLength={16} autofocus
  onFocus={() => setTyping(true)} onBlur={() => setTyping(false)}
  style={{ width: 360, padding: { horizontal: 12, vertical: 8 }, border: 2, borderColor: "#445", fontSize: 24, color: "#fff" }}
  focusStyle={{ borderColor: "#7aa2f7" }} />
```
- Focusable by default (acts as `tabIndex={0}`); Escape blurs; clipboard/IME/selection handled by Bevy.
- **No submit event, no placeholder, no password mode, no undo.** Handle Enter via `bevy.on("keyDown")` while focused. React can't blur it: cyberpunk remounts the field (bumps its `key`) on Enter (`screens/newgame/IdCard.tsx:38-52`).
- Escape's `keyDown` reaches listeners **before** the field's `onBlur`, and Space/arrows/Enter are global keys too → while a field is focused, mount a swallowing intent layer (`const [typing, setTyping] = useState(false); useIntents((i) => { if (i === "confirm") submit(); }, typing);`, §7.2) so they don't also drive the menu.
- Global `keyDown`/`keyUp` still fire while typing, and Bevy's `ButtonInput<KeyCode>` sees every keystroke → gate Rust hotkeys with the `ui_typing` run condition (§7.5).
- **No gamepad text entry**: a pad-only player needs an on-screen keyboard we build (a grid of `<button>`s writing into `value`).
- `value` from React replaces text only when it differs; rejecting an edit in `onChange` does not revert the field.

### 4.9 `<surface>` (probably not needed for v1)
UI rendered into a texture for a 3D mesh (in-world billboards/scoreboards): `Surfaces::create(&mut images, name, SurfaceSpec { size, clear_color, mode, scale_factor })` → `Handle<Image>` for a material; tag the mesh `SurfacePointer::new(name)` to make it clickable. Each surface spawns its own UI camera (→ mark yours `IsDefaultUiCamera`). Live surfaces without a tagged mesh render every frame.

## 5. Styling cheat sheet

`style` (and `hoverStyle`, `pressStyle`, `focusStyle`, `focusVisibleStyle`) take a `BevyStyle` object (type exported by `bevy-react`; also `bevy-react/jsx`). Full list: `docs/dist/reference/style-properties.md`.

```tsx
import type { BevyStyle } from "bevy-react";
const panel: BevyStyle = {
  // layout (taffy flexbox/grid, keywords camelCase only)
  display: "flex", flexDirection: "column", justifyContent: "spaceBetween", alignItems: "center",
  gap: 12, padding: { horizontal: 24, vertical: 16 }, margin: "0 auto",
  width: "40vw", minWidth: 360, height: "auto", aspectRatio: 16 / 9,
  positionType: "absolute", left: 0, top: 0,           // relative | absolute | fixed
  overflowY: "scroll", scrollbar: "default",
  // paint
  backgroundColor: "rgba(10, 14, 24, 0.55)",
  border: 2, borderColor: "#7aa2f7", borderRadius: 12, outline: { width: 2, offset: 3, color: "#fff" },
  boxShadow: [{ color: "#0008", yOffset: 6, blurRadius: 18 }],
  backgroundGradient: { type: "linear", angle: 90, stops: [{ color: "#ff4d6d" }, { color: "#ffb703" }] },
  backdropFilter: { name: "blur", params: { radius: 12 } },
  opacity: 0.95, transform: { translateY: -4, scale: 1.02, rotate: 0 },
  zIndex: 1, globalZIndex: 5, focusPolicy: "block", cursor: "pointer",
  // text (only on <text>/<editableText>, no inheritance)
  color: "#e6edf3", fontFamily: "Display", fontSize: 28, fontWeight: "bold", letterSpacing: "0.05em",
  lineHeight: 1.2, textAlign: "center", lineBreak: "noWrap", textShadow: { color: "#000a", offsetX: 2, offsetY: 2 },
  transition: { transform: { stiffness: 220, damping: 18 }, backgroundColor: { duration: 150 } },
};
```

| Topic | bevy-react rule (differences from CSS in **bold**) |
|---|---|
| Units | number = logical px; `"50%"`, `"10vw"`, `"10vh"`, `"10vmin"`, `"10vmax"`, `"2rem"` (Bevy `RemSize`, default **20px**), `"1.5em"` (own fontSize on text, root size elsewhere), `"auto"`. **No `calc()`, `pt`, `ch`** → value becomes 0 + `length` warning. Logical px × window scale factor × `UiScale`; viewport units ignore `UiScale`. |
| Box model | **`boxSizing` defaults to `"borderBox"`**. Percent padding/margin/border are of parent width. |
| Flex | No `flex` shorthand (set `flexGrow`/`flexShrink`/`flexBasis`). No `order`, no RTL. `minWidth: 0` to let items shrink below content. `display: "block"` exists; `display: "none"` removes from layout+picking. |
| Grid | `gridTemplateColumns: "repeat(2, 1flex)"` (`flex` unit = `minmax(0,1fr)`, bevy-react-specific), `1fr`, `px`, `%`, `rem`, `auto`, `min-content`, `max-content`. **No `minmax()`, auto-fill/fit, multi-track `repeat()`, named lines/areas, vw/vh tracks.** `gridRow`/`gridColumn` strings. |
| Positioning | `absolute` is always relative to the **direct parent** (no positioned-ancestor search). `fixed` = viewport; **not** affected by ancestor transform/filter/opacity layers. No `sticky`. |
| Colors | hex (3/4/6/8), CSS named, `rgb()/rgba()/hsl()/hwb()/oklab()/oklch()`. **No `currentColor`, `color-mix()`, `lab()`**. Invalid → **magenta** + warning. `interpolateColor` takes **hex only**. |
| Borders | `border` (widths, layout-affecting), `borderColor` (string or per-side object, **no transition channel**), `borderRadius` (corners clockwise from top-left; `"50% / 50%"` for ellipse; `%` of shorter side without slash), `outline` (no layout space). **No dashed/dotted. Overflow clipping is rectangular (not rounded).** |
| Shadows | `boxShadow` `{ color, xOffset, yOffset, blurRadius, spreadRadius }` or array — **array order paints first at the back (reverse of CSS)**; no inset; no transition; cut off when the node is a layer (put it on an unpromoted wrapper). Text: `textShadow` hard-edged, no blur. |
| Gradients | `backgroundGradient` / `borderGradient`: `{ type: "linear", angle, stops }`, `{ type: "radial", position, shape, stops }`, `{ type: "conic", start, position, stops }`; default `colorSpace: "oklab"`; angle **0 = to top, clockwise**. Arrays layer. |
| Background image | `backgroundImage: { src: "path.png" \| { texture: "name" }, mode: "stretch"\|"repeat"\|"repeatX"\|"repeatY", scale, tint }` — always an object. |
| Filters | `filter: { name, params }` or array (chain order). Built-ins: `blur(radius=20)`, `brightness/contrast/saturate(amount=1)`, `grayscale/sepia/invert(amount=1 = full!)`, `hueRotate(angle)`, `bloom(radius, threshold, intensity)`, `chromaticAberration`, `gradientMap(angle, stops≤6, amount)` (= gradient text), `outline(width, color, softness)`, `shadow(color, offsetX, offsetY, spread)` (drop-shadow of the silhouette), `pinch`. **A bare `{ name: "blur" }` is a visible 20px blur.** `bloom`/`shadow` discard earlier chain entries → put them first. Promotes to a layer. |
| Backdrop | `backdropFilter`: same chains, applied to **the UI camera's rendered 3D frame (post-tonemap) under the node**, drawn opaque under the node's background. UI beneath is covered, not blurred. Translucent `backgroundColor` tints the glass. Re-runs **every frame** while set (cost ∝ area); forces enclosing layers to re-capture each frame. `transform3d` doesn't move the glass. |
| Opacity | On a node **with children** → group opacity via a composited layer (subtree captured; content outside the border box, incl. own `boxShadow`/`outline`, is **cut off**). `groupAlpha: false` → per-node fill alpha only. `opacity: 0` still takes hover/clicks → use `display: "none"` or unmount. |
| Transforms | `transform: { translateX, translateY, scale, scaleX, scaleY, rotate }` — post-layout, pivot = **center (no origin)**, no skew. `%` translate = own size. State-style `transform` replaces the whole object → restate fields. `transform3d: { perspective, rotateX/Y/Z, translateX/Y/Z, scale, origin }` promotes a layer; animating it never re-captures. |
| Z order | Tree order; `zIndex` among siblings; `globalZIndex` lifts a subtree into the top-level order (app tree = 0, `<root>` default 1, `<anchor>` -1). Hit-testing follows paint order. Inside a layer, `globalZIndex` can't escape the layer. |
| Pointer | `focusPolicy: "block" \| "pass"` (default `pass`; `<button>` = `block`) — about **pointer**, not keyboard focus. |
| Fonts | Rust-registered only: `ReactUiPlugin::default_font(path)`, `.font("Display", "fonts/x.ttf")` → `fontFamily: "Display"`. Exact, case-sensitive; fallback lists `"Display, Body"`; generic keywords need Bevy `system_font_discovery`. One file per family: static weights need one name each (`"Rajdhani Bold"`); variable fonts cover all weights. `fontWeight` is a **string** (`"bold"`, `"800"`), not a number. |
| Text | `fontSize` number/px/rem/em/vw/vh (no `%`); default `1rem` = 20px. `lineBreak: "noWrap"` (not `whiteSpace`). Unsetting `textAlign`/`lineBreak` keeps the last value → always set explicitly in state styles. `color` has **no transition** (use `interpolateColor` binding). |
| Cursor | `cursor: "pointer"` etc. (CSS kebab works) or a name registered with `ReactUiPlugin::cursor(name, path, hotspot)`; registering `"default"` replaces the arrow app-wide. bevy-react owns the primary window's cursor icon. |
| Image rendering | `imageRendering: "auto" \| "bilinear" \| "trilinear" \| "nearest"` (not on portals/canvas/`{texture}` sources). |
| Layout rounding | `layoutRounding: false` on the parent of nodes whose size animates through layout (no pixel stepping). |
| Layers / cache | Promoted by: group `opacity`, `filter`, `backdropFilter`, `morphFilter`, `transform3d`, `cache: "always" \| "never"`. Cached until content changes; moving/fading/filter params are free. `cache: "never"` for live portals/externally drawn textures inside layers. |

Recommended global scaling (from cyberpunk `main.rs` `fit_ui_to_1080p`): author all px at 1080p and set the window's `scale_factor_override = physical_height / 1080` (compare-before-write each frame) — or set Bevy's `UiScale` resource. Then `fontSize: 32` is 32 px at 1080p everywhere.
```rust
// cyberpunk/main.rs:143-157 (window created with WindowResolution::new(1600, 900).with_scale_factor_override(900.0 / 1080.0))
// use bevy::window::{PrimaryWindow, WindowResolution};   (not in bevy::prelude)
fn fit_ui_to_1080p(mut windows: Query<&mut Window, With<PrimaryWindow>>) {
    for mut window in &mut windows {
        let height = window.resolution.physical_height();
        if height == 0 { continue; }
        let scale = height as f32 / 1080.0;
        if window.resolution.scale_factor_override().is_none_or(|s| (s - scale).abs() > 1e-4) {
            window.resolution.set_scale_factor_override(Some(scale));
        }
    }
}
```
Note (`main.rs:135-142`): bevy_winit answers an override change by resizing the window to keep its logical size. Bevy's `UiScale` resource is the alternative that doesn't touch the window.

## 6. Animations

All easing runs in Bevy; React renders only end states. Four easings only: `"linear"` (default — **not** CSS `ease`), `"easeIn"`, `"easeOut"`, `"easeInOut"`. Springs: `{ stiffness=100, damping=10, mass=1 }`.

### 6.1 Style transitions
```tsx
<button
  style={{ backgroundColor: "#1b2333", transform: { scale: 1 },               // resting values in BASE style
           transition: { backgroundColor: { duration: 120 }, transform: { stiffness: 260, damping: 20 } } }}
  hoverStyle={{ backgroundColor: "#2b3a55" }} pressStyle={{ transform: { scale: 0.96 } }} />
```
- Channels (explicit only, **no `all`**): `transform`, `opacity`, `backgroundColor`, `size` (width/height/maxWidth/maxHeight, real relayout), `borderRadius` (relayout), `backgroundGradient`, `borderGradient`, `filter`, `backdropFilter`, `transform3d`, `morphFilter` (default 300 ms easeInOut even without entry), `scroll`, `layout` (FLIP: eases the laid-out rect, cheap), `sharedElement`.
- Triggered by re-render changes AND hover/press/focus state changes.
- **No enter/exit animations**: first style is adopted as-is; unmount is instant. For entrances use a shared value (`useEnter` pattern below) or mount with the start value and flip after ~1 frame via `setTimeout` (a bare `useEffect` may land in the same Bevy frame and snap).
- Snaps (no channel): text `color`, `borderColor`, positions (`left/top`), padding, margin, `gap`, `minWidth/minHeight`. Changing units (`px`→`%`, value→`auto`) snaps. Unsetting `filter`/`backdropFilter`/`transform3d`/gradient snaps → keep an identity entry (`blur radius 0`, `transform3d: {}`) in the base style.
- `rotate` eases numerically (350→10 goes the long way).
- An `{ animated }` binding on a property disables that property's transition (whole group for transform/filter/…).

### 6.2 Shared values (Reanimated-style)
```tsx
import { useSharedValue, withTiming, withSpring, withRepeat, withSequence, withDelay,
         interpolate, interpolateColor, cancelAnimation, Easing } from "bevy-react";

const t = useSharedValue(0);                          // stable handle per component; t.id: number
useEffect(() => { t.value = withRepeat(withTiming(1, { duration: 600, easing: "easeInOut" }), { reverse: true }); }, [t]);
<node style={{
  opacity: { animated: t },
  transform: { scale: { animated: interpolate(t, [0, 1], [1, 1.08]) } },
  backgroundColor: { animated: interpolateColor(t, [0, 1], ["#ff4d6d", "#ffd166"]) },   // hex only
}} />
```
- `x.value = n` sets instantly (stops driver); `x.value = withSpring(1)` animates from the live Bevy-side reading (no jumps). Reading `x.value` returns the **last JS-assigned** value, never the live one.
- Drivers: `withTiming(to, { duration=300, easing, onComplete })`, `withSpring(to, { stiffness, damping, mass, onComplete })`, `withRepeat(driver, { count?, reverse=false, onComplete })`, `withSequence(...drivers, cb?)`, `withDelay(ms, driver, cb?)`. Only the outermost driver's callback fires (`(finished: boolean) => void`).
- Assign drivers in handlers/effects, never during render (restarts every render).
- Lifetime = creating component. Unmount frees it (callback fires `false`).
- Bindable: `opacity`, `transform.*` (px / factor / degrees), layout lengths (`width/height/min*/max*/left/right/top/bottom/flexBasis/gap/rowGap/columnGap` — relayout every frame!), `aspectRatio`, `borderRadius` (all corners), colors via `interpolateColor` (`backgroundColor`, single `borderColor`, `color`, `backgroundImage.tint`, gradient stops), `transform3d` fields, filter/backdrop/morph params (`{ animated: sv, seed: maxValue }`), SVG numeric attrs. **Not**: padding, margin, text content, image `atlas.index`.
- Bindings work in the **base `style` only** (ignored with `styleBinding` warning in state styles).
- `interpolate` clamps outside the input range (add stops for overshoot); repeated stop = step.
- Rust drive: `bevy_react::animations::SharedValues` → `get(id) -> Option<f32>`, `set(id, value)` (stops any driver); run `.before(AnimationSet::Apply)` to show the same frame. Idle values cost nothing; bound nodes are written only on frames the value changed. Devtools Animations tab lists them ("idle" = Rust-driven, no meter).

Entrance helper (cyberpunk `ui/src/hooks.ts:42-51`):
```ts
export function useEnter(x = -24, delay = 0, duration = 280): BevyStyle {
  const t = useSharedValue(0);
  useEffect(() => { t.value = withDelay(delay, withTiming(1, { duration, easing: "easeOut" })); }, [t, delay, duration]);
  return { opacity: { animated: t }, transform: { translateX: { animated: interpolate(t, [0, 1], [x, 0]) } } };
}
```

### 6.3 Shared elements (`sharedTag`)
```tsx
// car card in the grid                              // hero in the detail screen (same commit swaps them)
<button sharedTag={`car-${id}`} style={{ width: 160, height: 100, transition: { sharedElement: { duration: 350, easing: "easeInOut" } } }} />
<button sharedTag={`car-${id}`} style={{ width: 640, height: 400, transition: { sharedElement: { duration: 350, easing: "easeInOut" } } }} />
```
- Pairs when ONE commit unmounts a tagged node and mounts one with the same tag, **same element type, same UI root** (`<root>`/`<surface>` are separate roots). Incoming node needs `transition: { sharedElement }`; give both the same `transition` so it works both ways.
- Size flies through real layout (put it in a fixed-size slot; `layoutRounding: false` on the re-flowing container); position by translation; outgoing node vanishes at once. No completion callback. Give the incoming node an explicit size.

### 6.4 Morph filters (screen transitions)
```tsx
<node style={{ width: "100%", height: "100%",
  morphFilter: { key: screen, name: "crossfade", params: { spread: 0.6 } },   // or "linearWipe" {angle, softness}, "pixelize"
  transition: { morphFilter: { duration: 420, easing: "easeInOut" } } }}>   // override = full timing (omitted easing → linear)
  {screen === "main" && <MainMenu />}{screen === "garage" && <Garage />}
</node>
```
- Only a `key` change morphs; first mount never animates (node needs ≥1 rendered frame). Mid-morph key change restarts from the in-flight blend. Size change across the key stretches the old image → keep the container fixed-size.
- `morphFilter` keeps the node a composited layer **even when idle** (memory + live portals inside freeze). Cyberpunk arms it only during a change (`App.tsx:77-105`): set `morph` → wait 60 ms → swap screen + flip key in one commit → remove `morphFilter` after `MORPH_MS + 120`, with `cache: "never"` while armed so live content keeps moving.
- Enter/exit of a single panel: morph between an empty carrier and content (docs `styling/morph-filters.md` "Morphing from and to nothing"): mount empty → after ~50 ms flip key + content → on exit keep carrier 300 ms.
- Custom morphs/filters: `#[react_morph_filter(shader = "shaders/x.wesl")]` / `#[react_filter(shader = "...", time = true)]` + `add_react_morph_filter::<T>()` / `add_react_filter::<T>()`; every field needs `#[serde(default…)]` (TS params are optional). See `docs/dist/extending/custom-filters.md`, cyberpunk `filters.rs`.

## 7. Input & navigation

### 7.1 What the library gives vs. what we build

| Need | Out of the box | We build |
|---|---|---|
| Mouse hover/click/drag | `onClick`, `onPointerDown/Move/Up/Enter/Leave/Hover`, `onWheel`, `hoverStyle`/`pressStyle` (Bevy-side, no re-render) | — |
| Keyboard | window-wide `keyDown`/`keyUp` events (`KeyboardEventData`), Enter/Space click a **focused** `<button>` | intent mapping, auto-repeat policy, modal key ownership |
| Gamepad | `gamepadConnected`/`gamepadDisconnected`/`gamepadInput` events, `bevy.gamepad.getAll()`, `bevy.gamepad.rumble()/stopRumble()` | **everything for navigation**: edge detection, stick thresholds, hold-to-repeat, focus model, A=confirm/B=back |
| Focus | Bevy `InputFocus` via `tabIndex`; `onFocus`/`onBlur`; `focusStyle`/`focusVisibleStyle`; Tab nav if the app adds `TabNavigationPlugin` | no `focus()`/`blur()` from React (only `autofocus` on `<editableText>`, a click, Tab, or Rust `InputFocus::set`) |
| Spatial (D-pad) nav | **none** in bevy-react ("No built-in focus navigation") | React-side focus manager (recommended) or Bevy's `AutoDirectionalNavigation` from Rust (§7.6) |
| Stop UI input reaching gameplay | mouse: `PointerCapture` resource; **keys/pads: nothing** (React can't consume or `preventDefault`) | Rust-side gating by app state (§7.5) |
| Event bubbling | **none** — one gesture → one element (topmost owner); no `stopPropagation`, no per-element key handlers | — |

Event payload facts:
- `KeyboardEventData`: `key` (logical: `"a"`, `"A"`, `"Enter"`, `"Escape"`, `"ArrowUp"`, **space bar = `"Space"`**), `code` (physical Bevy `KeyCode` name: `"KeyW"`, `"Digit1"`, `"F1"`, `"ShiftLeft"`, `"SuperLeft"`), `text: string | null`, `repeat` (OS auto-repeat), `ctrlKey/shiftKey/altKey/metaKey`.
- `gamepadInput: GamepadInputData = { buttons: GamepadButtonChange[], axes: GamepadAxisChange[] }`, at most once per frame, only frames with changes, all pads mixed. `GamepadButtonChange = { gamepad, button: GamepadButtonName, pressed, value }` (`pressed` stays true across analog changes → **do your own press-edge detection**). `GamepadAxisChange = { gamepad, axis: GamepadAxisName, value }` in -1..1, **stick up = +1** (inverse of web). Dead zones from Bevy `GamepadSettings`; idle pads send nothing.
- Button names: `south` (A/Cross), `east` (B/Circle), `north`, `west`, `leftTrigger`/`rightTrigger` (**bumpers**), `leftTrigger2`/`rightTrigger2` (analog triggers), `select`, `start`, `mode`, `leftThumb`, `rightThumb`, `dPadUp/Down/Left/Right`, `{ other: n }`. Axes: `leftStickX/Y`, `rightStickX/Y`, `leftZ`, `rightZ`.
- Pad ids count up, never reused (reconnect = new id). Needs Bevy feature `bevy_gilrs` (in Bevy defaults). Input before React's first applied render is dropped. `GamepadConnectedData = { gamepad, name, vendorId, productId }`.
- Rumble: `bevy.gamepad.rumble({ gamepad, duration /*ms*/, strongMotor /*0..1*/, weakMotor /*0..1*/ })` — all four required; overlapping rumbles add up; `stopRumble({ gamepad })` stops all.
- All handlers run ≥1 frame after the input, on the JS thread. Visual feedback that must be instant → state styles.

### 7.2 Reference: one input translator + intent stack (`ui/src/input.ts`)
Our design (typechecked, not run), adapted from the demos' gamepad menu (`examples/demos/ui/src/demos/events/gamepadDemo/GamepadDemo.tsx:86-188`, `menuNav.ts`: rising-edge detection keyed `` `${gamepad}:${name}` ``, `axisDir` at ±0.5, hold-to-repeat 350 ms then 120 ms). Difference: it is global and only the top-most mounted layer (screen < dialog < pause) receives intents — a generalized version of cyberpunk's `modalOpen()` counter.
```ts
import { useEffect, useRef } from "react";
import { on, type GamepadAxisName, type GamepadButtonName } from "./bevy";

export type Intent = "up" | "down" | "left" | "right" | "confirm" | "back" | "prevTab" | "nextTab" | "pause";
export type IntentHandler = (intent: Intent, pad: number | null) => void;   // pad null = keyboard

type Nav = { stack: { current: IntentHandler }[]; off?: () => void };
const g = globalThis as { __nav?: Nav };
const nav: Nav = (g.__nav ??= { stack: [] });          // survives hot reload (module code re-runs)

/** Receive intents while mounted and top-most; later-mounted layers shadow earlier ones. */
export function useIntents(handler: IntentHandler, enabled = true) {
  const ref = useRef(handler);
  ref.current = handler;
  useEffect(() => {
    if (!enabled) return;
    nav.stack.push(ref);
    return () => { const i = nav.stack.indexOf(ref); if (i >= 0) nav.stack.splice(i, 1); };
  }, [enabled]);
}
const fire: IntentHandler = (i, pad) => nav.stack.at(-1)?.current(i, pad);

const KEYS: Record<string, Intent> = { ArrowUp: "up", ArrowDown: "down", ArrowLeft: "left", ArrowRight: "right",
  Enter: "confirm", Space: "confirm", Escape: "back", KeyQ: "prevTab", KeyE: "nextTab" };
const BUTTONS: Record<string, Intent> = { dPadUp: "up", dPadDown: "down", dPadLeft: "left", dPadRight: "right",
  south: "confirm", east: "back", leftTrigger: "prevTab", rightTrigger: "nextTab", start: "pause" };
const DIRS = new Set<Intent>(["up", "down", "left", "right"]);
const REPEAT_DELAY_MS = 350, REPEAT_MS = 120;           // the demos' values

const nameOf = (n: GamepadButtonName | GamepadAxisName) => (typeof n === "string" ? n : `other(${n.other})`);
const axisDir = (v: number): -1 | 0 | 1 => (v >= 0.5 ? 1 : v <= -0.5 ? -1 : 0);

export function installInput() {                         // call at the top of index.tsx, before mount()
  nav.off?.();                                           // hot reload: drop the previous run's subscriptions
  const prevPressed = new Map<string, boolean>(), prevAxis = new Map<string, -1 | 0 | 1>();
  const timers = new Map<string, ReturnType<typeof setTimeout>>();
  const stop = (k: string) => { clearTimeout(timers.get(k)); timers.delete(k); };
  const act = (k: string, i: Intent, pad: number) => {
    fire(i, pad);
    if (!DIRS.has(i)) return;                            // hold-to-repeat for directions only
    const tick = () => { fire(i, pad); timers.set(k, setTimeout(tick, REPEAT_MS)); };
    timers.set(k, setTimeout(tick, REPEAT_DELAY_MS));
  };
  const offs = [
    on("keyDown", (e) => {
      const i = KEYS[e.key] ?? KEYS[e.code];
      if (i && (!e.repeat || DIRS.has(i))) fire(i, null); // keyboard repeat = OS auto-repeat
    }),
    on("gamepadInput", ({ buttons, axes }) => {
      for (const b of buttons) {
        const name = nameOf(b.button), k = `${b.gamepad}:${name}`, was = prevPressed.get(k) ?? false;
        prevPressed.set(k, b.pressed);
        if (b.pressed && !was) { const i = BUTTONS[name]; if (i) act(k, i, b.gamepad); } // rising edge only
        else if (!b.pressed && was) stop(k);
      }
      for (const a of axes) {
        const name = nameOf(a.axis);
        if (name !== "leftStickX" && name !== "leftStickY") continue;
        const k = `${a.gamepad}:${name}`, dir = axisDir(a.value);
        if (dir === (prevAxis.get(k) ?? 0)) continue;
        prevAxis.set(k, dir);
        stop(k);
        if (dir !== 0) act(k, name === "leftStickX" ? (dir > 0 ? "right" : "left") : (dir > 0 ? "up" : "down"), a.gamepad); // stick up = +1
      }
    }),
    on("gamepadDisconnected", ({ gamepad }) => { for (const k of [...timers.keys()]) if (k.startsWith(`${gamepad}:`)) stop(k); }),
  ];
  nav.off = () => { offs.forEach((off) => off()); timers.forEach(clearTimeout); };
}
```
```tsx
// ui/src/index.tsx
installInput();
mount(<App />);
```

### 7.3 A list menu on top of it
```tsx
// sfx(name) = bevy.sound.play wrapper (§13.4); item/itemFocused/label = BevyStyle objects
function useMenuIndex(count: number, onConfirm: (i: number) => void, onBack?: () => void) {
  const [index, setIndex] = useState(0);
  useIntents((intent) => {                       // handler sees the latest render (ref inside useIntents)
    switch (intent) {
      case "up": sfx("hover"); setIndex((i) => (i + count - 1) % count); break;
      case "down": sfx("hover"); setIndex((i) => (i + 1) % count); break;
      case "confirm": sfx("click"); onConfirm(index); break;
      case "back": if (onBack) { sfx("back"); onBack(); } break;
    }
  });
  return [index, setIndex] as const;
}

function MainMenu({ items }: { items: { id: string; label: string; run: () => void }[] }) {
  const [index, setIndex] = useMenuIndex(items.length, (i) => items[i].run());
  return (
    <node style={{ flexDirection: "column", gap: 8 }}>
      {items.map((it, i) => (
        <button key={it.id} onClick={it.run} onPointerEnter={() => setIndex(i)}   // mouse and pad share one highlight
          style={i === index ? itemFocused : item}>
          <text style={label}>{it.label}</text>
        </button>
      ))}
    </node>
  );
}
```
- Highlight via React state (one cheap re-render per move) + `transition` on the item styles. Do NOT give these buttons `tabIndex`: a focused `<button>` also clicks on Enter/Space → double activation with our `confirm` intent.
- Grids/carousels: same hook with 2-D index math; sliders consume `left/right` while focused (`setValue`) — decide per row.
- Long lists: no `scrollIntoView`. Keep a fixed row height and drive `scrollTop={Math.max(0, index * ROW - visibleRows * ROW / 2)}` + `transition: { scroll: { duration: 120 } }` on the `overflowY: "scroll"` container.
- **Split-screen car select**: the handler's second arg is the pad id (`null` = keyboard) → keep `Record<pad, index>`; join/leave on `confirm`/`back` per pad; roster from `usePads()` (demos `gamepadDemo/usePads.ts`: seed `bevy.gamepad.getAll()`, follow `gamepadConnected`/`gamepadDisconnected`).
- Text entry with a pad: build an on-screen keyboard (grid of buttons) — `<editableText>` only takes real keys.

### 7.3b When it matters WHICH player pressed (car select, per-player pause): send intents from Rust
The `gamepad` id in `gamepadInput` is bevy-react's own wire id (monotonic u32, never reused) — **not** the Bevy `Gamepad` entity, and the mapping is private (`crates/core/src/gamepad.rs:269`, `mod gamepad` is not `pub`). Rust, which drives the cars from `Gamepad` entities, therefore can't tell which wire id is which player. Keep pad→player assignment in Rust and push player-indexed intents (same `Intent` names as §7.2):
```rust
#[derive(serde::Serialize, ts_rs::TS, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum Intent { Up, Down, Left, Right, Confirm, Back, PrevTab, NextTab, Pause }

#[react_event(name = "input.intent")]
pub struct IntentEvent { pub player: Option<u8>, pub intent: Intent }   // None = pad not joined yet

fn pad_intents(pads: Query<(Entity, &Gamepad)>, slots: Res<PlayerSlots>, events: ReactEvents) {
    const MAP: [(GamepadButton, Intent); 9] = [(GamepadButton::DPadUp, Intent::Up), (GamepadButton::DPadDown, Intent::Down),
        (GamepadButton::DPadLeft, Intent::Left), (GamepadButton::DPadRight, Intent::Right), (GamepadButton::South, Intent::Confirm),
        (GamepadButton::East, Intent::Back), (GamepadButton::LeftTrigger, Intent::PrevTab),     // LeftTrigger = left bumper
        (GamepadButton::RightTrigger, Intent::NextTab), (GamepadButton::Start, Intent::Pause)];
    for (pad, gamepad) in &pads {
        for (button, intent) in MAP {
            if gamepad.just_pressed(button) { events.send(&IntentEvent { player: slots.player_of(pad), intent }); }
        }
    }   // + stick thresholds / hold-repeat with a Timer, like §7.2
}
```
In `input.ts` subscribe `on("input.intent", ({ player, intent }) => fire(intent, player))` instead of `gamepadInput` (keyboard can stay JS-side). Run the system only while a menu is up (not racing, or paused); the same Rust pad→player table also drives the cars, so menus and gameplay can't disagree about who is player 2. UNVERIFIED design (no example), but uses only documented pieces.

### 7.4 Mouse/pointer → world isolation (Rust)
```rust
use bevy_react::{PointerCapture, PointerCaptureSet};
fn orbit_garage_camera(capture: Res<PointerCapture>, /* mouse input */) {
    if capture.is_captured() { return; }  // dragging || over_ui || wheel_captured
    // ...
}
app.add_systems(Update, orbit_garage_camera.after(PointerCaptureSet));
```
- `over_ui` counts only **interactive** nodes (pointer handler, `onClick`, `hoverStyle`/`pressStyle`, `<button>`, `<editableText>`). A plain panel, even `focusPolicy: "block"`, does not capture → give overlay backdrops `hoverStyle={{}}` (cyberpunk `Dialog.tsx`, `theme.ts:159`: "Any hover style makes a node interactive (it then owns the pointer)").
- `focusPolicy: "block"` only stops Bevy picking/hover from reaching nodes/meshes beneath.

### 7.5 Keyboard/gamepad → gameplay isolation (Rust is the gate)
React cannot stop `ButtonInput<KeyCode>` / `Gamepad` from seeing a press, so every input has exactly one owner. ARCHITECTURE.md §5.13: during a race **Bevy owns pause toggling**; React renders the pause menu and sends resume/restart/quit. Make it "Rust opens, React closes":
```rust
#[derive(States, Default, Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum AppState { #[default] Menu, Loading, Racing }
#[derive(Resource, Default, PartialEq)] struct Paused(bool);

#[react_message(name = "race.resume")] pub struct Resume;      // TS: bevy.race.resume(null) — idempotent "set", never "toggle"

fn open_pause(/* rc_input intents */ mut paused: ResMut<Paused>, mut time: ResMut<Time<Virtual>>) {
    // on Start/Esc while racing and NOT paused: paused.0 = true; time.pause(); then the app.state event tells React
}
fn on_resume(_: On<Resume>, mut paused: ResMut<Paused>, mut time: ResMut<Time<Virtual>>) {
    paused.0 = false;
    time.unpause();
}
// "is the player typing into an <editableText>?" — no message needed: the field is Bevy's InputFocus and carries
// bevy::text::EditableText (+ bevy::ui_widgets::TextInput) (crates/core/src/elements/editable.rs)
fn ui_typing(focus: Res<bevy::input_focus::InputFocus>, fields: Query<(), With<bevy::text::EditableText>>) -> bool {
    focus.get().is_some_and(|e| fields.contains(e))
}
// vehicle input only while racing, not paused, not typing:
app.add_systems(Update, drive_cars.run_if(in_state(AppState::Racing))
    .run_if(resource_equals(Paused(false))).run_if(not(ui_typing)));
```
- While paused, Rust must **not** react to Start/Esc again: Esc/B in a pause sub-screen (settings) means "back" to React, and a Rust toggle would resume the race underneath it. Back from the pause menu's root → `bevy.race.resume(null)`.
- While racing, React registers no intent layer for the HUD, so A/B held for driving never trigger menu actions.
- Typing: gate Rust hotkeys with `ui_typing` (or, as the docs suggest, a message from the field's `onFocus`/`onBlur`).
- The button that confirmed "Resume" may still be held when gameplay resumes → optionally ignore vehicle input for a few frames after unpausing.
- Rumble for a specific player goes through Rust (rc_input knows the pad); React's `bevy.gamepad.rumble` takes the wire id (§7.3b).

### 7.6 Alternative: Bevy-side spatial navigation (UNVERIFIED with bevy-react; no example uses it)
Bevy 0.20 ships `bevy::input_focus::directional_navigation::DirectionalNavigationPlugin` + `bevy::ui::auto_directional_navigation::{AutoDirectionalNavigation, AutoDirectionalNavigator}` (geometry-based neighbor search, `AutoNavigationConfig`). Rust could insert `AutoDirectionalNavigation` on React nodes that have `TabIndex` (not a bridge-owned component, so allowed), map D-pad to `navigator.navigate(CompassOctant::North)`, and set `InputFocusVisible(true)`; then `focusStyle`/`focusVisibleStyle`/`onFocus`/`onBlur` follow with no React state. Caveats: it is **z-index/layer agnostic** (an open `<root>` dialog and the menu under it are one flat set → add/remove the component per active layer), single global focus (useless for 4 simultaneous cursors), and A must still be routed (a focused `<button>` only auto-clicks on Enter/Space). Prefer §7.2 unless a screen needs geometry-driven navigation.

### 7.7 Tab navigation (keyboard only, optional)
`app.add_plugins(bevy::input_focus::tab_navigation::TabNavigationPlugin);` — bevy-react already marks UI roots as tab groups. Order = tree order of `tabIndex >= 0`, positives **after** zeros (opposite of HTML). Window UI first, then each `<root>`, then anchors; `<surface>` is its own scope. `display: "none"` subtrees are **not** skipped (unmount them). Bevy warns on the first Tab after startup. Focus from Rust: `focus.set(entity, FocusCause::Navigated)` on `ResMut<InputFocus>` (entity via `ReactNodes::get(name)`).

## 8. Cameras, split-screen and overlays

### 8.1 Which camera the UI uses
- bevy-react's main tree and every `<root>` are ordinary bevy_ui roots **without `UiTargetCamera`** → bevy_ui's `DefaultUiCamera::get()`: the single camera marked `IsDefaultUiCamera` (two marked → warning + fallback), else **the highest-`order` camera targeting the primary window** (`bevy_ui/src/ui_node.rs:3453-3479`). Only `<surface>` sets `UiTargetCamera` (to the camera it spawns).
- The UI lays out against that camera's **viewport** (vw/vh, `window.size()`, `resize`, `fixed` all follow it). `bevy.window.size()`/`resize` report the `IsDefaultUiCamera` viewport (window size if none is marked).
- `<anchor>` projects through `IsDefaultUiCamera`, else an arbitrary first camera (`crates/anchor/src/position.rs:195-215`) — never a portal camera if you mark yours.
- `backdropFilter` samples **the UI camera's `ViewTarget` main texture** at the start of its UI pass, i.e. that camera's tonemapped 3D frame without UI (`crates/core/src/layer/render/backdrop.rs`, `camera_main_texture`).
- **Pitfall:** with several viewport player cameras and NO marker, the main tree, every `<root>`, the anchor layer and devtools all draw **inside the highest-`order` player's viewport**, while `resize` reports the full window (`crates/core/src/window.rs:88-101`). Always mark the UI camera.
- Layer features (`filter`, `backdropFilter`, group `opacity`, `transform3d`, `morphFilter`, `cache`) need the UI camera to be an active `Camera2d` or `Camera3d` (`crates/core/src/layer/render.rs:485, 512-520`).
- Example setups: every 3D example uses ONE `Camera3d` + `IsDefaultUiCamera` (cyberpunk `datascape.rs:127-172`: `Hdr`, `Msaa::Off`, `Bloom`, `DepthOfField`, … + `IsDefaultUiCamera`); minimal uses a bare `Camera2d`. **No example does split-screen or a separate overlay camera.**

### 8.2 Recommended ReCharged camera rig
One full-window React tree over 1–4 player views:
```rust
use bevy::camera::Viewport;                  // Bevy 0.20: bevy_camera is bevy::camera (also bevy::camera::Hdr, ::visibility::RenderLayers)
use bevy::ui::IsDefaultUiCamera;

// Overlay UI camera: no viewport, highest order, draws only the UI.
commands.spawn((
    Camera2d,
    Camera { order: 100, clear_color: ClearColorConfig::None, ..default() },   // rig A; rig B: Custom(Color::NONE)
    IsDefaultUiCamera,
));

// One 3D camera per local player (LDR: no `Hdr` component; default `Msaa`).
for (i, rect) in split_rects(window_physical_size, players).into_iter().enumerate() {
    commands.spawn((
        Camera3d::default(),
        Camera {
            order: i as isize,
            viewport: Some(Viewport { physical_position: rect.min, physical_size: rect.size(), ..default() }),
            // rig A: only the first player camera clears; the others must not (see 8.3)
            clear_color: if i == 0 { ClearColorConfig::Default } else { ClearColorConfig::None },
            ..default()
        },
        PlayerCamera(i as u8),
    ));
}
```
React side: one root `<node style={{ width: "100%", height: "100%" }}>` with a grid that mirrors the viewport split; each cell hosts a `<PlayerHud player={i} />`:
```tsx
const GRID: Record<number, BevyStyle> = {
  1: { display: "grid", gridTemplateColumns: "1fr", gridTemplateRows: "1fr" },
  2: { display: "grid", gridTemplateColumns: "1fr", gridTemplateRows: "repeat(2, 1flex)" },   // horizontal split
  3: { display: "grid", gridTemplateColumns: "repeat(2, 1flex)", gridTemplateRows: "repeat(2, 1flex)" },
  4: { display: "grid", gridTemplateColumns: "repeat(2, 1flex)", gridTemplateRows: "repeat(2, 1flex)" },
};
<node style={{ width: "100%", height: "100%", ...GRID[players] }}>
  {Array.from({ length: players }, (_, i) => (
    <node key={i} style={{ overflowX: "clip", overflowY: "clip" }}><PlayerHud player={i} /></node>
  ))}
  {players === 3 && <node><SharedMinimap /></node>}
</node>
```
- Keep the Rust viewport math and the React grid in sync (both derived from window size; Rust recomputes on `WindowResized`) — or, as ARCHITECTURE.md plans (`PlayerViewports` → `viewports` event), position each HUD `positionType: "absolute"` at the logical rects Rust sends (one source of truth; UI px must equal window logical px, i.e. no `UiScale`, see §5 scaling). Whole-window overlays (pause, countdown, results) are `<root>`s on top.
- Alternative (cyberpunk's game view): render each player into a texture (`RenderTargets::create` + `PortalCamera`) and lay out `<portal>`s in React (React owns the split; pause blur via `filter` + `cache: "never"`). Costs: textures capped at **2048 px/side** (a 2560-wide single view gets upscaled), `<anchor>` doesn't work inside portals, `backdropFilter` can't see portal content. Use viewports for races, portals for previews.

> **Orchestrator update (2026-10-18):** the Bevy 0.20 digest GPU-verified that an overlay
> `Camera2d` with `ClearColorConfig::None` + `IsDefaultUiCamera` composites correctly over two
> **HDR** viewport cameras (0.20 alpha-composites mixed-HDR stacks; see `bevy-0.20-notes.md` §9 and
> its verification log). ReCharged therefore uses that rig for split-screen (ARCHITECTURE §5.8),
> with translucent panels instead of `backdropFilter`; rig B below is the fallback if the real
> bevy-react UI misbehaves on it. Single-view screens host the UI on the 3D camera.

### 8.3 Why those clear colors (Bevy 0.20 internals; UNVERIFIED by running — prototype the 4-player rig early)
- Cameras with the same `(target, texture usages, main texture format [HDR or not], Msaa)` share the window's main textures (`bevy_render/src/view/mod.rs:1380-1392` `main_texture_key`), but **each camera gets its own `ColorAttachment` and clears the whole shared texture on first use if its `clear_color` isn't `None`** (`view/mod.rs:1514-1517`, `texture/texture_attachment.rs:50-52`). Viewport cameras still look right on screen because each one blits only its viewport to the window (`bevy_core_pipeline/src/upscaling/node.rs:92-95`); later cameras blend (`upscaling/mod.rs:63-80`: `sorted_camera_index_for_target > 0` → `BlendState::ALPHA_BLENDING`).
- **Check against ARCHITECTURE.md §5.8/§5.9**: a `UiCamera` that "never clears the world" (`ClearColorConfig::None`) is only correct in rig A (all LDR, only player 0 clears). With the planned per-camera HDR/bloom/SSAO/motion blur it either blits a never-cleared stale texture (HDR players → separate texture) or "last player + clear color" (shared texture) over the other viewports → use rig B for the UiCamera.
- **Rig A (frosted glass over gameplay)**: all cameras LDR with equal `Msaa` → one shared texture. Player 0 clears, players 1..n and the overlay use `ClearColorConfig::None`, so the shared texture accumulates every viewport; the overlay's UI (and `backdropFilter`, which samples that texture) sees all of them, and its full-window blit is correct. With default clears on players 1..n the overlay would blit "last player + clear color" over the others. No HDR/post-process on player cameras here: Bevy's full-screen post passes (e.g. HDR tonemapping, `bevy_core_pipeline/src/tonemapping/node.rs`, no viewport) would re-process neighbours' pixels in the accumulated texture. LDR cameras tonemap in-shader (that node returns early for `!camera.hdr`).
- **Rig B (players may use `Hdr`/bloom/post FX)**: keep players' default clears; the overlay `Camera2d` clears to transparent (its own texture, or — with LDR players — the shared one after the players already blitted) and is blended on top. bevy_ui draws with straight `ALPHA_BLENDING` (`bevy_ui_render/src/pipeline.rs:99`), so over a transparent clear the result is premultiplied → blend it as such or translucent UI darkens:
  ```rust
  Camera {
      order: 100,
      clear_color: ClearColorConfig::Custom(Color::NONE),
      output_mode: bevy::camera::CameraOutputMode::Write {
          blend_state: Some(bevy::render::render_resource::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
          clear_color: ClearColorConfig::None,
      },
      ..default()
  }
  ```
  `backdropFilter` then has no 3D frame to blur (use translucent fills).
- **Rig C (per-player HUD on each player camera; UNVERIFIED, no example)**: render each HUD as `<root name="hud-p{i}">` and insert `UiTargetCamera(player_cam)` on it from Rust — `UiTargetCamera` is not bridge-owned and bevy-react only writes it on `<surface>` roots (`crates/surface/src/registry.rs:192`). Then `vw`/`vh` (the `<root>` default `100vw × 100vh`) resolve to that player's viewport and `backdropFilter` samples that player's frame (viewport offset handled, `crates/core/src/layer/render/backdrop.rs:268-283`), HDR included. The main tree (pause, results) still needs a full-window UI camera (rig B overlay). Caveats: click/focus hit-testing compares depth only across UI cameras, the wheel ignores offset viewports, `resize` reports only the default camera.
  ```rust
  fn bind_player_huds(nodes: ReactNodes, cams: Query<(Entity, &PlayerCamera)>, have: Query<&UiTargetCamera>, mut commands: Commands) {
      for (cam, p) in &cams {
          let Some(root) = nodes.get(&format!("hud-p{}", p.0)) else { continue };
          if have.get(root).ok().map(|t| t.0) != Some(cam) { commands.entity(root).insert(UiTargetCamera(cam)); }
      }
  }   // Update, .after(bevy_react::ReactApplySet); re-inserts after remount/full reload
  ```
- Single-view screens (menus/garage) can also simply mark the one 3D camera `IsDefaultUiCamera` (what every example does) — backdrop blur works there in any config.
- `Camera2d` defaults to `Tonemapping::Linear` (`bevy_core_pipeline/src/core_2d/mod.rs:56`); MSAA writeback for `ClearColorConfig::None` cameras is automatic (`MsaaWriteback::Auto`).

### 8.4 Other multi-camera rules
- Mark the UI camera `IsDefaultUiCamera` whenever portal/surface cameras exist.
- Portal cameras: tag `PortalCamera(name)`, give them their own `RenderLayers`; put the scene they film on that layer. GPU light clustering ignores render layers (cyberpunk spaces dioramas 2000 units apart).
- `<surface>` spawns its own `Camera2d` per surface with **`order: -1`** (`crates/surface/src/registry.rs:204-206`) → keep player cameras at `order >= 0`; give portal cameras an order below the UI camera so their texture is fresh when the UI samples it. Every camera on one target needs a distinct `order` (Bevy warns otherwise).
- `SurfacePointerRay::Cursor` (default) casts from the first active window camera — ambiguous with an overlay/split-screen; write `SurfacePointerRay::Ray(..)` yourself if you use surfaces.
- `<anchor>` with a 2D overlay as `IsDefaultUiCamera` projects through the 2D camera (useless). Alternative to §3.3: keep `<anchor name="p2" entity={bits}>` in React and re-project in Rust per player camera in `PostUpdate` `.after(bevy_react::anchor::position_anchored_nodes).before(bevy::ui::UiSystems::Layout)`, overwriting the anchor's `UiTransform` translation/`Visibility` (anchors already own those every frame).
- The scroll wheel ignores an offset UI-camera viewport (docs `styling/overflow.md` Limits) → another reason to keep the UI camera full-window.

## 9. Hot reload, devtools, production

### 9.1 Hot reload (native only)
```sh
(cd ui && npm run watch)   # terminal 1: rebuilds app.js on save
cargo run                  # terminal 2 (repo root)
```
- Bevy polls `app.js` mtime every 0.3 s (`crates/core/src/host/native.rs`) and re-executes it in the live V8 isolate → React Fast Refresh. Rust changes need a rebuild/restart as usual.
- Keeps hook state unless the component's hook list/initial values changed (then it and its subtree remount). npm-package components always remount. Shared values and running animations survive a state-keeping reload; a full reload clears them (re-bind ids in effects, §3.1).
- **Every effect re-runs on every reload** → every effect needs a cleanup (else double subscriptions).
- **Module-level state is re-created** (stores, `createContext`, top-level `bevy.on(...)`, timers). Park singletons on `globalThis`:
```ts
export function hmrSingleton<T>(key: string, init: () => T): T {      // from docs/dist/tooling/hot-reload.md
  const g = globalThis as unknown as Record<string, T | undefined>;
  return (g[key] ??= init());
}
const NavContext = hmrSingleton("__navContext", () => createContext<Nav | null>(null));
```
- A syntax error fails the esbuild rebuild (old `app.js` stays live). A bundle that throws on load is rejected (previous version stays live; error in terminal + devtools Console). A component that throws while rendering is a normal React error (`[js] react error:` in the log); without an error boundary React unmounts the tree → add a top-level error boundary that shows a fallback and logs.
- `vendor.js` is never reloaded: after upgrading `react`/`bevy-react`, restart both the watcher and the game.
- Production bundles still hot-reload but remount everything; release binaries watch too unless `hot_reload(false)`.

### 9.2 Devtools (debug builds + `devtools` feature + dev bundle)
- **F12** toggles (`DevtoolsConfig { enabled, toggle_key: KeyCode::F12, pause_key: KeyCode::F11, settings_path: Some(".bevy-react-devtools.json".into()) }`, all `..default()`-able).
- Tabs: **Nodes** (tree incl. component names, per-`<root>` chips; live style/prop editing — transient), **Layers** (why promoted, repaint counter, texture memory), **Animations** (shared values + transitions; pause/slow `Time<Virtual>` and scrub), **Console** (JS console + Rust warnings, last 500), **Bridge** (every op batch/message/request/event with JSON and cost split `js/flush/translate/command/layout`). Header: dock/float, "push the app aside", pick-node-on-screen, outline, **paint flashing** (a box that stays lit on a still screen = a write repeated every frame).
- The panel owns the pointer over it (`PointerCapture` reports it). Pause in the Animations tab pauses `Time<Virtual>` → your game too; JS timers keep running on wall time.
- Warnings (unknown prop/style/color/length…) go to the terminal once per distinct message in every build, and flag the node in devtools.

### 9.3 Release
`npm run build:prod` + `cargo build --release` + `hot_reload(false)` + exe-relative bundle path (§1.4). Devtools Rust code is compiled in (inert) unless the feature is dropped.

## 10. Testing & diagnostics

### 10.1 What exists
- **Typecheck**: `npm run typecheck` (`tsc --noEmit`). esbuild never type-checks, so CI must run it. A stale `bevy.ts` is the main source of runtime "unknown message"/"malformed request" errors → CI: `cargo run -- --export-bindings ui/src/bevy.ts && git diff --exit-code -- ui/src/bevy.ts`.
- **No test renderer / testing-library** in the npm package (exports: `.`, `./jsx`, `./jsx-runtime`, `./jsx-dev-runtime`, `./build-lib`). Pure TS logic (menu reducers, nav, stores) is testable with any runner if those modules don't import `./bevy`; the library's own `js/test/*.test.mjs` stubs `globalThis.__bevyHost = { op_flush(){}, op_emit(){}, op_request(){}, op_animate(){}, op_next_event: () => new Promise(() => {}) }` before importing the bundled runtime.
- **Headless UI round-trip tests** (`examples/demos/tests/*.rs`, harness `tests/common/mod.rs`): spawn the real JS thread on the built bundle with `bevy_react::js_thread::spawn_js_thread(..)`, play Bevy's role over channels (send `Outbound::UiEvent { kind: "click", .. }`, `Outbound::Event { name, value }`), record op batches into a tree and assert (`find_button`, `wait_op`, `Op::UpdateText`). Skips if `ui/dist/app.js` is missing; a crashed runtime panics `"JS thread died (runtime crashed)"`. Uses `bevy_react_core` with feature `test_util` — **unstable, doc-hidden**; copy only if we really need UI integration tests.
- **Scripted runs** (`--shoot` in cyberpunk/synth/coldwake/civilization): Rust sends `#[react_event(name = "debug.act")] Act { action: String }` on a timeline; React handles it with `useDebug(verb, run)` (`cyberpunk/ui/src/hooks.ts:33-38`); the clock starts at React's first commit (`Query<(), With<ReactNode>>` non-empty). Good pattern for automated screenshots/smoke runs.
- Data-contract tests: cyberpunk/coldwake `include_str!` a TS file and assert ids match Rust tables (`cyberpunk/dioramas/mod.rs:386-403`, `coldwake/holo/mod.rs:436-447`).

### 10.2 Seeing JS logs and errors from Rust
- Every `console.*` goes through `op_log` → `tracing` with **target `bevy_react_core::js`**: `error`→ERROR, `warn`→WARN, `debug`/`trace`→DEBUG, `log`/`info`/`dir`/`table`→INFO (`crates/core/src/js_thread.rs:111-126`). Filter: `RUST_LOG=info,bevy_react_core::js=debug cargo run`.
- Runtime error prefixes (all `console.error`): `[js] react error:` (uncaught/caught/recoverable React errors), `[js] handler error:` (an `onClick`/`onPointer*` threw), `[js] listener error:` (a `bevy.on` callback threw), `[js] animation callback error:`, `[js] unhandled promise rejection:` (logged and swallowed — the app keeps running), `[js] fast refresh error:`.
- bevy-react's own invalid-value warnings (unknown prop/style/color, `featureMissing`, `nameAmbiguous`, …) go to **target `bevy_react`** (`crates/core/src/diag.rs:260`), once per distinct message, in every build. Unknown message name → WARN `no handler registered for react message`; malformed payload → ERROR `malformed react message`.
- Startup/reload failures: missing bundle → panic; bundle that throws on first run → ERROR `initial runtime build failed: …` and **no UI, no panic** (JS thread exits; `js_thread.rs:329`); bad hot reload → WARN `update rejected (…); keeping the previous working version`.
- Programmatic (debug builds with `devtools` feature only; stubbed to empty otherwise): `bevy_react::console_log::since(seq) -> (Vec<ConsoleEntry>, u64)`; `ConsoleEntry { seq, time_ms, source: Source::{Js, Rust}, level: Level::{Debug, Info, Warn, Error}, message }` (`crates/core/src/console_log.rs`). Use it for a dev "UI error" toast or a smoke check:
```rust
#[cfg(debug_assertions)]
fn fail_on_js_errors(mut seen: Local<u64>) {
    let (entries, next) = bevy_react::console_log::since(*seen);
    *seen = next;
    for e in entries.iter().filter(|e| e.level == bevy_react::console_log::Level::Error) {
        error!("UI error: {}", e.message);   // or panic! in a --smoke run
    }
}
```
- Devtools Console tab shows the same ring (last 500, collected from startup).

## 11. Performance rules

1. **Re-render freely, mutate sparingly.** Props/styles are diffed field-by-field; an identical re-render sends nothing. Cost scales with nodes *created or changed* per commit (mounting ~1000 nodes ≈ 36 ms end-to-end on a 5800X — `docs/dist/performance.md`). Don't mount big screens mid-race; mount results/podium only at race end.
2. **Nothing per frame through React.** No `setState` per frame, no per-frame events. Continuous visuals = shared values driven by Rust (§3.1) or transitions. Events ≤ ~10 Hz, batched for all players.
3. **Animate `transform`/`opacity`/colors**; `width/height/left/top/gap/borderRadius/padding` relayout the UI every changed frame. `transition: { layout }` (FLIP) is cheap; `transition: { size }` relayouts.
4. **Layers cost a texture each** (node size × DPI + filter reach). Group `opacity` on a container promotes it → for long faded lists use `groupAlpha: false`. `filter`, `transform3d`, `morphFilter` (even idle!), `cache` promote. Keep identity entries (`blur radius 0`) only where an ease-out is needed.
5. **`backdropFilter` and `cache: "never"` re-run every frame** and force every enclosing layer to re-capture every frame. Use backdrop glass on a few panels, not on every list row; don't nest it inside other layers.
6. **Live portals = a full extra camera render per frame while visible.** Tag with `PortalCamera` so bevy-react can switch them off; use `RenderMode::Snapshot` for thumbnails; one turntable preview at a time.
7. **CPU rasterizers:** `<svg>` re-rasterizes entirely on any shape change/resize; `<canvas>` re-uploads the whole texture per batch; `.svg` `<image>` re-rasterizes per size change. Static outlines fine; moving markers = UI nodes with bound transforms.
8. **Stable identities:** `draw` painters in `useCallback`; context values in `useMemo` (synth `App.tsx:41-44` comment: a fresh value re-renders every consumer).
9. **Text** changes re-shape and relayout; keep frequently updated text in small leaf components and fixed-size boxes.
10. **`RemSize` changes relayout everything and re-render all layers** — settings-only, never animated.
11. Measure with devtools: Bridge tab (per-batch `js/flush/translate/command/layout` ms), Layers tab (repaint counters climbing on a still screen = bug), paint flashing.

## 12. Gotchas & sharp edges

**React/JS side**
- `mount(<App />)` never resolves — don't `await`; it must run on every hot-reload execution (re-parks the event loop).
- Top-level elements sit in a centered column with 16px gaps → render one full-size root `<node>`.
- Bare strings outside `<text>` render white 20px built-in font. Spans don't inherit styles.
- **No bubbling/capture/`stopPropagation`.** A child with *only* `onPointerEnter` (any `onPointer*`) **owns clicks** and swallows its `onClick` ancestor's clicks. A `<button>` without `onClick` still swallows clicks.
- `<node onClick>` lets the press through to the 3D scene; `<button>` blocks.
- `hoverStyle={{}}` (even empty) makes a node "interactive": it claims the pointer for `PointerCapture` and owns hover — cyberpunk uses it on modal backdrops.
- No `focus()`/`blur()` from React. To blur an `<editableText>` on Enter, remount it (`key` bump) — cyberpunk `screens/newgame/IdCard.tsx:38-52`.
- A focused `<button>` (`tabIndex`) clicks on Enter/Space **in addition to** your global `keyDown` handler → double activation. Don't give menu buttons `tabIndex` when you navigate with your own state.
- Docs mention `visibility: "hidden"` (focus.md) but **no `visibility` style exists in 0.8.1** (`js/src/generated/style.ts`, `reference/style-properties.md`). Tab navigation does not skip `display: "none"` subtrees → unmount instead.
- Events are not buffered; listeners are global (unsubscribe in cleanup); requests can hang forever if unanswered; a request can't be cancelled.
- A message sent in the same commit that mounts a node arrives in Bevy **before** that node exists.
- Unit-struct message = `bevy.x.y(null)`; unit-struct request = `bevy.x.y()`.
- `fontWeight` is a string; `interpolateColor` hex-only; `transition` has no `all` and defaults to **linear**; no enter/exit transitions; first style is adopted without animating.
- `{ animated }` bindings are ignored in `hoverStyle`/`pressStyle`/`focusStyle` (`styleBinding` warning) and they disable that property's transition.
- State-style `transform`/`padding`/`backgroundImage` replace the whole value (unspecified fields reset, not merged).
- `boxShadow` arrays paint first-at-back (reverse of CSS); `bloom`/`shadow` filters discard earlier chain entries; bare `{ name: "blur" }` = 20px; `grayscale`/`sepia`/`invert` default to full effect.
- Unsetting `textAlign`/`lineBreak` keeps the previous value.
- `"start"`/`"end"` ≠ `"flexStart"`/`"flexEnd"`; keywords are camelCase only (`"spaceBetween"`).
- Timers (`setTimeout`) run on wall time and keep running while the game/devtools is paused.
- Hot reload re-runs module top-level code and all effects (see §9.1).
- 64-bit ints: typed `bigint` in TS but arrive as JS numbers.
- Codegen panics: a top-level name `emit`/`request`/`on`/`addEventListener`/`removeEventListener`; a name used as both namespace and method; message + request with the same name. Built-in names win: an app binding reusing one is left out of `bevy.ts`.

**Rust side**
- bevy-react spawns **no camera**; mark the UI camera `IsDefaultUiCamera` when any other camera exists (portals, surfaces, split-screen), else bevy_ui picks the highest-`order` window camera and `<anchor>` "the first camera found".
- The bridge owns `Node`, `BackgroundColor`, `BorderColor`, `Outline`, `BoxShadow`, `ZIndex`, `GlobalZIndex`, `LayoutConfig`, `Visibility`, `UiTransform`, `ImageNode`, `Text*`, `ScrollPosition`, `FocusPolicy`, `Pickable`, `ReactInteraction`, `TabIndex`, `TabGroup`, `Name`, `Children`, `ChildOf` on React entities — writes get overwritten (sometimes much later). Add your own components freely.
- Children you parent under a React node get orphaned (render as top-level UI roots!) when React changes that node's children.
- `Entity` ids from `ReactNodes` go stale on unmount and on full reload — look up by name each frame or watch `RemovedComponents<ReactNode>`. Order readers `.after(ReactApplySet)`; for layout (`ComputedNode`, `UiGlobalTransform`) run in `PostUpdate` after `UiSystems::Layout`.
- `ReactNodes::get(name)` with duplicate names warns `nameAmbiguous`; use `all(name)`.
- React cannot stop `ButtonInput<KeyCode>`/gamepad input reaching your systems; mouse capture is only advisory (`PointerCapture`).
- Request observers whose params fail validation never run → promise hangs.
- `hot_reload` defaults to `true` even in release builds.
- Custom element attribute deserialize failure fails the **whole commit** (TypeError on the JS side, nothing applied).
- A bundle that throws at startup leaves the game running with **no UI and no panic** — watch for `initial runtime build failed` in the log (CI smoke run).
- Gamepad wire ids (`gamepadInput.gamepad`) can't be mapped to Bevy `Gamepad` entities from Rust (§7.3b).
- No ErrorBoundary ships with the library; an uncaught render error unmounts the whole React tree.
- Bevy 0.20 (not bevy-react): every camera with a clear color clears the **whole** shared main texture, and full-screen post passes (HDR tonemapping, `bevy_core_pipeline/src/tonemapping/node.rs`) ignore the viewport. Harmless with default clears (each camera blits only its viewport), but an overlay UI camera then only "sees" the last player → pick rig A (LDR, accumulate) or rig B (transparent overlay) in §8.3. UNVERIFIED by running.

## 13. Patterns from the examples

Paths relative to `/home/tul/Projects/bevy-react/examples/`.

### 13.1 cyberpunk: screens without a router (`cyberpunk/ui/src/App.tsx`)
- One `useState<Screen>` (`"splash" | "title" | "menu" | "newgame" | "load" | "save" | "settings" | "credits" | "loading" | "game"`) and conditional rendering inside one full-size morph container (`App.tsx:231-293`). Every screen gets `onBack`/`onPick` props; each screen handles its own Esc.
- Navigation goes through `go(next, apply?)` so the morph is armed before the swap (`App.tsx:88-105`):
```ts
const go = (next: Screen, apply?: () => void) => {
  setDialog(null);
  const t = timers.current;
  clearTimeout(t.end);
  const flip = () => { apply?.(); setScreen(next); setMorph(next); t.end = setTimeout(() => setMorph(null), MORPH_MS + 120); };
  if (morph) { clearTimeout(t.flip); flip(); } else { setMorph(screen); t.flip = setTimeout(flip, 60); }
};
// container style while armed (App.tsx:231-246):
// { cache: "never", morphFilter: { key: morph, name: settings.uiGlitch ? "glitchSwap" : "crossfade", params: … },
//   transition: { morphFilter: { duration: MORPH_MS, easing: "linear" } } }   // MORPH_MS = 420
```
- Sub-screens are local state with early returns (`screens/settings/Settings.tsx:99-100`); tabs morph with an always-on `morphFilter: { key: tab, … }` plus `key={tab}` on the child (`Settings.tsx:132-162`).
- **Pause menu = the main menu with other entries**: `<MainMenu entries={inGame ? PAUSE : MAIN} onPick={pick} onBack={inGame ? () => go("game") : undefined} />` (`App.tsx:250-255`); Esc in game: `useKeys((e) => { if (e.key === "Escape" && screen === "game" && !dialog) { sfx("back"); go("menu"); } })` (`App.tsx:187-193`). Pause is **visual only** there (the game world `<portal target="world" style={{ cache: "never", filter: { name: "blur", params: { radius: paused ? 14 : 0 } }, transition: { filter: { duration: 300 } } }} />`, `screens/InGame.tsx:16-28`) — a racing game must also pause the simulation in Rust (§7.5).
- Confirm dialog = full-size node with `focusPolicy: "block"` + `hoverStyle={{}}` (owns the pointer) rendered last inside the container; a module counter `modalOpen()` lets screens ignore keys while it's up (`screens/Dialog.tsx:9-15, 69-86`). Our intent stack (§7.2) generalizes this.
- Menu selection = index state shared by mouse and keys; `select(i)` plays a hover cue (`screens/MainMenu.tsx:30-47`):
```ts
const [selected, setSelected] = useState(0);
const select = (i: number) => { if (i === selected) return; sfx("hover"); setSelected(i); };
useKeys((e) => {
  if (modalOpen()) return;
  if (e.key === "ArrowDown") select((selected + 1) % entries.length);
  else if (e.key === "ArrowUp") select((selected + entries.length - 1) % entries.length);
  else if (e.key === "Enter") onPick(entries[selected].id);
  else if (e.key === "Escape" && onBack) { sfx("back"); onBack(); }
}, true);   // allow OS auto-repeat
```

### 13.2 State flow hooks (`cyberpunk/ui/src/hooks.ts:13-29`; same in synth/civilization `hooks.ts`)
```ts
export function useEvent<K extends keyof ReactEvents>(name: K, run: (value: ReactEvents[K]) => void) {
  const latest = useRef(run);
  latest.current = run;                                        // always the latest render's closure
  useEffect(() => on(name, (value) => latest.current(value)), [name]);
}
export function useKeys(run: (e: KeyboardEventData) => void, repeat = false) {
  useEvent("keyDown", (e) => { if (e.repeat && !repeat) return; run(e); });
}
```
`useWindowSize()` (synth/civilization): `bevy.window.size().then(setSize, () => {})` + `return bevy.on("resize", setSize)`.

### 13.3 Settings wired to Bevy (`cyberpunk/settings.rs`, `ui/src/screens/settings/store.ts`)
```rust
#[react_message(name = "settings.video")]
pub struct VideoSettings { pub mode: u32 /* 0 windowed, 1 borderless, 2 fullscreen */, pub width: u32, pub height: u32, pub vsync: bool }
// handler writes Window fields only when they differ (settings.rs:151-186):
let mode = match ev.mode { 1 => WindowMode::BorderlessFullscreen(MonitorSelection::Current),
    2 => WindowMode::Fullscreen(MonitorSelection::Current, VideoModeSelection::Current), _ => WindowMode::Windowed };
if window.mode != mode { window.mode = mode; }
let present = if ev.vsync { PresentMode::AutoVsync } else { PresentMode::AutoNoVsync };
if window.present_mode != present { window.present_mode = present; }
```
- `settings.graphics` (fov, aberration, focus, flare, blur) inserts/removes camera components (`Bloom`, `DepthOfField`, `ChromaticAberration`, `MotionBlur`) and stashes removed ones in a `Stash` resource to restore exact tuning (`settings.rs:82-126`); `settings.gamma` writes `ColorGrading` sections (`:188-195`); `sound.volume` replaces a `Volume` resource that is **both the message and the resource** (`#[react_message(name = "sound.volume")] #[derive(Resource)] pub struct Volume { master, sfx, music }`, `sound/mod.rs:39-54`).
- TS store = `useSyncExternalStore` over a `globalThis` object (survives hot reload) (`settings/store.ts:14-39`), pushed to Bevy by a `usePush` effect that sends only when the JSON changes, and skips the video group on mount so it doesn't override the window `main.rs` opened (`store.ts:44-82`):
```ts
function usePush<T>(value: T, push: (value: T) => void, onMount = true) {
  const key = JSON.stringify(value);
  const sent = useRef(onMount ? "" : key);
  useEffect(() => { if (sent.current === key) return; sent.current = key; push(JSON.parse(key)); }, [key, push]);
}
```
- **No persistence** in the example (session only). We need a `settings.load` request + `settings.save` message backed by a Rust file (JS has no storage APIs).

### 13.4 Sound cues from React (`sound.play`, not `sound.cue`)
```ts
// cyberpunk/ui/src/sound.ts
export type Sfx = "hover" | "click" | "back" | "tab" | "error" | "confirm" | "boot";
export function sfx(name: Sfx) { bevy.sound.play({ name }); }
```
```rust
// cyberpunk/sound/mod.rs:39-54, 94-120
#[react_message(name = "sound.play")] pub struct Play { pub name: String }
const RETRIGGER: f64 = 0.04;   // drop restarts within 40 ms so a fast list sweep doesn't buzz
fn play(ev: On<Play>, mut bank: ResMut<Bank>, volume: Res<Volume>, time: Res<Time<Real>>,
        mut unknown: Local<HashSet<String>>, mut commands: Commands) {
    let Some((_, sound, last)) = bank.0.iter_mut().find(|(name, ..)| *name == ev.name) else {
        if unknown.insert(ev.name.clone()) { warn!("sound.play: no sound named {:?}", ev.name); }
        return;
    };
    let now = time.elapsed_secs_f64();
    if now - *last < RETRIGGER { return; }
    *last = now;
    commands.spawn((AudioPlayer(sound.clone()),
        PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(volume.master * volume.sfx))));
}
```
Cue map used there: `hover` on selection-index change (pointer enter or arrows), `click` on pick, `back` on every Esc/back, `tab` on tab switch/stepper, `confirm`, `error`, `boot`. For ReCharged prefer a typed cue so a typo is a compile error, not a runtime warning:
```rust
#[react_message(name = "ui.sound")]
pub struct PlayCue { pub cue: Cue }                     // TS: bevy.ui.sound({ cue: "hover" })

#[derive(serde::Deserialize, ts_rs::TS, Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum Cue { Hover, Click, Back, Tab, Confirm, Error, Countdown, Go }
```
(Keep cyberpunk's 40 ms retrigger guard; rumble for the same events goes through `bevy.gamepad.rumble`.)

### 13.5 Resync after remount/hot reload (`coldwake`)
Rust caches what it last told React and only sends changes; React asks for a resend on mount:
```tsx
useEffect(() => bevy.player.sync(null), []);              // coldwake/ui/src/App.tsx:46
```
```rust
#[react_message(name = "player.sync")] pub struct Sync;     // coldwake/player/mod.rs:62-63
fn on_sync(_: On<Sync>, mut told: ResMut<Told>) { told.0 = None; }   // :186-188 → next frame resends everything
```
Alternative to a snapshot request; good for change-gated HUD state.

### 13.6 Other reusable bits
- **Owning the pointer** for HUD panels: `export const OWNS_POINTER = {};` used as `hoverStyle={OWNS_POINTER}` (`civilization/ui/src/theme.ts:86-89`); world camera checks `PointerCapture` `.after(PointerCaptureSet)` and ignores the wheel `if capture.dragging || capture.wheel_captured || over_hud(..)` (`civilization/camera.rs:118-121, 207-278`).
- **Minimap portal** (`civilization/camera.rs:178-201`): orthographic `Camera3d` + `targets.create(&mut images, "minimap", RenderTargetSpec::default())` + `PortalCamera("minimap".into())` + `RenderLayers::layer(1)`; click-to-jump with `onPointerDown={(e) => bevy.map.jump({ u: e.x, v: e.y })}` (`e.x/e.y` are 0..1 in the element) (`hud/ActionPanel.tsx:32, 99-110`).
- **Live 3D cards** (`cyberpunk/dioramas/mod.rs:191-270`): one camera per card, each on its own `RenderLayers::layer(10 + i)`, `Hdr`, `Msaa::Off`, `Fxaa`, `PortalCamera(name)`; worlds placed 2000 units apart because "Bevy's GPU light clustering ignores render layers"; snapshot thumbnails re-`invalidate`d at 1, 3, 8, 20, 45 s because the first frame renders before pipelines compile/lights cluster.
- **Entrance animations without re-render**: `useEnter` (cyberpunk `hooks.ts:42-51`), `useSlideIn`/`useFadeIn` (civilization `hooks.ts:46-69`).
- **Gamepad menu with edge detection + hold-repeat + rumble**: `demos/ui/src/demos/events/gamepadDemo/{GamepadDemo.tsx:86-188, menuNav.ts, usePads.ts}` (REPEAT_DELAY_MS 350, REPEAT_MS 120, `axisDir` ±0.5, Y inverted).
- **Charts**: `<svg>` `polyline`/`polygon` with labels/tooltips as normal nodes on top so hover never re-rasterizes (`civilization/ui/src/screens/charts.tsx:147-181`). Compositing is in linear light → translucent fills look brighter than on the web; civilization pre-mixes opaque colors (`charts.tsx:157-160`).
- **1080p authoring**: `fit_ui_to_1080p` (`cyberpunk/main.rs:134-157`, §5).
