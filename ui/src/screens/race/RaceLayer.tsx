// Everything React shows during a race session: the HUD (`ui/src/hud`), the pause menu and its
// sub-screens, and the post-race flow — Results → Cup standings / Versus scoreboard → the mode's
// next step (GDD §3.3). The results wait a moment after the race ends so the finish banner shows.
import { useEffect, useState, type ReactNode } from "react";

import { bevy, type AppStatus } from "../../bevy";
import { ScreenName } from "../../nav/input";
import { createMorpher, morphStyle, router, type Route } from "../../nav/routes";
import { draft, draftFromSetup } from "../../state/draft";
import { hmrSingleton } from "../../state/store";
import { openDialog } from "../../state/overlays";
import { Hud } from "../../hud/Hud";
import { demoMode, session } from "../../state/stores";
import { useStore } from "../../state/useStore";
import { appStrings } from "../../strings/app";
import { results as words, standings as cupWords } from "../../strings/race";
import { fill } from "../../theme/styles";
import { color } from "../../theme/tokens";
import { HowToPlay } from "../HowToPlay";
import { Settings } from "../settings/Settings";
import { TrackSelect } from "../TrackSelect";
import { Pause } from "./Pause";
import { Results, type Action } from "./Results";
import { CupStandings, VersusScore } from "./Standings";

type View = "results" | "standings" | "score" | "changeTrack";

const raceMorph = hmrSingleton("__rc_raceMorph", createMorpher);
/** The finish banner's moment before the results take over. */
const RESULTS_DELAY_MS = 2600;

export function RaceLayer({ status }: { status: AppStatus }) {
  const sess = useStore(session);
  const morph = useStore(raceMorph.morph);
  const [view, setView] = useState<View>("results");
  const [sub, setSub] = useState<"howTo" | "settings" | null>(null);
  const finished = status.phase === "finished";
  const results = useAfter(finished, demoMode.get() ? 0 : RESULTS_DELAY_MS);
  const modeKind = sess.setup?.mode.kind;
  // Every race end (and a new session's mode) starts the post-race flow at the results.
  useEffect(() => setView("results"), [finished, modeKind]);
  useEffect(() => {
    if (!status.paused) setSub(null);
  }, [status.paused]);

  const show = (next: View) => raceMorph.run(() => setView(next));
  // Change track / Next track start from what this session races (laps, rivals, weather …).
  const changeTrack = () => {
    const setup = session.get().setup;
    if (setup) draft.set(draftFromSetup(draft.get(), setup));
    show("changeTrack");
  };
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
      { label: a.changeTrack, run: changeTrack },
      { label: a.menu, run: () => toMenu("main") },
    ],
    timeTrial: [
      { label: a.retry, primary: true, run: () => bevy.race.restart(null) },
      { label: a.changeTrack, run: changeTrack },
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
    { label: a.nextTrack, run: changeTrack },
    { label: a.lobby, run: () => toMenu("main", "lobby") },
    { label: a.menu, run: () => toMenu("main") },
  ];

  let content: ReactNode = null;
  if (results) {
    content = (
      <node style={{ ...fill, backgroundColor: color.scrim }}>
        {view === "results" && <Results actions={resultActions[mode]} />}
        {view === "standings" && <CupStandings actions={standingActions} />}
        {view === "score" && <VersusScore actions={scoreActions} />}
        {view === "changeTrack" && (
          <TrackSelect inRace={{ onPick: (next) => bevy.race.nextTrack(next), onBack: () => show(mode === "versus" ? "score" : "results") }} />
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
  const screen = results ? view : status.paused ? (sub === "howTo" ? "pauseHowToPlay" : sub === "settings" ? "pauseSettings" : "pause") : "race";
  return (
    <node style={fill}>
      <ScreenName name={screen} />
      {/* Hidden (kept mounted: feed, bindings and banners survive) while the pause menu is open. */}
      {!results && (
        <node style={{ ...fill, display: status.paused ? "none" : "flex" }}>
          <Hud status={status} />
        </node>
      )}
      <node style={{ ...fill, ...morphStyle(morph) }}>{content}</node>
    </node>
  );
}

/** True once `on` has held for `ms` (false again as soon as it drops). */
function useAfter(on: boolean, ms: number) {
  const [late, setLate] = useState(false);
  useEffect(() => {
    setLate(false);
    if (!on) return;
    const timer = setTimeout(() => setLate(true), ms);
    return () => clearTimeout(timer);
  }, [on, ms]);
  return on && late;
}
