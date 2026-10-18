// The frame every menu screen shares: kicker, title with its charge underline, subtitle, content,
// and the footer with hints (right) and context (left).
import type { ReactNode } from "react";

import { Hints, type HintItem } from "./Hints";
import { useEnter } from "../hooks";
import { fill, text } from "../theme/styles";
import { color, frame, space } from "../theme/tokens";

type Props = {
  title: string;
  kicker?: string;
  subtitle?: string;
  hints: HintItem[];
  headerRight?: ReactNode;
  footerLeft?: ReactNode;
  children: ReactNode;
  /** Hints show this slot's device glyphs. */
  slot?: number;
  lobby?: boolean;
};

export function Screen({ title, kicker, subtitle, hints, headerRight, footerLeft, children, slot, lobby }: Props) {
  const enter = useEnter(-40, 0);
  return (
    <node style={{ ...fill, flexDirection: "column", padding: { left: frame.x, right: frame.x, top: frame.top, bottom: frame.bottom } }}>
      <node style={{ flexDirection: "row", alignItems: "flexEnd", justifyContent: "spaceBetween", flexShrink: 0 }}>
        <node style={{ ...enter, flexDirection: "column", gap: 10 }}>
          {kicker && <text style={{ ...text.caps, color: color.cyan, fontSize: 17 }}>{kicker.toUpperCase()}</text>}
          <text style={text.title}>{title.toUpperCase()}</text>
          <ChargeLine />
          {subtitle && <text style={{ ...text.body, margin: { top: 4 } }}>{subtitle}</text>}
        </node>
        {headerRight}
      </node>
      <node style={{ flexGrow: 1, flexDirection: "column", margin: { top: space.xl, bottom: space.l }, minHeight: 0 }}>{children}</node>
      <node style={{ height: 48, flexShrink: 0, flexDirection: "row", alignItems: "center", justifyContent: "spaceBetween" }}>
        <node style={{ flexDirection: "row", alignItems: "center", gap: space.m }}>{footerLeft}</node>
        <Hints items={hints} slot={slot} lobby={lobby} />
      </node>
    </node>
  );
}

/** The cyan→volt underline of titles. */
export function ChargeLine({ width = 150 }: { width?: number }) {
  return (
    <node style={{ flexDirection: "row", gap: 6, alignItems: "center" }}>
      <node
        style={{
          width,
          height: 5,
          borderRadius: 3,
          backgroundGradient: { type: "linear", angle: 90, stops: [{ color: color.cyan }, { color: color.volt }] },
        }}
      />
      <node style={{ width: 14, height: 5, borderRadius: 3, backgroundColor: color.volt }} />
      <node style={{ width: 6, height: 5, borderRadius: 3, backgroundColor: color.volt, opacity: 0.6 }} />
    </node>
  );
}
