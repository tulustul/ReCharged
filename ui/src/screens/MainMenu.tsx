import type { BevyStyle } from "bevy-react/jsx";

import { bevy } from "../bevy";
import { Button } from "../components/Button";
import { Panel } from "../components/Panel";
import { appStrings } from "../strings/app";
import { colors, fontSize } from "../theme/tokens";

/** The dev test track every subsystem tests on. */
const TEST_TRACK = "test_oval";

export function MainMenu() {
  return (
    <Panel>
      <text style={title}>{appStrings.title}</text>
      <text style={tagline}>{appStrings.tagline}</text>
      <Button label={appStrings.testDrive} onClick={() => bevy.race.start({ track: TEST_TRACK })} />
      <Button label={appStrings.quit} onClick={() => bevy.app.quit(null)} />
    </Panel>
  );
}

const title: BevyStyle = { color: colors.cyan, fontSize: fontSize.title, fontWeight: "bold" };

const tagline: BevyStyle = { color: colors.volt, fontSize: fontSize.tagline, margin: { bottom: 16 } };
