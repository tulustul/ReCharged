// Menu screens and their stack, plus screen transitions: plain fades everywhere, the charge morph
// only from the main menu into a mode (GDD §14). The morph is armed only while a change is in
// flight (cyberpunk's pattern): promote the screen container under the current screen, swap the
// screen a few frames later, demote it once the morph has run.
import type { BevyStyle } from "bevy-react";

import { createStore, hmrSingleton, type Store } from "../state/store";
import { ms } from "../theme/motion";
import { pop, popTo, push } from "./router";

export type Route =
  | "splash"
  | "title"
  | "main"
  | "cupSelect"
  | "cupEnd"
  | "trackSelect"
  | "carSelect"
  | "lobby"
  | "garage"
  | "records"
  | "howToPlay"
  | "settings"
  | "credits";

export type MorphKind = "fade" | "charge";
export type Morph = { key: number; kind: MorphKind } | null;

/** Delay between arming a morph (capturing the old screen) and swapping the content. */
const ARM_MS = 60;

export type Morpher = { morph: Store<Morph>; run: (apply: () => void, kind?: MorphKind) => void };

export function createMorpher(): Morpher {
  const morph = createStore<Morph>(null);
  let seq = 0;
  let flipTimer: ReturnType<typeof setTimeout> | undefined;
  let endTimer: ReturnType<typeof setTimeout> | undefined;
  const duration = (kind: MorphKind) => (kind === "charge" ? ms.charge : ms.fade);
  return {
    morph,
    run(apply, kind = "fade") {
      clearTimeout(endTimer);
      const flip = () => {
        apply();
        morph.set({ key: ++seq, kind });
        endTimer = setTimeout(() => morph.set(null), duration(kind) + 120);
      };
      if (morph.get()) {
        clearTimeout(flipTimer);
        flip();
      } else {
        morph.set({ key: ++seq, kind });
        flipTimer = setTimeout(flip, ARM_MS);
      }
    },
  };
}

/** Style of a container whose content changes through `morph`. */
export function morphStyle(morph: Morph): BevyStyle {
  if (!morph) return {};
  return {
    // Whatever is live underneath keeps moving mid-change.
    cache: "never",
    morphFilter:
      morph.kind === "charge"
        ? { key: morph.key, name: "chargeSwap" }
        : { key: morph.key, name: "crossfade", params: { spread: 0.25, softness: 0.6 } },
    transition: { morphFilter: { duration: morph.kind === "charge" ? ms.charge : ms.fade, easing: "easeInOut" } },
  };
}

export const router = hmrSingleton("__rc_router", () => createStore<Route[]>(["splash"]));
export const menuMorph = hmrSingleton("__rc_menuMorph", createMorpher);

export const go = (route: Route, kind: MorphKind = "fade") => menuMorph.run(() => router.update((s) => push(s, route)), kind);
export const back = () => menuMorph.run(() => router.update(pop));
export const backTo = (route: Route) => menuMorph.run(() => router.update((s) => popTo(s, (r) => r === route)));
export const resetTo = (...stack: Route[]) => menuMorph.run(() => router.set(stack));
