// UI-7 Track select: track cards (preview, lock, records), layout toggles and race options within
// the track's allowed presets. Focusing a card previews it (its options and records show); Confirm
// on a card selects it and moves the focus onto Start / Choose car (Confirm, Confirm is the fast
// path), Up from there reaches every option, Back or Left returns to the card. Quick Race / Time
// Trial continue to car select; Versus (P1 picks) starts the race; after a race it picks the next
// layout with its options (`race.nextTrack`).
import { useEffect } from "react";

import { bevy, type NextTrack, type UiTrack } from "../bevy";
import { Badge } from "../components/Badge";
import { Caption, LockOverlay } from "../components/bits";
import { Button } from "../components/Button";
import { Cycler, OptionRow, Toggle } from "../components/controls";
import { MedalIcon, StarIcon } from "../components/icons";
import { Panel } from "../components/Panel";
import { Screen } from "../components/Screen";
import { TrackArt } from "../components/TrackArt";
import { formatTime } from "../format";
import { useSeen } from "../hooks";
import { cycle, enter } from "../nav/focus";
import { back, go } from "../nav/routes";
import { useFocusNav } from "../nav/useFocusNav";
import { sfx } from "../sound";
import { draft, maxAi, nextTrackMessage, normalize, startGate, startMessage, weatherChoices, withTrack, type RaceDraft } from "../state/draft";
import { lobbyChoices } from "../state/lobby";
import { catalog, lobby, profile, records } from "../state/stores";
import { isFresh, lockText, trackUnlocked } from "../state/unlocks";
import { useStore } from "../state/useStore";
import { difficultyName, hint, medalName, timeName, weatherName, word } from "../strings/common";
import { trackSelect } from "../strings/menu";
import { focusTransition } from "../theme/motion";
import { text } from "../theme/styles";
import { color, radius, space } from "../theme/tokens";

type Props = { inRace?: { onPick: (next: NextTrack) => void; onBack: () => void } };

type Row = { id: string; label: string; value?: string; step?: (delta: number) => void; toggle?: boolean; locked?: boolean };

