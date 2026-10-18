// Everything React shows during a race session: the HUD (T05b; a small status chip until then),
// the pause menu and its sub-screens, and the post-race flow — Results → Cup standings / Versus
// scoreboard → the mode's next step (GDD §3.3).
import { useEffect, useState } from "react";

import { bevy, type AppStatus } from "../../bevy";
import { createMorpher, morphStyle, router, type Route } from "../../nav/routes";
import { hmrSingleton } from "../../state/store";
import { openDialog } from "../../state/overlays";
import { catalog, session } from "../../state/stores";
import { useStore } from "../../state/useStore";
import { appStrings } from "../../strings/app";
import { results as words, standings as cupWords } from "../../strings/race";
import { fill, text } from "../../theme/styles";
import { color, frame, radius } from "../../theme/tokens";
import { HowToPlay } from "../HowToPlay";
import { Settings } from "../settings/Settings";
import { TrackSelect } from "../TrackSelect";
import { Pause } from "./Pause";
import { Results, type Action } from "./Results";
import { CupStandings, VersusScore } from "./Standings";

type View = "results" | "standings" | "score" | "changeTrack";

const raceMorph = hmrSingleton("__rc_raceMorph", createMorpher);

export function RaceLayer({ status }: { status: AppStatus }) {
  const sess = useStore(session);
  const morph = useStore(raceMorph.morph);
  const [view, setView] = useState<View>("results");
  const [sub, setSub] = useState<"howTo" | "settings" | null>(null);
  const finished = status.phase === "finished";
  const modeKind = sess.setup?.mode.kind;
  // Every race end (and a new session's mode) starts the post-race flow at the results.
  useEffect(() => setView("results"), [finished, modeKind]);
  useEffect(() => {
    if (!status.paused) setSub(null);
  }, [status.paused]);

  const show = (next: View) => raceMorph.run(() => setView(next));
  const toMenu = (...stack: Route[]) => {
    router.set(stack);
    bevy.race.quit(null);
  };
  const mode = sess.setup?.mode.kind ?? "quickRace";
  const cupDone = !!sess.cup?.finished || !!sess.results?.outcome?.cupFinished;
  const a = words.actions;
  const quitCup = () =>
    openDialog({
      title: cupWords.quitTitle,
      text: cupWords.quitText,
      actions: [{ label: cupWords.quitCup, tone: "danger", run: () => toMenu("main", "cupSelect") }, { label: appStrings.cancel }],
      cancel: 1,
      focus: 1,
    });
  const resultActions: Record<typeof mode, Action[]> = {
    championship: [
      { label: a.continue, primary: true, run: () => show("standings") },
      { label: a.restart, run: () => bevy.race.restart(null) },
    ],
    quickRace: [
      { label: a.restart, primary: true, run: () => bevy.race.restart(null) },
      { label: a.changeTrack, run: () => show("changeTrack") },
      { label: a.menu, run: () => toMenu("main") },
    ],
    timeTrial: [
      { label: a.retry, primary: true, run: () => bevy.race.restart(null) },
      { label: a.changeTrack, run: () => show("changeTrack") },
      { label: a.menu, run: () => toMenu("main") },
    ],
    versus: [{ label: a.continue, primary: true, run: () => show("score") }],
  };
  const standingActions: Action[] = cupDone
    ? [
        {
          label: cupWords.toCeremony,
          primary: true,
          run: () => {
            // rc_game returns to the menu with the finished cup; the ceremony shows there.
            router.set(["main", "cupSelect", "cupEnd"]);
            bevy.race.next(null);
          },
        },
      ]
    : [
        { label: cupWords.nextRace, primary: true, run: () => bevy.race.next(null) },
        { label: cupWords.quitCup, run: quitCup },
      ];
  const scoreActions: Action[] = [
    { label: a.rematch, primary: true, run: () => bevy.race.rematch(null) },
    { label: a.nextTrack, run: () => show("changeTrack") },
    { label: a.lobby, run: () => toMenu("main", "lobby") },
    { label: a.menu, run: () => toMenu("main") },
  ];

  let content = <StatusChip status={status} />;
  if (finished) {
    content = (
      <node style={{ ...fill, backgroundColor: color.scrim }}>
        {view === "results" && <Results actions={resultActions[mode]} />}
        {view === "standings" && <CupStandings actions={standingActions} />}
        {view === "score" && <VersusScore actions={scoreActions} />}
        {view === "changeTrack" && (
          <TrackSelect inRace={{ onPick: (variant) => bevy.race.nextTrack(variant), onBack: () => show(mode === "versus" ? "score" : "results") }} />
        )}
      </node>
    );
  } else if (status.paused) {
    content =
      sub === "howTo" ? (
        <node style={{ ...fill, backgroundColor: color.scrim }}>
          <HowToPlay onBack={() => setSub(null)} />
        </node>
      ) : sub === "settings" ? (
        <node style={{ ...fill, backgroundColor: color.scrim }}>
          <Settings onBack={() => setSub(null)} />
        </node>
      ) : (
        <Pause onHowTo={() => setSub("howTo")} onSettings={() => setSub("settings")} />
      );
  }
  return <node style={{ ...fill, ...morphStyle(morph) }}>{content}</node>;
}

/** ponytail: a status chip stands in for the race HUD until T05b builds it. */
function StatusChip({ status }: { status: AppStatus }) {
  const setup = useStore(session).setup;
  const track = useStore(catalog)?.tracks.find((t) => t.id === setup?.track)?.name ?? setup?.track ?? "";
  return (
    <node style={{ positionType: "absolute", left: frame.x / 2, top: frame.top / 2, padding: { horizontal: 16, vertical: 8 }, borderRadius: radius.m, backgroundColor: "rgba(4, 8, 20, 0.6)" }}>
      <text style={{ ...text.caps, color: color.textDim }}>{`${track} · ${status.phase ?? ""}`.toUpperCase()}</text>
    </node>
  );
}
