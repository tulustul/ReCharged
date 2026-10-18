// UI-2 Splash: a short text card (no logos); any button or a click skips it.
import { useEffect, useRef } from "react";
import { useSharedValue, withDelay, withSequence, withTiming } from "bevy-react";

import { useNav } from "../nav/input";
import { resetTo } from "../nav/routes";
import { appStrings } from "../strings/app";
import { fill, text } from "../theme/styles";
import { color } from "../theme/tokens";

/** How long the card stays, ms. */
const SPLASH_MS = 2600;

export function Splash() {
  const fade = useSharedValue(0);
  const done = useRef(false);
  const finish = () => {
    if (done.current) return;
    done.current = true;
    resetTo("title");
  };
  useEffect(() => {
    fade.value = withSequence(
      withDelay(250, withTiming(1, { duration: 500, easing: "easeOut" })),
      withDelay(1250, withTiming(0, { duration: 450, easing: "easeIn" })),
    );
    const timer = setTimeout(finish, SPLASH_MS);
    return () => clearTimeout(timer);
  }, [fade]);
  useNav(finish);
  const [made, ...rest] = appStrings.splash.split(" with ");
  return (
    <button onClick={finish} style={{ ...fill, backgroundColor: color.bgDeep, alignItems: "center", justifyContent: "center" }}>
      <node style={{ flexDirection: "column", alignItems: "center", gap: 14, opacity: { animated: fade } }}>
        <text style={{ ...text.caps, fontSize: 20, letterSpacing: 6 }}>{`${made} with`.toUpperCase()}</text>
        <text style={{ ...text.h2, fontWeight: "600", color: color.text }}>{rest.join(" with ")}</text>
      </node>
    </button>
  );
}
