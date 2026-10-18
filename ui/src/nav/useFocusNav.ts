// A screen's focus: regions moved by `ui.nav` directions and set by pointer hover, with the confirm /
// back / tab / adjust actions routed to the screen. Plays the hover and back cues.
import { useState } from "react";

import type { NavEvent } from "../bevy";
import { sfx } from "../sound";
import { clampFocus, enter, isDir, move, type Focus, type FocusMap } from "./focus";
import { useNav } from "./input";

export type FocusHandlers = {
  confirm?: (focus: Focus) => void;
  back?: () => void;
  prevTab?: () => void;
  nextTab?: () => void;
  /** Left/right on the focused item; return true when it changed a value (no focus move). */
  adjust?: (focus: Focus, delta: -1 | 1) => boolean;
  /** Ignore events this returns false for (e.g. only P1 picks the Versus track). */
  accept?: (event: NavEvent) => boolean;
};

export function useFocusNav(map: FocusMap, initial: Focus, handlers: FocusHandlers, z?: number, enabled = true) {
  const [raw, setFocus] = useState(initial);
  const focus = clampFocus(map, raw, initial.region);
  const moveTo = (next: Focus) => {
    if (next.region === focus.region && next.index === focus.index) return;
    sfx("hover");
    setFocus(next);
  };
  useNav(
    (e) => {
      if (handlers.accept && !handlers.accept(e)) return;
      if (isDir(e.action)) {
        if ((e.action === "left" || e.action === "right") && handlers.adjust?.(focus, e.action === "left" ? -1 : 1)) return;
        moveTo(move(map, focus, e.action));
      } else if (e.action === "confirm") handlers.confirm?.(focus);
      else if (e.action === "back" && handlers.back) {
        sfx("back");
        handlers.back();
      } else if (e.action === "prevTab") handlers.prevTab?.();
      else if (e.action === "nextTab") handlers.nextTab?.();
    },
    z,
    enabled,
  );
  /** Pointer hover focuses an item. */
  const hover = (region: string, index: number) => moveTo(enter(focus, region, map[region]?.count ?? 1, index));
  const is = (region: string, index: number) => focus.region === region && focus.index === index;
  return { focus, setFocus, hover, is };
}
