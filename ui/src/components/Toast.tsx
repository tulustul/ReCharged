// Toasts (unlocks, Golden Batteries, saves, errors): top-right, above everything, click-through;
// each dismisses itself after TOAST_MS of Bevy's frame clock (a shared-value driver, not a JS
// timer), so a toast raised while the first frames stall (boot) is still seen.
import { useEffect } from "react";
import { useSharedValue, withTiming } from "bevy-react";

import { BatteryIcon, BoltIcon, CheckIcon, WarningIcon } from "./icons";
import { useEnter } from "../hooks";
import { dismissToast, toasts, type ToastSpec } from "../state/overlays";
import { useStore } from "../state/useStore";
import { text } from "../theme/styles";
import { color, frame, layer, radius, space } from "../theme/tokens";

/** Below the screen headers (titles, tab bars). */
const TOP = 210;
/** How long a toast stays, ms. */
const TOAST_MS = 4500;

export function ToastHost() {
  const list = useStore(toasts);
  if (list.length === 0) return null;
  return (
    <root name="toasts" style={{ globalZIndex: layer.toast, alignItems: "flexEnd", padding: { top: TOP, right: frame.x / 2 } }}>
      <node style={{ flexDirection: "column", gap: space.s, alignItems: "flexEnd" }}>
        {list.map((t) => (
          <ToastCard key={t.id} toast={t} />
        ))}
      </node>
    </root>
  );
}

const tone = {
  info: { accent: color.cyan, icon: <CheckIcon size={30} color={color.cyan} /> },
  success: { accent: color.success, icon: <BatteryIcon size={30} color={color.gold} /> },
  error: { accent: color.danger, icon: <WarningIcon size={30} color={color.danger} /> },
  unlock: { accent: color.volt, icon: <BoltIcon size={30} /> },
};

function ToastCard({ toast }: { toast: ToastSpec }) {
  const enter = useEnter(60, 0);
  const life = useSharedValue(0);
  useEffect(() => {
    life.value = withTiming(1, { duration: TOAST_MS, onComplete: (completed) => completed && dismissToast(toast.id) });
  }, [life, toast.id]);
  const { accent, icon } = tone[toast.tone];
  return (
    <node
      style={{
        ...enter,
        width: 460,
        flexDirection: "row",
        alignItems: "center",
        gap: space.m,
        padding: { vertical: 16, horizontal: 20 },
        borderRadius: radius.l,
        border: { left: 5 },
        borderColor: accent,
        backgroundColor: "rgba(8, 14, 30, 0.92)",
        boxShadow: [{ color: "rgba(0, 0, 0, 0.45)", yOffset: 8, blurRadius: 24 }],
      }}
    >
      {icon}
      <node style={{ flexDirection: "column", gap: 2, flexShrink: 1 }}>
        <text style={{ ...text.label, color: accent }}>{toast.title}</text>
        {toast.text && <text style={{ ...text.small, color: color.text }}>{toast.text}</text>}
      </node>
    </node>
  );
}
