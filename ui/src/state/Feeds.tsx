// Keeps the stores current: every store is seeded by a snapshot request on mount (events are not
// buffered) and then follows Bevy events; app-state changes re-request what they invalidate.
// Mounted once by App; hot reload re-runs the effects (cleanups unsubscribe).
import { useEffect } from "react";

import { bevy, on, type UiNotice, type UiUnlockedItem } from "../bevy";
import { dispatchNav, setDevice } from "../nav/input";
import { router } from "../nav/routes";
import { top } from "../nav/router";
import { sfx } from "../sound";
import { appStrings } from "../strings/app";
import { word } from "../strings/common";
import { closeDialog, openDialog, pushToast } from "./overlays";
import { hmrSingleton } from "./store";
import { appInfo, appState, bindings, catalog, demoMode, editor, loadingProgress, lobby, profile, records, session, settings } from "./stores";
import { unlockLabel } from "./unlocks";
import { useStore } from "./useStore";

const fail = (what: string) => (error: unknown) => console.error(`${what} failed:`, error);

/** Open "controller disconnected" dialogs by slot: taking the slot over closes them. */
const disconnected = hmrSingleton("__rc_disconnected", () => new Map<number, number>());

export function refreshProfile() {
  bevy.profile.get().then((p) => profile.set(p), fail("profile.get"));
  bevy.records.get().then((r) => records.set(r), fail("records.get"));
}

/** Settings and bindings (seeded at start; the settings screens refresh them on open). */
export function refreshSettings() {
  bevy.settings.get().then((x) => settings.set(x), fail("settings.get"));
  bevy.bindings.get().then((x) => bindings.set(x), fail("bindings.get"));
}

export function refreshSession() {
  bevy.race.snapshot().then((s) => demoMode.get() || session.set(s), fail("race.snapshot"));
}

/** Results and Cup end list the unlocks in their own panel: no toasts on top of them. */
function unlocksListed(): boolean {
  const s = appState.get();
  return (s?.state === "inRace" && s.phase === "finished") || top(router.get()) === "cupEnd";
}

function onUnlocked(items: UiUnlockedItem[]) {
  sfx("unlock");
  if (!unlocksListed()) {
    for (const item of items) pushToast({ tone: "unlock", title: appStrings.unlockedTitle, text: unlockLabel(item, catalog.get()) });
  }
  refreshProfile();
}

function trackName(id: string) {
  return catalog.get()?.tracks.find((t) => t.id === id)?.name ?? id;
}

function onNotice(n: UiNotice) {
  switch (n.kind) {
    case "goldenBattery":
      sfx("unlock");
      // In the race the HUD already announces it in the player's view.
      if (appState.get()?.state === "inRace") return;
      pushToast({ tone: "success", title: appStrings.batteryTitle, text: appStrings.batteryText(trackName(n.track)) });
      return;
    case "recordsDiscarded":
      pushToast({ tone: "info", title: appStrings.recordsDiscardedTitle, text: appStrings.recordsDiscardedText(trackName(n.track)) });
      return;
    case "controllerDisconnected": {
      const versus = session.get().setup?.mode.kind === "versus";
      const resume = () => {
        disconnected.delete(n.slot);
        bevy.race.resume(null);
      };
      const id = openDialog({
        title: appStrings.disconnectedTitle(word.player(n.slot)),
        text: appStrings.disconnectedText,
        icon: "controller",
        actions: [
          { label: appStrings.resume, tone: "primary", run: resume },
          ...(versus
            ? [
                {
                  label: appStrings.dropPlayer,
                  run: () => {
                    bevy.input.dropPlayer({ slot: n.slot });
                    resume();
                  },
                },
              ]
            : []),
        ],
      });
      disconnected.set(n.slot, id);
      return;
    }
    case "error":
      sfx("error");
      openDialog({ title: appStrings.errorTitle, text: appStrings.loadFailedText, detail: n.detail, icon: "warning", actions: [{ label: appStrings.ok }], cancel: 0 });
      return;
    case "saveFailed":
      pushToast({ tone: "error", title: appStrings.saveFailedTitle, text: n.detail });
      return;
    case "saveRestored":
      pushToast({ tone: "info", title: appStrings.saveRestoredTitle, text: appStrings.saveRestoredText });
      return;
  }
}

export function Feeds() {
  useEffect(() => {
    const offs = [
      on("ui.nav", dispatchNav),
      on("app.state", (s) => appState.set(s)),
      on("input.device", (e) => {
        if (e.kind === "joined" || e.kind === "lastUsed") setDevice(e.slot, e.device);
        // A button press on any device took the disconnected slot over: carry on racing.
        const dialog = e.kind === "joined" ? disconnected.get(e.slot) : undefined;
        if (dialog !== undefined) {
          closeDialog(dialog);
          disconnected.delete(e.slot);
          bevy.race.resume(null);
        }
      }),
      on("lobby.slots", (l) => demoMode.get() || lobby.set(l)),
      on("loading.progress", (p) => loadingProgress.set(p.progress)),
      on("race.results", (results) => session.update((s) => ({ ...s, results }))),
      on("cup.standings", (cup) => session.update((s) => ({ ...s, cup }))),
      on("versus.score", (versus) => session.update((s) => ({ ...s, versus }))),
      on("profile.changed", (p) => profile.set(p)),
      on("bindings.changed", (b) => bindings.set(b)),
      on("profile.unlocked", (u) => onUnlocked(u.items)),
      on("toast", onNotice),
      on("editor.status", (s) => editor.set(s)),
    ];
    bevy.app.snapshot().then((s) => appState.set(s), fail("app.snapshot"));
    bevy.app.info().then((i) => appInfo.set(i), fail("app.info"));
    bevy.catalog.get().then((c) => catalog.set(c), fail("catalog.get"));
    bevy.lobby.snapshot().then((l) => lobby.set(l), fail("lobby.snapshot"));
    bevy.editor.snapshot().then((e) => editor.set(e), fail("editor.snapshot"));
    refreshProfile();
    refreshSession();
    refreshSettings();
    return () => offs.forEach((off) => off());
  }, []);

  const status = useStore(appState);
  const state = status?.state;
  const phase = status?.phase;
  useEffect(() => {
    if (state === "loading" || state === "inRace") {
      loadingProgress.set(state === "loading" ? 0 : 1);
      refreshSession();
      // Started without the menus (--quick-race): returning should land in the main menu.
      const route = top(router.get());
      if (route === "splash" || route === "title") router.set(["main"]);
    }
    if (state === "menu") {
      refreshProfile();
      refreshSession();
    }
  }, [state]);
  useEffect(() => {
    if (phase === "finished" || phase === "setup") refreshSession();
  }, [phase]);
  return null;
}
