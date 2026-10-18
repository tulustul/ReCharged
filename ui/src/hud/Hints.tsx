// Bottom-centre of a view: "Race ends in…" during the soft-lock guard, then one line of help — the
// current onboarding hint with the action's button for this player's device, the spectate hint after
// finishing, or "Skip intro" during the fly-by.
import type { ReactNode } from "react";

import type { UiDriveAction, UiHint, UiScheme } from "../bevy";
import { Glyph } from "../components/Glyph";
import { useEnter } from "../hooks";
import { useDevice } from "../nav/input";
import { bindings as bindingsStore } from "../state/stores";
import { useStore } from "../state/useStore";
import { hintText, hud as words } from "../strings/hud";
import { color } from "../theme/tokens";
import { fs } from "./layout";
import { chip, label, num, tone } from "./look";

const HINT_ACTION: Record<UiHint, UiDriveAction | null> = {
  driftCharge: "drift",
  boostReady: "boost",
  itemReady: "useItem",
  holdReset: "reset",
  wrongWay: null,
};

type Props = { slot: number; k: number; hint: { id: number; hint: UiHint } | null; finished: boolean; intro: boolean; endsIn: number | null };

export function Hints({ slot, k, hint, finished, intro, endsIn }: Props) {
  return (
    <node style={{ flexDirection: "column", alignItems: "center", gap: Math.round(12 * k) }}>
      {endsIn !== null && (
        <node style={{ flexDirection: "row", alignItems: "center", gap: Math.round(10 * k), padding: { horizontal: Math.round(18 * k), vertical: Math.round(8 * k) }, ...chip(k, true), borderColor: tone.bad }}>
          <text style={num(fs(30, k, 20), color.text)}>{words.endsIn(endsIn)}</text>
        </node>
      )}
      {intro ? (
        <Line k={k} text={words.skipIntro} glyph={<MenuConfirm slot={slot} k={k} />} />
      ) : finished ? (
        <Line k={k} text={words.spectate} glyph={<ActionGlyph slot={slot} action="camera" k={k} />} />
      ) : hint ? (
        <Line key={hint.id} k={k} text={hintText[hint.hint]} glyph={HINT_ACTION[hint.hint] ? <ActionGlyph slot={slot} action={HINT_ACTION[hint.hint]!} k={k} /> : null} />
      ) : null}
    </node>
  );
}

function Line({ text, glyph, k }: { text: string; glyph: ReactNode; k: number }) {
  const enter = useEnter(0, 18);
  return (
    <node style={{ ...enter, flexDirection: "row", alignItems: "center", gap: Math.round(12 * k), padding: { horizontal: Math.round(18 * k), vertical: Math.round(9 * k) }, ...chip(k, true) }}>
      {glyph}
      <text style={label(fs(24, k, 18))}>{text}</text>
    </node>
  );
}

const schemeOf = (device: string): UiScheme => (device === "gamepad" ? "gamepad" : device === "keyboardB" ? "keyboardB" : "keyboardA");

/** The first button bound to a driving action on the slot's current device. */
function ActionGlyph({ slot, action, k }: { slot: number; action: UiDriveAction; k: number }) {
  const device = useDevice(slot);
  const all = useStore(bindingsStore);
  const binding = all?.schemes.find((s) => s.scheme === schemeOf(device))?.actions.find((a) => a.action === action)?.bindings[0];
  return binding ? <Glyph binding={binding} size={fs(30, k, 22)} /> : null;
}

function MenuConfirm({ slot, k }: { slot: number; k: number }) {
  return <Glyph action="confirm" device={useDevice(slot)} size={fs(30, k, 22)} />;
}
