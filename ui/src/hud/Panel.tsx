// The free quadrant of a 3-player split (GDD §11): live standings and the shared minimap with every
// human's dot ringed and numbered; colour-blind markers (setting) as the racers' dots.
import type { UiHudSetup, UiHudTick, UiRect } from "../bevy";
import { rgbHex } from "../format";
import { hud as words } from "../strings/hud";
import { color, slotColor } from "../theme/tokens";
import { fs, hudLayout } from "./layout";
import { caps, chip, label, num, tone } from "./look";
import { MarkerIcon, Minimap } from "./Minimap";
import type { Dot } from "./values";

export function Panel({ rect, setup, tick, dots }: { rect: UiRect; setup: UiHudSetup; tick: UiHudTick; dots: Dot[] }) {
  const { k, pad } = hudLayout(rect, setup.hudScale);
  const humans = setup.racers.flatMap((r, i) => (r.slot !== null ? [i] : []));
  const map = Math.round(Math.min(rect.h - 2 * pad, rect.w * 0.5));
  const order = tick.standings.length > 0 ? tick.standings : setup.racers.map((_, i) => i);
  return (
    <node style={{ positionType: "absolute", left: rect.x, top: rect.y, width: rect.w, height: rect.h, flexDirection: "row", alignItems: "center", gap: pad, padding: pad, backgroundColor: "rgba(3, 7, 18, 0.94)", border: 1, borderColor: "rgba(0, 0, 0, 0.6)" }}>
      <node style={{ flexDirection: "column", flexGrow: 1, flexShrink: 1, gap: Math.round(6 * k) }}>
        <text style={{ ...caps(fs(18, k), color.cyan), margin: { bottom: Math.round(6 * k) } }}>{words.standings}</text>
        {order.map((index, i) => {
          const r = setup.racers[index];
          if (!r) return null;
          const human = r.slot !== null;
          const dot = Math.max(10, Math.round(12 * k));
          const shape = setup.colorblindMarkers && r.marker.kind === "shape" ? r.marker.shape : null;
          return (
            <node key={index} style={{ flexDirection: "row", alignItems: "center", gap: Math.round(10 * k), padding: { horizontal: Math.round(10 * k), vertical: Math.round(3 * k) }, ...(human ? { ...chip(k, true), borderColor: slotColor[r.slot ?? 0] } : {}) }}>
              <text style={{ ...num(fs(24, k, 18), i === 0 ? tone.gold : color.text), width: Math.round(Math.max(30 * k, 24)) }}>{String(i + 1)}</text>
              {shape ? (
                <MarkerIcon shape={shape} size={Math.round(Math.max(26 * k, 20))} fill={rgbHex(r.color)} />
              ) : (
                <node style={{ width: dot, height: dot, borderRadius: 99, backgroundColor: rgbHex(r.color) }} />
              )}
              <text style={label(fs(22, k, 18), human ? color.text : color.textDim)}>{r.name}</text>
            </node>
          );
        })}
      </node>
      {setup.outline.length >= 4 && <Minimap size={map} outline={setup.outline} racers={setup.racers} dots={dots} own={humans} k={k} labels dotScale={1.7} markers={setup.colorblindMarkers} />}
    </node>
  );
}
