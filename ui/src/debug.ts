// Scripted steps for automated screenshots (`RECHARGED_UI_SCRIPT`, see rc_ui `debug.rs`):
// `go <route> [mode]` jumps to a screen, `demo <scene>` fills the stores with sample data for
// screens whose data only exists after real races (`demo hud …`: the race HUD, see `hud/demo.ts`),
// `tab <id>` is handled by tabbed screens, `editor <command>` / `editor select <kind> <index>` /
// `editor tab <tab>` / `editor move x y z` / `editor place <asset> [x y z]` … drive the track editor.
// Navigation is scripted through the real input path (`--input-script`,
// `docs/design/automation.md`).
import { bevy, type AppStatus, type EditorAsset, type EditorCommand, type EditorGizmo, type EditorItem, type EditorTab, type UiCupStandings, type UiResults, type UiVersusScore } from "./bevy";
import { libraryQuery } from "./editor/search";
import { useDebug } from "./hooks";
import { demoHud, endHudDemo } from "./hud/demo";
import { resetTo, type Route } from "./nav/routes";
import { draft, type MenuMode } from "./state/draft";
import { lobbyChoices } from "./state/lobby";
import { openDialog, pushToast } from "./state/overlays";
import { appState, catalog, demoMode, loadingProgress, lobby, profile, session } from "./state/stores";

const RIVALS = ["Dash", "Mayhem", "Penny", "Turbo Tom", "Glitch", "Rusty", "Nova"];
const COLORS: [number, number, number][] = [
  [0, 0.85, 1],
  [1, 0.55, 0.12],
  [0.9, 0.12, 0.16],
  [1, 0.45, 0.4],
  [0.15, 0.4, 1],
  [0.6, 0.3, 1],
  [0.72, 0.39, 0.18],
  [0.91, 0.92, 0.95],
];
const RIVAL_CARS = ["sparky", "dumpster", "zip", "zip", "sparky", "sparky", "zip"];

function sampleResults(points: boolean): UiResults {
  const names = ["You", ...RIVALS];
  const order = [2, 0, 3, 1, 4, 6, 5, 7];
  return {
    entries: order.map((i, rank) => ({
      rank: rank + 1,
      name: names[i],
      car: i === 0 ? "zip" : RIVAL_CARS[i - 1],
      slot: i === 0 ? 0 : null,
      finished: rank < 7,
      timeMs: 112_430 + rank * 1840 + (rank > 3 ? 2100 : 0),
      bestLapMs: 36_820 + rank * 410,
      points: points ? [10, 8, 6, 5, 4, 3, 2, 1][rank] : null,
    })),
    outcome: {
      newBestLap: true,
      newBestRace: false,
      medal: null,
      nextMedal: null,
      ghostSaved: false,
      recordsDiscarded: false,
      unlocked: [],
      cupFinished: false,
    },
  };
}

function sampleStandings(finished: boolean): UiCupStandings {
  const names = ["You", ...RIVALS];
  const points = finished ? [34, 31, 30, 22, 18, 12, 9, 6] : [16, 18, 13, 10, 8, 6, 4, 2];
  const rows = names
    .map((name, i) => ({
      name,
      car: i === 0 ? "zip" : RIVAL_CARS[i - 1],
      slot: i === 0 ? 0 : null,
      color: COLORS[i],
      points: points[i],
      gained: [10, 8, 6, 5, 4, 3, 2, 1][i % 8],
      positions: finished ? [1, 3, 2, 1].map((p) => ((p + i) % 8) + 1) : [2, 1].map((p) => ((p + i) % 8) + 1),
    }))
    .sort((a, b) => b.points - a.points);
  return { cup: "rookie_cup", played: finished ? 4 : 2, finished, finalPosition: finished ? 1 : null, trophy: finished ? "gold" : null, rows };
}

const sampleScore: UiVersusScore = {
  races: 3,
  rows: [
    { slot: 1, points: 26, gained: 10, wins: 2, dropped: false },
    { slot: 0, points: 22, gained: 8, wins: 1, dropped: false },
    { slot: 3, points: 14, gained: 5, wins: 0, dropped: false },
    { slot: 2, points: 9, gained: 2, wins: 0, dropped: false },
  ],
};

function inRace(over: Partial<AppStatus>) {
  const now = appState.get();
  appState.set({ state: "inRace", phase: "finished", paused: false, showroom: now?.showroom ?? false, ...over });
}

function inMenu() {
  const now = appState.get();
  appState.set({ state: "menu", phase: null, paused: false, showroom: now?.showroom ?? false });
}

function setup(mode: "quickRace" | "timeTrial" | "versus" | "championship") {
  const humans = mode === "versus" ? 3 : 1;
  session.update((s) => ({
    ...s,
    setup: {
      mode: mode === "championship" ? { kind: "championship", cup: "rookie_cup", race: 1 } : { kind: mode },
      track: "toy_town",
      reverse: false,
      mirror: false,
      laps: 3,
      weather: { kind: "fixed", weather: "clear" },
      time: "morning",
      items: true,
      difficulty: "pro",
      racers: Array.from({ length: 8 }, (_, i) => ({
        name: i < humans ? `P${i + 1}` : RIVALS[i - humans],
        car: "zip",
        slot: i < humans ? i : null,
        color: [0, 0.85, 1] as [number, number, number],
        marker: i < humans ? { kind: "player" as const, slot: i } : { kind: "shape" as const, shape: "circle" as const },
      })),
    },
  }));
}