export function TrackSelect({ inRace }: Props) {
  const cat = useStore(catalog);
  const p = useStore(profile);
  const d = useStore(draft);
  const slots = useStore(lobby)?.slots ?? [];
  const tracks = cat?.tracks ?? [];
  const versus = d.mode === "versus";
  const players = versus ? Math.max(2, slots.length) : 1;
  const track = tracks.find((t) => t.id === d.track) ?? tracks[0];
  const update = (change: Partial<RaceDraft>) => {
    if (track) draft.set(normalize({ ...d, ...change }, track, p, players));
  };
  const rows = track ? optionRows(d, track, update, players) : [];
  const exit = inRace?.onBack ?? back;

  const start = () => {
    if (!track || !trackUnlocked(p, track.id)) return sfx("error");
    sfx("select");
    if (inRace) return inRace.onPick(nextTrackMessage(d));
    if (!versus) return go("carSelect");
    if (!startGate(performance.now())) return;
    const choices = lobbyChoices.get();
    const cars = cat?.cars.filter((c) => p?.unlocked.cars.includes(c.id)) ?? [];
    const players = slots.map((s) => ({ slot: s.slot, car: cars[choices[s.slot]?.car ?? 0]?.id ?? cars[0]?.id ?? "", paint: choices[s.slot]?.paint ?? 0 }));
    bevy.race.start(startMessage(d, players));
  };

  const map = {
    tracks: { count: tracks.length, cols: 3, right: "options" },
    options: { count: rows.length + 1, cols: 1, left: "tracks" },
  };
  const selected = Math.max(0, tracks.findIndex((t) => t.id === d.track));
  /** Confirm on a card: it is the selection; the focus goes to Start (its options are above). */
  const select = () => {
    if (!track || !trackUnlocked(p, track.id)) return sfx("error");
    sfx("select");
    nav.setFocus(enter(nav.focus, "options", rows.length + 1, rows.length));
  };
  const nav = useFocusNav(
    map,
    { region: "tracks", index: selected },
    {
      confirm: (f) => {
        if (f.region === "tracks") return select();
        if (f.index === rows.length) return start();
        const row = rows[f.index];
        if (row?.toggle !== undefined || row?.step) adjust(row, 1);
      },
      // From the options back to the selected card; from the cards out of the screen.
      back: () => (nav.focus.region === "options" ? nav.setFocus(enter(nav.focus, "tracks", tracks.length, selected)) : exit()),
      adjust: (f, delta) => {
        const row = f.region === "options" ? rows[f.index] : undefined;
        if (!row) return false;
        adjust(row, delta);
        return true;
      },
      // Versus: P1 picks the track.
      accept: (e) => !versus || e.slot === null || e.slot === 0,
    },
  );
  const adjust = (row: Row, delta: number) => {
    if (row.locked) return sfx("error");
    sfx("toggle");
    row.step?.(delta);
  };
  // The focused card is previewed: it becomes the draft's track (laps reset to its default).
  const focusedTrack = nav.focus.region === "tracks" ? tracks[nav.focus.index] : undefined;
  useEffect(() => {
    const d = draft.get();
    if (focusedTrack && focusedTrack.id !== d.track) draft.set(withTrack(d, focusedTrack, profile.get(), players));
  }, [focusedTrack, players]);
  useSeen(track ? { kind: "track", id: track.id } : null, !!track && isFresh(p, { kind: "track", id: track.id }));

  const goLabel = inRace || versus ? trackSelect.start : trackSelect.next;
  const modeTitle = trackSelect.title[d.mode === "championship" ? "quickRace" : d.mode];
  return (
    <Screen
      title={modeTitle}
      subtitle={trackSelect.subtitle}
      hints={[
        { action: "move", label: hint.select },
        nav.focus.region === "tracks" ? { action: "confirm", label: hint.select, onPress: select } : { action: "confirm", label: goLabel, onPress: start },
        { action: "back", label: hint.back, onPress: exit },
      ]}
    >
      <node style={{ flexDirection: "row", gap: space.xl, flexGrow: 1, minHeight: 0 }}>
        <node style={{ flexGrow: 1, display: "grid", gridTemplateColumns: "repeat(3, 1flex)", gridTemplateRows: "repeat(2, 1flex)", gap: space.l }}>
          {tracks.map((t, i) => (
            <TrackCard
              key={t.id}
              track={t}
              focused={nav.is("tracks", i)}
              selected={t.id === d.track}
              onHover={() => nav.hover("tracks", i)}
              // A click on another card selects it; on the selected one it continues.
              onPress={() => (t.id === d.track ? start() : nav.hover("tracks", i))}
            />
          ))}
        </node>
        <Panel style={{ width: 600, gap: 6, flexShrink: 0 }} padding={22}>
          {track && d.mode === "timeTrial" && <text style={{ ...text.caps, color: color.volt, margin: { left: 10, bottom: 6 } }}>{trackSelect.ttRules.toUpperCase()}</text>}
          {rows.map((row, i) => (
            <OptionRow key={row.id} label={row.label} height={54} focused={nav.is("options", i)} disabled={row.locked} onHover={() => nav.hover("options", i)} onPress={() => adjust(row, 1)}>
              {row.toggle !== undefined ? (
                <Toggle value={row.toggle} labels={[word.off, word.on]} onChange={() => adjust(row, 1)} />
              ) : (
                <Cycler label={row.value ?? ""} dim={row.locked} onStep={(delta) => adjust(row, delta)} />
              )}
            </OptionRow>
          ))}
          {track?.indoor && <text style={{ ...text.small, fontSize: 16, margin: { left: 26 } }}>{trackSelect.indoorWeather}</text>}
          <node style={{ flexGrow: 1 }} />
          {track && <RecordsBox track={track} reverse={d.reverse} mirror={d.mirror} />}
          <Button
            label={goLabel}
            variant="primary"
            focused={nav.is("options", rows.length)}
            locked={!!track && !trackUnlocked(p, track.id)}
            onHover={() => nav.hover("options", rows.length)}
            onPress={start}
            style={{ justifyContent: "center", margin: { top: space.s } }}
          />
        </Panel>
      </node>
    </Screen>
  );
}

function optionRows(d: RaceDraft, track: UiTrack, update: (c: Partial<RaceDraft>) => void, players: number): Row[] {
  const p = profile.get();
  const layout: Row[] = [
    { id: "layout", label: trackSelect.layout, value: d.reverse ? word.reverse : word.forward, step: () => update({ reverse: !d.reverse }), locked: !p?.unlocked.reverse },
    { id: "mirror", label: trackSelect.mirror, toggle: d.mirror, step: () => update({ mirror: !d.mirror }), locked: !p?.unlocked.mirror },
  ];
  const choices = weatherChoices(track);
  const wi = Math.max(0, choices.findIndex((c) => JSON.stringify(c) === JSON.stringify(d.weather)));
  const weatherLabel = d.weather.kind === "fixed" ? weatherName[d.weather.weather] : d.weather.kind === "dynamic" ? word.dynamic : word.random;
  const ti = Math.max(0, track.times.indexOf(d.time));
  const diffs = p?.unlocked.difficulties ?? ["rookie", "pro"];
  const race: Row[] =
    d.mode === "timeTrial"
      ? []
      : [
          { id: "laps", label: trackSelect.laps, value: String(d.laps), step: (k) => update({ laps: cycle(d.laps - 1, k, 9) + 1 }) },
          { id: "ai", label: trackSelect.ai, value: d.ai === 0 ? trackSelect.noAi : String(d.ai), step: (k) => update({ ai: cycle(d.ai, k, maxAi(d.mode, players) + 1) }) },
          { id: "difficulty", label: trackSelect.difficulty, value: difficultyName[d.difficulty], step: (k) => update({ difficulty: diffs[cycle(diffs.indexOf(d.difficulty), k, diffs.length)] }) },
          { id: "items", label: trackSelect.items, toggle: d.items, step: () => update({ items: !d.items }) },
        ];
  return [
    ...layout,
    ...race,
    { id: "weather", label: trackSelect.weather, value: weatherLabel, step: (k) => update({ weather: choices[cycle(wi, k, choices.length)] }) },
    { id: "time", label: trackSelect.time, value: timeName[d.time], step: (k) => update({ time: track.times[cycle(ti, k, track.times.length)] }) },
  ];
}

