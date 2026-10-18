// Versus lobby choices per slot (pure + store): each joined player has their own focus — a car
// row and a paint row — and a ready flag. Joining and leaving are rc_input's (`lobby.slots`).
import type { UiNavAction } from "../bevy";
import { cycle } from "../nav/focus.ts";
import { createStore, hmrSingleton, sameJson } from "./store.ts";

export type SlotChoice = { car: number; paint: number; row: "car" | "paint"; ready: boolean };

/** Choices by slot number. */
export type LobbyChoices = Record<number, SlotChoice>;

export const lobbyChoices = hmrSingleton("__rc_lobbyChoices", () => createStore<LobbyChoices>({}, sameJson));

/** Choices for the joined slots: new slots get a car of their own, departed slots are dropped. */
export function syncSlots(choices: LobbyChoices, joined: number[], cars: number): LobbyChoices {
  const next: LobbyChoices = {};
  for (const slot of joined) next[slot] = choices[slot] ?? { car: cars > 0 ? slot % cars : 0, paint: 0, row: "car", ready: false };
  return next;
}

/** What one slot's navigation step does; `paints` = selectable paints of a car. Returns the new
 *  choice and the cue to play. */
export function slotStep(
  c: SlotChoice,
  action: UiNavAction,
  cars: number,
  paints: (car: number) => number,
): { choice: SlotChoice; cue: "hover" | "select" | "back" | null } {
  if (action === "confirm") return { choice: { ...c, ready: !c.ready }, cue: c.ready ? "back" : "select" };
  if (c.ready) return { choice: c, cue: null };
  if (action === "up" || action === "down") {
    return { choice: { ...c, row: c.row === "car" ? "paint" : "car" }, cue: "hover" };
  }
  if (action === "left" || action === "right") {
    const delta = action === "left" ? -1 : 1;
    if (c.row === "car") {
      const car = cycle(c.car, delta, cars);
      return { choice: { ...c, car, paint: Math.min(c.paint, Math.max(0, paints(car) - 1)) }, cue: "hover" };
    }
    return { choice: { ...c, paint: cycle(c.paint, delta, paints(c.car)) }, cue: "hover" };
  }
  return { choice: c, cue: null };
}

/** Versus can start: at least two players joined and everyone ready. */
export function allReady(joined: number[], choices: LobbyChoices): boolean {
  return joined.length >= 2 && joined.every((slot) => choices[slot]?.ready);
}
