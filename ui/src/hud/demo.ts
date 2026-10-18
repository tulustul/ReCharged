// HUD demo scenes for screenshots (`RECHARGED_UI_SCRIPT`: `demo hud [1-4] [racing|warnings|roulette|
// finish|tt|countdown]`). Over a real race the real views, racers and outline stay; without one the
// scene brings its own. While `demoMode` is on the feed leaves these stores alone.
import type { UiHudSetup, UiHudTick, UiPlayerHud, UiRacer, UiRaceEvent } from "../bevy";
import { slotColor } from "../theme/tokens";
import { countdown, hudSetup, hudTick, hudViews, liveSetup, onRaceEvent, resetMoments } from "./store";
import { demoValues, MAX_PLAYERS, type DemoValues } from "./values";

export type HudScene = "racing" | "warnings" | "roulette" | "finish" | "tt" | "countdown";
const SCENES: HudScene[] = ["racing", "warnings", "roulette", "finish", "tt", "countdown"];

const RIVALS = ["Mayhem", "Dash", "Penny", "Turbo Tom", "Glitch", "Rusty", "Nova"];
const RIVAL_COLORS: [number, number, number][] = [
  [1, 0.45, 0.1],
  [0.9, 0.12, 0.16],
  [0.6, 0.3, 1],
  [0.15, 0.4, 1],
  [0.72, 0.39, 0.18],
  [0.91, 0.92, 0.95],
  [1, 0.45, 0.6],
];

const hexRgb = (hex: string): [number, number, number] => [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255) as [number, number, number];

/** A rounded-rectangle loop in the minimap square, for scenes without a real track. */
function sampleOutline(): number[] {
  const points: number[] = [];
  for (let i = 0; i < 96; i++) {
    const a = (i / 96) * Math.PI * 2;
    const c = Math.cos(a);
    const s = Math.sin(a);
    // Superellipse (n = 4) with a dent: reads as a track, not a circle.
    const x = 0.5 + 0.42 * Math.sign(c) * Math.abs(c) ** 0.5;
    const y = 0.5 + 0.3 * Math.sign(s) * Math.abs(s) ** 0.5 - (Math.abs(a - Math.PI / 2) < 0.6 ? 0.12 * Math.cos((a - Math.PI / 2) * 2.6) : 0);
    points.push(Math.round(x * 1e4) / 1e4, Math.round(y * 1e4) / 1e4);
  }
  return points;
}

function sampleRacers(slots: number[]): UiRacer[] {
  const humans = slots.map((slot) => ({ name: `P${slot + 1}`, car: "zip", slot, color: hexRgb(slotColor[slot] ?? slotColor[0]) }));
  const rivals = RIVALS.slice(0, 8 - humans.length).map((name, i) => ({ name, car: "sparky", slot: null, color: RIVAL_COLORS[i] }));
  return [...humans, ...rivals];
}

function player(slot: number, racer: number, i: number, scene: HudScene): UiPlayerHud {
  const base: UiPlayerHud = {
    slot,
    racer,
    position: [2, 4, 1, 6][i] ?? 3,
    lap: 2,
    lapMs: [21_340, 19_870, 23_110, 17_420][i] ?? 20_000,
    lastLapMs: 36_420,
    bestLapMs: [35_810, 36_420, 35_120, 37_900][i] ?? 36_000,
    finishMs: null,
    kmh: [63, 51, 70, 22][i] ?? 50,
    charge: [0.6, 0.1, 1, 0.3][i] ?? 0.5,
    boostReady: i !== 1,
    spending: i === 2,
    item: (["sparkRocket", "turboCell", null, "shieldBubble"] as const)[i] ?? null,
    roulette: i === 2,
    wrongWay: false,
    lockOn: false,
    goo: null,
    shield: null,
  };
  switch (scene) {
    case "warnings":
      return { ...base, lockOn: i === 0, goo: i === 1 ? 2.4 : null, shield: i === 2 || i === 0 ? 6.2 : null, wrongWay: i === 3, roulette: false, item: base.item ?? "gooBomb" };
    case "roulette":
      return { ...base, item: null, roulette: true, lap: 3 };
    case "finish":
      return i < 2 ? { ...base, position: i === 0 ? 1 : 3, lap: 3, lapMs: 0, finishMs: [112_430, 116_210][i] ?? 0, item: null, roulette: false } : { ...base, lap: 3 };
    case "tt":
      return { ...base, position: 1, item: null, roulette: false, boostReady: true, spending: false };
    case "countdown":
      return { ...base, position: [5, 6, 7, 8][i] ?? 8, lap: 1, lapMs: 0, lastLapMs: null, bestLapMs: null, kmh: 0, charge: 0, boostReady: false, spending: false, item: null, roulette: false };
    default:
      return base;
  }
}

