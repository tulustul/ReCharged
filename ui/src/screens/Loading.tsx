// UI-10 Loading: track name and art, mode, a tip, progress (`loading.progress`).
import { useState } from "react";

import { Spinner } from "../components/bits";
import { ChargeLine } from "../components/Screen";
import { TrackArt } from "../components/TrackArt";
import { layoutLabel, percent } from "../format";
import { catalog, loadingProgress, session } from "../state/stores";
import { useStore } from "../state/useStore";
import { timeName, weatherName, word } from "../strings/common";
import { loading, race } from "../strings/race";
import { fill, text } from "../theme/styles";
import { color, frame, radius, space } from "../theme/tokens";

/** Width of the progress bar, px. */
const BAR = 1100;

export function Loading() {
  const setup = useStore(session).setup;
  const cat = useStore(catalog);
  const progress = useStore(loadingProgress);
  const [tip] = useState(() => loading.tips[Math.floor(Math.random() * loading.tips.length)]);
  const track = cat?.tracks.find((t) => t.id === setup?.track);
  const mode = setup?.mode;
  const kicker = mode
    ? mode.kind === "championship"
      ? `${cat?.cups.find((c) => c.id === mode.cup)?.name ?? race.modeName.championship} · ${race.raceOf(mode.race + 1, cat?.cups.find((c) => c.id === mode.cup)?.races.length ?? 4)}`
      : race.modeName[mode.kind]
    : loading.title;
  const weather = setup?.weather.kind === "fixed" ? weatherName[setup.weather.weather] : setup?.weather.kind === "dynamic" ? word.dynamic : word.random;
  return (
    <node style={{ ...fill, backgroundGradient: { type: "linear", angle: 160, stops: [{ color: "#0c1836" }, { color: "#050913" }] } }}>
      <node style={{ ...fill, flexDirection: "column", justifyContent: "spaceBetween", padding: { left: frame.x, right: frame.x, top: 150, bottom: frame.bottom + 20 } }}>
        <node style={{ flexDirection: "row", justifyContent: "spaceBetween", alignItems: "center", gap: space.xl }}>
          <node style={{ flexDirection: "column", gap: 14, flexShrink: 1 }}>
            <text style={{ ...text.caps, color: color.cyan, fontSize: 20 }}>{kicker.toUpperCase()}</text>
            <text style={{ ...text.hero, fontSize: 112, lineBreak: "wordBoundary" }}>{(track?.name ?? setup?.track ?? "").toUpperCase()}</text>
            <ChargeLine width={240} />
            {setup && (
              <text style={{ ...text.h3, fontWeight: "500", color: color.textDim }}>
                {[layoutLabel(setup.reverse, setup.mirror, word), `${setup.laps} ${word.laps.toLowerCase()}`, weather, timeName[setup.time]].join("  ·  ")}
              </text>
            )}
          </node>
          {track && <TrackArt track={track} width={800} height={450} title />}
        </node>
        <node style={{ flexDirection: "column", gap: space.l }}>
          <node style={{ flexDirection: "row", alignItems: "center", gap: space.m, maxWidth: 1200 }}>
            <text style={{ ...text.caps, color: color.volt }}>{loading.tip.toUpperCase()}</text>
            <text style={{ ...text.body, color: color.text }}>{tip}</text>
          </node>
          <node style={{ flexDirection: "row", alignItems: "center", gap: space.l }}>
            <Spinner />
            <node style={{ width: BAR, height: 10, borderRadius: radius.pill, backgroundColor: "rgba(40, 64, 120, 0.5)" }}>
              <node
                style={{
                  width: BAR,
                  height: 10,
                  borderRadius: radius.pill,
                  backgroundGradient: { type: "linear", angle: 90, stops: [{ color: color.cyan }, { color: color.volt }] },
                  // The glow also compiles bevy_ui's box-shadow pipeline for this LDR camera, which
                  // split-screen races share: their HUD glows then never compile mid-race.
                  boxShadow: [{ color: color.cyanGlow, blurRadius: 14 }],
                  transform: { scaleX: Math.max(progress, 0.02), translateX: (-(1 - Math.max(progress, 0.02)) * BAR) / 2 },
                  transition: { transform: { duration: 350, easing: "easeOut" } },
                }}
              />
            </node>
            <text style={{ ...text.label, width: 80 }}>{percent(progress)}</text>
          </node>
        </node>
      </node>
    </node>
  );
}
