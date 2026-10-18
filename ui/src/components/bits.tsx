// Small display components: LockOverlay, Spinner, StatBar, Grid.
import { useEffect, type ReactNode } from "react";
import { interpolate, useSharedValue, withRepeat, withTiming } from "bevy-react";

import { LockIcon } from "./icons";
import { fill, text } from "../theme/styles";
import { color, radius, size, space } from "../theme/tokens";

/** Dims locked content and says how to unlock it (on a solid tag, readable over busy cards). */
export function LockOverlay({ reason, compact = false }: { reason: string; compact?: boolean }) {
  return (
    <node
      style={{
        ...fill,
        borderRadius: radius.l,
        backgroundColor: "rgba(3, 6, 14, 0.62)",
        alignItems: "center",
        justifyContent: "center",
        padding: space.m,
      }}
    >
      <node
        style={{
          flexDirection: compact ? "row" : "column",
          alignItems: "center",
          gap: compact ? 10 : space.s,
          maxWidth: "100%",
          padding: compact ? { horizontal: 14, vertical: 10 } : { horizontal: 20, vertical: 16 },
          borderRadius: radius.m,
          border: 1,
          borderColor: color.lineStrong,
          backgroundColor: "rgba(6, 11, 24, 0.94)",
        }}
      >
        <LockIcon size={compact ? 24 : 38} color={color.textDim} />
        <text style={{ ...text.small, color: color.text, textAlign: "center", fontWeight: "600", flexShrink: 1 }}>{reason}</text>
      </node>
    </node>
  );
}

/** A turning charge ring. */
export function Spinner({ diameter = 44, tint = color.cyan }: { diameter?: number; tint?: string }) {
  const turn = useSharedValue(0);
  useEffect(() => {
    turn.value = withRepeat(withTiming(1, { duration: 900, easing: "linear" }));
  }, [turn]);
  return (
    <node style={{ width: diameter, height: diameter, transform: { rotate: { animated: interpolate(turn, [0, 1], [0, 360]) } } }}>
      <svg viewBox="0 0 24 24" style={{ width: diameter, height: diameter }}>
        <circle cx={12} cy={12} r={9} fill="none" stroke={color.line} strokeWidth={2.6} />
        <path d="M12 3 a9 9 0 0 1 9 9" fill="none" stroke={tint} strokeWidth={2.6} strokeLinecap="round" />
      </svg>
    </node>
  );
}

/** A 1–`max` star stat as segments. */
export function StatBar({ label, value, max = 5, tint = color.cyan }: { label: string; value: number; max?: number; tint?: string }) {
  return (
    <node style={{ flexDirection: "column", gap: 8 }}>
      <node style={{ flexDirection: "row", justifyContent: "spaceBetween" }}>
        <text style={text.caps}>{label.toUpperCase()}</text>
        <text style={{ ...text.caps, color: color.text }}>{`${value}/${max}`}</text>
      </node>
      <node style={{ flexDirection: "row", gap: 5 }}>
        {Array.from({ length: max }, (_, i) => (
          <node
            key={i}
            style={{
              flexGrow: 1,
              height: 10,
              borderRadius: 3,
              backgroundColor: i < value ? tint : "rgba(255, 255, 255, 0.09)",
              transition: { backgroundColor: { duration: 180 } },
            }}
          />
        ))}
      </node>
    </node>
  );
}

/** A grid with `cols` equal columns. */
export function Grid({ cols, gap = space.m, children }: { cols: number; gap?: number; children: ReactNode }) {
  return <node style={{ display: "grid", gridTemplateColumns: `repeat(${cols}, 1flex)`, gap }}>{children}</node>;
}

/** Small caps section label. */
export function Caption({ children, tint = color.textDim }: { children: string; tint?: string }) {
  return <text style={{ ...text.caps, color: tint, fontSize: size.tiny }}>{children.toUpperCase()}</text>;
}
