// A track's picture: its preview capture when one exists, else a stylised card in the
// environment's colours (previews are rendered later by rc_render).
import type { UiTrack } from "../bevy";
import { text } from "../theme/styles";
import { color, radius } from "../theme/tokens";

/** Environment → two gradient colours (art bible palettes, simplified). */
const PALETTE: Record<string, [string, string]> = {
  toy_store: ["#ff7a1a", "#5b2a86"],
  house: ["#c98a4b", "#3a2416"],
  garden: ["#58b947", "#14402a"],
  supermarket: ["#38c7c0", "#123a4f"],
  workshop: ["#e8a33a", "#2b2f36"],
  street: ["#4a7bd8", "#1b1f3d"],
};

export function TrackArt({ track, width, height, dim = false, title = false }: { track: UiTrack; width: number | string; height: number | string; dim?: boolean; title?: boolean }) {
  const [a, b] = PALETTE[track.environment] ?? [color.cyanMid, color.navy];
  return (
    <node
      style={{
        width,
        height,
        borderRadius: radius.l,
        overflowX: "clip",
        overflowY: "clip",
        backgroundGradient: { type: "linear", angle: 135, stops: [{ color: a }, { color: b }] },
        border: 1,
        borderColor: "rgba(255, 255, 255, 0.14)",
        flexDirection: "column",
        justifyContent: "flexEnd",
        padding: 18,
      }}
    >
      {track.preview ? (
        <image src={track.preview} style={{ positionType: "absolute", left: 0, top: 0, width: "100%", height: "100%", imageRendering: "trilinear" }} />
      ) : (
        <>
          {/* A sweeping racing line across the card. */}
          <svg viewBox="0 0 160 90" style={{ positionType: "absolute", left: 0, top: 0, width: "100%", height: "100%" }}>
            <path d="M -10 70 C 30 20, 60 95, 95 45 S 150 10, 175 40" fill="none" stroke="rgba(255, 255, 255, 0.22)" strokeWidth={9} strokeLinecap="round" />
            <path d="M -10 70 C 30 20, 60 95, 95 45 S 150 10, 175 40" fill="none" stroke="rgba(255, 255, 255, 0.5)" strokeWidth={1.2} strokeLinecap="round" />
          </svg>
          {title && typeof height === "number" && (
            <text style={{ ...text.title, fontSize: Math.round(height * 0.2), lineBreak: "wordBoundary", color: "rgba(255, 255, 255, 0.92)", textShadow: { color: "rgba(0, 0, 0, 0.35)", offsetX: 3, offsetY: 3 } }}>
              {track.name.toUpperCase()}
            </text>
          )}
        </>
      )}
      {dim && <node style={{ positionType: "absolute", left: 0, top: 0, right: 0, bottom: 0, backgroundColor: "rgba(3, 6, 14, 0.55)" }} />}
    </node>
  );
}
