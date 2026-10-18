// Runtime side of navigation: `ui.nav` events from Bevy go to the top-most nav layer. Screens,
// overlays and dialogs register layers with `useNav`; pointer events set focus directly.
import { useEffect, useRef } from "react";

import type { NavEvent, UiDeviceKind } from "../bevy";
import { devices } from "../state/stores";
import { hmrSingleton } from "../state/store";
import { useStore } from "../state/useStore";
import { layer } from "../theme/tokens";
import { createNavStack } from "./stack";

const stack = hmrSingleton("__rc_navStack", () => createNavStack<NavEvent>());

/** Routes one `ui.nav` event (Feeds subscribes it); also tracks the slot's glyph device. */
export function dispatchNav(event: NavEvent) {
  if (event.slot !== null && event.device !== "mouse") setDevice(event.slot, event.device);
  stack.dispatch(event);
}

export function setDevice(slot: number, device: UiDeviceKind) {
  devices.update((list) => (list[slot] === device ? list : list.map((d, i) => (i === slot ? device : d))));
}

/** Receives `ui.nav` while mounted and top-most (`z`: screen < overlay < dialog). */
export function useNav(handler: (event: NavEvent) => void, z: number = layer.screen, enabled = true) {
  const ref = useRef(handler);
  ref.current = handler;
  useEffect(() => (enabled ? stack.add(z, ref) : undefined), [z, enabled]);
}

/** The glyph device of a player slot. */
export function useDevice(slot = 0): UiDeviceKind {
  return useStore(devices)[slot] ?? "keyboardA";
}
