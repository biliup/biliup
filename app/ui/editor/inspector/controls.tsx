import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
import { editorEvents } from "@/app/lib/editor/environment";
import { editorRoot } from "@/app/lib/editor/environment";
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The Inspector's property controls (D129): the pieces the text and element inspectors are built
// from, in the manner of a design tool's property panel. A section (collapsible, with an optional
// action on its header), a labelled row with a reset, a number you can type or scrub (drag its
// label), a slider with its number, a colour (swatch, hex, opacity), a segmented choice, a toggle
// and a popover. Every control previews while it moves (onPreview: a local draft) and commits once
// (onCommit: one project.edit, one ⌘Z): scrubbing a slider or a label is one edit.
import { resolveColor } from "@/app/lib/editor/core/visual/render";
import { ChevronDown, ChevronRight, RotateCcw } from "lucide-react";
import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";

import { type ProjectModel } from "@/app/lib/editor/state/project";
import { swatches } from "@/app/ui/editor/preview/text";
import {Slider} from './fields';
import {useControlGesture} from './gesture';
import type {GestureOrigin} from '../workspace/pointerGesture';
import {isTextComposition} from '@/app/lib/editor/keyboard';
import { setSectionOpen, useSectionOpen } from "@/app/ui/editor/inspector/sections";
import { Tooltip } from "@/app/ui/editor/controls/Tooltip";

/**
 * A titled block of properties; the header can collapse it and hold an action (add, remove). With
 * `remember` (a section name) stores its open state in this browser, and a closed
 * one keeps its controls mounted, hidden (their values stay readable).
 */
export function Section({ title, testId, action, children, open: initial = true, remember, reveal }: { title: string; testId?: string; action?: ReactNode; children?: ReactNode; open?: boolean; remember?: string; reveal?: string }) {
  useEditorLanguage();
  const [local, setLocal] = useState(initial);
  useEffect(()=>{if(reveal&&!remember)setLocal(true)},[reveal,remember]);
  const saved = useSectionOpen(remember ?? "");
  const open = remember ? saved : local;
  const toggle = () => (remember ? setSectionOpen(remember, !open) : setLocal(!open));
  return (
    <section className="psec" data-testid={testId} data-section={remember} data-open={open}>
      <div className="psec-head">
        <button className="psec-title" aria-expanded={open} data-testid={testId ? `${testId}-toggle` : undefined} onClick={toggle}>
          {open ? <ChevronDown /> : <ChevronRight />}
          {typeof title === 'string' ? editorText(title) : title}
        </button>
        {action && <div className="psec-actions">{action}</div>}
      </div>
      {remember ? children && <div className="psec-body" hidden={!open}>{children}</div> : open && children && <div className="psec-body">{children}</div>}
    </section>
  );
}

/** One property: its label, its control, and a reset when it differs from its default. */
export function Prop({ label, children, onReset, testId }: { label: string; children: ReactNode; onReset?: (() => void) | undefined; testId?: string }) {
  useEditorLanguage();
  return (
    <div className="prop" data-testid={testId}>
      <span className="prop-k">{typeof label === 'string' ? editorText(label) : label}</span>
      <div className="prop-v">{children}</div>
      {onReset ? (
        <Tooltip label={editorText("Reset {0}", { 0: label })}>
          <button className="prop-reset" data-testid={testId ? `${testId}-reset` : undefined} onClick={onReset}>
            <RotateCcw />
          </button>
        </Tooltip>
      ) : (
        <span className="prop-reset-gap" />
      )}
    </div>
  );
}

const clampTo = (v: number, min?: number, max?: number) => Math.min(max ?? Infinity, Math.max(min ?? -Infinity, v));
const roundTo = (v: number, step: number) => {
  return Number((Math.round(v / step) * step).toFixed(6));
};

/**
 * A number: typed (Enter or blur commits, Esc puts it back, ↑/↓ step, ⇧ ten steps) or scrubbed by
 * dragging its label left and right (previewed while dragging, one commit on release). `unit`
 * shows after the value. An empty field commits undefined (the default) when `clearable`.
 */
