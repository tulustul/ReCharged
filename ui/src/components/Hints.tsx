// Footer hints: the glyph of the player's current device and what the button does. Clickable, so
// a mouse user can always go back or switch tabs.
import { Glyph } from "./Glyph";
import type { MenuAction } from "../glyphs";
import { useDevice } from "../nav/input";
import { color, radius, size } from "../theme/tokens";

export type HintItem = { action: MenuAction; label: string; onPress?: () => void };

export function Hints({ items, slot = 0, lobby = false }: { items: HintItem[]; slot?: number; lobby?: boolean }) {
  const device = useDevice(slot);
  return (
    <node style={{ flexDirection: "row", alignItems: "center", gap: 10 }}>
      {items.map((h) => {
        const content = (
          <>
            <Glyph action={h.action} device={device} lobby={lobby} size={30} />
            <text style={{ fontSize: size.small, fontWeight: "600", color: color.text, lineBreak: "noWrap" }}>{h.label}</text>
          </>
        );
        return h.onPress ? (
          <button
            key={`${h.action}-${h.label}`}
            onClick={h.onPress}
            style={hint}
            hoverStyle={{ backgroundColor: "rgba(255, 255, 255, 0.08)" }}
            pressStyle={{ backgroundColor: "rgba(255, 255, 255, 0.14)" }}
          >
            {content}
          </button>
        ) : (
          <node key={`${h.action}-${h.label}`} style={hint}>
            {content}
          </node>
        );
      })}
    </node>
  );
}

const hint = {
  flexDirection: "row",
  alignItems: "center",
  gap: 10,
  padding: { horizontal: 12, vertical: 6 },
  borderRadius: radius.m,
  backgroundColor: color.clear,
  transition: { backgroundColor: { duration: 100 } },
} as const;
