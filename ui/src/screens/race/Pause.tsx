// UI-12 Pause: Resume, Restart, How to play, Settings, Quit (confirm). Bevy opens it (rc_input →
// PauseRequest); React closes it with the idempotent `race.resume`. Back resumes.
import { bevy } from "../../bevy";
import { Button } from "../../components/Button";
import { Hints } from "../../components/Hints";
import { Panel } from "../../components/Panel";
import { ChargeLine } from "../../components/Screen";
import { useEnter } from "../../hooks";
import { useFocusNav } from "../../nav/useFocusNav";
import { router } from "../../nav/routes";
import { sfx } from "../../sound";
import { openDialog } from "../../state/overlays";
import { catalog, session } from "../../state/stores";
import { useStore } from "../../state/useStore";
import { appStrings } from "../../strings/app";
import { hint } from "../../strings/common";
import { pause, race } from "../../strings/race";
import { fill, ownsPointer, text } from "../../theme/styles";
import { color, frame, layer, space } from "../../theme/tokens";

type Props = { onHowTo: () => void; onSettings: () => void };

export function Pause({ onHowTo, onSettings }: Props) {
  const setup = useStore(session).setup;
  const cat = useStore(catalog);
  const cup = setup?.mode.kind === "championship";
  const resume = () => bevy.race.resume(null);
  const quit = () =>
    openDialog({
      title: cup ? pause.quitCupTitle : pause.quitTitle,
      text: cup ? pause.quitCupText : pause.quitText,
      actions: [
        {
          label: pause.quit,
          tone: "danger",
          run: () => {
            if (cup) router.set(["main", "cupSelect"]);
            bevy.race.quit(null);
          },
        },
        { label: appStrings.cancel },
      ],
      cancel: 1,
      focus: 1,
    });
  const entries = [
    { label: pause.resume, run: resume },
    { label: pause.restart, run: () => bevy.race.restart(null) },
    { label: pause.howToPlay, run: onHowTo },
    { label: pause.settings, run: onSettings },
    { label: pause.quit, run: quit },
  ];
  const pick = (i: number) => {
    sfx("select");
    entries[i].run();
  };
  const nav = useFocusNav({ items: { count: entries.length, cols: 1, wrap: true } }, { region: "items", index: 0 }, { confirm: (f) => pick(f.index), back: resume }, layer.overlay);
  const enter = useEnter(-50, 0);
  const track = cat?.tracks.find((t) => t.id === setup?.track)?.name ?? "";
  return (
    <node style={{ ...fill, backgroundColor: color.scrim, focusPolicy: "block", padding: { left: frame.x, top: frame.top, bottom: frame.bottom } }} hoverStyle={ownsPointer}>
      <node style={{ ...enter, flexDirection: "column", gap: space.l, height: "100%" }}>
        <node style={{ flexDirection: "column", gap: 10 }}>
          <text style={{ ...text.caps, color: color.cyan }}>{`${setup ? race.modeName[setup.mode.kind] : ""} · ${track}`.toUpperCase()}</text>
          <text style={text.title}>{pause.title.toUpperCase()}</text>
          <ChargeLine />
        </node>
        <Panel style={{ width: 560, gap: 10 }} padding={24}>
          {entries.map((e, i) => (
            <Button key={e.label} label={e.label} variant="menu" focused={nav.is("items", i)} onHover={() => nav.hover("items", i)} onPress={() => pick(i)} />
          ))}
        </Panel>
        <node style={{ flexGrow: 1 }} />
        <Hints
          items={[
            { action: "confirm", label: hint.select, onPress: () => pick(nav.focus.index) },
            { action: "back", label: pause.resume, onPress: resume },
          ]}
        />
      </node>
    </node>
  );
}
