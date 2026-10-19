// Shared hooks: Bevy events, scripted debug steps, entrance motion, the split-screen context.
import { createContext, useContext, useEffect, useRef } from "react";
import { interpolate, useSharedValue, withDelay, withSpring, type BevyStyle } from "bevy-react";

import { bevy, on, type ReactEvents, type UiUnlockedItem } from "./bevy";
import { draft } from "./state/draft";
import { hmrSingleton } from "./state/store";
import { spring } from "./theme/motion";

/** Listens to a Bevy event while mounted; `run` always sees the latest render. */
export function useEvent<K extends keyof ReactEvents>(name: K, run: (value: ReactEvents[K]) => void) {
  const latest = useRef(run);
  latest.current = run;
  useEffect(() => on(name, (value) => latest.current(value)), [name]);
}

/** A scripted step (`RECHARGED_UI_SCRIPT` → `debug.act`) for this verb: `run(rest of the line)`. */
export function useDebug(verb: string, run: (arg: string) => void) {
  useEvent("debug.act", ({ action }) => {
    const [head, ...rest] = action.split(" ");
    if (head === verb) run(rest.join(" "));
  });
}

/** Slides in from (`x`, `y`) px after `delay` ms on mount (transform only: no layer). */
export function useEnter(x = 0, y = 24, delay = 0): BevyStyle {
  const t = useSharedValue(0);
  useEffect(() => {
    t.value = withDelay(delay, withSpring(1, spring.soft));
  }, [t, delay]);
  return {
    transform: {
      translateX: { animated: interpolate(t, [0, 1], [x, 0]) },
      translateY: { animated: interpolate(t, [0, 1], [y, 0]) },
    },
  };
}

/** Clears the NEW badge of `item` once it is focused (rc_game `MarkSeen`). */
export function useSeen(item: UiUnlockedItem | null, fresh: boolean) {
  const key = item ? JSON.stringify(item) : "";
  useEffect(() => {
    if (item && fresh) bevy.profile.seen({ item });
    // `key` identifies the item; the object itself changes identity every render.
  }, [key, fresh]);
}

/** Hides the 3D showroom car while mounted (screens that cover the turntable: Garage grid, Credits
 * roll, Cup-end podium); the wide shot of the player's car returns afterwards. */
export function useRoomBackdrop() {
  useEffect(() => {
    const backdrop = (focus: "room" | "wide") => {
      const d = draft.get();
      bevy.menu.backdrop({ car: d.car, paint: d.paint, focus });
    };
    backdrop("room");
    return () => backdrop("wide");
  }, []);
}

/** True inside split-screen race overlays: panels use translucent fills instead of glass. */
export const SplitContext = hmrSingleton("__rc_splitContext", () => createContext(false));
export const useSplit = () => useContext(SplitContext);
