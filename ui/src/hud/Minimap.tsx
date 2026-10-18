// The minimap (bottom-left in 1P/2P, the panel in 3P): the track outline with the start marked and a
// dot per racer in its colour. Dots move by shared values (Bevy writes them every frame); the own
// dot is bigger with a white ring, other humans carry a dark ring (and their number when large).
import { memo, useMemo } from "react";
import { interpolate, type SharedValue } from "bevy-react";

import type { UiRacer } from "../bevy";
import { rgbHex } from "../format";
import { color } from "../theme/tokens";
import { fs } from "./layout";
import { chip, num } from "./look";
import type { Dot } from "./values";

type Props = { size: number; outline: number[]; racers: UiRacer[]; dots: Dot[]; own: number[]; k: number; labels?: boolean; dotScale?: number };

export function Minimap({ size, outline, racers, dots, own, k, labels = false, dotScale = 1 }: Props) {
  const order = racers.map((r, i) => ({ r, i })).sort((a, b) => rank(a.r, a.i, own) - rank(b.r, b.i, own));
  return (
    <node style={{ width: size, height: size, ...chip(k), borderRadius: Math.round(22 * k), overflowX: "clip", overflowY: "clip" }}>
      <Outline points={outline} size={size} />
      {order.map(({ r, i }) => {
        const mine = own.includes(i);
        const human = r.slot !== null;
        const d = Math.round(Math.max(mine ? 24 * k : human ? 19 * k : 14 * k, mine ? 15 : 10) * dotScale);
        return (
          <MapDot
            key={i}
            dot={dots[i]}
            size={size}
            d={d}
            fill={rgbHex(r.color)}
            ring={mine ? "#ffffff" : human ? color.ink : null}
            label={labels && human ? String((r.slot ?? 0) + 1) : null}
            k={k}
          />
        );
      })}
    </node>
  );
}

/** Draw order: AI, then other humans, then the own dots on top. */
function rank(r: UiRacer, i: number, own: number[]) {
  return own.includes(i) ? 2 : r.slot !== null ? 1 : 0;
}

/** Static: rasterized once per size (the outline never changes within an attempt). */
const Outline = memo(function Outline({ points, size }: { points: number[]; size: number }) {
  const scaled = useMemo(() => points.map((v) => Math.round(v * 1000)), [points]);
  if (scaled.length < 4) return null;
  return (
    <svg viewBox="0 0 1000 1000" style={{ positionType: "absolute", left: 0, top: 0, width: size, height: size }}>
      <polygon points={scaled} fill="none" stroke="rgba(3, 7, 18, 0.85)" strokeWidth={52} strokeLinejoin="round" />
      <polygon points={scaled} fill="none" stroke="rgba(226, 236, 255, 0.92)" strokeWidth={22} strokeLinejoin="round" />
      <circle cx={scaled[0]} cy={scaled[1]} r={24} fill={color.volt} stroke="rgba(3, 7, 18, 0.9)" strokeWidth={8} />
    </svg>
  );
});

type DotProps = { dot: Dot | undefined; size: number; d: number; fill: string; ring: string | null; label: string | null; k: number };

/** Placed at minimap 0–1 coordinates; parked far outside until Bevy writes it (initial −1). */
function MapDot({ dot, size, d, fill, ring, label, k }: DotProps) {
  if (!dot) return null;
  const to = (v: SharedValue) => ({ animated: interpolate(v, [-1, -0.9, 1], [-10000, -0.9 * size, size]) });
  return (
    <node
      style={{
        positionType: "absolute",
        left: -d / 2,
        top: -d / 2,
        width: d,
        height: d,
        borderRadius: d,
        backgroundColor: fill,
        border: ring ? Math.max(2, Math.round(3 * k)) : 0,
        borderColor: ring ?? color.clear,
        alignItems: "center",
        justifyContent: "center",
        transform: { translateX: to(dot.x), translateY: to(dot.y) },
      }}
    >
      {label && <text style={{ ...num(fs(16, k, 13), color.ink), textShadow: { color: color.clear, offsetX: 0, offsetY: 0 } }}>{label}</text>}
    </node>
  );
}
