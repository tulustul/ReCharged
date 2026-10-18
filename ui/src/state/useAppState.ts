// Wires the app-state store to Bevy: snapshot on mount (events are not buffered), then `app.state`.
import { useEffect, useSyncExternalStore } from "react";

import { bevy, on } from "../bevy";
import { createAppStateStore, type AppStateStore } from "./appState";

// hmr-safe singleton: hot reload re-runs module code, the store must survive it.
const g = globalThis as { __rcAppState?: AppStateStore };
export const appState: AppStateStore = (g.__rcAppState ??= createAppStateStore());

export function useAppState() {
  useEffect(() => {
    const off = on("app.state", (status) => appState.set(status));
    bevy.app.snapshot().then(
      (status) => appState.set(status),
      (error) => console.error("app.snapshot failed", error),
    );
    return off;
  }, []);
  return useSyncExternalStore(appState.subscribe, appState.get);
}
