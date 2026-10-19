// Runtime side of navigation: `ui.nav` events from Bevy go to the top-most nav layer. Screens,
// overlays and dialogs register layers with `useNav`; pointer motion sets focus directly. The
// shown screen is reported to Bevy (`ui.screen`) and every nav is debug-logged with the screen
// that got it (`RUST_LOG=bevy_react_core::js=debug`).
import { useEffect, useRef } from "react";

import { bevy, type NavEvent, type UiDeviceKind } from "../bevy";
import { devices } from "../state/stores";
import { hmrSingleton } from "../state/store";
import { useStore } from "../state/useStore";
import { layer } from "../theme/tokens";
import { createNavStack } from "./stack";

const stack = hmrSingleton("__rc_navStack", () => createNavStack<NavEvent>());
const screen = hmrSingleton("__rc_screen", () => ({ name: "" }));

/** Routes one `ui.nav` event (Feeds subscribes it); also tracks the slot's glyph device. */
export function dispatchNav(event: NavEvent) {
  if (event.slot !== null && event.device !== "mouse") setDevice(event.slot, event.device);
  const handled = stack.dispatch(event);
  const who = `${event.device}${event.slot === null ? "" : ` P${event.slot + 1}`}${event.repeat ? " (repeat)" : ""}`;
  console.debug(`ui.nav ${event.action} from ${who} → ${screen.name}${handled ? "" : ": no handler"}`);
}

/** The screen last reported (for logs). */
export const screenName = () => screen.name;

/** Reports the shown screen to Bevy (`ui.screen`: logged, and what `--input-script`'s
 * `wait screen <name>` waits for). Renders nothing. */
export function ScreenName({ name }: { name: string }) {
  useEffect(() => {
    if (screen.name === name) return;
    screen.name = name;
    bevy.ui.screen({ screen: name });
  }, [name]);
  return null;
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
