// Modal dialogs (confirm, errors, controller disconnected): the top dialog of the dialog store, in
// its own <root> above everything, owning navigation (dialog nav layer) and the pointer.
import { useState } from "react";

import { Button } from "./Button";
import { ControllerIcon, WarningIcon } from "./icons";
import { Panel } from "./Panel";
import { useEnter } from "../hooks";
import { isDir } from "../nav/focus";
import { useNav } from "../nav/input";
import { closeDialog, dialogs, type DialogSpec } from "../state/overlays";
import { useStore } from "../state/useStore";
import { sfx } from "../sound";
import { fill, ownsPointer, text } from "../theme/styles";
import { color, layer, space } from "../theme/tokens";

export function DialogHost() {
  const list = useStore(dialogs);
  const top = list[list.length - 1];
  return top ? <DialogView key={top.id} spec={top} /> : null;
}

function DialogView({ spec }: { spec: DialogSpec }) {
  const [focus, setFocus] = useState(spec.focus ?? 0);
  const enter = useEnter(0, 30);
  const count = spec.actions.length;
  const select = (i: number) => {
    if (i === focus) return;
    sfx("hover");
    setFocus(i);
  };
  const run = (i: number, cue: "select" | "back" = "select") => {
    sfx(cue);
    closeDialog(spec.id);
    spec.actions[i]?.run?.();
  };
  useNav((e) => {
    if (isDir(e.action)) {
      const delta = e.action === "left" || e.action === "up" ? -1 : 1;
      select(Math.min(count - 1, Math.max(0, focus + delta)));
    } else if (e.action === "confirm") run(focus);
    else if (e.action === "back" && spec.cancel !== undefined) run(spec.cancel, "back");
  }, layer.dialog);

  return (
    <root name="dialog" style={{ globalZIndex: layer.dialog }}>
      <node
        style={{ ...fill, backgroundColor: color.scrim, alignItems: "center", justifyContent: "center", focusPolicy: "block" }}
        hoverStyle={ownsPointer}
      >
        <node style={enter}>
          <Panel style={{ width: 820, gap: space.l, padding: 44 }} padding={44}>
            <node style={{ flexDirection: "row", alignItems: "center", gap: space.m }}>
              {spec.icon === "warning" && <WarningIcon size={44} />}
              {spec.icon === "controller" && <ControllerIcon size={48} color={color.cyan} />}
              <text style={{ ...text.h2, lineBreak: "wordBoundary", flexShrink: 1 }}>{spec.title}</text>
            </node>
            {spec.text && <text style={text.body}>{spec.text}</text>}
            {spec.detail && <text style={{ ...text.small, color: color.textFaint }}>{spec.detail}</text>}
            <node style={{ flexDirection: "row", gap: space.m, justifyContent: "flexEnd", margin: { top: space.s } }}>
              {spec.actions.map((a, i) => (
                <Button
                  key={a.label}
                  label={a.label}
                  variant={a.tone ?? "secondary"}
                  focused={i === focus}
                  onHover={() => select(i)}
                  onPress={() => run(i, i === spec.cancel ? "back" : "select")}
                  style={{ minWidth: 200, justifyContent: "center" }}
                />
              ))}
            </node>
          </Panel>
        </node>
      </node>
    </root>
  );
}
