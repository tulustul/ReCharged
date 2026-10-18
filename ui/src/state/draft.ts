// The race being set up in the menus (mode → track & options → car), and the option rules for a
// track (GDD §3, §7.2). Pure apart from the store; Rust validates again on `race.start`.
import type {
  StartRace,
  UiDifficulty,
  UiPlayerChoice,
  UiProfile,
  UiTimeOfDay,
  UiTrack,
  UiWeatherChoice,
  UiWeatherKind,
} from "../bevy";
import { createStore, hmrSingleton, sameJson } from "./store.ts";

export type MenuMode = "quickRace" | "timeTrial" | "versus" | "championship";

export type RaceDraft = {
  mode: MenuMode;
  /** Cup id (championship). */
  cup: string | null;
  track: string;
  reverse: boolean;
  mirror: boolean;
  laps: number;
  ai: number;
  difficulty: UiDifficulty;
  items: boolean;
  weather: UiWeatherChoice;
  time: UiTimeOfDay;
  /** Player 1's car outside Versus. */
  car: string;
  paint: number;
};

export const MAX_RACERS = 8;
export const TIME_TRIAL_LAPS = 3;

export const DEFAULT_DRAFT: RaceDraft = {
  mode: "quickRace",
  cup: null,
  track: "",
  reverse: false,
  mirror: false,
  laps: 3,
  ai: 7,
  difficulty: "pro",
  items: true,
  weather: { kind: "fixed", weather: "clear" },
  time: "noon",
  car: "",
  paint: 0,
};

export const draft = hmrSingleton("__rc_draft", () => createStore<RaceDraft>(DEFAULT_DRAFT, sameJson));

/** Weather choices of a track: its presets, Random, and Dynamic outdoors. */
export function weatherChoices(track: UiTrack): UiWeatherChoice[] {
  const fixed: UiWeatherChoice[] = track.weather.map((weather) => ({ kind: "fixed", weather }));
  const random: UiWeatherChoice = { kind: "random" };
  const start: UiWeatherKind = track.weather[0] ?? "clear";
  return track.indoor ? [...fixed, random] : [...fixed, random, { kind: "dynamic", start }];
}

/** Most AI rivals allowed next to `players` humans (Time Trial: none). */
export const maxAi = (mode: MenuMode, players: number) => (mode === "timeTrial" ? 0 : MAX_RACERS - Math.max(1, players));

/** `d` repaired for `track`: allowed presets, unlocked layouts and difficulty, mode rules. */
export function normalize(d: RaceDraft, track: UiTrack, profile: UiProfile | null, players = 1): RaceDraft {
  const choices = weatherChoices(track);
  const weatherOk = choices.some((c) => sameJson(c, d.weather));
  const time = track.times.includes(d.time) ? d.time : track.times.includes("noon") ? "noon" : (track.times[0] ?? "noon");
  const difficulties = profile?.unlocked.difficulties ?? ["rookie", "pro"];
  const tt = d.mode === "timeTrial";
  return {
    ...d,
    track: track.id,
    reverse: d.reverse && !!profile?.unlocked.reverse,
    mirror: d.mirror && !!profile?.unlocked.mirror,
    laps: tt ? TIME_TRIAL_LAPS : Math.min(9, Math.max(1, d.laps)),
    ai: Math.min(maxAi(d.mode, players), Math.max(0, d.ai)),
    difficulty: difficulties.includes(d.difficulty) ? d.difficulty : (difficulties[difficulties.length - 1] ?? "pro"),
    items: tt ? false : d.items,
    weather: weatherOk ? d.weather : choices[0],
    time,
  };
}

/** The draft after picking `track` (laps reset to the track's default). */
export function withTrack(d: RaceDraft, track: UiTrack, profile: UiProfile | null, players = 1): RaceDraft {
  return normalize({ ...d, laps: track.defaultLaps }, track, profile, players);
}

/** The `race.start` message for a draft and its players. */
export function startMessage(d: RaceDraft, players: UiPlayerChoice[]): StartRace {
  const mode: StartRace["mode"] =
    d.mode === "championship" && d.cup ? { kind: "championship", cup: d.cup, race: 0 } : { kind: d.mode === "championship" ? "quickRace" : d.mode };
  if (mode.kind === "championship") {
    // The cup decides layout, laps, field, weather and difficulty.
    return { mode, track: d.track, reverse: false, mirror: false, players };
  }
  const tt = mode.kind === "timeTrial";
  return {
    mode,
    track: d.track,
    reverse: d.reverse,
    mirror: d.mirror,
    players,
    laps: tt ? TIME_TRIAL_LAPS : d.laps,
    ai: tt ? 0 : d.ai,
    difficulty: d.difficulty,
    items: tt ? false : d.items,
    weather: d.weather,
    time: d.time,
  };
}
