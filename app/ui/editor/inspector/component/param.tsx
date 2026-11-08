import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// One component param's control, generated from its schema (D76, rebuilt in D130): text (a
// textarea for multi-line), a colour (swatch, hex, opacity), a number with its unit (a slider too
// when the schema bounds it; the label scrubs), a choice (segmented when short, else a menu), a
// switch, an image picker. Each sits in a labelled row with a reset to the manifest's default.
// Typing and sliding preview live; each commit is one edit (planComponentProps, the whole props).
import { COMPONENT_IMAGE, componentDefaults, type ComponentInfo } from "@/app/lib/editor/core/visual/components";
import { STICKER_ART } from "@/app/lib/editor/core/visual/stickers";
import {stickerLabel} from '@/app/lib/editor/sticker-library';
import { FOLLOW_PLATFORM_NAMES } from "@/app/lib/editor/core/visual/elementsMore";
import type { Item } from "@/app/lib/editor/core/timeline/types";

import type { ProjectModel } from "@/app/lib/editor/state/project";
import { planComponentProps } from "@/app/ui/editor/preview/components";
import { type PlanResult } from "@/app/ui/editor/inspector/fields";
import {useProjectEdit} from '../../workspace/projectEdit';
import {editorGestureOrigin, type GestureOrigin} from '../../workspace/pointerGesture';
import { useTextDraft } from '../text/draft';
import { findItem } from '@/app/ui/editor/timeline/edit';
import { NumInput, Prop, Segmented, SettledColorField, SliderNum, Toggle } from "@/app/ui/editor/inspector/controls";
import { labelOf, withProp, type Schema } from "@/app/ui/editor/inspector/component/schema";

/**
 * A text field: typing shows in the preview at once (a local draft), saved after a pause, on blur
 * or Enter (⌘Enter when multi-line); Esc puts the saved value back.
 */
export function TextParam({ project, value, multiline, disabled, testId, read, plan, maxLength }: { project:ProjectModel; value: string; multiline: boolean; disabled: boolean; testId: string; read:(project:ProjectModel)=>string|undefined; plan:(project:ProjectModel,value:string)=>PlanResult; maxLength?: number }) {
  useEditorLanguage();
  const field=useTextDraft({project,value,multiline,disabled,read,plan});
  return multiline ? (
    <textarea className="field input text-area mono-area" data-testid={testId} rows={3} maxLength={maxLength} disabled={disabled} spellCheck={false} {...field.input} />
  ) : (
    <input className="field input" data-testid={testId} maxLength={maxLength} disabled={disabled} spellCheck={false} {...field.input} />
  );
}


const same = (a: unknown, b: unknown) => JSON.stringify(a) === JSON.stringify(b);

