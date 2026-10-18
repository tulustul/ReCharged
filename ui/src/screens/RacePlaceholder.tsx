import type { BevyStyle } from "bevy-react/jsx";

import type { AppStatus } from "../bevy";
import { bevy } from "../bevy";
import { Button } from "../components/Button";
import { Panel } from "../components/Panel";
import { appStrings } from "../strings/app";
import { colors, fontSize } from "../theme/tokens";

// ponytail: placeholder until the race HUD (T05b) and pause menu (T05).
export function RacePlaceholder({ status }: { status: AppStatus }) {
  return (
    <Panel>
      <text style={title}>{appStrings.racePlaceholder}</text>
      <text style={phase}>{`${status.phase ?? ""}${status.paused ? " · paused" : ""}`}</text>
      <Button label={appStrings.restart} onClick={() => bevy.race.restart(null)} />
      <Button label={appStrings.backToMenu} onClick={() => bevy.race.quit(null)} />
    </Panel>
  );
}

const title: BevyStyle = { color: colors.text, fontSize: fontSize.title / 2, fontWeight: "bold" };

const phase: BevyStyle = { color: colors.textDim, fontSize: fontSize.body };
