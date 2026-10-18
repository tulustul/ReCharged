// UI-3 Title: the animated ReCharged logo, "Press any button", version. Any button (or a click)
// continues; the device used becomes Player 1's glyph device.
import { useEffect, useRef } from "react";
import { interpolate, interpolateColor, useSharedValue, withRepeat, withSequence, withTiming } from "bevy-react";

import { BoltIcon } from "../components/icons";
import { setDevice, useNav } from "../nav/input";
import { resetTo } from "../nav/routes";
import { sfx } from "../sound";
import { appInfo } from "../state/stores";
import { useStore } from "../state/useStore";
import { appStrings } from "../strings/app";
import { fill, text } from "../theme/styles";
import { color, frame, size } from "../theme/tokens";

/** Width of the charge bar under the logo, px. */
const BAR = 760;

export function Title() {
  const info = useStore(appInfo);
  const charge = useSharedValue(0);
  const pulse = useSharedValue(1);
  const done = useRef(false);
  useEffect(() => {
    charge.value = withRepeat(
      withSequence(withTiming(1, { duration: 1500, easing: "easeInOut" }), withTiming(1, { duration: 900 }), withTiming(0, { duration: 1 })),
    );
    pulse.value = withRepeat(withTiming(0.35, { duration: 900, easing: "easeInOut" }), { reverse: true });
  }, [charge, pulse]);
  const go = () => {
    if (done.current) return;
    done.current = true;
    sfx("select");
    resetTo("main");
  };
  useNav((e) => {
    if (e.slot === null) setDevice(0, e.device);
    go();
  });
  const hero = { ...text.hero, textShadow: { color: "#06263d", offsetX: 7, offsetY: 7 } };
  return (
    <button onClick={go} style={{ ...fill, backgroundColor: color.clear, alignItems: "center", justifyContent: "center" }}>
      <node style={{ flexDirection: "column", alignItems: "center", gap: 26, margin: { bottom: 80 } }}>
        <node style={{ flexDirection: "row", alignItems: "center", gap: 18 }}>
          <BoltIcon size={128} />
          <text style={hero}>
            {appStrings.nameRe}
            <text style={{ ...hero, color: color.cyan }}>{appStrings.nameCharged}</text>
          </text>
        </node>
        {/* The charge bar fills from the left (scale + shift: transforms pivot on the centre). */}
        <node style={{ width: BAR, height: 12, borderRadius: 6, backgroundColor: "rgba(40, 64, 120, 0.5)" }}>
          <node
            style={{
              width: BAR,
              height: 12,
              borderRadius: 6,
              backgroundGradient: { type: "linear", angle: 90, stops: [{ color: color.cyan }, { color: color.volt }] },
              transform: {
                scaleX: { animated: charge },
                translateX: { animated: interpolate(charge, [0, 1], [-BAR / 2, 0]) },
              },
            }}
          />
        </node>
        <text style={{ ...text.h3, fontWeight: "500", color: color.textDim }}>{appStrings.tagline}</text>
      </node>
      <node style={{ positionType: "absolute", left: 0, right: 0, bottom: 150, justifyContent: "center" }}>
        {/* A colour pulse, not opacity: an animated opacity would make the text its own layer. */}
        <text style={{ ...text.h3, fontSize: size.h3, letterSpacing: 3, color: { animated: interpolateColor(pulse, [0, 1], ["#55617e", "#eef2fb"]) } }}>
          {appStrings.pressAnyButton.toUpperCase()}
        </text>
      </node>
      {info && (
        <text style={{ ...text.small, positionType: "absolute", right: frame.x, bottom: frame.bottom, color: color.textFaint }}>
          {appStrings.version(info.version)}
        </text>
      )}
    </button>
  );
}
