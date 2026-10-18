// Top-right of a view: the held item (icon + name), the roulette spinning through every item while
// a Volt Cube is drawn, or an empty slot.
import { useEffect } from "react";
import { interpolate, useSharedValue, withRepeat, withSpring, withTiming } from "bevy-react";

import type { UiItemKind } from "../bevy";
import { itemName } from "../strings/hud";
import { spring } from "../theme/motion";
import { color } from "../theme/tokens";
import { fs } from "./layout";
import { caps, chip } from "./look";
import { ItemIcon, ROULETTE } from "./itemIcons";

type Props = { item: UiItemKind | null; roulette: boolean; k: number };

export function ItemSlot({ item, roulette, k }: Props) {
  const box = Math.round(124 * k);
  const icon = Math.round(box * 0.72);
  return (
    <node style={{ flexDirection: "column", alignItems: "center", gap: Math.round(8 * k) }}>
      <node
        style={{
          width: box,
          height: box,
          alignItems: "center",
          justifyContent: "center",
          overflowX: "clip",
          overflowY: "clip",
          ...chip(k, item !== null || roulette),
          borderRadius: Math.round(20 * k),
          border: 2,
          borderColor: item ? color.cyan : roulette ? color.lineStrong : "rgba(235, 242, 255, 0.4)",
          backgroundColor: item || roulette ? "rgba(5, 10, 24, 0.82)" : color.clear,
          boxShadow: item ? [{ color: color.cyanGlow, blurRadius: Math.round(22 * k) }] : undefined, // never `[]`: it paints a default black shadow
        }}
      >
        {roulette ? <Roulette size={icon} /> : item ? <Held key={item} kind={item} size={icon} /> : null}
      </node>
      {item && !roulette && (
        <node style={{ padding: { horizontal: Math.round(10 * k), vertical: Math.round(4 * k) }, ...chip(k) }}>
          <text style={caps(fs(17, k), color.text)}>{itemName[item].toUpperCase()}</text>
        </node>
      )}
    </node>
  );
}

/** Pops in when the roulette lands. */
function Held({ kind, size }: { kind: UiItemKind; size: number }) {
  const pop = useSharedValue(0.4);
  useEffect(() => {
    pop.value = withSpring(1, spring.bouncy);
  }, [pop]);
  return (
    <node style={{ transform: { scale: { animated: pop } } }}>
      <ItemIcon kind={kind} size={size} />
    </node>
  );
}

/** A strip of every item scrolling through the window (Bevy drives it; no re-render). */
function Roulette({ size }: { size: number }) {
  const t = useSharedValue(0);
  const strip = [...ROULETTE, ROULETTE[0]];
  const step = size * 1.25;
  useEffect(() => {
    t.value = withRepeat(withTiming(1, { duration: 700, easing: "linear" }));
  }, [t]);
  return (
    <node style={{ positionType: "absolute", top: (size * 1.39 - step) / 2, left: 0, right: 0, flexDirection: "column", alignItems: "center", transform: { translateY: { animated: interpolate(t, [0, 1], [0, -step * ROULETTE.length]) } } }}>
      {strip.map((kind, i) => (
        <node key={i} style={{ height: step, alignItems: "center", justifyContent: "center" }}>
          <ItemIcon kind={kind} size={size} />
        </node>
      ))}
    </node>
  );
}
