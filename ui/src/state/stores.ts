// The app's stores (hmr-safe singletons). `Feeds.tsx` seeds each with a snapshot request on mount
// and keeps it current from Bevy events; screens only read them.
import type {
  AppInfo,
  AppStatus,
  UiBindings,
  UiCatalog,
  UiDeviceKind,
  UiLobby,
  UiProfile,
  UiRaceSession,
  UiRecords,
  UiSettings,
} from "../bevy";
import { createStore, hmrSingleton, sameJson } from "./store.ts";

const store = <T>(key: string, initial: T) =>
  hmrSingleton(`__rc_${key}`, () => createStore<T>(initial, sameJson));

export const appState = store<AppStatus | null>("appState", null);
export const appInfo = store<AppInfo | null>("appInfo", null);
export const catalog = store<UiCatalog | null>("catalog", null);
export const profile = store<UiProfile | null>("profile", null);
export const records = store<UiRecords | null>("records", null);
/** Settings as last applied from the UI (rc_game clamps and saves them). */
export const settings = store<UiSettings | null>("settings", null);
export const bindings = store<UiBindings | null>("bindings", null);
export const lobby = store<UiLobby | null>("lobby", null);
export const session = store<UiRaceSession>("session", { setup: null, results: null, cup: null, versus: null, hud: null });
export const loadingProgress = store<number>("loadingProgress", 0);
/** Glyph device per player slot (the last device that player used). */
export const devices = store<UiDeviceKind[]>("devices", ["keyboardA", "keyboardA", "keyboardA", "keyboardA"]);
/** Scripted demo scenes (debug.ts) own the session data: snapshots must not overwrite them. */
export const demoMode = store<boolean>("demoMode", false);
