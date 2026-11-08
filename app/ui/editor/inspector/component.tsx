import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The element (component) Inspector (D76; rebuilt as a property panel in D130): every param of the
// component's manifest as a typed control (component/param.tsx), in sections by x-group (Content,
// Style, Layout, Motion, then any other), lists of rows in their own section, and a device frame's
// device and footage. Controls are built from each component's manifest.
import { componentInfo } from "@/app/lib/editor/core/visual/components";
import type { Item } from "@/app/lib/editor/core/timeline/types";
import { useEffect, useRef, useState } from "react";

import type { ProjectModel } from "@/app/lib/editor/state/project";
import { Section } from "@/app/ui/editor/inspector/controls";
import { DeviceParam, ScreenFootage } from "@/app/ui/editor/inspector/component/device";
import { ListParam } from "@/app/ui/editor/inspector/component/list";
import { Param } from "@/app/ui/editor/inspector/component/param";
import { GROUPS, groupOf, mainTextParam, type Schema } from "@/app/ui/editor/inspector/component/schema";
import { onTextFocus } from "@/app/ui/editor/inspector/focus";

export function ComponentEditor({ project, item, locked }: { project: ProjectModel; item: Item; locked: boolean }) {
  useEditorLanguage();
  const info = componentInfo(item.component);
  const root = useRef<HTMLDivElement>(null);
  const [textFocus,setTextFocus]=useState(0);
  // F23 (D162): a double click on the element on the canvas focuses its main text, selected.
  const main = info ? mainTextParam(info) : undefined;
  useEffect(() => {
    if (!main) return;
    return onTextFocus(item.id, () => {
      const field = root.current?.querySelector<HTMLInputElement | HTMLTextAreaElement>(`[data-testid="param-${CSS.escape(main)}"]`);
      if (!field || field.disabled || !field.offsetParent) return false;
      field.scrollIntoView({ block: "nearest" });
      field.focus();
      field.select();
      return true;
    },()=>setTextFocus(value=>value+1));
  }, [item.id, main]);
  if (!info) return null;
  const params = Object.entries(info.params.properties ?? {}) as Array<[string, Schema]>;
  const device = info.id === "component.device-frame";
  const lists = params.filter(([, s]) => s.type === "array" && s.items);
  const plain = params.filter(([name, s]) => !(s.type === "array" && s.items) && !(device && name === "device"));
  const groups = [...new Set([...GROUPS.filter((g) => plain.some(([, s]) => groupOf(s) === g)), ...plain.map(([, s]) => groupOf(s))])];
  return (
    <div className="pstack" ref={root} data-testid="component-editor" data-component={info.id} data-item={item.id}>
      {device && (
        <Section title={editorText("Device")} testId="param-sec-device">
          <DeviceParam project={project} item={item} disabled={locked} />
        </Section>
      )}
      {groups.map((g) => (
        <Section key={g} title={g} testId={`param-sec-${g.toLowerCase()}`} reveal={main&&g===groupOf(info.params.properties![main] as Schema)?String(textFocus):undefined}>
          {plain
            .filter(([, s]) => groupOf(s) === g)
            .map(([name, schema]) => (
              <Param key={name} project={project} item={item} info={info} name={name} schema={schema} disabled={locked} />
            ))}
        </Section>
      ))}
      {lists.map(([name, schema]) => (
        <ListParam key={name} project={project} item={item} info={info} name={name} schema={schema} disabled={locked} />
      ))}
      {device && <ScreenFootage project={project} item={item} disabled={locked} />}
    </div>
  );
}
