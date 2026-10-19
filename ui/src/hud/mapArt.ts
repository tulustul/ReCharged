// Minimap drawing data (pure, tested): rc_ui sends the centreline and both racing edges as x, y
// pairs in the 0–1 square; the minimap draws them in a 0–1000 viewBox as a filled ribbon (one quad
// per pair of edge points, all in one path: with the nonzero rule the quads join seamlessly and
// overlapping parts of the track — bridges, crossings — stay filled), its edges, the start line,
// the next gate and small chevrons showing the direction of travel.

/** The viewBox size of the minimap's art. */
export const ART = 1000;

const at = (pts: number[], i: number): [number, number] => [Math.round(pts[2 * i] * ART), Math.round(pts[2 * i + 1] * ART)];

/** The pairs of `pts` scaled to the viewBox, as a flat array (for `<polygon>` / `<polyline>`). */
export const scaled = (pts: number[]): number[] => pts.map((v) => Math.round(v * ART));

/** The track ribbon between the edges as one path of quads (empty without matching edges). */
export function ribbonPath(left: number[], right: number[]): string {
  const n = left.length / 2;
  if (n < 2 || right.length !== left.length) return "";
  const parts: string[] = [];
  for (let i = 0; i < n; i++) {
    const j = (i + 1) % n;
    const quad = [at(left, i), at(left, j), at(right, j), at(right, i)];
    parts.push(`M${quad.map(([x, y]) => `${x} ${y}`).join(" L")} Z`);
  }
  return parts.join(" ");
}

/** Gate `index` of the flat gate list (x1, y1, x2, y2 each) in the viewBox, if it exists. */
export function gateLine(gates: number[], index: number): [number, number, number, number] | null {
  const g = gates.slice(4 * index, 4 * index + 4);
  if (g.length < 4) return null;
  const [x1, y1, x2, y2] = g.map((v) => Math.round(v * ART));
  return [x1, y1, x2, y2];
}

/** `count` chevrons spread along the centreline, each pointing along the direction of travel
 * (`size` = their length in viewBox units), as flat point lists for `<polygon>`. */
export function directionMarks(outline: number[], count: number, size: number): number[][] {
  const n = outline.length / 2;
  if (n < 3 || count < 1) return [];
  const marks: number[][] = [];
  for (let k = 0; k < count; k++) {
    // Off the start line, evenly spaced.
    const i = Math.floor(((k + 0.5) / count) * n);
    const [ax, ay] = at(outline, (i - 1 + n) % n);
    const [bx, by] = at(outline, (i + 1) % n);
    const len = Math.hypot(bx - ax, by - ay);
    if (len < 1e-6) continue;
    const [dx, dy] = [(bx - ax) / len, (by - ay) / len];
    const [cx, cy] = at(outline, i);
    const half = size / 2;
    const tip = [cx + dx * half, cy + dy * half];
    const back = [cx - dx * half, cy - dy * half];
    const wing = [-dy * half * 0.9, dx * half * 0.9];
    const notch = [cx - dx * half * 0.2, cy - dy * half * 0.2];
    marks.push([tip[0], tip[1], back[0] + wing[0], back[1] + wing[1], notch[0], notch[1], back[0] - wing[0], back[1] - wing[1]].map(Math.round));
  }
  return marks;
}
