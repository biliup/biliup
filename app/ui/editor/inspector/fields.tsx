import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
import { editorRoot } from "@/app/lib/editor/environment";
import {beginEditorGesture,editorGestureOrigin,ownsEditorGesture,type GestureOrigin} from '../workspace/pointerGesture';
import {isTextComposition} from '@/app/lib/editor/keyboard';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The Inspector's shared pieces (split out of panels/ItemEditors.tsx): number fields, sliders,
// colour pickers, tabs and groups, and how an edit is drafted in the preview and sent (one
// project.edit through actions.itemEdit: validated, one ⌘Z, in the agent's next note).

import type { Op } from "@/app/lib/editor/core/timeline/types";
import { useEffect, useRef, useState, type ReactNode } from "react";

import { draftOf } from "@/app/ui/editor/timeline/edit";
import { getPlayback, playback, type PreviewDraft } from "@/app/ui/editor/preview/playback";
import { type ProjectModel } from "@/app/lib/editor/state/project";
import { actions, endPreview, getState, previewEdits } from "@/app/lib/editor/state/store";

import {useControlGesture} from './gesture';


/** How long typing has to pause before it's saved (it's also saved on blur and ⌘Enter). */
export const TYPING_PAUSE_MS = 800;

export type PlanResult = { readonly ops: ReadonlyArray<Op>; readonly summary: string } | { readonly problem: string };

/** Show a plan's result in the preview without sending it; returns the draft (to drop later). */
export function showDraft(project: ProjectModel, plan: PlanResult): PreviewDraft | undefined {
  if (!("ops" in plan) || !plan.ops.length) return undefined;
  const result = draftOf(project, plan.ops);
  if (!("project" in result)) return undefined;
  const draft = { base: project, project: result.project };
  playback.setDraft(draft);
  return draft;
}
export const dropDraft = (draft: PreviewDraft | undefined) => {
  if (draft && getPlayback().draft === draft) playback.setDraft(undefined);
};

/** The revision and folder an edit is planned against (see actions.itemEdit). */
export const origin = () => ({ baseRevision: getState().snapshot?.revision, dir: getState().snapshot?.dir });

/** Send an edit planned against the newest project; the draft (if any) goes once it's applied. */
export function send(make: (project: ProjectModel) => PlanResult, draft?: PreviewDraft, from = origin()) {
  void actions.itemEdit(make, from).finally(() => dropDraft(draft));
}

/** Pixels the pointer moves before a press on a number field is a scrub, not a click. */
export const SCRUB_THRESHOLD = 3;

const clampTo = (v: number, min: number | undefined, max: number | undefined) => Math.min(max ?? Infinity, Math.max(min ?? -Infinity, v));
/** Rounded to the step's decimals (and never more than 6). */
const roundTo = (v: number, step: number) => {
  const decimals = Math.min(6, Math.max(0, -Math.floor(Math.log10(step) + 1e-9)));
  return Number(v.toFixed(decimals));
};

/**
 * A number field: typed freely, committed on Enter or blur, Esc puts the saved value back.
 * D209 (issue #21): press and drag left or right on the field or its label to scrub it (one step
 * a pixel, ⇧ ten, ⌥ a tenth; the pointer is locked so it never hits the screen's edge). While it
 * drags, the change shows in the preview (its edits are previewed, not sent: previewEdits); the
 * release commits once, one ⌘Z. A press that doesn't move is a click: it focuses for typing.
 * ↑ and ↓ step (⇧ ten) and preview; Enter or blur commits, Esc drops it.
 */
