import assert from "node:assert/strict";
import { test } from "node:test";

import { createAppStateStore, screenFor } from "./appState.ts";

test("store keeps the latest status and notifies only on change", () => {
  const store = createAppStateStore();
  let calls = 0;
  const unsubscribe = store.subscribe(() => calls++);
  assert.equal(store.get(), null);

  store.set({ state: "menu", phase: null, paused: false });
  const menu = store.get();
  store.set({ state: "menu", phase: null, paused: false });
  assert.equal(calls, 1, "an identical status does not notify");
  assert.equal(store.get(), menu, "and keeps the snapshot identity");

  store.set({ state: "inRace", phase: "countdown", paused: false });
  store.set({ state: "inRace", phase: "countdown", paused: true });
  assert.equal(calls, 3);
  assert.deepEqual(store.get(), { state: "inRace", phase: "countdown", paused: true });

  unsubscribe();
  store.set({ state: "menu", phase: null, paused: false });
  assert.equal(calls, 3, "unsubscribed listeners are not called");
});

test("every app state maps to a screen", () => {
  assert.equal(screenFor(null), "none");
  assert.equal(screenFor({ state: "boot", phase: null, paused: false }), "none");
  assert.equal(screenFor({ state: "menu", phase: null, paused: false }), "menu");
  assert.equal(screenFor({ state: "loading", phase: null, paused: false }), "loading");
  assert.equal(screenFor({ state: "unloading", phase: null, paused: false }), "loading");
  assert.equal(screenFor({ state: "inRace", phase: "racing", paused: false }), "race");
});
