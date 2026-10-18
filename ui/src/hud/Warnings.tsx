// Top-centre warnings: a Spark Rocket locked on (an arrow toward it, turned every frame by a shared
// value), the Goo Bomb fuse and the shield timer. Pulse unless Reduce flashing is on.
import { useEffect, useRef } from "react";
import { interpolateColor, useSharedValue, withRepeat, withTiming, type SharedValue } from "bevy-react";

import type { UiItemKind } from "../bevy";
import { hud as words } from "../strings/hud";
import { color } from "../theme/tokens";
import { fs } from "./layout";
import { caps, chip, num, tone } from "./look";
import { ItemIcon } from "./itemIcons";

type Props = { lockOn: boolean; goo: number | null; shield: number | null; arrow: SharedValue; k: number; calm: boolean };

export function Warnings({ lockOn, goo, shield, arrow, k, calm }: Props) {
  if (!lockOn && goo === null && shield === null) return null;
  return (
    <node style={{ flexDirection: "row", alignItems: "center", gap: Math.round(14 * k) }}>
      {lockOn && <LockOn arrow={arrow} k={k} calm={calm} />}
      {goo !== null && <Countdown label={words.goo} icon="gooBomb" secs={goo} tint={tone.warn} k={k} />}
      {shield !== null && <Countdown label={words.shield} icon="shieldBubble" secs={shield} tint={color.cyan} k={k} />}
    </node>
  );
}

/** A border colour that blinks between `a` and `b` (steady `a` when calm). */
function useBlink(a: string, b: string, calm: boolean) {
  const t = useSharedValue(0);
  useEffect(() => {
    t.value = calm ? 0 : withRepeat(withTiming(1, { duration: 260 }), { reverse: true });
  }, [t, calm]);
  return calm ? a : { animated: interpolateColor(t, [0, 1], [a, b]) };
}

function LockOn({ arrow, k, calm }: { arrow: SharedValue; k: number; calm: boolean }) {
  const ring = Math.round(Math.max(64 * k, 44));
  const border = useBlink(tone.bad, "#ffd0d6", calm);
  return (
    <node style={{ flexDirection: "row", alignItems: "center", gap: Math.round(12 * k), padding: { left: Math.round(10 * k), right: Math.round(18 * k), vertical: Math.round(8 * k) }, ...chip(k, true), border: 2, borderColor: border }}>
      <node style={{ width: ring, height: ring, alignItems: "center", justifyContent: "center", borderRadius: ring, border: 2, borderColor: "rgba(255, 84, 104, 0.5)" }}>
        <ItemIcon kind="sparkRocket" size={Math.round(ring * 0.55)} />
        {/* The arrowhead orbits the ring toward the rocket: 0° = ahead (up), clockwise. */}
        <node style={{ positionType: "absolute", left: 0, top: 0, width: ring, height: ring, transform: { rotate: { animated: arrow } } }}>
          <svg viewBox="0 0 64 64" style={{ width: ring, height: ring }}>
            <polygon points={[32, 0, 22, 14, 42, 14]} fill={tone.bad} stroke="#ffffff" strokeWidth={2} />
          </svg>
        </node>
      </node>
      <text style={num(fs(30, k, 20), tone.bad)}>{words.rocketLocked}</text>
    </node>
  );
}

/** A fuse or shield: icon, label, seconds and a bar that runs down from the first value seen. */
function Countdown({ label, icon, secs, tint, k }: { label: string; icon: UiItemKind; secs: number; tint: string; k: number }) {
  const max = useRef(secs);
  max.current = Math.max(max.current, secs, 0.1);
  const w = Math.round(Math.max(150 * k, 110));
  return (
    <node style={{ flexDirection: "row", alignItems: "center", gap: Math.round(10 * k), padding: { left: Math.round(10 * k), right: Math.round(16 * k), vertical: Math.round(8 * k) }, ...chip(k, true), border: 2, borderColor: tint }}>
      <ItemIcon kind={icon} size={Math.round(Math.max(44 * k, 30))} />
      <node style={{ flexDirection: "column", gap: Math.round(5 * k) }}>
        <node style={{ flexDirection: "row", justifyContent: "spaceBetween", alignItems: "flexEnd", width: w }}>
          <text style={caps(fs(16, k), tint)}>{label}</text>
          <text style={num(fs(26, k, 18))}>{secs.toFixed(1)}</text>
        </node>
        <node style={{ width: w, height: Math.max(4, Math.round(6 * k)), borderRadius: 4, backgroundColor: "rgba(255, 255, 255, 0.15)" }}>
          <node style={{ width: `${Math.round((secs / max.current) * 100)}%`, height: "100%", borderRadius: 4, backgroundColor: tint }} />
        </node>
      </node>
    </node>
  );
}