function events(slots: number[], scene: HudScene): UiRaceEvent[] {
  const [a = 0, b = 1, c = 2, d = 3] = slots;
  switch (scene) {
    case "racing":
      return [
        { kind: "landed", slot: a, by: "sparkRocket", victim: "Mayhem", blocked: false },
        { kind: "lap", slot: a, lap: 1, ms: 35_810, best: false, fastest: false },
        { kind: "hint", slot: b, hint: "driftCharge" },
        { kind: "lap", slot: c, lap: 2, ms: 35_120, best: true, fastest: true },
        { kind: "hit", slot: d, by: "oilSlick", attacker: "Dash", blocked: false },
      ];
    case "warnings":
      return [
        { kind: "hit", slot: b, by: "shockOrb", attacker: "Dash", blocked: false },
        { kind: "hit", slot: c, by: "sparkRocket", attacker: "Penny", blocked: true },
      ];
    case "roulette":
      return [
        { kind: "finalLap", slot: a },
        { kind: "hint", slot: b, hint: "itemReady" },
        { kind: "goldenBattery", slot: c },
      ];
    case "finish":
      return [
        { kind: "finished", slot: a, rank: 1, ms: 112_430 },
        { kind: "finished", slot: b, rank: 3, ms: 116_210 },
      ];
    case "tt":
      return [
        { kind: "split", slot: a, secs: -0.42 },
        { kind: "lap", slot: a, lap: 1, ms: 35_120, best: true, fastest: false },
      ];
    case "countdown":
      return [{ kind: "countdown", n: 2 }];
  }
}

/** Fills the HUD stores with `arg`'s scene; returns the race phase the scene shows. */
export function demoHud(arg: string): "racing" | "countdown" {
  const words = arg.split(" ").filter(Boolean);
  const count = Number(words.find((w) => /^[1-4]$/.test(w)) ?? "1");
  const scene = SCENES.find((s) => words.includes(s)) ?? "racing";
  const real = hudViews.get().players.map((v) => v.slot);
  const slots = scene === "tt" ? [real[0] ?? 0] : real.length > 0 ? real : Array.from({ length: count }, (_, i) => i);
  const base = liveSetup.get();
  const racers = base && base.racers.filter((r) => r.slot !== null).length >= slots.length ? base.racers : sampleRacers(slots);
  const setup: UiHudSetup = {
    mode: scene === "tt" ? { kind: "timeTrial" } : slots.length > 1 ? { kind: "versus" } : { kind: "quickRace" },
    laps: 3,
    items: scene !== "tt",
    ghost: scene === "tt",
    racers: scene === "tt" ? racers.filter((r) => r.slot !== null).slice(0, 1) : racers,
    outline: base && base.outline.length >= 4 ? base.outline : sampleOutline(),
    speedUnit: base?.speedUnit ?? "kmh",
    hudScale: base?.hudScale ?? 1,
    minimap: true,
    reduceFlashing: base?.reduceFlashing ?? false,
  };
  const index = (slot: number) => Math.max(0, setup.racers.findIndex((r) => r.slot === slot));
  const tick: UiHudTick = {
    raceMs: scene === "countdown" ? 0 : 57_160,
    countdown: scene === "countdown" ? 2 : 0,
    endsIn: scene === "finish" ? 8 : null,
    standings: setup.racers.map((_, i) => i).sort((x, y) => ((x * 5) % 8) - ((y * 5) % 8)),
    players: slots.map((slot, i) => player(slot, index(slot), i, scene)),
  };
  resetMoments();
  hudSetup.set(setup);
  hudTick.set(tick);
  events(slots, scene).forEach(onRaceEvent);
  if (scene === "countdown") countdown.set({ id: -2, n: 2 });
  // Racers spread along the outline; the arrow points behind-right (a rocket closing in).
  const pts = setup.outline;
  const n = pts.length / 2;
  const values: DemoValues = {
    players: Array.from({ length: MAX_PLAYERS }, (_, i) => ({ speed: [35, 28, 39, 12][i] ?? 30, charge: tick.players[i]?.charge ?? 0.5, glow: [0.7, 0, 0.3, 0][i] ?? 0, arrow: 140 })),
    dots: setup.racers.map((_, i) => {
      const at = Math.floor(((0.12 + i * 0.075) % 1) * n) * 2;
      return [pts[at] ?? 0.5, pts[at + 1] ?? 0.5];
    }),
  };
  demoValues.set(values);
  return scene === "countdown" ? "countdown" : "racing";
}

/** Leaves the demo: Rust drives the shared values again (the next snapshot/tick restores data). */
export function endHudDemo() {
  demoValues.set(null);
}
