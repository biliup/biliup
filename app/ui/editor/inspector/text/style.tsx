import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The text Inspector's Style tab (D129), laid out like a design tool's property panel: the look (one
// optional dropdown), Typography (font, weight, italic, size, spacing, line height, case,
// alignment), Fill, Stroke, Shadow, Background and Layout (align to the frame, position, width,
// rotation, opacity). Everything previews live and is one edit when committed (a slider or a
// scrubbed label once on release); several selected texts change together. What draws it is core
// textHtml, the export's path too, so the export matches.
import { TEXT_STYLES, textLook } from "@/app/lib/editor/core/visual/render";
import type { Item } from "@/app/lib/editor/core/timeline/types";
import { AlignCenter, AlignCenterHorizontal, AlignCenterVertical, AlignEndHorizontal, AlignEndVertical, AlignLeft, AlignRight, AlignStartHorizontal, AlignStartVertical, Italic, Minus, Plus, RotateCcw } from "lucide-react";

import { type ProjectModel } from "@/app/lib/editor/state/project";
import { measureText } from "@/app/ui/editor/preview/measure";
import { caseOf, planCase, planFrameAlign, planPreset, planResetLook, planShadow, planStroke, planStyle, planTextBox, planTransform, textBox, textDecorOf, type FrameEdge, type TextCase } from "@/app/ui/editor/preview/text";
import { NumInput, Prop, Section, Segmented, SettledColorField, SliderNum } from "@/app/ui/editor/inspector/controls";
import { FontPicker } from "@/app/ui/editor/inspector/text/fonts";
import { useTextEdit, type TextEdit } from "@/app/ui/editor/inspector/text/edit";

const WEIGHTS = [100, 200, 300, 400, 500, 600, 700, 800, 900] as const;
const WEIGHT_NAMES: Readonly<Record<number, string>> = { 100: "Thin", 200: "Extra light", 300: "Light", 400: "Regular", 500: "Medium", 600: "Semibold", 700: "Bold", 800: "Heavy", 900: "Black" };
const CASES: ReadonlyArray<readonly [TextCase, string, string]> = [
  ["none", "Aa", "As typed"],
  ["upper", "AA", "Capitals"],
  ["lower", "aa", "Lower case"],
  ["title", "Ab", "Title case"],
];
const ALIGNS = [
  ["left", <AlignLeft key="l" />, "Align left"],
  ["center", <AlignCenter key="c" />, "Align centre"],
  ["right", <AlignRight key="r" />, "Align right"],
] as const;
const EDGES: ReadonlyArray<readonly [FrameEdge, React.ReactNode, string]> = [
  ["left", <AlignStartVertical key="l" />, "Left of the frame"],
  ["hcenter", <AlignCenterVertical key="h" />, "Centre of the frame"],
  ["right", <AlignEndVertical key="r" />, "Right of the frame"],
  ["top", <AlignStartHorizontal key="t" />, "Top of the frame"],
  ["vcenter", <AlignCenterHorizontal key="v" />, "Middle of the frame"],
  ["bottom", <AlignEndHorizontal key="b" />, "Bottom of the frame"],
];

function LookPicker({ item, edit, disabled }: { item: Item; edit: TextEdit; disabled: boolean }) {
  useEditorLanguage();
  const preset = item.style?.preset ?? "";
  const known = !!TEXT_STYLES[preset];
  return (
    <div className="look-row">
      <select className="pselect" data-testid="insp-preset" value={known ? preset : ""} disabled={disabled} onChange={(e) => {
          // Read now: the plan runs later, when React has put the controlled value back.
          const look = e.target.value;
          if (look) edit.commit((p, id) => planPreset(p, id, look));
        }}>
        {!known && <option value="">{preset ? textLook(preset).name || editorText("Custom") : editorText("No look")}</option>}
        {Object.entries(TEXT_STYLES).map(([id, look]) => (
          <option key={id} value={id}>
            {look.name}
          </option>
        ))}
      </select>
      <button className="icon-btn" data-testid="insp-reset-look" title={editorText("Reset to the look")} aria-label={editorText("Reset to the look")} disabled={disabled || !known} onClick={() => edit.commit((p, id) => planResetLook(p, id))}>
        <RotateCcw />
      </button>
    </div>
  );
}

function Typography({ project, item, edit, disabled }: { project: ProjectModel; item: Item; edit: TextEdit; disabled: boolean }) {
  useEditorLanguage();
  const box = textBox(item, project);
  const d = textDecorOf(item, project);
  const s = item.style ?? {};
  const maxSize = Math.round(project.canvas.height * 0.4);
  const reset = (key: keyof NonNullable<Item["style"]>, label: string) => (s[key] !== undefined && TEXT_STYLES[s.preset ?? ""] ? () => edit.commit((p, id) => planStyle(p, id, { [key]: null }, `Use the look's ${label}`)) : undefined);
  return (
    <Section title={editorText("Typography")} testId="insp-sec-type">
      <Prop label={editorText("Font")} onReset={reset("font", "font")} testId="insp-prop-font">
        <FontPicker testId="insp-font" value={s.font ?? ""} disabled={disabled} onPick={(font,origin) => edit.commit((p, id) => planStyle(p, id, { font: font || null }, font ? editorText('Set font to {0}',{0:font}) : editorText("Use the look's font")),origin)} />
      </Prop>
      <Prop label={editorText("Weight")} onReset={reset("weight", "weight")}>
        <div className="prop-pair">
          <select
            className="pselect"
            data-testid="insp-weight"
            value={box.weight}
            disabled={disabled}
            onChange={(e) => {
              const weight = Number(e.target.value);
              edit.commit((p, id) => planStyle(p, id, { weight }, `Set weight to ${WEIGHT_NAMES[weight] ?? weight}`));
            }}
          >
            {WEIGHTS.map((w) => (
              <option key={w} value={w}>
                {WEIGHT_NAMES[w]}
              </option>
            ))}
          </select>
          <button className={`icon-btn square ${s.italic ? "active" : ""}`} data-testid="insp-italic" title={editorText("Italic")} aria-label={editorText("Italic")} aria-pressed={!!s.italic} disabled={disabled} onClick={() => edit.commit((p, id) => planStyle(p, id, { italic: s.italic ? null : true }, s.italic ? "Set text upright" : "Set text in italic"))}>
            <Italic />
          </button>
        </div>
      </Prop>
      <Prop label={editorText("Size")} onReset={reset("size", "size")}>
        <SliderNum onCancel={edit.cancel}
          testId="insp-size"
          value={box.size}
          min={1}
          max={2000}
          sliderMax={Math.max(maxSize, 8)}
          unit="px"
          disabled={disabled}
          onPreview={(v) => edit.preview((p, id) => planStyle(p, id, { size: v }))}
          onCommit={(v,origin) => edit.commit((p, id) => planStyle(p, id, { size: v }, "Resize text"),origin)}
        />
      </Prop>
      <div className="prop-grid">
        <NumInput onCancel={edit.cancel}
          testId="insp-tracking"
          label={editorText("Spacing")}
          title={editorText("Letter spacing (em)")}
          unit="em"
          value={Math.round(d.tracking * 1000) / 1000}
          step={0.01}
          min={-0.5}
          max={2}
          clearable
          disabled={disabled}
          onPreview={(v) => edit.preview((p, id) => planStyle(p, id, { tracking: v }))}
          onCommit={(v,origin) => edit.commit((p, id) => planStyle(p, id, { tracking: v ?? null }, v === undefined ? "Use the look's letter spacing" : `Set letter spacing to ${v} em`),origin)}
        />
        <NumInput onCancel={edit.cancel}
          testId="insp-line-height"
          label={editorText("Line")}
          title={editorText("Line height (× the size)")}
          value={Math.round(d.lineHeight * 100) / 100}
          step={0.05}
          min={0.5}
          max={4}
          clearable
          disabled={disabled}
          onPreview={(v) => edit.preview((p, id) => planStyle(p, id, { lineHeight: v }))}
          onCommit={(v,origin) => edit.commit((p, id) => planStyle(p, id, { lineHeight: v ?? null }, v === undefined ? "Use the look's line height" : `Set line height to ${v}`),origin)}
        />
      </div>
      <Prop label={editorText("Case")}>
        <Segmented testId="insp-case" attr="case" options={CASES.map(([v, l, t]) => [v, l, t] as const)} value={caseOf(item)} disabled={disabled} onChange={(c) => edit.commit((p, id) => planCase(p, id, c))} />
      </Prop>
      <Prop label={editorText("Align")} onReset={reset("align", "alignment")}>
        <Segmented testId="insp-align" attr="align" options={ALIGNS} value={box.align} disabled={disabled} onChange={(align) => edit.commit((p, id) => planStyle(p, id, { align }, `Align text ${align}`))} />
      </Prop>
    </Section>
  );
}

function Fill({ project, item, edit, disabled }: { project: ProjectModel; item: Item; edit: TextEdit; disabled: boolean }) {
  useEditorLanguage();
  const box = textBox(item, project);
  return (
    <Section title={editorText("Fill")} testId="insp-sec-fill">
      <SettledColorField onCancel={edit.cancel}
        project={project}
        testId="insp-color"
        value={typeof box.color === "string" ? box.color : undefined}
        disabled={disabled}
        onPreview={(color) => edit.preview((p, id) => planStyle(p, id, { color }))}
        onCommit={(color,origin) => edit.commit((p, id) => planStyle(p, id, { color }, "Recolour text"),origin)}
      />
    </Section>
  );
}

/** A section's add (+) or remove (−) action. */
const AddRemove = ({ on, testId, disabled, onAdd, onRemove, noun }: { on: boolean; testId: string; disabled: boolean; onAdd: () => void; onRemove: () => void; noun: string }) =>
  on ? (
    <button className="icon-btn" data-testid={`${testId}-none`} title={editorText("Remove {0}", { 0: noun })} aria-label={editorText("Remove {0}", { 0: noun })} disabled={disabled} onClick={onRemove}>
      <Minus />
    </button>
  ) : (
    <button className="icon-btn" data-testid={`${testId}-on`} title={editorText("Add {0}", { 0: noun })} aria-label={editorText("Add {0}", { 0: noun })} disabled={disabled} onClick={onAdd}>
      <Plus />
    </button>
  );

function Decor({ project, item, edit, disabled }: { project: ProjectModel; item: Item; edit: TextEdit; disabled: boolean }) {
  useEditorLanguage();
  const d = textDecorOf(item, project);
  const wrap = d.background ? (d.background.wrap ?? "box") : "none";
  return (
    <>
      <Section title={editorText("Stroke")} testId="insp-sec-stroke" action={<AddRemove on={!!d.stroke} testId="insp-stroke" noun="the stroke" disabled={disabled} onAdd={() => edit.commit((p, id) => planStroke(p, id, {}))} onRemove={() => edit.commit((p, id) => planStroke(p, id, false))} />}>
        {d.stroke && (
          <div className="prop-grid">
            <SettledColorField onCancel={edit.cancel} project={project} testId="insp-stroke-color" value={d.stroke.color ?? "#000000"} noAlpha disabled={disabled} onPreview={(color) => edit.preview((p, id) => planStroke(p, id, { color }))} onCommit={(color,origin) => edit.commit((p, id) => planStroke(p, id, { color }),origin)} />
            <NumInput onCancel={edit.cancel} testId="insp-stroke-width" label={editorText("Width")} unit="px" value={d.stroke.width} step={1} min={0} max={200} disabled={disabled} onPreview={(v) => edit.preview((p, id) => planStroke(p, id, { width: v }))} onCommit={(v,origin) => v !== undefined && edit.commit((p, id) => planStroke(p, id, { width: v }),origin)} />
          </div>
        )}
      </Section>
      <Section title={editorText("Shadow")} testId="insp-sec-shadow" action={<AddRemove on={!!d.shadow} testId="insp-shadow" noun="the shadow" disabled={disabled} onAdd={() => edit.commit((p, id) => planShadow(p, id, {}))} onRemove={() => edit.commit((p, id) => planShadow(p, id, false))} />}>
        {d.shadow && (
          <>
            <SettledColorField onCancel={edit.cancel}
              project={project}
              testId="insp-shadow-color"
              value={d.shadow.color ?? "#000000"}
              alpha={{ value: d.shadow.opacity ?? 1, onCommit: (v,origin) => edit.commit((p, id) => planShadow(p, id, { opacity: v }),origin) }}
              disabled={disabled}
              onPreview={(color) => edit.preview((p, id) => planShadow(p, id, { color }))}
              onCommit={(color,origin) => edit.commit((p, id) => planShadow(p, id, { color }),origin)}
            />
            <div className="prop-grid three">
              {(["x", "y", "blur"] as const).map((k) => (
                <NumInput onCancel={edit.cancel}
                  key={k}
                  testId={`insp-shadow-${k}`}
                  label={k === "blur" ? editorText("Blur") : k.toUpperCase()}
                  value={d.shadow![k] ?? 0}
                  min={k === "blur" ? 0 : -500}
                  max={500}
                  disabled={disabled}
                  onPreview={(v) => edit.preview((p, id) => planShadow(p, id, { [k]: v }))}
                  onCommit={(v,origin) => v !== undefined && edit.commit((p, id) => planShadow(p, id, { [k]: v }),origin)}
                />
              ))}
            </div>
          </>
        )}
      </Section>
      <Section title={editorText("Background")} testId="insp-sec-box">
        <Segmented
          testId="insp-box"
          attr="wrap"
          options={[
            ["none", "None"],
            ["box", "Box"],
            ["lines", "Per line"],
          ]}
          value={wrap}
          disabled={disabled}
          onChange={(w) => edit.commit((p, id) => planTextBox(p, id, w === "none" ? false : { wrap: w }))}
        />
        {d.background && (
          <>
            <SettledColorField onCancel={edit.cancel}
              project={project}
              testId="insp-box-color"
              value={d.background.color ?? "#000000"}
              alpha={{ value: d.background.opacity ?? 1, onCommit: (v,origin) => edit.commit((p, id) => planTextBox(p, id, { opacity: v }),origin) }}
              disabled={disabled}
              onPreview={(color) => edit.preview((p, id) => planTextBox(p, id, { color }))}
              onCommit={(color,origin) => edit.commit((p, id) => planTextBox(p, id, { color }),origin)}
            />
            <div className="prop-grid">
              <NumInput onCancel={edit.cancel} testId="insp-box-padding" label={editorText("Padding")} unit="px" value={d.background.padding ?? 0} min={0} max={1000} disabled={disabled} onPreview={(v) => edit.preview((p, id) => planTextBox(p, id, { padding: v }))} onCommit={(v,origin) => v !== undefined && edit.commit((p, id) => planTextBox(p, id, { padding: v }),origin)} />
              <NumInput onCancel={edit.cancel} testId="insp-box-radius" label={editorText("Radius")} unit="px" value={d.background.radius ?? 0} min={0} max={1000} disabled={disabled} onPreview={(v) => edit.preview((p, id) => planTextBox(p, id, { radius: v }))} onCommit={(v,origin) => v !== undefined && edit.commit((p, id) => planTextBox(p, id, { radius: v }),origin)} />
            </div>
          </>
        )}
      </Section>
    </>
  );
}

function Layout({ project, item, edit, disabled }: { project: ProjectModel; item: Item; edit: TextEdit; disabled: boolean }) {
  useEditorLanguage();
  const box = textBox(item, project);
  const text = item.type === "text";
  const rotate = item.transform?.rotate ?? 0;
  const opacity = Math.round((item.transform?.opacity ?? 1) * 100);
  return (
    <Section title={editorText("Layout")} testId="insp-sec-layout">
      {text && (
        <div className="frame-align" role="group" aria-label={editorText("Align to the frame")}>
          {EDGES.map(([edge, icon, title]) => (
            <button key={edge} className="icon-btn" data-testid="insp-frame-align" data-edge={edge} title={title} aria-label={title} disabled={disabled} onClick={() => edit.commit((p, id) => planFrameAlign(p, id, edge, measureText(p.tracks.flatMap((t) => t.items).find((i) => i.id === id) ?? item, p)))}>
              {icon}
            </button>
          ))}
        </div>
      )}
      <div className="prop-grid">
        <NumInput onCancel={edit.cancel} testId="insp-x" label={editorText("X")} unit="px" value={box.x} disabled={disabled} onPreview={(v) => edit.preview((p, id) => planTransform(p, id, { x: v }))} onCommit={(v,origin) => v !== undefined && edit.commit((p, id) => planTransform(p, id, { x: v }),origin)} />
        <NumInput onCancel={edit.cancel} testId="insp-y" label={editorText("Y")} unit="px" value={box.y} disabled={disabled} onPreview={(v) => edit.preview((p, id) => planTransform(p, id, { y: v }))} onCommit={(v,origin) => v !== undefined && edit.commit((p, id) => planTransform(p, id, { y: v }),origin)} />
        <NumInput onCancel={edit.cancel}
          testId="insp-w"
          label={editorText("W")}
          unit="px"
          value={box.w}
          placeholder={editorText("Auto")}
          min={1}
          clearable
          disabled={disabled}
          onPreview={(v) => edit.preview((p, id) => planTransform(p, id, { w: v }))}
          onCommit={(v,origin) => edit.commit((p, id) => planTransform(p, id, { w: v ?? null }, v === undefined ? "Fit the text box to its text" : "Set the text box width"),origin)}
        />
        {text && (
          <NumInput onCancel={edit.cancel}
            testId="insp-rotate"
            label="↻"
            title={editorText("Rotation")}
            unit="°"
            value={rotate}
            step={1}
            min={-180}
            max={180}
            disabled={disabled}
            onPreview={(v) => edit.preview((p, id) => planTransform(p, id, { rotate: v }))}
            onCommit={(v,origin) => v !== undefined && edit.commit((p, id) => planTransform(p, id, { rotate: v }, `Rotate text ${v}°`),origin)}
          />
        )}
      </div>
      {text && (
        <Prop label={editorText("Opacity")} onReset={opacity !== 100 ? () => edit.commit((p, id) => planTransform(p, id, { opacity: null }, "Make text opaque")) : undefined}>
          <SliderNum onCancel={edit.cancel}
            testId="insp-opacity"
            value={opacity}
            min={0}
            max={100}
            unit="%"
            disabled={disabled}
            onPreview={(v) => edit.preview((p, id) => planTransform(p, id, { opacity: v / 100 }))}
            onCommit={(v,origin) => edit.commit((p, id) => planTransform(p, id, { opacity: v / 100 }, `Set text opacity to ${v}%`),origin)}
          />
        </Prop>
      )}
    </Section>
  );
}

/** The Style tab of a text (and of captions, which share its look but not rotation, opacity or frame alignment). */
export function TextStyleTab({ project, item, disabled }: { project: ProjectModel; item: Item; disabled: boolean }) {
  useEditorLanguage();
  const edit = useTextEdit(project, item);
  return (
    <div className="pstack" data-testid="text-style" data-targets={edit.ids.length}>
      <LookPicker item={item} edit={edit} disabled={disabled} />
      <Typography project={project} item={item} edit={edit} disabled={disabled} />
      <Fill project={project} item={item} edit={edit} disabled={disabled} />
      <Decor project={project} item={item} edit={edit} disabled={disabled} />
      <Layout project={project} item={item} edit={edit} disabled={disabled} />
    </div>
  );
}

