// Logical focus model (pure): a screen is a set of named regions — vertical lists, rows and
// grids — linked to their neighbours by direction. `move` handles one `ui.nav` direction; pointer
// hover sets the focus directly. No geometry: what a direction does is declared, so it is
// predictable and testable.

export type Dir = "up" | "down" | "left" | "right";

export type Region = {
  /** Items in the region (0 = skipped). */
  count: number;
  /** Columns: 1 = vertical list, `count` = horizontal row, otherwise a grid. */
  cols: number;
  /** Wrap around inside the region when there is no neighbour that way. */
  wrap?: boolean;
  /** Neighbour region per direction (entered at its remembered index). */
  up?: string;
  down?: string;
  left?: string;
  right?: string;
};

export type FocusMap = Record<string, Region>;

export type Focus = {
  region: string;
  index: number;
  /** Last index per region, restored when a region is entered again. */
  memory?: Record<string, number>;
};

export const isDir = (action: string): action is Dir =>
  action === "up" || action === "down" || action === "left" || action === "right";

/** Moves the focus one step in `dir`. */
export function move(map: FocusMap, focus: Focus, dir: Dir): Focus {
  const region = map[focus.region];
  if (!region || region.count <= 0) return focus;
  const cols = Math.max(1, Math.min(region.cols, region.count));
  const rows = Math.ceil(region.count / cols);
  const i = Math.min(Math.max(focus.index, 0), region.count - 1);
  const row = Math.floor(i / cols);
  const col = i % cols;
  const last = region.count - 1;

  let next: number | null = null;
  if (dir === "left" && col > 0) next = i - 1;
  if (dir === "right" && col < cols - 1 && i < last) next = i + 1;
  if (dir === "up" && row > 0) next = i - cols;
  if (dir === "down" && row < rows - 1) next = Math.min(i + cols, last);
  if (next !== null) return { ...focus, index: next };

  const neighbour = region[dir];
  if (neighbour && (map[neighbour]?.count ?? 0) > 0) return enter(focus, neighbour, map[neighbour].count);

  if (region.wrap) {
    if (dir === "left") next = Math.min(row * cols + cols - 1, last);
    if (dir === "right") next = row * cols;
    if (dir === "up") next = Math.min((rows - 1) * cols + col, last);
    if (dir === "down") next = col;
  }
  return next === null || next === i ? focus : { ...focus, index: next };
}

/** Focus `region` at its remembered index (or `index`), remembering where it left from. */
export function enter(focus: Focus, region: string, count: number, index?: number): Focus {
  const memory = { ...focus.memory, [focus.region]: focus.index };
  const at = index ?? memory[region] ?? 0;
  return { region, index: Math.max(0, Math.min(at, count - 1)), memory };
}

/** Keeps the focus valid after region sizes changed (falls back to `fallback` region). */
export function clampFocus(map: FocusMap, focus: Focus, fallback: string): Focus {
  const region = map[focus.region];
  if (!region || region.count <= 0) {
    const count = map[fallback]?.count ?? 0;
    return focus.region === fallback ? { ...focus, index: 0 } : enter(focus, fallback, Math.max(count, 1));
  }
  const index = Math.max(0, Math.min(focus.index, region.count - 1));
  return index === focus.index ? focus : { ...focus, index };
}

/** Steps `index` by `delta` through `count` items, wrapping (tabs, cyclers, carousels). */
export function cycle(index: number, delta: number, count: number): number {
  if (count <= 0) return 0;
  return (((index + delta) % count) + count) % count;
}

/** Steps `value` by `steps` × `step`, snapped to the step grid within [min, max] (sliders). */
export function stepValue(value: number, steps: number, min: number, max: number, step: number): number {
  const next = Math.round((value + steps * step - min) / step) * step + min;
  return Math.min(max, Math.max(min, Number(next.toFixed(6))));
}

/** Scroll offset that keeps list row `index` centred where possible (lists have no scrollIntoView). */
export function followScroll(index: number, rowHeight: number, gap: number, rows: number, viewport: number): number {
  const pitch = rowHeight + gap;
  const content = rows * pitch - gap;
  const target = index * pitch - (viewport - rowHeight) / 2;
  return Math.max(0, Math.min(target, Math.max(0, content - viewport)));
}