export function NumInput({value,onCommit,onPreview,onCancel,testId,label,unit,step=1,min,max,placeholder,disabled,clearable,title,ariaLabel}:{value:number|undefined;onCommit:(v:number|undefined,origin?:GestureOrigin)=>void;onPreview?:(v:number)=>void;onCancel?:()=>void;testId:string;label?:ReactNode;unit?:string;step?:number;min?:number;max?:number;placeholder?:string;disabled?:boolean;clearable?:boolean;title?:string;ariaLabel?:string}){
 useEditorLanguage()
 const shown=value===undefined?'':String(Number(value.toFixed(6)))
 const [text,setText]=useState(shown)
 const editing=useRef(false)
 const composing=useRef(false),blurred=useRef(false)
 const gesture=useControlGesture<number|undefined>({value,disabled,
  onPreview:next=>{if(next!==undefined)onPreview?.(next)},
  onCommit:(next,origin)=>{editing.current=false;onCommit(next,origin)},
  onCancel:()=>{editing.current=false;setText(shown);onCancel?.()},
 })
 useEffect(()=>{if(!editing.current)setText(shown)},[shown])
 const commit=(raw=text)=>{
  if(!editing.current||composing.current)return
  const trimmed=raw.trim(),number=trimmed===''?undefined:Number(trimmed.replace(',','.'))
  if((number===undefined&&!clearable)||(number!==undefined&&!Number.isFinite(number)))return gesture.cancel()
  editing.current=false
  gesture.change(number===undefined?undefined:clampTo(number,min,max),false)
  gesture.commit()
 }
 const down=(event:React.PointerEvent<HTMLSpanElement>)=>{
  if(disabled||event.button!==0)return
  event.preventDefault()
  const x=event.clientX,from=value??0
  let moved=false
  if(!gesture.pointer(event,next=>{
   const dx=next.clientX-x
   if(!moved&&Math.abs(dx)<2)return
   moved=true
   const number=clampTo(roundTo(from+Math.round(dx/2)*step*(next.shiftKey?10:1),step),min,max)
   editing.current=true;setText(String(number));gesture.change(number)
  }))return
  editing.current=true
 }
 return <label className={`nfield ${disabled?'disabled':''}`} title={title?editorText(title):undefined}>
  {label!==undefined&&<span className="nfield-k" data-testid={`${testId}-scrub`} onPointerDown={down}>{typeof label==='string'?editorText(label):label}</span>}
  <input data-testid={testId} aria-label={ariaLabel} type="text" inputMode="decimal" spellCheck={false} value={text} placeholder={placeholder} disabled={disabled}
   onFocus={event=>{blurred.current=false;editing.current=gesture.begin();event.currentTarget.select()}}
   onChange={event=>{if(gesture.begin()){editing.current=true;setText(event.currentTarget.value)}}}
   onBlur={event=>{blurred.current=true;if(!gesture.isPointer())commit(event.currentTarget.value)}}
   onCompositionStart={()=>{composing.current=true}}
   onCompositionEnd={event=>{composing.current=false;if(blurred.current)commit(event.currentTarget.value)}}
   onKeyDown={event=>{
    if(composing.current||isTextComposition(event.nativeEvent))return
    if(event.key==='Enter'){event.preventDefault();event.stopPropagation();event.currentTarget.blur()}
    else if(event.key==='Escape'){event.preventDefault();event.stopPropagation();gesture.cancel();event.currentTarget.blur()}
    else if(event.key==='ArrowUp'||event.key==='ArrowDown'){
     event.preventDefault();event.stopPropagation()
     if(!gesture.begin())return
     const raw=text.trim(),base=raw===''?(value??0):Number(raw.replace(',','.'))
     const by=step*(event.shiftKey?10:1)*(event.key==='ArrowUp'?1:-1)
     const number=clampTo(roundTo((Number.isFinite(base)?base:(value??0))+by,step),min,max)
     editing.current=true;setText(String(number));gesture.change(number)
    }
   }}
  />
  {unit&&<span className="nfield-u">{unit}</span>}
 </label>
}