function TrackCard({ track, focused, selected, onHover, onPress }: { track: UiTrack; focused: boolean; selected: boolean; onHover: () => void; onPress: () => void }) {
  const cat = useStore(catalog);
  const p = useStore(profile);
  const unlocked = trackUnlocked(p, track.id);
  return (
    <button
      onPointerHover={onHover}
      onClick={onPress}
      style={{
        flexDirection: "column",
        gap: 10,
        padding: 10,
        borderRadius: radius.l,
        border: 2,
        borderColor: focused ? color.cyan : selected ? "rgba(25, 217, 255, 0.35)" : color.clear,
        backgroundColor: focused ? "rgba(18, 34, 72, 0.85)" : "rgba(10, 18, 40, 0.6)",
        transform: { scale: focused ? 1.03 : 1 },
        transition: focusTransition,
      }}
    >
      <node style={{ flexGrow: 1, minHeight: 0 }}>
        <TrackArt track={track} width="100%" height="100%" dim={!unlocked} />
      </node>
      <node style={{ flexDirection: "row", alignItems: "center", justifyContent: "spaceBetween", padding: { horizontal: 6, bottom: 4 } }}>
        <text style={{ ...text.h3, fontSize: 26, color: unlocked ? color.text : color.textFaint }}>{track.name.toUpperCase()}</text>
        <node style={{ flexDirection: "row", gap: 2 }}>
          {[1, 2, 3].map((n) => (
            <StarIcon key={n} size={20} filled={n <= track.difficulty} color={unlocked ? color.volt : color.textFaint} />
          ))}
        </node>
      </node>
      {isFresh(p, { kind: "track", id: track.id }) && (
        <node style={{ positionType: "absolute", right: 18, top: 18 }}>
          <Badge label={word.new} />
        </node>
      )}
      {!unlocked && <LockOverlay reason={lockText(track.unlock, cat)} compact />}
    </button>
  );
}

function RecordsBox({ track, reverse, mirror }: { track: UiTrack; reverse: boolean; mirror: boolean }) {
  const rec = useStore(records)?.layouts.find((r) => r.track === track.id && r.reverse === reverse && r.mirror === mirror);
  const cars = useStore(catalog)?.cars ?? [];
  const carName = (id: string) => cars.find((c) => c.id === id)?.name ?? id;
  const line = (label: string, value: string) => (
    <node style={{ flexDirection: "row", justifyContent: "spaceBetween" }}>
      <text style={text.small}>{label}</text>
      <text style={{ ...text.small, color: color.text }}>{value}</text>
    </node>
  );
  return (
    <node style={{ flexDirection: "column", gap: 6, padding: 16, borderRadius: radius.m, backgroundColor: "rgba(4, 8, 20, 0.45)" }}>
      <node style={{ flexDirection: "row", justifyContent: "spaceBetween", alignItems: "center" }}>
        <Caption>{trackSelect.bestLap}</Caption>
        {rec?.medal && (
          <node style={{ flexDirection: "row", gap: 6, alignItems: "center" }}>
            <MedalIcon size={20} color={color[rec.medal]} />
            <text style={{ ...text.small, color: color[rec.medal] }}>{medalName[rec.medal]}</text>
          </node>
        )}
      </node>
      {rec ? (
        <>
          {line(trackSelect.bestLap, rec.bestLap ? `${formatTime(rec.bestLap.ms)} · ${carName(rec.bestLap.car)}` : word.none)}
          {line(trackSelect.bestRace, rec.bestRace ? `${formatTime(rec.bestRace.ms)} · ${carName(rec.bestRace.car)}` : word.none)}
        </>
      ) : (
        <text style={text.small}>{trackSelect.noRecord}</text>
      )}
    </node>
  );
}
