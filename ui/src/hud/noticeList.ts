// HUD notices (pure): each view's short list, and hits the player's item landed collapsing into
// one line per item ("Oil Slick hit 3 racers") instead of a chain of names.
import type { UiItemKind } from "../bevy";
import { hud as words, itemName } from "../strings/hud.ts";

export type Notice = {
  id: number;
  slot: number;
  tone: "good" | "bad" | "info" | "gold";
  text: string;
  item?: UiItemKind;
  /** A landed hit: when the latest victim was hit (ms) and how many there were. */
  landed?: { at: number; count: number };
};

/** Notices shown per view at once (older ones drop off). */
export const MAX_NOTICES = 3;
/** A hit by the same item within this long (ms) of the previous one joins its line. */
export const LANDED_MERGE_MS = 1000;

/** `list` with `notice` as its slot's newest line (replacing a line with the same id). */
export function withNotice(list: Notice[], notice: Notice): Notice[] {
  const others = list.filter((n) => n.slot !== notice.slot);
  const own = list.filter((n) => n.slot === notice.slot && n.id !== notice.id).slice(1 - MAX_NOTICES);
  return [...others, ...own, notice];
}

/** The line for a hit `slot`'s `item` landed on `victim` at `now` (ms): a new one ("Oil Slick hit
 * Dash!"), or the same item's recent line counting one more victim. */
export function landedNotice(list: Notice[], id: number, slot: number, item: UiItemKind, victim: string, now: number): Notice {
  const recent = list.find((n) => n.slot === slot && n.item === item && n.landed !== undefined && now - n.landed.at <= LANDED_MERGE_MS);
  const count = (recent?.landed?.count ?? 0) + 1;
  const name = itemName[item];
  return {
    id: recent?.id ?? id,
    slot,
    tone: "good",
    item,
    text: count > 1 ? words.notice.landedMany(name, count) : words.notice.landed(name, victim),
    landed: { at: now, count },
  };
}
