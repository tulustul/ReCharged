import { useEffect } from "react";
import { useSharedValue, withSpring } from "bevy-react";

import { spring } from "../theme/motion";
import { color, radius } from "../theme/tokens";

/** The volt "NEW" pill; pops in with a bounce. */
export function Badge({ label, tone = color.volt }: { label: string; tone?: string }) {
  const pop = useSharedValue(0.4);
  useEffect(() => {
    pop.value = withSpring(1, spring.bouncy);
  }, [pop]);
  return (
    <node
      style={{
        padding: { horizontal: 10, vertical: 3 },
        borderRadius: radius.pill,
        backgroundColor: tone,
        transform: { scale: { animated: pop } },
        flexShrink: 0,
      }}
    >
      <text style={{ fontSize: 15, fontWeight: "800", color: color.ink, letterSpacing: 1.5, lineBreak: "noWrap" }}>{label}</text>
    </node>
  );
}
