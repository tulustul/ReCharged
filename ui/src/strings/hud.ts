// Race HUD strings (GDD §13).
import type { UiHint, UiItemKind, UiRespawnCause, UiSpeedUnit, UiTurn } from "../bevy";

export const itemName: Record<UiItemKind, string> = {
  turboCell: "Turbo Cell",
  sparkRocket: "Spark Rocket",
  leechCoil: "Leech Coil",
  shockOrb: "Shock Orb",
  empPulse: "EMP Pulse",
  gooBomb: "Goo Bomb",
  oilSlick: "Oil Slick",
  springPad: "Spring Pad",
  shieldBubble: "Shield Bubble",
  stormStrike: "Storm Strike",
};

export const hintText: Record<UiHint, string> = {
  driftCharge: "Drift to charge your battery",
  boostReady: "Battery charged — boost!",
  itemReady: "Item ready — use it!",
  holdReset: "Stuck? Hold Reset",
  wrongWay: "Wrong way — turn around",
};

export const unitName: Record<UiSpeedUnit, string> = { kmh: "km/h", mph: "mph" };

export const hud = {
  lap: "LAP",
  finalLap: "FINAL LAP",
  time: "TIME",
  best: "BEST",
  bestTag: "BEST",
  ghost: "vs GHOST",
  timeTrial: "TIME TRIAL",
  go: "GO!",
  wrongWay: "WRONG WAY",
  offTrack: "OFF TRACK",
  thisWay: "THIS WAY",
  respawnIn: (secs: number) => `Back on the track in ${secs.toFixed(1)} s`,
  /** The turn indicator's words: "HAIRPIN LEFT", "SHARP RIGHT", "LEFT", "EASY RIGHT". */
  turn: (t: UiTurn) => {
    const side = t.right ? "RIGHT" : "LEFT";
    if (t.angle >= 150) return `HAIRPIN ${side}`;
    return t.severity >= 3 ? `SHARP ${side}` : t.severity === 2 ? side : `EASY ${side}`;
  },
  rocketLocked: "ROCKET LOCKED",
  goo: "GOO BOMB",
  shield: "SHIELD",
  boostReady: "BOOST READY",
  charge: "CHARGE",
  finished: "FINISHED",
  waiting: "Waiting for other players…",
  endsIn: (secs: number) => `Race ends in ${secs}…`,
  spectate: "Watch other racers",
  skipIntro: "Skip intro",
  standings: "STANDINGS",
  lapDone: (lap: number) => `LAP ${lap}`,
  notice: {
    perfectStart: "Perfect start!",
    stall: "Too early — wheelspin!",
    fastestLap: "Fastest lap!",
    goldenBattery: "Golden Battery found!",
    missedCheckpoint: "Missed checkpoint — go back",
    rescued: {
      offTrack: "Off the track too long — put back",
      fell: "Fell off — put back on the track",
      stuck: "Stuck — put back on the track",
    } satisfies Record<UiRespawnCause, string>,
    hitBy: (item: string, who: string | null) => (who ? `Hit by ${item} — ${who}` : `Hit by ${item}`),
    blocked: (item: string) => `Shield blocked the ${item}`,
    landed: (item: string, who: string) => `${item} hit ${who}!`,
    landedMany: (item: string, count: number) => `${item} hit ${count} racers`,
    landedBlocked: (who: string) => `${who}'s shield blocked it`,
  },
} as const;
