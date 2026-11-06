import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// List parameters use the element's schema and the same controls, drafts and save queue as its other properties.
import { componentDefaults, type ComponentInfo } from '@/app/lib/editor/core/visual/components';
import type { Item } from '@/app/lib/editor/core/timeline/types';
import type { ProjectModel } from '@/app/lib/editor/state/project';
import { Minus, Plus } from 'lucide-react';
import type { ReactNode } from 'react';
import { findItem } from '../../timeline/edit';
import { planComponentProps } from '../../preview/components';
import { useProjectEdit } from '../../workspace/projectEdit';
import { editorGestureOrigin, type GestureOrigin } from '../../workspace/pointerGesture';
import { NumInput, Prop, Section, SettledColorField, Toggle } from '../controls';
import type { PlanResult } from '../fields';
import { useTextDraft } from '../text/draft';
import { labelOf, withProp, type Schema } from './schema';

type Path = ReadonlyArray<string | number>;
const record = (value: unknown): value is Record<string, unknown> => !!value && typeof value === 'object' && !Array.isArray(value);
const changed = () => ({problem: editorText('This list changed. Start the edit again.')});

function readAt(value: unknown, path: Path): unknown {
  for (const key of path) {
    if (typeof key === 'number') { if (!Array.isArray(value)) return; value = value[key]; }
    else { if (!record(value)) return; value = value[key]; }
  }
  return value;
}

/** Replace a leaf without changing any other fields or assigning to an out-of-date row. */
function replaceAt(value: unknown, path: Path, next: unknown): {value: unknown} | {problem: string} {
  if (!path.length) return {value: next};
  const [key, ...rest] = path;
  if (typeof key === 'number') {
    if (!Array.isArray(value) || key < 0 || key >= value.length) return changed();
    const child = replaceAt(value[key], rest, next);
    if ('problem' in child) return child;
    const result = [...value]; result[key] = child.value;
    return {value: result};
  }
  if (key === undefined || !record(value)) return changed();
  const child = replaceAt(value[key], rest, next);
  if ('problem' in child) return child;
  const result = {...value};
  if (child.value === undefined) delete result[key]; else result[key] = child.value;
  return {value: result};
}

/** New rows start with valid types, required fields, enum values and minimum row counts. */
function blank(schema: Schema): unknown {
  if (schema.default !== undefined) return schema.default;
  if (schema.enum?.length) return schema.enum[0];
  if (schema.type === 'array') return Array.from({length: schema.minItems ?? 0}, () => blank(schema.items ?? {}));
  if (schema.type === 'object') return Object.fromEntries((schema.required ?? []).map(key => [key, blank(schema.properties?.[key] ?? {})]));
  if (schema.type === 'boolean') return false;
  if (schema.type === 'number' || schema.type === 'integer') {
    const low = schema.minimum ?? (schema.exclusiveMinimum === undefined ? -Infinity : schema.exclusiveMinimum + 1);
    const high = schema.maximum ?? (schema.exclusiveMaximum === undefined ? Infinity : schema.exclusiveMaximum - 1);
    return Math.min(high, Math.max(low, 0));
  }
  return '';
}

interface ListEdit {
  project: ProjectModel;
  disabled: boolean;
  name: string;
  read: (project: ProjectModel) => unknown;
  plan: (project: ProjectModel, path: Path, next: unknown) => PlanResult;
  commit: (path: Path, next: unknown, origin?: GestureOrigin) => void;
  remove: (path: Path, index: number) => void;
  preview: (path: Path, next: unknown) => void;
  cancel: () => void;
}

export function ListParam({ project, item, info, name, schema, disabled }: { project: ProjectModel; item: Item; info: ComponentInfo; name: string; schema: Schema; disabled: boolean }) {
  useEditorLanguage();
  const editing = useProjectEdit();
  const fallback = componentDefaults(info.id)[name];
  const read = (p: ProjectModel) => findItem(p, item.id)?.item.props?.[name] ?? fallback;
  const plan = (p: ProjectModel, path: Path, next: unknown, removedColumn?: number): PlanResult => {
    if (!findItem(p, item.id)) return changed();
    const result = replaceAt(read(p), path, next);
    if ('problem' in result) return result;
    const props = withProp(p, item.id, name, result.value);
    // A compare table's names and per-row values identify the same columns.
    if (item.component === 'component.compare-table' && name === 'names' && !path.length && removedColumn !== undefined && Array.isArray(props.rows)) {
      props.rows = props.rows.map(row => {
        if (!record(row)) return row;
        const nextRow = {...row};
        for (const key of ['values', 'text']) {
          const cells = row[key];
          if (Array.isArray(cells)) nextRow[key] = cells.filter((_, index) => index !== removedColumn);
        }
        return nextRow;
      });
    }
    return planComponentProps(p, item.id, props, editorText('Edit {0}', {0: editorText(labelOf(name))}));
  };
  const edit: ListEdit = {
    project, disabled, name, read, plan,
    commit: (path, next, origin = editorGestureOrigin()) => { void editing.commit(p => plan(p, path, next), origin); },
    remove: (path, index) => { void editing.commit(p => {
      const rows = readAt(read(p), path);
      return Array.isArray(rows) && index < rows.length ? plan(p, path, rows.filter((_, i) => i !== index), index) : changed();
    }, editorGestureOrigin()); },
    preview: (path, next) => editing.preview(project, plan(project, path, next)),
    cancel: editing.cancel,
  };
  return <Section title={editorText(labelOf(name))} testId={`param-sec-${name}`}>
    <ArrayRows edit={edit} schema={schema} path={[]} value={read(project)} />
  </Section>;
}

