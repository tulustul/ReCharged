// App-state store: the latest `app.state` from Bevy. Pure (no bevy-react import), so `node --test`
// can run it; the wiring to Bevy lives in useAppState.ts.
import type { AppStatus } from "../bevy";

export type AppStateStore = {
  get(): AppStatus | null;
  set(next: AppStatus): void;
  subscribe(listener: () => void): () => void;
};

export function createAppStateStore(): AppStateStore {
  let current: AppStatus | null = null;
  const listeners = new Set<() => void>();
  return {
    get: () => current,
    set(next) {
      if (current && current.state === next.state && current.phase === next.phase && current.paused === next.paused) {
        return; // unchanged: keep the snapshot identity so React does not re-render
      }
      current = next;
      listeners.forEach((listener) => listener());
    },
    subscribe(listener) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
  };
}

/** The top-level screen for an app status. */
export type Screen = "none" | "menu" | "loading" | "race";

export function screenFor(status: AppStatus | null): Screen {
  switch (status?.state) {
    case "menu":
      return "menu";
    case "loading":
    case "unloading":
      return "loading";
    case "inRace":
      return "race";
    default:
      return "none";
  }
}