/** Both appearances use the same slider and numeric gesture implementations. */
export function SliderNum({value,min,max,step=1,unit,testId,disabled,onPreview,onCommit,onCancel,sliderMax}:{value:number;min:number;max:number;step?:number;unit?:string;testId:string;disabled?:boolean;onPreview:(v:number)=>void;onCommit:(v:number,origin?:GestureOrigin)=>void;onCancel:()=>void;sliderMax?:number}){
 useEditorLanguage()
 const [local,setLocal]=useState<number|undefined>(undefined)
 const preview=(next:number)=>{setLocal(next);onPreview(next)}
 const cancel=()=>{setLocal(undefined);onCancel()}
 const commit=(next:number,origin?:GestureOrigin)=>{setLocal(undefined);onCommit(next,origin)}
 return <div className="slider-num">
  <Slider testId={`${testId}-slider`} value={value} min={min} max={Math.max(sliderMax??max,value)} step={step} disabled={disabled} onPreview={preview} onCommit={commit} onCancel={cancel}/>
  <NumInput testId={testId} value={local??value} unit={unit} step={step} min={min} max={max} disabled={disabled} onPreview={preview} onCancel={cancel} onCommit={(next,origin)=>next!==undefined&&commit(next,origin)}/>
 </div>
}
/** A segmented choice: icons or short words, one active. */
export function Segmented<T extends string>({ options, value, onChange, testId, disabled, attr = "value" }: { options: ReadonlyArray<readonly [T, ReactNode, string?]>; value: T | undefined; onChange: (v: T) => void; testId: string; disabled?: boolean; attr?: string }) {
  useEditorLanguage();
  return (
    <div className="pseg" role="group">
      {options.map(([v, content, title]) => {
        const button = (
          <button key={v} className={value === v ? "active" : ""} data-testid={testId} {...{ [`data-${attr}`]: v }} aria-label={typeof title === 'string' ? editorText(title) : title} aria-pressed={value === v} disabled={disabled} onClick={() => value !== v && onChange(v)}>
            {typeof content === 'string' ? editorText(content) : content}
          </button>
        );
        return title ? (
          <Tooltip key={v} label={typeof title === 'string' ? editorText(title) : title}>
            {button}
          </Tooltip>
        ) : (
          button
        );
      })}
    </div>
  );
}

/**
 * A popover anchored under an element, drawn over everything (the Inspector scrolls and would clip it). Outside click or Esc closes it.
 * `align` "end" (the default) lines its right edge up with the anchor's; "start" its left edge, for an anchor near the window's left
 * (the left panel: growing leftwards would cover the rail, D263). Either way it stays inside the window.
 */
export function Popover({ anchor, onClose, children, testId, width = 240, align = "end" }: { anchor: HTMLElement | null; onClose: (reason?:'finish'|'cancel') => void; children: ReactNode; testId?: string; width?: number; align?: "start" | "end" }) {
  useEditorLanguage();
  const ref = useRef<HTMLDivElement>(null);
  const [at, setAt] = useState<{ left: number; top: number } | undefined>(undefined);
  useLayoutEffect(() => {
    const element=ref.current;
    if (!anchor||!element) return;
    const place=()=>{
      const r=anchor.getBoundingClientRect(),box=element.getBoundingClientRect();
      const left=Math.max(8,Math.min(window.innerWidth-box.width-8,align==='start'?r.left:r.right-box.width));
      const below=r.bottom+6,top=Math.max(8,Math.min(window.innerHeight-box.height-8,below+box.height>window.innerHeight-8?r.top-6-box.height:below));
      setAt(previous=>previous?.left===left&&previous.top===top?previous:{left,top});
    };
    place();
    const observer=new ResizeObserver(place);observer.observe(element);observer.observe(anchor);
    window.addEventListener('resize',place);window.addEventListener('scroll',place,true);
    return()=>{observer.disconnect();window.removeEventListener('resize',place);window.removeEventListener('scroll',place,true)};
  }, [anchor, width, align]);
  useEffect(() => {
    const down = (e: PointerEvent) => {
      if (ref.current?.contains(e.target as Node) || anchor?.contains(e.target as Node)) return;
      onClose('finish');
    };
    const key = (e: KeyboardEvent) => {if(!isTextComposition(e)&&e.key==='Escape'){e.preventDefault();e.stopPropagation();onClose('cancel')}};
    window.addEventListener("pointerdown", down, true);
    editorEvents.addEventListener("keydown", key, true);
    return () => {
      window.removeEventListener("pointerdown", down, true);
      editorEvents.removeEventListener("keydown", key, true);
    };
  }, [anchor, onClose]);
  return createPortal(
    <div ref={ref} className="ppop" data-testid={testId} style={{ width, maxWidth:'calc(100vw - 16px)',left: at?.left ?? -9999, top: at?.top ?? -9999 }}>
      {children}
    </div>,
    editorRoot(),
  );
}

