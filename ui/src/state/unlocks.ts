// Unlock queries over the profile snapshot (pure). Rust decides what is unlocked; React only asks.
import type { UiCatalog, UiPaint, UiProfile, UiUnlockedItem, UiUnlockRule } from "../bevy";
import { appStrings } from "../strings/app.ts";
import { difficultyName, paintSetName } from "../strings/common.ts";
import { lockReason } from "../strings/menu.ts";

export const carUnlocked = (p: UiProfile | null, id: string) => !!p?.unlocked.cars.includes(id);
export const trackUnlocked = (p: UiProfile | null, id: string) => !!p?.unlocked.tracks.includes(id);
export const cupUnlocked = (p: UiProfile | null, id: string) => !!p?.unlocked.cups.includes(id);
export const paintUnlocked = (p: UiProfile | null, paint: UiPaint) =>
  paint.set === "base" || !!p?.unlocked.paintSets.includes(paint.set);

/** The player-facing reason something is locked. */
export function lockText(rule: UiUnlockRule, catalog: UiCatalog | null): string {
  const cupName = (id: string) => catalog?.cups.find((c) => c.id === id)?.name ?? id;
  switch (rule.kind) {
    case "start":
      return lockReason.start;
    case "cupPodium":
      return lockReason.cupPodium(cupName(rule.cup));
    case "cupGold":
      return lockReason.cupGold(cupName(rule.cup));
    case "goldMedals":
      return lockReason.goldMedals(rule.count);
    case "goldenBatteries":
      return lockReason.goldenBatteries(rule.count);
  }
}

/** Whether `item` is still marked NEW (unlocked but not yet viewed). */
export function isFresh(p: UiProfile | null, item: UiUnlockedItem): boolean {
  return !!p?.fresh.some((f) => sameItem(f, item));
}

export function sameItem(a: UiUnlockedItem, b: UiUnlockedItem): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

/** What an unlock is called in a toast ("New car: Torque"). */
export function unlockLabel(item: UiUnlockedItem, catalog: UiCatalog | null): string {
  const u = appStrings.unlocks;
  switch (item.kind) {
    case "car":
      return u.car(catalog?.cars.find((c) => c.id === item.id)?.name ?? item.id);
    case "track":
      return u.track(catalog?.tracks.find((t) => t.id === item.id)?.name ?? item.id);
    case "cup":
      return u.cup(catalog?.cups.find((c) => c.id === item.id)?.name ?? item.id);
    case "item":
      switch (item.item.kind) {
        case "reverse":
          return u.reverse;
        case "mirror":
          return u.mirror;
        case "difficulty":
          return u.difficulty(difficultyName[item.item.difficulty]);
        case "paintSet":
          return u.paintSet(paintSetName[item.item.set]);
        case "credits":
          return u.credits;
      }
  }
}

/** The best trophy of a cup. */
export const trophyOf = (p: UiProfile | null, cup: string) => p?.trophies.find((t) => t.cup === cup)?.trophy ?? null;
