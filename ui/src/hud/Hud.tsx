// The race HUD (T05b): one PlayerHud per local player's view (rects from rc_ui `viewports`, in UI
// px) plus the standings/minimap panel in the free quadrant of a 3-player split. Owns the feed
// (snapshot on mount, then events) and the per-frame shared values bound to Rust.
import { useEffect } from "react";

import type { AppStatus } from "../bevy";
import { useStore } from "../state/useStore";
import { fill } from "../theme/styles";
import { fallbackViews } from "./layout";
import { Panel } from "./Panel";
import { PlayerHud } from "./PlayerHud";
import { hudSetup, hudTick, hudViews, resetMoments, startHudFeed } from "./store";
import { demoValues, useHudValues } from "./values";

export function Hud({ status }: { status: AppStatus }) {
  const setup = useStore(hudSetup);
  const tick = useStore(hudTick);
  const views = useStore(hudViews);
  const values = useHudValues(useStore(demoValues));
  useEffect(() => startHudFeed(), []);
  // A new attempt (restart) starts clean; so does the next race.
  useEffect(() => {
    if (status.phase === "setup") resetMoments();
  }, [status.phase]);
  useEffect(() => () => resetMoments(), []);
  if (!setup || !tick) return null;
  const layout = views.players.length > 0 ? views : fallbackViews(tick.players.map((p) => p.slot));
  return (
    <node style={fill}>
      {layout.players.map((view) => (
        <PlayerHud key={view.slot} view={view} setup={setup} tick={tick} values={values.players[view.slot] ?? values.players[0]} dots={values.dots} views={layout.players.length} phase={status.phase} />
      ))}
      {layout.panel && <Panel rect={layout.panel} setup={setup} tick={tick} dots={values.dots} />}
    </node>
  );
}
