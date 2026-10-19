// UI-2 Splash: a short text card (no logos); any button or a click skips it. Its clock is Bevy's
// frame clock (the fade's completion ends it), not a wall-clock timer: when the first frames stall
// (pipelines compiling in automated runs) the card still shows once frames are presented.
import { useEffect, useRef } from "react";
import { useSharedValue, withDelay, withSequence, withTiming } from "bevy-react";

import { useNav } from "../nav/input";
import { resetTo } from "../nav/routes";
import { appStrings } from "../strings/app";
import { fill, text } from "../theme/styles";
import { color } from "../theme/tokens";

/** How long the card stays after fading out, ms (2.6 s in all). */
const TAIL_MS = 150;

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
      withTiming(0, { duration: TAIL_MS }),
      (completed) => completed && finish(),
    );
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
