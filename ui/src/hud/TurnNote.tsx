// The turn indicator (GDD §13): the corner ahead — which way and how far it bends (the arrow) and
// how hard it is (colour, and the words outside compact views) — from about 2.5 s before it until
// its end. PlayerHud hides it while the player is lost or driving the wrong way: those warnings
// point the way instead.
import type { UiTurn } from "../bevy";
import { hud as words } from "../strings/hud";
import { color } from "../theme/tokens";
import { fs } from "./layout";
import { caps, chip, tone } from "./look";
import { ICON_BOX, turnIcon } from "./turnIcon";

const INK = "rgba(3, 7, 18, 0.9)";

/** Height the indicator takes (PlayerHud keeps it free, so nothing jumps when it shows). */
export const turnNoteHeight = (k: number) => iconSize(k) + 2 * Math.round(6 * k) + 2;

const iconSize = (k: number) => Math.round(Math.max(54 * k, 34));

export const turnTint = (severity: number) => (severity >= 3 ? tone.bad : severity === 2 ? color.volt : color.text);

export function TurnNote({ turn, k, compact }: { turn: UiTurn; k: number; compact: boolean }) {
  const tint = turnTint(turn.severity);
  const { stem, head } = turnIcon(turn.angle, turn.right);
  const icon = iconSize(k);
  return (
    <node style={{ flexDirection: turn.right ? "row" : "rowReverse", alignItems: "center", gap: Math.round(10 * k), padding: { horizontal: Math.round(14 * k), vertical: Math.round(6 * k) }, ...chip(k, true), border: 2, borderColor: tint }}>
      {!compact && <text style={caps(fs(22, k), tint)}>{words.turn(turn)}</text>}
      <svg viewBox={`0 0 ${ICON_BOX} ${ICON_BOX}`} style={{ width: icon, height: icon }}>
        <polyline points={stem} fill="none" stroke={INK} strokeWidth={10} strokeLinecap="round" strokeLinejoin="round" />
        <polygon points={head} fill={INK} stroke={INK} strokeWidth={5} strokeLinejoin="round" />
        <polyline points={stem} fill="none" stroke={tint} strokeWidth={5.5} strokeLinecap="round" strokeLinejoin="round" />
        <polygon points={head} fill={tint} />
      </svg>
    </node>
  );
}
