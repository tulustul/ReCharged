import { useEffect } from "react";
import type { BevyStyle } from "bevy-react/jsx";

import { bevy } from "./bevy";
import { ErrorBoundary } from "./ErrorBoundary";
import { Loading } from "./screens/Loading";
import { MainMenu } from "./screens/MainMenu";
import { RacePlaceholder } from "./screens/RacePlaceholder";
import { screenFor } from "./state/appState";
import { useAppState } from "./state/useAppState";
import { appStrings } from "./strings/app";
import { colors, fontSize, space } from "./theme/tokens";

export function App() {
  useEffect(() => {
    bevy.ui.ready(null); // again after every hot reload: harmless, Bevy only notes it
  }, []);
  return (
    <node style={root}>
      <ErrorBoundary fallback={(error) => <ErrorScreen error={error} />}>
        <Screens />
      </ErrorBoundary>
    </node>
  );
}

function Screens() {
  const status = useAppState();
  switch (screenFor(status)) {
    case "menu":
      return <MainMenu />;
    case "loading":
      return <Loading />;
    case "race":
      return status ? <RacePlaceholder status={status} /> : null;
    default:
      return null;
  }
}

function ErrorScreen({ error }: { error: Error }) {
  return (
    <node style={errorBox}>
      <text style={errorTitle}>{appStrings.errorTitle}</text>
      <text style={errorText}>{String(error.message)}</text>
    </node>
  );
}

const root: BevyStyle = {
  width: "100%",
  height: "100%",
  justifyContent: "center",
  alignItems: "center",
};

const errorBox: BevyStyle = { flexDirection: "column", gap: space.s, padding: space.l };

const errorTitle: BevyStyle = { color: colors.danger, fontSize: fontSize.title / 2, fontWeight: "bold" };

const errorText: BevyStyle = { color: colors.text, fontSize: fontSize.body };
