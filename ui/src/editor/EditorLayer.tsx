// The track editor's screen while editing (rc_editor): a top bar with the track, its save state and
// the commands, the build warnings, and a controls line. Test drives show the game's race UI instead.
import type { EditorStatus } from "../bevy";
import { bevy } from "../bevy";
import { Button } from "../components/Button";
import { editorStrings as words } from "../strings/editor";
import { glass, text } from "../theme/styles";
import { color, space } from "../theme/tokens";

export function EditorLayer({ status }: { status: EditorStatus }) {
  return (
    <node style={{ width: "100%", height: "100%", flexDirection: "column", justifyContent: "spaceBetween", padding: space.m }}>
      <node style={{ ...glass, flexDirection: "row", alignItems: "center", gap: space.l, padding: { horizontal: space.l, vertical: space.s } }} hoverStyle={{}}>
        <node style={{ flexDirection: "column", flexGrow: 1 }}>
          <text style={text.caps}>{words.title}</text>
          <text style={text.h3}>{status.name}</text>
        </node>
        <text style={{ ...text.label, color: status.dirty ? color.warning : color.textDim }}>{status.dirty ? words.unsaved : words.saved}</text>
        <Button label={words.save} compact onPress={() => bevy.editor.save(null)} />
        <Button label={words.drive} compact variant="primary" focused onPress={() => bevy.editor.drive(null)} />
      </node>
      <node style={{ flexDirection: "column", alignItems: "flexStart", gap: space.s }}>
        {status.error !== null && <Note tone={color.danger} lines={[status.error]} />}
        {status.lints.length > 0 && <Note tone={color.warning} title={words.lints(status.lints.length)} lines={status.lints} />}
        <node style={{ ...glass, padding: { horizontal: space.m, vertical: space.xs } }}>
          <text style={text.small}>{words.controls}</text>
        </node>
      </node>
    </node>
  );
}

function Note({ tone, title, lines }: { tone: string; title?: string; lines: string[] }) {
  return (
    <node style={{ ...glass, flexDirection: "column", gap: space.xs, padding: space.m, maxWidth: 900, borderColor: tone }}>
      {title !== undefined && <text style={{ ...text.caps, color: tone }}>{title}</text>}
      {lines.map((line, i) => (
        <text key={i} style={text.small}>{line}</text>
      ))}
    </node>
  );
}
