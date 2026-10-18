// Item icons (48×48 vector, own shapes): a distinct silhouette and colour per item, so the slot
// reads by shape as well as colour.
import type { ReactNode } from "react";

import type { UiItemKind } from "../bevy";
import { color } from "../theme/tokens";

const INK = color.ink;

const ICONS: Record<UiItemKind, ReactNode> = {
  turboCell: (
    <>
      <rect x={6} y={14} width={32} height={20} rx={4} fill={color.volt} />
      <rect x={38} y={20} width={4} height={8} rx={1.5} fill={color.volt} />
      <polygon points={[25, 15, 16, 26, 22, 26, 19, 33, 29, 21, 23, 21]} fill={INK} />
    </>
  ),
  sparkRocket: (
    <>
      <polygon points={[17, 24, 9, 35, 17, 32]} fill="#c2412b" />
      <polygon points={[31, 24, 39, 35, 31, 32]} fill="#c2412b" />
      <path d="M24 4 C31 10 32 20 31 31 L17 31 C16 20 17 10 24 4 Z" fill="#ff6a3d" />
      <circle cx={24} cy={17} r={3.5} fill="#ffffff" />
      <polygon points={[19, 32, 24, 45, 29, 32]} fill={color.volt} />
    </>
  ),
  leechCoil: (
    <>
      <polyline points={[8, 24, 12, 15, 16, 33, 20, 15, 24, 33, 28, 15, 32, 33, 36, 15, 40, 24]} fill="none" stroke="#b06bff" strokeWidth={3.5} strokeLinejoin="round" />
      <circle cx={6} cy={24} r={4} fill="#b06bff" />
      <circle cx={42} cy={24} r={4} fill="#d6b3ff" />
    </>
  ),
  shockOrb: (
    <>
      <polyline points={[24, 3, 21, 8, 26, 9, 23, 13]} fill="none" stroke={color.cyanHi} strokeWidth={2.5} />
      <polyline points={[24, 45, 27, 40, 22, 39, 25, 35]} fill="none" stroke={color.cyanHi} strokeWidth={2.5} />
      <polyline points={[3, 24, 8, 27, 9, 22, 13, 25]} fill="none" stroke={color.cyanHi} strokeWidth={2.5} />
      <polyline points={[45, 24, 40, 21, 39, 26, 35, 23]} fill="none" stroke={color.cyanHi} strokeWidth={2.5} />
      <circle cx={24} cy={24} r={11} fill={color.cyan} />
      <circle cx={20.5} cy={20.5} r={4} fill="#d8fbff" />
    </>
  ),
  empPulse: (
    <>
      <circle cx={24} cy={24} r={19} fill="none" stroke="#8ab4ff" strokeWidth={2.5} opacity={0.55} />
      <circle cx={24} cy={24} r={12} fill="none" stroke="#8ab4ff" strokeWidth={3} />
      <circle cx={24} cy={24} r={5} fill="#cfe0ff" />
    </>
  ),
  gooBomb: (
    <>
      <path d="M29 17 Q33 8 40 9" fill="none" stroke="#c9a26b" strokeWidth={3} strokeLinecap="round" />
      <polygon points={[41, 4, 42.5, 7.5, 46, 8, 43, 10, 44, 14, 41, 11.5, 38, 14, 39, 10, 36, 8, 39.5, 7.5]} fill={color.volt} />
      <circle cx={22} cy={28} r={13} fill="#5be35b" />
      <circle cx={13} cy={40} r={3} fill="#3fb847" />
      <circle cx={30} cy={42} r={2.5} fill="#3fb847" />
      <circle cx={17} cy={23} r={3.5} fill="#d7ffd2" opacity={0.6} />
    </>
  ),
  oilSlick: (
    <>
      <path d="M24 5 C30 15 37 22 37 30 A13 13 0 0 1 11 30 C11 22 18 15 24 5 Z" fill="#262a36" stroke="#7a8cff" strokeWidth={2} />
      <path d="M17 30 A7 7 0 0 0 22 37" fill="none" stroke="#5ef2c6" strokeWidth={2.5} strokeLinecap="round" />
    </>
  ),
  springPad: (
    <>
      <rect x={10} y={7} width={28} height={5} rx={2} fill="#ffc24d" />
      <polyline points={[34, 13, 14, 18, 34, 23, 14, 28, 34, 33, 14, 37]} fill="none" stroke="#ffb020" strokeWidth={3.5} strokeLinejoin="round" />
      <rect x={7} y={37} width={34} height={6} rx={2} fill="#c97d0c" />
    </>
  ),
  shieldBubble: (
    <>
      <circle cx={24} cy={24} r={18} fill="#4de8ff" opacity={0.22} stroke="#4de8ff" strokeWidth={3} />
      <path d="M24 13 L33 16.5 C33 26 29 31 24 35 C19 31 15 26 15 16.5 Z" fill="#4de8ff" />
      <path d="M13 17 A13 13 0 0 1 21 10" fill="none" stroke="#e6fdff" strokeWidth={2.5} strokeLinecap="round" />
    </>
  ),
  stormStrike: (
    <>
      <path d="M13 27 A7 7 0 0 1 15 13 A9.5 9.5 0 0 1 33 12 A7.5 7.5 0 0 1 36 27 Z" fill="#8f7dff" />
      <polygon points={[25, 22, 18, 35, 23, 35, 20, 46, 31, 30, 26, 30, 29, 22]} fill={color.volt} />
    </>
  ),
};

/** Roulette order (every item once, then the first again for a seamless loop). */
export const ROULETTE: UiItemKind[] = ["turboCell", "sparkRocket", "shieldBubble", "gooBomb", "shockOrb", "oilSlick", "leechCoil", "springPad", "empPulse", "stormStrike"];

export function ItemIcon({ kind, size }: { kind: UiItemKind; size: number }) {
  return (
    <svg viewBox="0 0 48 48" style={{ width: size, height: size, flexShrink: 0 }}>
      {ICONS[kind]}
    </svg>
  );
}
