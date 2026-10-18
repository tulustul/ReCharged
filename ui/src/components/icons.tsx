// Small vector icons (24×24 user units), drawn by bevy-react's <svg>.
import { color } from "../theme/tokens";

type IconProps = { size?: number; color?: string };

const box = (size: number) => ({ width: size, height: size, flexShrink: 0 });

export function LockIcon({ size = 24, color: c = color.textDim }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" style={box(size)}>
      <path d="M7.5 10.5 V7.5 a4.5 4.5 0 0 1 9 0 V10.5" fill="none" stroke={c} strokeWidth={2.2} strokeLinecap="round" />
      <rect x={4.5} y={10} width={15} height={11} rx={2.5} fill={c} />
      <circle cx={12} cy={15.5} r={1.6} fill={color.navy} />
    </svg>
  );
}

export function BoltIcon({ size = 24, color: c = color.volt }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" style={box(size)}>
      <polygon points={[13.5, 1.5, 3.5, 13.5, 11, 13.5, 9.5, 22.5, 20.5, 9.5, 13, 9.5]} fill={c} />
    </svg>
  );
}

export function TrophyIcon({ size = 24, color: c = color.gold }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" style={box(size)}>
      <path d="M6 3 H18 V8 a6 6 0 0 1 -12 0 Z" fill={c} />
      <path d="M6 5 H3 V7 a4 4 0 0 0 4 4 M18 5 H21 V7 a4 4 0 0 1 -4 4" fill="none" stroke={c} strokeWidth={1.8} />
      <rect x={10.5} y={13} width={3} height={4.5} fill={c} />
      <rect x={7} y={17.5} width={10} height={3.5} rx={1} fill={c} />
    </svg>
  );
}

export function BatteryIcon({ size = 24, color: c = color.gold }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" style={box(size)}>
      <rect x={2} y={6.5} width={17.5} height={11} rx={2.5} fill="none" stroke={c} strokeWidth={2} />
      <rect x={20} y={10} width={2.5} height={4} rx={0.8} fill={c} />
      <polygon points={[11.5, 8, 6.5, 12.6, 9.6, 12.6, 8.8, 16, 13.6, 11.2, 10.6, 11.2]} fill={c} />
    </svg>
  );
}

export function MedalIcon({ size = 24, color: c = color.gold }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" style={box(size)}>
      <polygon points={[6, 2, 10, 2, 13, 9, 9, 9]} fill={color.cyanMid} />
      <polygon points={[18, 2, 14, 2, 11, 9, 15, 9]} fill={color.cyan} />
      <circle cx={12} cy={15} r={6.5} fill={c} />
      <circle cx={12} cy={15} r={4} fill="none" stroke={color.navy} strokeWidth={1.2} opacity={0.5} />
    </svg>
  );
}

export function CheckIcon({ size = 24, color: c = color.success }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" style={box(size)}>
      <polyline points={[4.5, 12.5, 9.5, 17.5, 19.5, 6.5]} fill="none" stroke={c} strokeWidth={3} strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

export function ChevronIcon({ size = 24, color: c = color.text, dir = "right" }: IconProps & { dir?: "left" | "right" }) {
  const points = dir === "right" ? [9, 5, 16, 12, 9, 19] : [15, 5, 8, 12, 15, 19];
  return (
    <svg viewBox="0 0 24 24" style={box(size)}>
      <polyline points={points} fill="none" stroke={c} strokeWidth={3} strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

export function StarIcon({ size = 24, color: c = color.volt, filled = true }: IconProps & { filled?: boolean }) {
  return (
    <svg viewBox="0 0 24 24" style={box(size)}>
      <polygon
        points={[12, 2.5, 14.8, 8.8, 21.5, 9.3, 16.4, 13.7, 18, 20.5, 12, 16.9, 6, 20.5, 7.6, 13.7, 2.5, 9.3, 9.2, 8.8]}
        fill={filled ? c : "none"}
        stroke={c}
        strokeWidth={1.6}
        strokeLinejoin="round"
      />
    </svg>
  );
}

export function WarningIcon({ size = 24, color: c = color.warning }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" style={box(size)}>
      <polygon points={[12, 2.5, 22.5, 20.5, 1.5, 20.5]} fill={c} />
      <rect x={10.8} y={8.5} width={2.4} height={6.5} rx={1} fill={color.navy} />
      <circle cx={12} cy={17.6} r={1.4} fill={color.navy} />
    </svg>
  );
}

export function ControllerIcon({ size = 24, color: c = color.textDim }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" style={box(size)}>
      <path d="M7 7 H17 a5 5 0 0 1 4.8 6.4 l-1.3 4.4 a2.4 2.4 0 0 1 -4.2 0.8 L14.5 16 H9.5 l-1.8 2.6 a2.4 2.4 0 0 1 -4.2 -0.8 L2.2 13.4 A5 5 0 0 1 7 7 Z" fill={c} />
      <rect x={6} y={10.2} width={4.4} height={1.6} rx={0.6} fill={color.navy} />
      <rect x={7.4} y={8.8} width={1.6} height={4.4} rx={0.6} fill={color.navy} />
      <circle cx={16.6} cy={9.8} r={1.1} fill={color.navy} />
      <circle cx={18.6} cy={11.8} r={1.1} fill={color.navy} />
    </svg>
  );
}

export function KeyboardIcon({ size = 24, color: c = color.textDim }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" style={box(size)}>
      <rect x={1.5} y={6} width={21} height={12} rx={2.2} fill={c} />
      {[4, 7.5, 11, 14.5, 18].map((x) => (
        <rect key={x} x={x} y={8.6} width={2.4} height={2.2} rx={0.5} fill={color.navy} />
      ))}
      <rect x={6} y={13.4} width={12} height={2.2} rx={0.6} fill={color.navy} />
    </svg>
  );
}
