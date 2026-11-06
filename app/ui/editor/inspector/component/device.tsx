import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// A device frame's own controls (D76): its device (which reshapes the frame and moves its footage)
// and the footage on its screen. Split out of panels/ComponentEditor.tsx.
import { DEVICES, type Device } from "@/app/lib/editor/core/visual/components";
import type { Item } from "@/app/lib/editor/core/timeline/types";
import { Smartphone } from "lucide-react";

import type { ProjectModel } from "@/app/lib/editor/state/project";
import { planAlignScreen, planDevice, planScreenFootage, screenClipOf } from "@/app/ui/editor/preview/components";
import { send } from "@/app/ui/editor/inspector/fields";
import { Prop, Section, Segmented } from "@/app/ui/editor/inspector/controls";

export function DeviceParam({ project, item, disabled }: { project: ProjectModel; item: Item; disabled: boolean }) {
  useEditorLanguage();
  const value = (item.props?.device as Device | undefined) ?? "phone";
  const clip = screenClipOf(project, item.id);
  const locked = disabled || !!(clip?.track.locked || clip?.item.locked);
  return (
    <Prop label={editorText("Device")}>
      <Segmented testId="param-device" options={DEVICES.map((d) => [d, editorText(d === "phone" ? "Phone" : d === "android" ? "Android" : "Browser")] as const)} value={value} disabled={locked} onChange={(d) => send((p) => planDevice(p, item.id, d))} />
    </Prop>
  );
}

/** A device frame's footage (D76): a clip under it on its screen. Pick one, change it, take it out, or fit it back to the screen. */
export function ScreenFootage({ project, item, disabled }: { project: ProjectModel; item: Item; disabled: boolean }) {
  useEditorLanguage();
  const clip = screenClipOf(project, item.id);
  const footage = project.media.filter((m) => m.kind === "video" || m.kind === "image");
  const locked = disabled || !!(clip?.track.locked || clip?.item.locked);
  return (
    <Section title={editorText("Screen")} testId="param-sec-screen">
      <select
        className="field input"
        data-testid="device-footage"
        value={clip?.item.media ?? ""}
        disabled={locked}
        onChange={(e) => {
          const id = e.target.value || null;
          send((p) => planScreenFootage(p, item.id, id));
        }}
      >
        <option value="">{editorText("Empty (shows what's under it)")}</option>
        {footage.map((m) => (
          <option key={m.id} value={m.id}>
            {m.name}
          </option>
        ))}
      </select>
      {clip && (
        <button className="btn" data-testid="device-align" disabled={locked} onClick={() => send((p) => planAlignScreen(p, item.id))}>
          <Smartphone />
          {editorText("Fit footage to the screen")}</button>
      )}
    </Section>
  );
}

