// Colour-blind racer markers (GDD §18): rc_core's `RacerMarker` gives every rival a shape by grid
// index (humans: their player number) and rc_ui sends it with each racer. These outlines match
// rc_vfx's name-tag markers (`marker_polygon`), so tags, minimap and standings show the same shape.
import type { UiMarkerShape } from "../bevy";

type Pt = [number, number];

/** Regular polygon of `n` corners at radius `r`, the first one on top (−1…1, y up). */
const ring = (n: number, r: number): Pt[] =>
  Array.from({ length: n }, (_, i) => {
    const a = Math.PI / 2 + (2 * Math.PI * i) / n;
    return [Math.cos(a) * r, Math.sin(a) * r];
  });

const ARM = 0.3;
const REACH = 0.9;

const OUTLINES: Record<UiMarkerShape, Pt[]> = {
  circle: ring(40, 0.9),
  triangle: [
    [0, 0.9],
    [-0.95, -0.75],
    [0.95, -0.75],
  ],
  square: [
    [-0.75, -0.75],
    [0.75, -0.75],
    [0.75, 0.75],
    [-0.75, 0.75],
  ],
  diamond: [
    [0, 0.95],
    [0.78, 0],
    [0, -0.95],
    [-0.78, 0],
  ],
  star: Array.from({ length: 10 }, (_, i): Pt => {
    const r = i % 2 === 0 ? 0.98 : 0.42;
    const a = Math.PI / 2 + (Math.PI * i) / 5;
    return [Math.cos(a) * r, Math.sin(a) * r];
  }),
  cross: [
    [-ARM, REACH],
    [ARM, REACH],
    [ARM, ARM],
    [REACH, ARM],
    [REACH, -ARM],
    [ARM, -ARM],
    [ARM, -REACH],
    [-ARM, -REACH],
    [-ARM, -ARM],
    [-REACH, -ARM],
    [-REACH, ARM],
    [-ARM, ARM],
  ],
  pentagon: ring(5, 0.95),
  hexagon: ring(6, 0.95),
};

/** The outline of `shape` as SVG polygon points in a 0–100 box (y down). */
export function markerPoints(shape: UiMarkerShape): number[] {
  return OUTLINES[shape].flatMap(([x, y]) => [Math.round(500 + 500 * x) / 10, Math.round(500 - 500 * y) / 10]);
}
