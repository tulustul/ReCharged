// Top-left of a view: position "2nd /8", "LAP 2/3" (FINAL LAP on the last), the race clock, current
// and best lap, Time Trial splits (colour and sign) and a flash of each completed lap.
import type { ReactNode } from "react";

import type { UiHudSetup, UiPlayerHud } from "../bevy";
import { formatTime } from "../format";
import { useEnter } from "../hooks";
import { hud as words } from "../strings/hud";
import { color, slotColor } from "../theme/tokens";
import { clock, fs, placeParts, split, splitHundredths } from "./layout";
import { caps, chip, num, tone } from "./look";
import type { Moments } from "./store";

type Props = { setup: UiHudSetup; p: UiPlayerHud; raceMs: number; moments: Moments; k: number; compact: boolean; showSlot: boolean };

export function Standing({ setup, p, raceMs, moments, k, compact, showSlot }: Props) {
  const tt = setup.mode.kind === "timeTrial";
  const final = p.lap === setup.laps && setup.laps > 1 && p.finishMs === null;
  const [place, suffix] = placeParts(p.position);
  return (
    <node style={{ flexDirection: "column", alignItems: "flexStart", gap: Math.round(8 * k) }}>
      {!tt && (
        <node style={{ flexDirection: "row", alignItems: "flexEnd", gap: Math.round(6 * k), padding: { left: Math.round(12 * k), right: Math.round(16 * k), top: Math.round(2 * k), bottom: Math.round(4 * k) }, ...chip(k) }}>
          {showSlot && <SlotBadge slot={p.slot} k={k} />}
          <text style={num(fs(112, k, 44))}>{place}</text>
          <text style={{ ...num(fs(44, k, 22)), margin: { bottom: Math.round(10 * k) } }}>{suffix}</text>
          <text style={{ ...num(fs(36, k, 20), color.textDim, "700"), margin: { bottom: Math.round(10 * k), left: Math.round(6 * k) } }}>{`/${setup.racers.length}`}</text>
        </node>
      )}
      {tt && (
        <node style={{ padding: { horizontal: Math.round(14 * k), vertical: Math.round(6 * k) }, ...chip(k) }}>
          <text style={caps(fs(20, k), color.cyan)}>{setup.ghost ? `${words.timeTrial} · ${words.ghost}` : words.timeTrial}</text>
        </node>
      )}
      <node style={{ flexDirection: "row", alignItems: "center", gap: Math.round(10 * k), padding: { horizontal: Math.round(14 * k), vertical: Math.round(6 * k) }, ...chip(k) }}>
        {final ? (
          <text style={num(fs(34, k, 20), color.volt)}>{words.finalLap}</text>
        ) : (
          <>
            <text style={caps(fs(18, k))}>{words.lap}</text>
            <text style={num(fs(34, k, 20))}>{`${p.lap}/${setup.laps}`}</text>
          </>
        )}
      </node>
      <node style={{ flexDirection: "column", gap: Math.round(4 * k), padding: { horizontal: Math.round(14 * k), vertical: Math.round(8 * k) }, ...chip(k) }}>
        <TimeRow label={words.time} value={p.finishMs !== null ? formatTime(p.finishMs) : clock(raceMs)} k={k} big />
        {!compact && p.finishMs === null && (
          <TimeRow label={words.lap} value={clock(p.lapMs)} k={k}>
            {moments.split && <Split key={moments.split.id} secs={moments.split.secs} k={k} />}
          </TimeRow>
        )}
        {!compact && <TimeRow label={words.best} value={formatTime(p.bestLapMs)} k={k} />}
        {compact && moments.split && <Split key={moments.split.id} secs={moments.split.secs} k={k} />}
      </node>
      {moments.lap && <LapFlash key={moments.lap.id} lap={moments.lap.lap} ms={moments.lap.ms} best={moments.lap.best} k={k} />}
    </node>
  );
}

function SlotBadge({ slot, k }: { slot: number; k: number }) {
  return (
    <node style={{ alignSelf: "center", padding: { horizontal: Math.round(10 * k), vertical: Math.round(4 * k) }, borderRadius: Math.round(8 * k), backgroundColor: slotColor[slot] ?? color.cyan, margin: { right: Math.round(6 * k) } }}>
      <text style={{ ...num(fs(28, k, 18), color.ink), textShadow: { color: "rgba(0, 0, 0, 0)", offsetX: 0, offsetY: 0 } }}>{`P${slot + 1}`}</text>
    </node>
  );
}

function TimeRow({ label, value, k, big = false, children }: { label: string; value: string; k: number; big?: boolean; children?: ReactNode }) {
  return (
    <node style={{ flexDirection: "row", alignItems: "center", gap: Math.round(12 * k) }}>
      <text style={{ ...caps(fs(16, k)), width: Math.round(Math.max(64 * k, 52)) }}>{label}</text>
      <text style={num(big ? fs(34, k, 22) : fs(26, k, 18), color.text, big ? "800" : "700")}>{value}</text>
      {children}
    </node>
  );
}

/** Split vs the ghost/best: green and "-" when ahead, red and "+" when behind, neutral "±0.00"
 * on a tie (at the hundredths shown). */
function Split({ secs, k }: { secs: number; k: number }) {
  const h = splitHundredths(secs);
  const [fill, edge, ink] = h < 0 ? ["rgba(61, 220, 132, 0.22)", tone.good, tone.good] : h > 0 ? ["rgba(255, 84, 104, 0.22)", tone.bad, tone.bad] : [tone.chip, tone.edge, color.text];
  const enter = useEnter(16, 0);
  return (
    <node style={{ ...enter, padding: { horizontal: Math.round(10 * k), vertical: Math.round(3 * k) }, borderRadius: Math.round(8 * k), backgroundColor: fill, border: 1, borderColor: edge }}>
      <text style={num(fs(24, k, 18), ink)}>{split(secs)}</text>
    </node>
  );
}

function LapFlash({ lap, ms, best, k }: { lap: number; ms: number; best: boolean; k: number }) {
  const enter = useEnter(-24, 0);
  return (
    <node style={{ ...enter, flexDirection: "row", alignItems: "center", gap: Math.round(10 * k), padding: { horizontal: Math.round(14 * k), vertical: Math.round(6 * k) }, ...chip(k, true), borderColor: best ? color.volt : tone.edge }}>
      <text style={caps(fs(16, k))}>{words.lapDone(lap)}</text>
      <text style={num(fs(28, k, 18))}>{formatTime(ms)}</text>
      {best && <text style={caps(fs(16, k), color.volt)}>{words.bestTag}</text>}
    </node>
  );
}
