// The front end: one screen at a time per app state (menus, loading, race), dialogs and toasts on
// top, the FPS meter above all. Every screen sits in an ErrorBoundary that reports to Bevy
// (`ui.error`) and offers a way back.
import { useEffect, useState } from "react";

import { bevy } from "./bevy";
import { Backdrop } from "./components/Backdrop";
import { Button } from "./components/Button";
import { DialogHost } from "./components/Dialog";
import { ToastHost } from "./components/Toast";
import { useDebugRoutes } from "./debug";
import { EditorLayer } from "./editor/EditorLayer";
import { SplitContext, useEvent } from "./hooks";
import { ErrorBoundary } from "./ErrorBoundary";
import { ScreenName, useNav } from "./nav/input";
import { back, menuMorph, morphStyle, router, type Route } from "./nav/routes";
import { top } from "./nav/router";
import { CarSelect } from "./screens/CarSelect";
import { Credits } from "./screens/Credits";
import { CupEnd } from "./screens/CupEnd";
import { CupSelect } from "./screens/CupSelect";
import { Garage } from "./screens/Garage";
import { HowToPlay } from "./screens/HowToPlay";
import { Loading } from "./screens/Loading";
import { Lobby } from "./screens/Lobby";
import { MainMenu } from "./screens/MainMenu";
import { RaceLayer } from "./screens/race/RaceLayer";
import { Records } from "./screens/Records";
import { Settings } from "./screens/settings/Settings";
import { Splash } from "./screens/Splash";
import { Title } from "./screens/Title";
import { TrackSelect } from "./screens/TrackSelect";
import { Feeds } from "./state/Feeds";
import { appState, editor, session, settings } from "./state/stores";
import { useStore } from "./state/useStore";
import { appStrings } from "./strings/app";
import { hint } from "./strings/common";
import { fill, text } from "./theme/styles";
import { color, radius, space } from "./theme/tokens";

export function App() {
  useEffect(() => {
    bevy.ui.ready(null); // again after every hot reload: harmless, Bevy only notes it
  }, []);
  useDebugRoutes();
  // Split-screen races host the UI on a transparent overlay camera: no 3D frame to blur, so
  // every panel (dialogs included) switches to translucent fills.
  const status = useStore(appState);
  const humans =
    useStore(session).setup?.racers.filter((r) => r.slot !== null).length ?? 1;
  const split = status?.state === "inRace" && humans > 1;
  return (
    <SplitContext.Provider value={split}>
      <node style={{ width: "100%", height: "100%" }}>
        <Feeds />
        <ErrorBoundary fallback={(error) => <ScreenError error={error} />}>
          <Main />
        </ErrorBoundary>
        <DialogHost />
        <ToastHost />
        <FpsMeter />
      </node>
    </SplitContext.Provider>
  );
}

function Main() {
  const status = useStore(appState);
  const editing = useStore(editor);
  // The track editor has no menus or loading screen: its own screen everywhere but test drives.
  if (editing && !editing.driving) {
    return (
      <>
        <ScreenName name="editor" />
        <EditorLayer status={editing} />
      </>
    );
  }
  switch (status?.state) {
    case "loading":
      return (
        <>
          <ScreenName name="loading" />
          <Loading />
        </>
      );
    case "unloading":
      // One frame between a race and the menu (or the next load): no flash of either screen.
      return (
        <node style={{ ...fill, backgroundColor: color.bg }}>
          <ScreenName name="unloading" />
        </node>
      );
    case "inRace":
      return <RaceLayer status={status} />;
    default:
      return (
        <>
          <Backdrop />
          <MenuHost />
        </>
      );
  }
}

function MenuHost() {
  const route = top(useStore(router));
  const morph = useStore(menuMorph.morph);
  return (
    <node style={{ ...fill, ...morphStyle(morph) }}>
      <ScreenName name={route} />
      <ErrorBoundary
        key={route}
        fallback={(error) => <ScreenError error={error} />}
      >
        <ScreenFor route={route} />
      </ErrorBoundary>
    </node>
  );
}

function ScreenFor({ route }: { route: Route }) {
  switch (route) {
    case "splash":
      return <Splash />;
    case "title":
      return <Title />;
    case "main":
      return <MainMenu />;
    case "cupSelect":
      return <CupSelect />;
    case "cupEnd":
      return <CupEnd />;
    case "trackSelect":
      return <TrackSelect />;
    case "carSelect":
      return <CarSelect />;
    case "lobby":
      return <Lobby />;
    case "garage":
      return <Garage />;
    case "records":
      return <Records />;
    case "howToPlay":
      return <HowToPlay onBack={back} />;
    case "settings":
      return <Settings onBack={back} />;
    case "credits":
      return <Credits />;
  }
}

/** The FPS meter setting's readout: `ui.fps` arrives twice a second while it is on. */
function FpsMeter() {
  const shown = useStore(settings)?.gameplay.fpsMeter ?? false;
  const [fps, setFps] = useState<number | null>(null);
  useEvent("ui.fps", (e) => setFps(e.fps));
  if (!shown || fps === null) return null;
  return (
    <node
      style={{
        positionType: "absolute",
        top: space.xs,
        right: space.s,
        padding: { horizontal: space.s, vertical: 2 },
        borderRadius: radius.s,
        backgroundColor: color.scrim,
      }}
    >
      <text style={{ ...text.caps, color: color.text }}>{`${fps} FPS`}</text>
    </node>
  );
}

/** Fallback of a broken screen: what happened and a way back. */
function ScreenError({ error }: { error: Error }) {
  useNav((e) => {
    if (e.action === "back" || e.action === "confirm") back();
  });
  return (
    <node style={{ ...fill, alignItems: "center", justifyContent: "center" }}>
      <node
        style={{
          width: 900,
          flexDirection: "column",
          gap: space.m,
          alignItems: "flexStart",
        }}
      >
        <text style={{ ...text.h2, color: color.danger }}>
          {appStrings.errorTitle}
        </text>
        <text style={text.body}>{appStrings.errorHint}</text>
        <text style={{ ...text.small, color: color.textFaint }}>
          {String(error.message)}
        </text>
        <Button
          label={hint.back}
          focused
          onPress={back}
          style={{ margin: { top: space.m } }}
        />
      </node>
    </node>
  );
}
