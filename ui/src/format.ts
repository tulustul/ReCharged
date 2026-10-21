// Formatters (pure): times, places, colours, distances.

/** `83456` → `"1:23.456"`; `null` → the placeholder. */
export function formatTime(ms: number | null | undefined, placeholder = "—"): string {
  if (ms === null || ms === undefined || !Number.isFinite(ms) || ms < 0) return placeholder;
  const total = Math.round(ms);
  const minutes = Math.floor(total / 60000);
  const seconds = Math.floor((total % 60000) / 1000);
  const millis = total % 1000;
  return `${minutes}:${String(seconds).padStart(2, "0")}.${String(millis).padStart(3, "0")}`;
}

/** `+1.204` / `-0.350` (seconds, with sign). */
export function formatDelta(ms: number): string {
  const sign = ms < 0 ? "-" : "+";
  return `${sign}${(Math.abs(ms) / 1000).toFixed(3)}`;
}

/** `1` → `"1st"`, `12` → `"12th"`, `22` → `"22nd"`. */
export function ordinal(n: number): string {
  const tens = n % 100;
  if (tens >= 11 && tens <= 13) return `${n}th`;
  switch (n % 10) {
    case 1:
      return `${n}st`;
    case 2:
      return `${n}nd`;
    case 3:
      return `${n}rd`;
    default:
      return `${n}th`;
  }
}

/** sRGB 0–1 triple → `"#rrggbb"`. */
export function rgbHex(rgb: readonly number[]): string {
  const channel = (v: number | undefined) =>
    Math.round(Math.min(Math.max(v ?? 0, 0), 1) * 255)
      .toString(16)
      .padStart(2, "0");
  return `#${channel(rgb[0])}${channel(rgb[1])}${channel(rgb[2])}`;
}

/** 0–1 → `"75%"`. */
export const percent = (v: number): string => `${Math.round(v * 100)}%`;

/** Seconds → `"2 h 05 min"` / `"12 min"`. */
export function formatDuration(secs: number): string {
  const minutes = Math.floor(Math.max(0, secs) / 60);
  const hours = Math.floor(minutes / 60);
  return hours > 0 ? `${hours} h ${String(minutes % 60).padStart(2, "0")} min` : `${minutes} min`;
}

/** `"12.3 km"`. */
export const formatKm = (km: number): string => `${km.toFixed(1)} km`;

/** Layout label: `"Forward"`, `"Reverse · Mirror"`. */
export function layoutLabel(reverse: boolean, mirror: boolean, words: { forward: string; reverse: string; mirror: string }) {
  const base = reverse ? words.reverse : words.forward;
  return mirror ? `${base} · ${words.mirror}` : base;
}

/** A diagnostic's value in a few characters: `0.42`, `61.3`, `1840`, `18.4k`, `2.1M`. */
export function compactNumber(v: number): string {
  const a = Math.abs(v);
  if (a >= 1e6) return `${(v / 1e6).toFixed(1)}M`;
  if (a >= 1e4) return `${(v / 1e3).toFixed(1)}k`;
  if (a >= 100) return v.toFixed(0);
  return v.toFixed(a >= 10 ? 1 : 2);
}
