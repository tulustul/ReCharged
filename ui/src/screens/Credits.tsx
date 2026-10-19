// UI-17 Credits: team, tools, licences, rolling upwards (Bevy animates it; React renders once).
import { useEffect } from "react";
import { interpolate, useSharedValue, withRepeat, withTiming } from "bevy-react";

import { BoltIcon } from "../components/icons";
import { Hints } from "../components/Hints";
import { back } from "../nav/routes";
import { useFocusNav } from "../nav/useFocusNav";
import { hint } from "../strings/common";
import { credits, creditsText } from "../strings/credits";
import { fill, text } from "../theme/styles";
import { color, frame, space } from "../theme/tokens";

/** Roll speed, UI px per second. */
const ROLL_PX_PER_S = 57;
/** Start of the roll (UI px below its resting place: the title shows near the bottom). */
const ROLL_FROM = 260;
/** Height of the rolling column, UI px, from the layout below: top padding, title, one block per
 * section (gap, caps title, lines of h3 with their gaps), the thanks line. */
const COLUMN_PX = 120 + 72 + credits.reduce((h, s) => h + 64 + 24 + s.lines.length * 46, 0) + 64 + 40 + 46;
/** End of the roll: the thanks line has scrolled up to the top. */
const ROLL_TO = 92 - COLUMN_PX;

export function Credits() {
  useFocusNav({}, { region: "none", index: 0 }, { back, confirm: back });
  const roll = useSharedValue(0);
  useEffect(() => {
    roll.value = withRepeat(withTiming(1, { duration: ((ROLL_FROM - ROLL_TO) / ROLL_PX_PER_S) * 1000, easing: "linear" }));
  }, [roll]);
  return (
    <node style={{ ...fill, overflowX: "clip", overflowY: "clip" }}>
      <node style={{ ...fill, flexDirection: "column", alignItems: "center", transform: { translateY: { animated: interpolate(roll, [0, 1], [ROLL_FROM, ROLL_TO]) } } }}>
        <node style={{ flexDirection: "column", alignItems: "center", gap: space.xxl, padding: { top: 120 } }}>
          <node style={{ flexDirection: "row", alignItems: "center", gap: 12 }}>
            <BoltIcon size={64} />
            <text style={{ ...text.title, fontWeight: "900" }}>{creditsText.title.toUpperCase()}</text>
          </node>
          {credits.map((section) => (
            <node key={section.title} style={{ flexDirection: "column", alignItems: "center", gap: 12 }}>
              <text style={{ ...text.caps, color: color.cyan, fontSize: 20, letterSpacing: 4 }}>{section.title.toUpperCase()}</text>
              {section.lines.map((line) => (
                <text key={line} style={{ ...text.h3, fontWeight: "500", fontFamily: undefined, color: color.text, textAlign: "center" }}>
                  {line}
                </text>
              ))}
            </node>
          ))}
          <text style={{ ...text.h2, color: color.volt, margin: { top: space.xl } }}>{creditsText.thanks.toUpperCase()}</text>
        </node>
      </node>
      <node style={{ positionType: "absolute", left: 0, right: 0, top: 0, height: 200, backgroundGradient: { type: "linear", angle: 180, stops: [{ color: "rgba(4, 7, 15, 0.95)" }, { color: "rgba(4, 7, 15, 0)" }] } }} />
      <node style={{ positionType: "absolute", left: 0, right: 0, bottom: 0, height: 220, backgroundGradient: { type: "linear", angle: 0, stops: [{ color: "rgba(4, 7, 15, 0.95)" }, { color: "rgba(4, 7, 15, 0)" }] } }} />
      <node style={{ positionType: "absolute", right: frame.x, bottom: frame.bottom }}>
        <Hints items={[{ action: "back", label: hint.back, onPress: back }]} />
      </node>
    </node>
  );
}