const HEX6 = /^#[0-9a-f]{6}$/i;
const HEX8 = /^#[0-9a-f]{8}$/i;

/**
 * A colour value split into what the field shows: its base (a theme token or #RRGGBB) and the
 * alpha an #RRGGBBAA carries (1 otherwise).
 */
export function splitColor(value: string | undefined): { base: string | undefined; alpha: number } {
  if (value && HEX8.test(value)) return { base: value.slice(0, 7).toUpperCase(), alpha: Math.round((parseInt(value.slice(7), 16) / 255) * 100) / 100 };
  return { base: value, alpha: 1 };
}
/** A base colour at an alpha, as saved: the base alone at 1, else #RRGGBBAA (a token resolved to its hex first). */
export function joinColor(project: ProjectModel, base: string, alpha: number): string {
  if (alpha >= 1) return base;
  const hex = resolveColor(project, base, "#FFFFFF");
  const six = HEX6.test(hex) ? hex : "#FFFFFF";
  return `${six}${Math.round(Math.max(0, alpha) * 255).toString(16).padStart(2, "0")}`.toUpperCase();
}

/**
 * A colour: a swatch (opens the theme colours, a few plain ones and the system picker), the hex (or
 * theme token) to type, and an opacity in %. The opacity is either the colour's own alpha
 * (#RRGGBBAA, `alpha` absent) or a separate field (`alpha` given: a shadow's or a box's opacity).
 * onPick(color, final): final false while the system picker streams (the caller drafts it and
 * commits once it settles); a swatch, the hex and the opacity commit at once.
 */