/** One param's control in its row, from its schema. */
export function Param({ project, item, info, name, schema, disabled }: { project: ProjectModel; item: Item; info: ComponentInfo; name: string; schema: Schema; disabled: boolean }) {
  useEditorLanguage();
  const props = (item.props ?? {}) as Record<string, unknown>;
  const value = props[name];
  const noun = info.name.toLowerCase();
  const required = info.params.required?.includes(name) ?? false;
  const label = labelOf(name);
  const fallback = componentDefaults(info.id)[name];
  const edit=useProjectEdit();
  const commit=(next:unknown,summary=`Change ${label.toLowerCase()} of ${noun}`,origin:GestureOrigin|undefined=editorGestureOrigin())=>edit.commit(p=>planComponentProps(p,item.id,withProp(p,item.id,name,next),summary),origin);
  const preview=(next:unknown)=>edit.preview(project,planComponentProps(project,item.id,withProp(project,item.id,name,next),''));
  // Reset: back to the manifest's default (or no value, when it has none and may be left out).
  const reset = !disabled && value !== undefined && !same(value, fallback) && (fallback !== undefined || !required) ? () => commit(fallback, `Reset ${label.toLowerCase()} of ${noun}`) : undefined;
  const testId = `param-${name}`;
  const unit = schema["x-unit"];

  if (schema.enum) {
    const options = schema.enum.map((v, index) => {
      const id = String(v);
      const art = name === 'art' ? STICKER_ART[id] : undefined;
      const title = (name === "platform" ? FOLLOW_PLATFORM_NAMES[id] : undefined) ?? (id.charAt(0).toUpperCase() + id.slice(1));
      return {id: String(index), value: v, label: art ? stickerLabel(art) : editorText(title)};
    });
    const selected = options.find(o => same(o.value, value ?? fallback))?.id ?? "";
    const choose = (id: string) => { const option = options.find(o => o.id === id); void commit(option?.value, option ? `Set ${label.toLowerCase()} of ${noun} to ${option.label}` : `Clear ${label.toLowerCase()} of ${noun}`); };
    const short = options.length <= 4 && options.every(o => o.label.length <= 8);
    return (
      <Prop label={label} onReset={reset} testId={`${testId}-row`}>
        {short ? (
          <Segmented testId={testId} options={options.map(o => [o.id, o.label] as const)} value={selected} disabled={disabled} onChange={choose} />
        ) : (
          <select className="pselect" data-testid={testId} value={selected} disabled={disabled} onChange={e => choose(e.target.value)}>
            {/* An optional choice without a default (a sticker's drawn art, D165) can be left unset. */}
            {!required && fallback === undefined && <option value="">{editorText("None")}</option>}
            {options.map((o) => (
              <option key={o.id} value={o.id}>
                {/* A sticker's drawn art by its name ("Hand-drawn circle"), not its id (D263); a follow handle's platform by its brand name (D275). */}
                {o.label}
              </option>
            ))}
          </select>
        )}
      </Prop>
    );
  }
  if (schema["x-control"] === "color") {
    return (
      <Prop label={label} onReset={reset} testId={`${testId}-row`}>
        <SettledColorField onCancel={edit.cancel} project={project} testId={testId} value={typeof value === "string" ? value : typeof fallback === "string" ? fallback : undefined} disabled={disabled} onPreview={preview} onCommit={(c,origin) => commit(c,undefined,origin)} />
      </Prop>
    );
  }
  if (schema.type === "string") {
    const multiline = schema["x-control"] === "multiline";
    const field = (
      <TextParam
        maxLength={schema.maxLength}
        project={project}
        value={typeof value === "string" ? value : ""}
        multiline={multiline}
        disabled={disabled}
        testId={testId}
        read={p=>{const current=findItem(p,item.id)?.item;if(!current||current.component!==item.component)return undefined;const text=current.props?.[name];return typeof text==='string'?text:''}}
        plan={(p,text)=>planComponentProps(p,item.id,withProp(p,item.id,name,text===''&&!required?undefined:text),`Change ${label.toLowerCase()} of ${noun}`)}
      />
    );
    return multiline ? (
      <div className="prop-stack" data-testid={`${testId}-row`}>
        <span className="prop-k">{label}</span>
        {field}
      </div>
    ) : (
      <Prop label={label} onReset={reset} testId={`${testId}-row`}>
        {field}
      </Prop>
    );
  }
  if (schema.type === "number" || schema.type === "integer") {
    const int = schema.type === "integer";
    const n = typeof value === "number" ? value : typeof fallback === "number" ? fallback : undefined;
    const bounded = schema.minimum !== undefined && schema.maximum !== undefined;
    const step = int ? 1 : bounded && schema.maximum! - schema.minimum! <= 5 ? 0.1 : 1;
    const round = (v: number) => (int ? Math.round(v) : v);
    return (
      <Prop label={label} onReset={reset} testId={`${testId}-row`}>
        {bounded ? (
          <SliderNum onCancel={edit.cancel} testId={testId} value={n ?? schema.minimum!} min={schema.minimum!} max={schema.maximum!} step={step} unit={unit} disabled={disabled} onPreview={(v) => preview(round(v))} onCommit={(v,origin) => commit(round(v),undefined,origin)} />
        ) : (
          <NumInput onCancel={edit.cancel} testId={testId} value={n} unit={unit} step={step} min={schema.minimum} max={schema.maximum} disabled={disabled} clearable={!required} onPreview={(v) => preview(round(v))} onCommit={(v,origin) => commit(v === undefined ? undefined : round(v),undefined,origin)} />
        )}
      </Prop>
    );
  }
  if (schema.type === "boolean") {
    const on = value === undefined ? fallback === true : value === true;
    return (
      <Prop label={label} onReset={reset} testId={`${testId}-row`}>
        <div className="prop-toggle">
          <Toggle testId={testId} label={label} value={on} disabled={disabled} onChange={(v) => commit(v ? true : fallback === true ? false : undefined, `${v ? "Turn on" : "Turn off"} ${label.toLowerCase()} of ${noun}`)} />
        </div>
      </Prop>
    );
  }
  // A media reference ({ media: id }): an image (a logo, a framed picture).
  if (schema.type === "object" && schema.properties?.media) {
    const id = value && typeof value === "object" ? (value as { media?: unknown }).media : undefined;
    // Only images the preview and export can draw in a component (no TIFF or HEIC).
    const images = project.media.filter((m) => m.kind === "image" && COMPONENT_IMAGE.test(m.path));
    return (
      <Prop label={label} onReset={!disabled && value !== undefined && !required ? () => commit(undefined, `Remove ${label.toLowerCase()} of ${noun}`) : undefined} testId={`${testId}-row`}>
        <select
          className="pselect"
          data-testid={testId}
          value={typeof id === "string" ? id : ""}
          disabled={disabled}
          onChange={(e) => (e.target.value ? commit({ media: e.target.value }, `Set ${label.toLowerCase()} of ${noun}`) : commit(undefined, `Remove ${label.toLowerCase()} of ${noun}`))}
        >
          <option value="">{images.length ? editorText("None") : editorText("No images")}</option>
          {images.map((m) => (
            <option key={m.id} value={m.id}>
              {m.name}
            </option>
          ))}
        </select>
      </Prop>
    );
  }
  return null;
}
