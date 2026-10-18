// Per-frame HUD visuals as shared values (zero bridge traffic per frame): React creates them once,
// sends their ids with `hud.bind`, and rc_ui writes them every frame — speed, charge, charging glow
// and the rocket arrow per player, a minimap dot per racer. Demo scenes write them from JS instead.
import { useEffect } from "react";
import { useSharedValue, type SharedValue } from "bevy-react";

import { bevy } from "../bevy";
import { createStore, hmrSingleton } from "../state/store";

export const MAX_PLAYERS = 4;
export const MAX_RACERS = 8;
/** Speed (u/s) that fills the speed bar: the fastest car's top speed × the 1.5 boost cap. */
export const GAUGE_MAX = 54;

export type PlayerValues = { speed: SharedValue; charge: SharedValue; glow: SharedValue; arrow: SharedValue };
export type Dot = { x: SharedValue; y: SharedValue };
export type HudValues = { players: PlayerValues[]; dots: Dot[] };

/** Values a demo scene shows (Rust does not drive them then). */
export type DemoValues = { players: { speed: number; charge: number; glow: number; arrow: number }[]; dots: [number, number][] };
export const demoValues = hmrSingleton("__rc_hud_demoValues", () => createStore<DemoValues | null>(null));

// The loops below always run the same number of times, so the hook order is stable.
function usePlayers(): PlayerValues[] {
  const players: PlayerValues[] = [];
  for (let i = 0; i < MAX_PLAYERS; i++) {
    players.push({ speed: useSharedValue(0), charge: useSharedValue(0), glow: useSharedValue(0), arrow: useSharedValue(0) });
  }
  return players;
}

function useDots(): Dot[] {
  const dots: Dot[] = [];
  for (let i = 0; i < MAX_RACERS; i++) dots.push({ x: useSharedValue(-1), y: useSharedValue(-1) });
  return dots;
}

/** The HUD's shared values, bound to Rust while mounted (or set from `demo` when given). */
export function useHudValues(demo: DemoValues | null): HudValues {
  const players = usePlayers();
  const dots = useDots();
  useEffect(() => {
    if (demo) {
      demo.players.forEach((p, i) => {
        const v = players[i];
        if (!v) return;
        v.speed.value = p.speed;
        v.charge.value = p.charge;
        v.glow.value = p.glow;
        v.arrow.value = p.arrow;
      });
      demo.dots.forEach(([x, y], i) => {
        if (!dots[i]) return;
        dots[i].x.value = x;
        dots[i].y.value = y;
      });
      return;
    }
    bevy.hud.bind({
      players: players.map((p, slot) => ({ slot, speed: p.speed.id, charge: p.charge.id, glow: p.glow.id, arrow: p.arrow.id })),
      dots: dots.map((d) => ({ x: d.x.id, y: d.y.id })),
    });
    return () => bevy.hud.unbind(null);
    // The values are stable for the component's life.
  }, [demo]);
  return { players, dots };
}