function ColorField({ project, value, fallback = "#FFFFFF", onPick, onCancel, onFinish, testId, disabled, alpha, noAlpha, hexOnly }: { project: ProjectModel; value: string | undefined; fallback?: string; onPick: (color: string, final: boolean) => void; onCancel:()=>void; onFinish:()=>void; testId: string; disabled?: boolean; alpha?: { value: number; onCommit: (v: number,origin?:GestureOrigin) => void }; noAlpha?: boolean; hexOnly?: boolean }) {
  useEditorLanguage();
  const [open, setOpen] = useState(false);
  const anchor = useRef<HTMLButtonElement>(null);
  const own = splitColor(value);
  const base = own.base ?? fallback;
  const a = alpha ? alpha.value : own.alpha;
  const css = resolveColor(project, base, fallback);
  const hex = HEX6.test(css) ? css.toUpperCase() : fallback;
  const pickBase = (next: string, final: boolean) => {
    if (hexOnly) {
      const resolved = resolveColor(project, next, "");
      if (HEX6.test(resolved)) onPick(resolved.toUpperCase(), final);
      return;
    }
    onPick(alpha ? next : joinColor(project, next, own.alpha), final);
  };
  const [text, setText] = useState(base);
  const editing=useRef(false);
  const typed=useControlGesture({value:base,disabled,
    onCommit:(next:string)=>{editing.current=false;pickBase(next,true)},
    onCancel:()=>{editing.current=false;setText(base);onCancel()},
  });
  useEffect(() => {if(!editing.current)setText(base)}, [base]);
  const commitText = () => {
    if(!editing.current)return;
    const t = text.trim();
    const token = !hexOnly && (Object.prototype.hasOwnProperty.call(project.theme?.colors ?? {}, t) || ["fg", "bg", "mute", "accent"].includes(t));
    const norm = /^#?[0-9a-f]{6}$/i.test(t) ? `#${t.replace("#", "").toUpperCase()}` : /^#?[0-9a-f]{3}$/i.test(t) ? `#${[...t.replace("#", "")].map((c) => c + c).join("").toUpperCase()}` : token ? t : undefined;
    if(!norm)return typed.cancel();
    typed.change(norm,false);typed.commit();
  };
  return (
    <div className="cfield">
      <Tooltip label={base}>
        <button ref={anchor} className="cfield-swatch" data-testid={`${testId}-swatch`} disabled={disabled} style={{ ["--c" as string]: css, ["--a" as string]: String(a) }} onClick={() => {if(open)onFinish();setOpen(!open)}} />
      </Tooltip>
      <input
        className="cfield-hex"
        data-testid={`${testId}-hex`}
        value={text}
        disabled={disabled}
        spellCheck={false}
        onFocus={(e) => {editing.current=typed.begin();e.currentTarget.select()}}
        onChange={(e) => {if(typed.begin()){editing.current=true;setText(e.currentTarget.value)}}}
        onBlur={commitText}
        onKeyDown={(e) => {
          if(isTextComposition(e.nativeEvent))return;
          if (e.key === "Enter") {e.preventDefault();e.stopPropagation();e.currentTarget.blur()}
          if (e.key === "Escape") {
            e.preventDefault();e.stopPropagation();typed.cancel();e.currentTarget.blur();
          }
        }}
      />
      {!noAlpha && (
        <NumInput
          testId={`${testId}-opacity`}
          value={Math.round(a * 100)}
          unit="%"
          min={0}
          max={100}
          disabled={disabled}
          onCommit={(v,origin) => {
            if (v === undefined) return;
            if (alpha) alpha.onCommit(v / 100,origin);
            else onPick(joinColor(project, base, v / 100), true);
          }}
        />
      )}
      {open && (
        <Popover anchor={anchor.current} onClose={reason=>{if(reason==='cancel'){typed.cancel();onCancel()}else{commitText();onFinish()}setOpen(false)}} testId={`${testId}-popover`} width={224}>
          <div className="ppop-swatches">
            {swatches(project).filter(s => !hexOnly || HEX6.test(s.css)).map((s) => (
              <Tooltip key={s.value} label={editorText(s.label)}>
                <button className={`swatch ${(hexOnly ? base.toUpperCase() === s.css.toUpperCase() : base === s.value) ? "active" : ""}`} data-testid={testId} data-color={s.value} disabled={disabled} style={{ background: s.css }} onClick={() => pickBase(s.value, true)} />
              </Tooltip>
            ))}
            <Tooltip label={editorText("Custom colour")}>
              <label className="swatch custom">
                <input type="color" data-testid={`${testId}-custom`} value={hex} disabled={disabled} onChange={(e) => pickBase(e.target.value.toUpperCase(), false)} />
              </label>
            </Tooltip>
          </div>
        </Popover>
      )}
    </div>
  );
}

/** A ColorField that commits a streaming system-picker colour once it settles (the caller just commits). */
export function SettledColorField(props: Omit<Parameters<typeof ColorField>[0], 'onPick'|'onFinish'|'onCancel'> & { onPreview?: (color: string) => void; onCommit: (color: string,origin?:GestureOrigin) => void; onCancel?:()=>void }) {
  useEditorLanguage();
  const {onPreview,onCommit,onCancel,...rest}=props;
  const timer=useRef<ReturnType<typeof setTimeout>|undefined>(undefined);
  const clear=()=>{clearTimeout(timer.current);timer.current=undefined};
  const gesture=useControlGesture({value:props.value??props.fallback??'#FFFFFF',disabled:props.disabled,onPreview,onCommit,onCancel:()=>{clear();onCancel?.()}});
  useEffect(()=>clear,[]);
  useEffect(()=>{if(props.disabled)clear()},[props.disabled]);
  return (
    <ColorField
      {...rest}
      value={gesture.local?.value??props.value}
      onCancel={()=>{clear();gesture.cancel()}}
      onFinish={()=>{clear();gesture.commit()}}
      onPick={(color, final) => {
        clear();gesture.change(color,!final);
        if(final)gesture.commit();else timer.current=setTimeout(()=>gesture.commit(),400);
      }}
    />
  );
}

/** An on/off switch. */
export function Toggle({ value, onChange, testId, disabled, label }: { value: boolean; onChange: (v: boolean) => void; testId: string; disabled?: boolean; label?: string }) {
  useEditorLanguage();
  return (
    <button className={`ptoggle ${value ? "on" : ""}`} role="switch" aria-checked={value} aria-label={typeof label === 'string' ? editorText(label) : label} data-testid={testId} disabled={disabled} onClick={() => onChange(!value)}>
      <span />
    </button>
  );
}
