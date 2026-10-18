import assert from "node:assert/strict";
import { test } from "node:test";

import { closeDialog, dialogs, dismissToast, MAX_TOASTS, openDialog, pushToast, toasts } from "./overlays.ts";
import { createStore, sameJson } from "./store.ts";

test("stores notify on change only and keep snapshot identity", () => {
  const store = createStore({ a: 1 }, sameJson);
  let calls = 0;
  const off = store.subscribe(() => calls++);
  const first = store.get();
  store.set({ a: 1 });
  assert.equal(calls, 0);
  assert.equal(store.get(), first);
  store.update((s) => ({ a: s.a + 1 }));
  assert.equal(calls, 1);
  off();
  store.set({ a: 5 });
  assert.equal(calls, 1, "unsubscribed");
});

test("dialogs stack and close by id", () => {
  const a = openDialog({ title: "Quit?", actions: [{ label: "Quit" }, { label: "Cancel" }], cancel: 1 });
  const b = openDialog({ title: "Really?", actions: [{ label: "Yes" }] });
  assert.deepEqual(
    dialogs.get().map((d) => d.title),
    ["Quit?", "Really?"],
  );
  closeDialog(a);
  assert.deepEqual(
    dialogs.get().map((d) => d.id),
    [b],
  );
  closeDialog(b);
  assert.equal(dialogs.get().length, 0);
});

test("toasts keep only the newest few", () => {
  const ids = Array.from({ length: MAX_TOASTS + 2 }, (_, i) => pushToast({ tone: "info", title: `t${i}` }));
  assert.equal(toasts.get().length, MAX_TOASTS);
  assert.equal(toasts.get()[0].title, "t2");
  dismissToast(ids[ids.length - 1]);
  assert.equal(toasts.get().length, MAX_TOASTS - 1);
});
