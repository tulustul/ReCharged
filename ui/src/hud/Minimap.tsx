// The minimap (bottom-left in 1P/2P, the panel in 3P): the track as a ribbon between its racing
// edges, with the start line, chevrons for the direction of travel, the next checkpoint and a dot
// per racer in its colour. A player's own minimap is heading-up: it turns so their view points up,
// follows their car (magnified by `zoom`) and draws them as an arrow in the middle; the shared 3P
// map stays north-up and whole. Dots move by shared values (Bevy writes them every frame); other
// humans carry a dark ring (and their number when large). With colour-blind markers on, rivals'
// dots take their marker shape and humans show their number.
import { memo, useMemo } from "react";
import { interpolate, type SharedValue } from "bevy-react";

import type { UiMarkerShape, UiRacer } from "../bevy";
import { rgbHex } from "../format";
import { color } from "../theme/tokens";
import { fs } from "./layout";
import { chip, num } from "./look";
import { ART, directionMarks, gateLine, ribbonPath, scaled } from "./mapArt";
import { markerPoints } from "./markers";
import type { Dot } from "./values";

/** What the minimap draws (x, y pairs in its 0–1 square; gates x1, y1, x2, y2 each). */
export type MapLines = { outline: number[]; left: number[]; right: number[]; gates: number[] };

type Props = {
  size: number;
  lines: MapLines;
  racers: UiRacer[];
  dots: Dot[];
  own: number[];
  k: number;
  labels?: boolean;
  dotScale?: number;
  /** Colour-blind markers (setting). */
  markers?: boolean;
  /** Heading-up: the map's rotation (degrees clockwise) for the player `own[0]`. */
  heading?: SharedValue;
  /** Magnification of the heading-up map (≥ 1). */
  zoom?: number;
  /** The next checkpoint to highlight. */
  gate?: number;
};

const INK = "rgba(3, 7, 18, 0.85)";
const EDGE = "rgba(236, 242, 255, 0.95)";

export function Minimap({ size, lines, racers, dots, own, k, labels = false, dotScale = 1, markers = false, heading, zoom = 1, gate }: Props) {
  const me = heading ? dots[own[0]] : undefined;
  const map = me ? Math.round(size * zoom) : size;
  const order = racers
    .map((r, i) => ({ r, i }))
    .filter(({ i }) => !(me && own.includes(i)))
    .sort((a, b) => rank(a.r, a.i, own) - rank(b.r, b.i, own));
  const content = (
    <>
      <Art lines={lines} size={map} k={k} />
      {gate !== undefined && gate !== 0 && <Gate gates={lines.gates} index={gate} size={map} k={k} />}
      {order.map(({ r, i }) => {
        const mine = own.includes(i);
        const human = r.slot !== null;
        const d = Math.round(Math.max(mine ? 24 * k : human ? 19 * k : 14 * k, mine ? 15 : 10) * dotScale);
        const shape = markers && r.marker.kind === "shape" ? r.marker.shape : null;
        return (
          <MapDot
            key={i}
            dot={dots[i]}
            size={map}
            d={shape ? Math.round(d * 1.3) : d}
            fill={rgbHex(r.color)}
            ring={mine ? "#ffffff" : human ? color.ink : null}
            label={(labels || markers) && human ? String((r.slot ?? 0) + 1) : null}
            shape={shape}
            k={k}
          />
        );
      })}
    </>
  );
  const follow = (v: SharedValue) => ({ animated: interpolate(v, [0, 1], [map / 2, -map / 2]) });
  return (
    <node style={{ width: size, height: size, ...chip(k), borderRadius: Math.round(22 * k), overflowX: "clip", overflowY: "clip" }}>
      {me && heading ? (
        // Turns about the middle of the frame; the inner node slides the player's dot there.
        <node style={{ positionType: "absolute", left: 0, top: 0, width: size, height: size, transform: { rotate: { animated: heading } } }}>
          <node style={{ positionType: "absolute", left: (size - map) / 2, top: (size - map) / 2, width: map, height: map, transform: { translateX: follow(me.x), translateY: follow(me.y) } }}>
            {content}
          </node>
        </node>
      ) : (
        content
      )}
      {me && <OwnArrow racer={racers[own[0]]} size={size} k={k} />}
    </node>
  );
}

/** Draw order: AI, then other humans, then the own dots on top. */
function rank(r: UiRacer, i: number, own: number[]) {
  return own.includes(i) ? 2 : r.slot !== null ? 1 : 0;
}

/** Viewbox units of `px` screen pixels on a map drawn `size` px wide. */
const units = (px: number, size: number) => (px * ART) / Math.max(size, 1);