export function NumberField({ label, value, onCommit, testId, placeholder, step = 1, min, max, disabled }: { label: string; value: number | undefined; onCommit: (v: number | undefined,origin?:GestureOrigin) => void; testId: string; placeholder?: string; step?: number; min?: number; max?: number; disabled?: boolean }) {
  useEditorLanguage();
  const shown = value === undefined ? "" : String(value);
  const [text, setText] = useState(shown);
  const [scrubbing, setScrubbing] = useState(false);
  const editing = useRef(false);
  const input = useRef<HTMLInputElement>(null);
  const stop=useRef<(()=>void)|undefined>(undefined);
  const editOrigin=useRef<GestureOrigin|undefined>(undefined);
  const end=(after:boolean)=>{if(editOrigin.current)endPreview(after,editOrigin.current.owner)};
  useEffect(()=>()=>{stop.current?.();end(false)},[]);
  useEffect(()=>{if(disabled){stop.current?.();end(false);editing.current=false;setText(shown)}},[disabled]);
  // A scrub just ended: the label's click that follows mustn't focus the field.
  const dragged = useRef(false);
  useEffect(() => {
    if (!editing.current) setText(shown);
  }, [shown]);
  const parse = (t: string) => {
    const trimmed = t.trim();
    // A comma is a decimal point too (typed on a Polish keyboard, say); shown values always use ".".
    return trimmed === "" ? undefined : Number(trimmed.replace(",", "."));
  };
  const valid = (n: number | undefined) => n === undefined || (Number.isFinite(n) && (min === undefined || n >= min) && (max === undefined || n <= max));
  const commit = () => {
    const origin=editOrigin.current;
    if(!editing.current||disabled||!origin||!ownsEditorGesture(origin)){
      editing.current=false;end(false);setText(shown);return;
    }
    editing.current = false;
    const n = parse(text);
    if (!valid(n) || n === value) {
      end(false);
      return setText(shown);
    }
    onCommit(n,origin);
    end(true);
  };
  /** Shows `n` in the field and its edit in the preview. */
  const preview = (n: number) => {
    if(!editOrigin.current||!ownsEditorGesture(editOrigin.current))return;
    setText(String(n));
    previewEdits(() => onCommit(n));
  };

  const onPointerDown = (e: React.PointerEvent<HTMLElement>) => {
    if (disabled || e.button !== 0) return;
    const el = input.current;
    if (!el) return;
    // Already typing in it: the press places the caret / selects text as usual.
    if (document.activeElement === el && e.target === el) return;
    stop.current?.();
    const origin=editorGestureOrigin();if(!origin)return;
    editOrigin.current=origin;
    e.preventDefault();
    const start = parse(text) ?? value ?? 0;
    const x0 = e.clientX;
    let lastX = e.clientX;
    let travel = 0;
    let moved = false;
    let current = start;
    let active = true;
    const target = e.currentTarget;
    const root=editorRoot();
    const move = (ev: PointerEvent) => {
      // Locked, the pointer stays put and reports how far it moved; else follow it.
      if (!moved && Math.abs(ev.clientX - x0) < SCRUB_THRESHOLD) return;
      // The travel up to the threshold counts too (lastX is still where the press was).
      const dx = document.pointerLockElement === target ? ev.movementX : ev.clientX - lastX;
      lastX = ev.clientX;
      if (!moved) {
        moved = true;
        editing.current = true;
        setScrubbing(true);
        root.classList.add("scrubbing");
        try {
          const ask = target.requestPointerLock?.() as unknown;
          if (ask && typeof (ask as Promise<void>).then === "function") void (ask as Promise<void>).then(()=>{
            if(!active&&document.pointerLockElement===target)document.exitPointerLock();
          }).catch(()=>{});
        } catch { /* Relative dragging also works without pointer lock. */ }
      }
      const factor = ev.shiftKey ? 10 : ev.altKey ? 0.1 : 1;
      travel += dx * factor;
      const next = roundTo(clampTo(start + Math.round(travel) * step, min, max), ev.altKey ? step / 10 : step);
      if (next === current) return;
      current = next;
      preview(next);
    };
    const cleanup=()=>{
      active=false;stop.current=undefined;
      if(document.pointerLockElement===target)document.exitPointerLock();
      root.classList.remove('scrubbing');setScrubbing(false);editing.current=false;
      if(moved){dragged.current=true;setTimeout(()=>{dragged.current=false},0)}
    };
    const up = (origin:GestureOrigin) => {
      cleanup();
      if (!moved) {
        // A click: type in it.
        editing.current=true;editOrigin.current=origin;
        el.focus();
        el.select();
        return;
      }
      if (current === value) {
        end(false);
        return setText(shown);
      }
      onCommit(current,origin);
      end(true);
    };
    stop.current=beginEditorGesture(e,{move,commit:up,cancel:()=>{cleanup();end(false);setText(shown)}});
  };

  return (
    <label className={`field num-field ${scrubbing ? "scrubbing" : ""}`} data-scrub={testId}
      onPointerDown={onPointerDown}
      onClick={(e) => {
        if (dragged.current) e.preventDefault();
      }}
    >
      <span className="k" title={editorText("Drag left or right to change it (⇧ faster, ⌥ finer)")}>
        {label}
      </span>
      <input
        ref={input}
        className="v"
        data-testid={testId}
        // Text, not type="number": Chromium shows a number input in the system locale ("0,5"),
        // and the app writes numbers with "." everywhere else. ↑/↓ step like a number input.
        type="text"
        inputMode="decimal"
        spellCheck={false}
        value={text}
        placeholder={placeholder}
        disabled={disabled}
        onFocus={() => {editing.current=true;editOrigin.current=editorGestureOrigin()}}
        onChange={(e) => setText(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => {
          if(isTextComposition(e.nativeEvent))return;
          if (e.key === "Enter") {e.preventDefault();e.stopPropagation();e.currentTarget.blur()}
          if (e.key === "ArrowUp" || e.key === "ArrowDown") {
            e.preventDefault();
            e.stopPropagation();
            const base = parse(text);
            const by = (e.shiftKey ? 10 : 1) * step;
            const next = roundTo(clampTo((base !== undefined && Number.isFinite(base) ? base : (value ?? 0)) + (e.key === "ArrowUp" ? by : -by), min, max), step);
            editing.current = true;
            preview(next);
          }
          if (e.key === "Escape") {
            e.preventDefault();e.stopPropagation();
            editing.current = false;
            setText(shown);
            end(false);
            const el = e.currentTarget;
            requestAnimationFrame(() => el.blur());
          }
        }}
      />
    </label>
  );
}

/** A range slider: one preview gesture, one save on release, explicit cancellation. */
export function Slider({value,min,max,step,onPreview,onCommit,onCancel,testId,disabled,title}:{value:number;min:number;max:number;step?:number;onPreview:(v:number)=>void;onCommit:(v:number,origin?:GestureOrigin)=>void;onCancel:()=>void;testId:string;disabled?:boolean;title?:string}){
 useEditorLanguage()
 const gesture=useControlGesture({value,disabled,onPreview,onCommit,onCancel})
 const rangeKeys=['ArrowLeft','ArrowRight','ArrowUp','ArrowDown','Home','End','PageUp','PageDown']
 return <input type="range" className="slider" data-testid={testId} min={min} max={max} step={step} title={title} value={gesture.local?.value??value} disabled={disabled}
  onFocus={()=>gesture.begin()}
  onPointerDown={event=>{if(event.button!==0||!gesture.pointer(event))event.preventDefault()}}
  onChange={event=>{if(gesture.isActive())gesture.change(Number(event.currentTarget.value))}}
  onKeyDown={event=>{
   if(isTextComposition(event.nativeEvent))return
   if(event.key==='Escape'){event.preventDefault();event.stopPropagation();gesture.cancel();event.currentTarget.blur()}
   else if(rangeKeys.includes(event.key)){event.stopPropagation();if(!gesture.begin())event.preventDefault()}
  }}
  onKeyUp={event=>{if(!isTextComposition(event.nativeEvent)&&rangeKeys.includes(event.key)){event.stopPropagation();gesture.commit()}}}
  onBlur={()=>{if(!gesture.isPointer())gesture.commit()}}
 />
}
export function Tabs<T extends string>({ tabs, value, onChange }: { tabs: ReadonlyArray<readonly [T, string, boolean?]>; value: T; onChange: (t: T) => void }) {
  useEditorLanguage();
  return (
    <div className="seg insp-tabs">
      {tabs.map(([id, label, disabled]) => (
        <button key={id} className={value === id ? "active" : ""} data-testid={`insp-tab-${id}`} disabled={disabled} onClick={() => onChange(id)}>
          {label}
        </button>
      ))}
    </div>
  );
}

export const Group = ({ label, children }: { label: string; children: ReactNode }) => (
  <div className="group">
    <div className="group-label">{label}</div>
    {children}
  </div>
);

/**
 * The text's content. Typing shows in the preview at once (a local draft) and is saved after a
 * pause of TYPING_PAUSE_MS, on blur, or with ⌘Enter; Esc puts the saved text back. A burst is
 * planned against the revision it started at, so an agent change to the same item meanwhile is
 * refused rather than overwritten (D6).
 */
