// A minimal external store (pure, no React): stores live on `globalThis` so hot reload — which
// re-runs every module — keeps their state.

export type Store<T> = {
  get(): T;
  set(next: T): void;
  update(fn: (current: T) => T): void;
  subscribe(listener: () => void): () => void;
};

export function createStore<T>(initial: T, equal: (a: T, b: T) => boolean = Object.is): Store<T> {
  let current = initial;
  const listeners = new Set<() => void>();
  const store: Store<T> = {
    get: () => current,
    set(next) {
      if (equal(current, next)) return; // keep the snapshot identity: no re-render
      current = next;
      listeners.forEach((listener) => listener());
    },
    update: (fn) => store.set(fn(current)),
    subscribe(listener) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
  };
  return store;
}

/** One instance per `key` for the lifetime of the JS isolate (survives hot reloads). */
export function hmrSingleton<T>(key: string, init: () => T): T {
  const g = globalThis as unknown as Record<string, T | undefined>;
  return (g[key] ??= init());
}

/** Structural equality for plain JSON data (snapshots from Bevy). */
export function sameJson<T>(a: T, b: T): boolean {
  return a === b || JSON.stringify(a) === JSON.stringify(b);
}
