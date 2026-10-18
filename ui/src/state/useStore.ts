import { useSyncExternalStore } from "react";

import type { Store } from "./store";

/** Re-renders when the store changes. */
export function useStore<T>(store: Store<T>): T {
  return useSyncExternalStore(store.subscribe, store.get);
}
