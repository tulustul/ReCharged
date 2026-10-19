// HUD state: setup, the 10 Hz tick and views from Bevy (snapshot on mount, then events), plus the
// short-lived moments race events create — countdown, notices, banners, splits, lap flashes and
// hints — kept per player slot and cleared by timers. Hmr-safe singletons.
import { bevy, on, type UiHint, type UiHudSetup, type UiHudTick, type UiItemKind, type UiRaceEvent, type UiViewports } from "../bevy";
import { createStore, hmrSingleton, sameJson } from "../state/store";
import { demoMode } from "../state/stores";
import { hud as words, itemName } from "../strings/hud";

const store = <T>(key: string, initial: T) => hmrSingleton(`__rc_hud_${key}`, () => createStore<T>(initial, sameJson));

export const hudSetup = store<UiHudSetup | null>("setup", null);
/** The latest real setup, also while a demo scene shows its own (demos build on it). */
export const liveSetup = store<UiHudSetup | null>("liveSetup", null);
export const hudTick = store<UiHudTick | null>("tick", null);
export const hudViews = store<UiViewports>("views", { players: [], panel: null });

export type Notice = { id: number; slot: number; tone: "good" | "bad" | "info" | "gold"; text: string; item?: UiItemKind };
export type Banner = { kind: "finalLap"; id: number } | { kind: "finished"; id: number; rank: number; ms: number };
export type Moments = {
  banner: Banner | null;
  split: { id: number; secs: number } | null;
  lap: { id: number; lap: number; ms: number; best: boolean } | null;
  hint: { id: number; hint: UiHint } | null;
};

/** The countdown number showing (0 = GO!), for every view. */
export const countdown = store<{ id: number; n: number } | null>("countdown", null);
export const notices = store<Notice[]>("notices", []);
export const moments = store<Record<number, Moments>>("moments", {});

const EMPTY: Moments = { banner: null, split: null, lap: null, hint: null };
const MAX_NOTICES = 3;
const ms = { go: 1000, notice: 3200, finalLap: 2600, split: 3000, lap: 3500, hint: 5000 } as const;

let nextId = 1;

/** Sets a slot's moment; a timer clears it again unless a newer one replaced it. */
function moment<K extends keyof Moments>(slot: number, key: K, value: NonNullable<Moments[K]>, lifeMs?: number) {
  moments.update((all) => ({ ...all, [slot]: { ...EMPTY, ...all[slot], [key]: value } }));
  // Demo scenes hold still for screenshots.
  if (lifeMs === undefined || demoMode.get()) return;
  setTimeout(() => {
    moments.update((all) => (all[slot]?.[key] === value ? { ...all, [slot]: { ...all[slot], [key]: null } } : all));
  }, lifeMs);
}

export function notify(slot: number, tone: Notice["tone"], text: string, item?: UiItemKind) {
  const notice: Notice = { id: nextId++, slot, tone, text, item };
  notices.update((list) => [...list.filter((n) => n.slot !== slot), ...list.filter((n) => n.slot === slot).slice(1 - MAX_NOTICES), notice]);
  if (!demoMode.get()) setTimeout(() => notices.update((list) => list.filter((n) => n.id !== notice.id)), ms.notice);
}

/** Forgets every moment (new attempt, leaving the race). */
export function resetMoments() {
  countdown.set(null);
  notices.set([]);
  moments.set({});
}

export function onRaceEvent(e: UiRaceEvent) {
  const id = nextId++;
  switch (e.kind) {
    case "countdown":
      return countdown.set({ id, n: e.n });
    case "go": {
      const go = { id, n: 0 };
      countdown.set(go);
      setTimeout(() => countdown.get() === go && countdown.set(null), ms.go);
      return;
    }
    case "perfectStart":
      return notify(e.slot, "good", words.notice.perfectStart);
    case "stall":
      return notify(e.slot, "bad", words.notice.stall);
    case "split":
      return moment(e.slot, "split", { id, secs: e.secs }, ms.split);
    case "lap":
      moment(e.slot, "lap", { id, lap: e.lap, ms: e.ms, best: e.best }, ms.lap);
      if (e.fastest) notify(e.slot, "gold", words.notice.fastestLap);
      return;
    case "finalLap":
      return moment(e.slot, "banner", { kind: "finalLap", id }, ms.finalLap);
    case "finished":
      return moment(e.slot, "banner", { kind: "finished", id, rank: e.rank, ms: e.ms });
    case "goldenBattery":
      return notify(e.slot, "gold", words.notice.goldenBattery);
    case "missedCheckpoint":
      return notify(e.slot, "bad", words.notice.missedCheckpoint);
    case "hint":
      return moment(e.slot, "hint", { id, hint: e.hint }, ms.hint);
    case "hit":
      return notify(e.slot, e.blocked ? "info" : "bad", e.blocked ? words.notice.blocked(itemName[e.by]) : words.notice.hitBy(itemName[e.by], e.attacker), e.by);
    case "landed":
      return notify(e.slot, e.blocked ? "info" : "good", e.blocked ? words.notice.landedBlocked(e.victim) : words.notice.landed(itemName[e.by], e.victim), e.by);
    case "endsIn":
      return; // the tick's `endsIn` drives the countdown text
  }
}

const fail = (error: unknown) => console.error("race.snapshot failed:", error);

/** Follows Bevy while the HUD is mounted (snapshot first: events are not buffered). Demo scenes own
 * setup, tick and moments while `demoMode` is on; views stay real. */
export function startHudFeed(): () => void {
  const live = () => !demoMode.get();
  const offs = [
    on("race.setup", (s) => {
      liveSetup.set(s);
      if (live()) hudSetup.set(s);
    }),
    on("race.hud", (t) => live() && hudTick.set(t)),
    on("race.event", (e) => live() && onRaceEvent(e)),
    on("viewports", (v) => hudViews.set(v)),
  ];
  bevy.race.snapshot().then((s) => {
    if (!s.hud) return;
    hudViews.set(s.hud.viewports);
    liveSetup.set(s.hud.setup);
    if (!live()) return;
    hudSetup.set(s.hud.setup);
    hudTick.set(s.hud.tick);
  }, fail);
  return () => offs.forEach((off) => off());
}