/** Static: rasterized once per size (the track never changes within an attempt). */
const Art = memo(function Art({ lines, size, k }: { lines: MapLines; size: number; k: number }) {
  const art = useMemo(() => {
    const edge = units(Math.max(1.6, 2.2 * k), size);
    return {
      outline: scaled(lines.outline),
      left: scaled(lines.left),
      right: scaled(lines.right),
      ribbon: ribbonPath(lines.left, lines.right),
      start: gateLine(lines.gates, 0),
      marks: directionMarks(lines.outline, 10, units(Math.max(9, 12 * k), size)),
      edge,
      halo: edge + units(3, size),
    };
  }, [lines, size, k]);
  if (art.outline.length < 4) return null;
  const box = { positionType: "absolute", left: 0, top: 0, width: size, height: size } as const;
  if (!art.ribbon) {
    // No edges (demo scenes without a track): the centreline alone.
    return (
      <svg viewBox={`0 0 ${ART} ${ART}`} style={box}>
        <polygon points={art.outline} fill="none" stroke={INK} strokeWidth={52} strokeLinejoin="round" />
        <polygon points={art.outline} fill="none" stroke="rgba(226, 236, 255, 0.92)" strokeWidth={22} strokeLinejoin="round" />
        <circle cx={art.outline[0]} cy={art.outline[1]} r={24} fill={color.volt} stroke={INK} strokeWidth={8} />
      </svg>
    );
  }
  return (
    <svg viewBox={`0 0 ${ART} ${ART}`} style={box}>
      <polygon points={art.left} fill="none" stroke={INK} strokeWidth={art.halo} strokeLinejoin="round" />
      <polygon points={art.right} fill="none" stroke={INK} strokeWidth={art.halo} strokeLinejoin="round" />
      <path d={art.ribbon} fill="rgba(150, 172, 214, 0.5)" />
      <polygon points={art.left} fill="none" stroke={EDGE} strokeWidth={art.edge} strokeLinejoin="round" />
      <polygon points={art.right} fill="none" stroke={EDGE} strokeWidth={art.edge} strokeLinejoin="round" />
      {art.marks.map((m, i) => (
        <polygon key={i} points={m} fill="rgba(236, 242, 255, 0.75)" />
      ))}
      {art.start && <GateStroke line={art.start} tint={color.volt} width={art.edge * 2.4} />}
    </svg>
  );
});

function GateStroke({ line, tint, width }: { line: [number, number, number, number]; tint: string; width: number }) {
  const [x1, y1, x2, y2] = line;
  return (
    <>
      <line x1={x1} y1={y1} x2={x2} y2={y2} stroke={INK} strokeWidth={width * 1.8} strokeLinecap="round" />
      <line x1={x1} y1={y1} x2={x2} y2={y2} stroke={tint} strokeWidth={width} strokeLinecap="round" />
    </>
  );
}

/** The next checkpoint across the ribbon (its own layer: changing it never redraws the track). */
const Gate = memo(function Gate({ gates, index, size, k }: { gates: number[]; index: number; size: number; k: number }) {
  const line = gateLine(gates, index);
  if (!line) return null;
  return (
    <svg viewBox={`0 0 ${ART} ${ART}`} style={{ positionType: "absolute", left: 0, top: 0, width: size, height: size }}>
      <GateStroke line={line} tint={color.cyan} width={units(Math.max(3, 4 * k), size)} />
    </svg>
  );
});

/** The player in the middle of their heading-up map: an arrow pointing the way they look. */
function OwnArrow({ racer, size, k }: { racer: UiRacer | undefined; size: number; k: number }) {
  const d = Math.round(Math.max(30 * k, 20));
  return (
    <node style={{ positionType: "absolute", left: (size - d) / 2, top: (size - d) / 2, width: d, height: d }}>
      <svg viewBox="0 0 48 48" style={{ width: d, height: d }}>
        <polygon points={[24, 3, 43, 43, 24, 34, 5, 43]} fill={racer ? rgbHex(racer.color) : color.cyan} stroke="#ffffff" strokeWidth={5} strokeLinejoin="round" />
      </svg>
    </node>
  );
}

type DotProps = { dot: Dot | undefined; size: number; d: number; fill: string; ring: string | null; label: string | null; shape: UiMarkerShape | null; k: number };

/** Placed at minimap 0–1 coordinates; parked far outside until Bevy writes it (initial −1). */
function MapDot({ dot, size, d, fill, ring, label, shape, k }: DotProps) {
  if (!dot) return null;
  const to = (v: SharedValue) => ({ animated: interpolate(v, [-1, -0.9, 1], [-10000, -0.9 * size, size]) });
  const place = { positionType: "absolute", left: -d / 2, top: -d / 2, width: d, height: d, transform: { translateX: to(dot.x), translateY: to(dot.y) } } as const;
  if (shape) {
    return (
      <node style={place}>
        <MarkerIcon shape={shape} size={d} fill={fill} />
      </node>
    );
  }
  return (
    <node
      style={{
        ...place,
        borderRadius: d,
        backgroundColor: fill,
        border: ring ? Math.max(2, Math.round(3 * k)) : 0,
        borderColor: ring ?? color.clear,
        alignItems: "center",
        justifyContent: "center",
      }}
    >
      {label && <text style={{ ...num(fs(16, k, 13), color.ink), textShadow: { color: color.clear, offsetX: 0, offsetY: 0 } }}>{label}</text>}
    </node>
  );
}

/** A colour-blind marker shape in the racer's colour with a dark outline (static: rasterized once
 * per size). */
export function MarkerIcon({ shape, size, fill }: { shape: UiMarkerShape; size: number; fill: string }) {
  return (
    <svg viewBox="-8 -8 116 116" style={{ width: size, height: size }}>
      <polygon points={markerPoints(shape)} fill={fill} stroke="rgba(3, 7, 18, 0.9)" strokeWidth={12} strokeLinejoin="round" />
    </svg>
  );
}