export function useDebugRoutes() {
  useDebug("editor", (arg) => {
    // `editor select point 3`, `editor tab lighting`, `editor lookAt x y z tx ty tz`,
    // `editor gizmo rotate`, `editor move|rotate|scale x y z [hold]` (a gizmo drag of the
    // selection: hold = live, uncommitted), `editor topView`, `editor drive` (simple commands
    // by kind).
    const [verb, kind, index] = arg.split(" ");
    const n = arg.split(" ").slice(1).map(Number);
    const v: [number, number, number] = [n[0], n[1], n[2]];
    const hold = arg.endsWith(" hold");
    const none: [number, number, number] = [0, 0, 0];
    const one: [number, number, number] = [1, 1, 1];
    if (verb === "select") bevy.editor.select({ item: { kind, index: Number(index) } as EditorItem });
    else if (verb === "tab") bevy.editor.tab({ tab: kind as EditorTab });
    else if (verb === "lookAt") bevy.editor.command({ command: { kind: "lookAt", from: v, at: [n[3], n[4], n[5]] } });
    else if (verb === "gizmo") bevy.editor.command({ command: { kind: "gizmo", mode: kind as EditorGizmo } });
    else if (verb === "move") bevy.editor.command({ command: { kind: "drag", translate: v, rotate: none, scale: one, hold } });
    else if (verb === "rotate") bevy.editor.command({ command: { kind: "drag", translate: none, rotate: v, scale: one, hold } });
    else if (verb === "scale") bevy.editor.command({ command: { kind: "drag", translate: none, rotate: none, scale: v, hold } });
    else if (verb === "search") libraryQuery.set(arg.slice("search ".length));
    else if (verb === "place" || verb === "swap") {
      // `editor place prop toy_bear [x y z]`, `editor place decor models/props/toy/sign.glb …`,
      // `editor place itemRow …`, `editor swap prop toy_duck`: without a position, place arms.
      const words = arg.split(" ").slice(1);
      const named = kind === "prop" || kind === "decor";
      const asset = (kind === "prop" ? { kind, id: words[1] } : kind === "decor" ? { kind, model: words[1] } : { kind }) as EditorAsset;
      const at = words.slice(named ? 2 : 1).map(Number);
      if (verb === "swap") bevy.editor.command({ command: { kind: "swap", asset } });
      else bevy.editor.place({ asset, at: at.length === 3 ? [at[0], at[1], at[2]] : null });
    } else bevy.editor.command({ command: { kind: verb } as EditorCommand });
  });
  useDebug("go", (arg) => {
    const [route, mode] = arg.split(" ") as [Route, MenuMode | undefined];
    if (mode) draft.update((d) => ({ ...d, mode }));
    if (route === "carSelect" && !draft.get().track) draft.update((d) => ({ ...d, track: catalog.get()?.tracks[0]?.id ?? "" }));
    if (appState.get()?.state === "inRace" && demoMode.get()) inMenu();
    resetTo(...(route === "splash" || route === "title" || route === "main" ? [route] : (["main", route] as Route[])));
  });
  useDebug("demo", (scene) => {
    demoMode.set(true);
    if (scene === "hud" || scene.startsWith("hud ")) {
      const phase = demoHud(scene.slice(3));
      // Over a real race the app state stays real; otherwise pretend one runs.
      if (appState.get()?.state !== "inRace") inRace({ phase });
      return;
    }
    switch (scene) {
      case "menu":
        demoMode.set(false);
        endHudDemo();
        return inMenu();
      case "results":
        setup("quickRace");
        session.update((s) => ({ ...s, results: sampleResults(false) }));
        return inRace({});
      case "cupResults":
        setup("championship");
        session.update((s) => ({ ...s, results: sampleResults(true), cup: sampleStandings(false) }));
        return inRace({});
      case "versusResults":
        setup("versus");
        session.update((s) => ({ ...s, results: sampleResults(true), versus: sampleScore }));
        return inRace({});
      case "pause":
        setup("quickRace");
        return inRace({ phase: "racing", paused: true });
      case "racing":
        setup("quickRace");
        return inRace({ phase: "racing" });
      case "loading":
        setup("championship");
        inRace({ state: "loading", phase: null });
        setTimeout(() => loadingProgress.set(0.6), 300);
        return;
      case "cupEnd":
        inMenu();
        session.update((s) => ({ ...s, cup: sampleStandings(true) }));
        return resetTo("main", "cupSelect", "cupEnd");
      case "lobby":
        inMenu();
        lobby.set({ open: true, maxSlots: 4, slots: [{ slot: 0, device: "keyboardA" }, { slot: 1, device: "gamepad" }, { slot: 2, device: "keyboardB" }] });
        resetTo("main", "lobby");
        // After the lobby mounted (it clears ready flags on entry).
        setTimeout(() => lobbyChoices.set({ 0: { car: 0, paint: 0, row: "car", ready: true }, 1: { car: 4, paint: 1, row: "paint", ready: false }, 2: { car: 2, paint: 2, row: "car", ready: false } }), 600);
        return;
      case "unlocks": {
        const p = profile.get();
        if (p) profile.set({ ...p, fresh: [{ kind: "car", id: "torque" }, { kind: "track", id: "maple_street" }, { kind: "cup", id: "pro_cup" }] });
        pushToast({ tone: "unlock", title: "Unlocked!", text: "New car: Torque" });
        pushToast({ tone: "success", title: "Golden Battery found!", text: "Toy Town — saved to your profile." });
        return;
      }
      case "dialog":
        openDialog({ title: "P2's controller disconnected", text: "Press a button on any controller, or Enter, to take over.", icon: "controller", actions: [{ label: "Resume", tone: "primary" }, { label: "Drop player" }] });
        return;
    }
  });
}
