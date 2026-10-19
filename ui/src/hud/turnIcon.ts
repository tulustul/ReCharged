// Turn indicator icons (pure, tested): an arrow bending the corner's way — a slight bend, a right
// angle or a hairpin, by the corner's heading change — in a 48 × 48 box, mirrored for left turns.

export const ICON_BOX = 48;

export type TurnShape = "slight" | "square" | "hairpin";

/** The icon for a corner turning `angle` degrees. */
export const turnShape = (angle: number): TurnShape => (angle >= 135 ? "hairpin" : angle >= 60 ? "square" : "slight");

/** Right-hand stems (x, y pairs, y down), each ending where its arrowhead starts. */
const STEMS: Record<TurnShape, number[]> = {
  slight: [17, 45, 17, 31, 19, 24, 23, 19, 28, 15],
  square: [13, 45, 13, 25, 15, 19, 20, 16, 30, 16],
  hairpin: [11, 45, 11, 22, 14, 14, 20, 10, 27, 10, 33, 14, 36, 21, 36, 27],
};

/** An arrowhead continuing the stem's last segment. */
function head(stem: number[], size: number): number[] {
  const [ax, ay, bx, by] = stem.slice(-4);
  const len = Math.hypot(bx - ax, by - ay);
  const [dx, dy] = [(bx - ax) / len, (by - ay) / len];
  const w = size * 0.85;
  return [bx + dx * size, by + dy * size, bx - dy * w, by + dx * w, bx + dy * w, by - dx * w];
}

const round = (v: number) => Math.round(v * 10) / 10;
const mirror = (pts: number[]) => pts.map((v, i) => (i % 2 === 0 ? ICON_BOX - v : v));

/** The icon of a corner: a stem for `<polyline>` and an arrowhead for `<polygon>`. */
export function turnIcon(angle: number, right: boolean): { stem: number[]; head: number[] } {
  const stem = STEMS[turnShape(angle)];
  const icon = { stem, head: head(stem, 9) };
  const side = right ? icon : { stem: mirror(icon.stem), head: mirror(icon.head) };
  return { stem: side.stem.map(round), head: side.head.map(round) };
}
