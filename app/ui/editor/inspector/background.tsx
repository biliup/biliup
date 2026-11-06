import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The Inspector's background editor (D53; one Background element since D131): the fill (a colour, a
// linear or a radial gradient), its colour stops (two, or three with a middle one), the angle of a
// linear gradient and film grain. Each change is one edit; a colour from the system picker and a
// slider preview while they move and commit once. Drawn by core solidBackground (preview and export).
import type { Item } from "@/app/lib/editor/core/timeline/types";
import { Minus, Plus } from "lucide-react";

import { type ProjectModel } from "@/app/lib/editor/state/project";
import { planBackground } from "@/app/ui/editor/preview/text";
import {useProjectEdit} from '../workspace/projectEdit';
import type {GestureOrigin} from '../workspace/pointerGesture';
import { Prop, Section, Segmented, SettledColorField, SliderNum } from "@/app/ui/editor/inspector/controls";

type Fill = "solid" | "linear" | "radial";
type Patch = Parameters<typeof planBackground>[2];

export function BackgroundEditor({ project, item, locked }: { project: ProjectModel; item: Item; locked: boolean }) {
  useEditorLanguage();
  const props = (item.props ?? {}) as { color?: string; to?: string; via?: string; angle?: number; gradient?: string; noise?: number };
  const fill: Fill = props.to === undefined ? "solid" : props.gradient === "radial" ? "radial" : "linear";
  const edit=useProjectEdit();
  const preview=(patch:Patch)=>edit.preview(project,planBackground(project,item.id,patch));
  const commit=(patch:Patch,origin?:GestureOrigin)=>edit.commit(p=>planBackground(p,item.id,patch),origin);
  const setFill = (next: Fill) =>
    commit(next === "solid" ? { to: null } : { to: props.to ?? "#3B1E54", gradient: next === "radial" ? "radial" : null, angle: next === "linear" ? (props.angle ?? 160) : null });
  const stop = (key: "color" | "via" | "to", label: string, testId: string, fallback: string, remove?: () => void) => (
    <Prop label={label} onReset={remove} testId={`${testId}-row`}>
      <SettledColorField onCancel={edit.cancel} project={project} testId={testId} value={props[key] ?? fallback} disabled={locked} onPreview={(c) => preview({ [key]: c })} onCommit={(c,origin) => commit({ [key]: c },origin)} />
    </Prop>
  );
  return (
    <div className="pstack" data-testid="background-editor" data-fill={fill}>
      <Section title={editorText("Fill")} testId="bg-sec-fill">
        <Segmented
          testId="bg-fill"
          options={[
            ["solid", "Solid"],
            ["linear", "Linear"],
            ["radial", "Radial"],
          ]}
          value={fill}
          disabled={locked}
          onChange={setFill}
        />
        {stop("color", fill === "solid" ? "Colour" : "Start", "bg-color", "#0B1D3A")}
        {fill !== "solid" && props.via !== undefined && stop("via", "Middle", "bg-via", "#000000")}
        {fill !== "solid" && stop("to", "End", "bg-to", "#3B1E54")}
        {fill !== "solid" && (
          <div className="stop-actions">
            {props.via === undefined ? (
              <button className="link-btn small" data-testid="bg-add-stop" disabled={locked} onClick={() => commit({ via: "#2A1A4A" })}>
                <Plus /> {editorText(" Middle colour")}</button>
            ) : (
              <button className="link-btn small" data-testid="bg-remove-stop" disabled={locked} onClick={() => commit({ via: null })}>
                <Minus /> {editorText(" Middle colour")}</button>
            )}
          </div>
        )}
        {fill === "linear" && (
          <Prop label={editorText("Angle")}>
            <SliderNum onCancel={edit.cancel} testId="bg-angle" value={props.angle ?? 180} min={0} max={360} unit="°" disabled={locked} onPreview={(v) => preview({ angle: v })} onCommit={(v,origin) => commit({ angle: v },origin)} />
          </Prop>
        )}
      </Section>
      <Section title={editorText("Grain")} testId="bg-sec-grain">
        <Prop label={editorText("Amount")} onReset={props.noise ? () => commit({ noise: null }) : undefined}>
          <SliderNum onCancel={edit.cancel} testId="bg-noise" value={Math.round((props.noise ?? 0) * 100)} min={0} max={100} unit="%" disabled={locked} onPreview={(v) => preview({ noise: v / 100 })} onCommit={(v,origin) => commit({ noise: v / 100 },origin)} />
        </Prop>
      </Section>
    </div>
  );
}