function ArrayRows({edit, schema, path, value}: {edit: ListEdit; schema: Schema; path: Path; value: unknown}) {
  const rows: ReadonlyArray<unknown> = Array.isArray(value) ? value : [];
  const itemSchema = schema.items ?? {};
  const testId = `param-${[edit.name, ...path].join('-')}`;
  return <div className="param-list" data-testid={testId}>
    {rows.map((row, index) => <div key={index} className="param-row" data-testid="param-row">
      <div className="param-row-content"><ListField edit={edit} schema={itemSchema} path={[...path, index]} value={row} required /></div>
      <button type="button" className="icon-btn" data-testid="param-row-remove" title={editorText('Remove list item {0}', {0: index + 1})} aria-label={editorText('Remove list item {0}', {0: index + 1})} disabled={edit.disabled || rows.length <= (schema.minItems ?? 0)} onClick={() => edit.remove(path, index)}><Minus /></button>
    </div>)}
    <button type="button" className="btn small-btn" data-testid="param-row-add" disabled={edit.disabled || (schema.maxItems !== undefined && rows.length >= schema.maxItems)} onClick={() => edit.commit(path, [...rows, blank(itemSchema)])}><Plus />{editorText('Add item')}</button>
  </div>;
}

function ListField({edit, schema, path, value, required, label}: {edit: ListEdit; schema: Schema; path: Path; value: unknown; required: boolean; label?: string}) {
  const testId = `param-${[edit.name, ...path].join('-')}`;
  const noun = label ?? editorText('List item {0}', {0: Number(path.at(-1)) + 1});
  const wrap = (content: ReactNode) => label ? <Prop label={label}>{content}</Prop> : content;
  const shown = value ?? schema.default;
  if (schema.enum) {
    const choices = schema.enum;
    return wrap(<select className="pselect" data-testid={testId} aria-label={noun} value={choices.findIndex(v => Object.is(v, shown))} disabled={edit.disabled} onChange={event => edit.commit(path, Number(event.target.value) < 0 ? undefined : choices[Number(event.target.value)])}>
      <option value={-1} disabled={required}>{editorText(required ? 'Select a value' : 'None')}</option>
      {choices.map((v, i) => <option key={i} value={i}>{editorText(String(v).charAt(0).toUpperCase() + String(v).slice(1))}</option>)}
    </select>);
  }
  if (schema.type === 'array') return <div className="param-nested">{label && <div className="param-list-label">{label}</div>}<ArrayRows edit={edit} schema={schema} path={path} value={shown} /></div>;
  if (schema.type === 'object') return <div className="param-row-fields">{Object.entries(schema.properties ?? {}).map(([key, field]) => <ListField key={key} edit={edit} schema={field} path={[...path, key]} value={record(shown) ? shown[key] : undefined} required={schema.required?.includes(key) ?? false} label={editorText(labelOf(key))} />)}</div>;
  if (schema.type === 'boolean') return wrap(<Toggle label={noun} testId={testId} disabled={edit.disabled} value={shown === true} onChange={on => edit.commit(path, on || required || schema.default === true ? on : undefined)} />);
  if (schema.type === 'number' || schema.type === 'integer') {
    const number = (next: number) => schema.type === 'integer' ? Math.round(next) : next;
    return wrap(<NumInput testId={testId} ariaLabel={noun} value={typeof shown === 'number' ? shown : undefined} min={schema.minimum} max={schema.maximum} step={schema.type === 'integer' ? 1 : 0.1} unit={schema['x-unit']} disabled={edit.disabled} clearable={!required} onPreview={v => edit.preview(path, number(v))} onCancel={edit.cancel} onCommit={(v, origin) => edit.commit(path, v === undefined ? undefined : number(v), origin)} />);
  }
  if (schema['x-control'] === 'color') return wrap(<SettledColorField project={edit.project} value={typeof shown === 'string' ? shown : undefined} testId={testId} disabled={edit.disabled} onPreview={v => edit.preview(path, v)} onCancel={edit.cancel} onCommit={(v, origin) => edit.commit(path, v, origin)} />);
  return wrap(<ListText edit={edit} path={path} value={typeof shown === 'string' ? shown : ''} schema={schema} required={required} testId={testId} label={noun} />);
}

function ListText({edit, path, value, schema, required, testId, label}: {edit: ListEdit; path: Path; value: string; schema: Schema; required: boolean; testId: string; label: string}) {
  const multiline = schema['x-control'] === 'multiline';
  const text = useTextDraft({
    project: edit.project, value, disabled: edit.disabled, multiline,
    read: p => {
      if (readAt(edit.read(p), path.slice(0, -1)) === undefined) return;
      const current = readAt(edit.read(p), path) ?? schema.default;
      return typeof current === 'string' ? current : '';
    },
    scope: p => JSON.stringify(edit.read(p) ?? null),
    plan: (p, next) => edit.plan(p, path, next === '' && !required ? undefined : next),
  });
  const props = {className: 'field input', 'data-testid': testId, 'aria-label': label, maxLength: schema.maxLength, disabled: edit.disabled, spellCheck: false, ...text.input};
  return multiline ? <textarea {...props} rows={3} /> : <input {...props} />;
}
