import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
import { editorEvents } from "@/app/lib/editor/environment";
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { useState, useRef, useEffect } from "react";
import { MAX_DB, SILENCE_DB, gainToDb } from "@/app/lib/editor/core/audio/audio";
import type { StereoLevel } from "@/app/ui/editor/audio/audio";
import { yLevel, levelY, dbLabel, panWords, panShort } from "@/app/ui/editor/audio/sound";
import {beginEditorGesture,type GestureOrigin} from '../../workspace/pointerGesture';
import {isTextComposition} from '@/app/lib/editor/keyboard';

/**
 * A relative drag (no jump on press): the value moves by the pointer's travel, `toValue` maps a
 * start value and the travel in pixels to the new value. Commits on release only if it changed.
 */
function useDrag<T>({ value, toValue, onPreview, onCommit, onCancel, disabled }: { value: T; toValue: (start: T, dx: number, dy: number) => T; onPreview: (v: T) => void; onCommit: (v: T, origin?:GestureOrigin) => void; onCancel:()=>void; disabled: boolean }) {
  const [local, setLocal] = useState<T | undefined>(undefined);
  const stop=useRef<(()=>void)|undefined>(undefined);
  useEffect(()=>()=>stop.current?.(),[]);
  useEffect(()=>{if(disabled)stop.current?.()},[disabled]);
  const onPointerDown = (e: React.PointerEvent) => {
    if (disabled || e.button !== 0) return;
    stop.current?.();
    e.preventDefault();
    const x0 = e.clientX;
    const y0 = e.clientY;
    const start = value;
    let last: T = value;
    let moved = false;
    const move = (ev: PointerEvent) => {
      if (!moved && Math.abs(ev.clientX - x0) < 2 && Math.abs(ev.clientY - y0) < 2) return;
      moved = true;
      last = toValue(start, ev.clientX - x0, ev.clientY - y0);
      setLocal(last);
      onPreview(last);
    };
    stop.current=beginEditorGesture(e,{
      move,
      commit:(origin)=>{
        stop.current=undefined;setLocal(undefined);
        if(moved&&last!==start)onCommit(last,origin);else onCancel();
      },
      cancel:()=>{stop.current=undefined;setLocal(undefined);onCancel()},
    });
  };
  return { shown: local ?? value, dragging: local !== undefined, onPointerDown };
}

/** A vertical fader in dB on the shared level scale (levelY): drag the thumb, double-click for 0 dB. */
export function Fader({ value, onPreview, onCommit, onCancel, disabled, testId, label }: { value: number; onPreview: (db: number) => void; onCommit: (db: number,origin?:GestureOrigin) => void; onCancel:()=>void; disabled: boolean; testId: string; label: string }) {
  useEditorLanguage();
  const ref = useRef<HTMLDivElement>(null);
  const { shown, onPointerDown } = useDrag({
    value,
    disabled,
    toValue: (start, _dx, dy) => {
      const h = ref.current?.clientHeight || 100;
      const db = yLevel(levelY(start) + dy / h);
      return Math.round(Math.min(MAX_DB, Math.max(SILENCE_DB, db)) * 2) / 2;
    },
    onPreview,
    onCommit,
    onCancel,
  });
  return (
    <div
      className={`fader ${disabled ? "disabled" : ""}`}
      ref={ref}
      data-testid={testId}
      data-db={shown}
      role="slider"
      tabIndex={disabled?-1:0}
      aria-disabled={disabled}
      aria-orientation="vertical"
      aria-label={label}
      aria-valuemin={SILENCE_DB}
      aria-valuemax={MAX_DB}
      aria-valuenow={shown}
      title={editorText("{0}: {1} dB (drag; double-click for 0 dB)", { 0: label, 1: dbLabel(shown) })}
      onPointerDown={onPointerDown}
      onDoubleClick={() => !disabled && value !== 0 && onCommit(0)}
      onKeyDown={e=>sliderKey(e,value,SILENCE_DB,MAX_DB,.5,6,disabled,onCommit)}
    >
      <div className="fader-track" />
      <div className="fader-zero" style={{ top: `${levelY(0) * 100}%` }} />
      <div className="fader-thumb" data-testid={`${testId}-thumb`} style={{ top: `${levelY(shown) * 100}%` }} />
    </div>
  );
}

/** A horizontal pan slider, −1 (left) … 1 (right): drag, double-click to centre. */
export function PanSlider({ value, onPreview, onCommit, onCancel, disabled, name }: { value: number; onPreview: (p: number) => void; onCommit: (p: number,origin?:GestureOrigin) => void; onCancel:()=>void; disabled: boolean; name: string }) {
  useEditorLanguage();
  const ref = useRef<HTMLDivElement>(null);
  const { shown, onPointerDown } = useDrag({
    value,
    disabled,
    toValue: (start, dx) => {
      const w = ref.current?.clientWidth || 60;
      const p = Math.min(1, Math.max(-1, start + (2 * dx) / w));
      return Math.abs(p) < 0.025 ? 0 : Math.round(p * 100) / 100;
    },
    onPreview,
    onCommit,
    onCancel,
  });
  const x = ((shown + 1) / 2) * 100;
  return (
    <div className="pan-row">
      <div
        className={`pan ${disabled ? "disabled" : ""}`}
        ref={ref}
        data-testid="mixer-pan"
        data-pan={shown}
        role="slider"
        tabIndex={disabled?-1:0}
        aria-disabled={disabled}
        aria-orientation="horizontal"
        aria-label={editorText("{0} pan", { 0: name })}
        aria-valuemin={-1}
        aria-valuemax={1}
        aria-valuenow={shown}
        title={editorText("Pan {0} (drag; double-click to centre)", { 0: panWords(shown) })}
        onPointerDown={onPointerDown}
        onDoubleClick={() => !disabled && value !== 0 && onCommit(0)}
        onKeyDown={e=>sliderKey(e,value,-1,1,.01,.1,disabled,onCommit)}
      >
        <div className="pan-fill" style={{ left: `${Math.min(50, x)}%`, width: `${Math.abs(x - 50)}%` }} />
        <div className="pan-centre" />
        <div className="pan-thumb" style={{ left: `${x}%` }} />
      </div>
      <span className="pan-label mono" data-testid="mixer-pan-label">
        {panShort(shown)}
      </span>
    </div>
  );
}

function sliderKey(e:React.KeyboardEvent,value:number,min:number,max:number,step:number,page:number,disabled:boolean,commit:(value:number)=>void){
  if(isTextComposition(e.nativeEvent)||e.metaKey||e.ctrlKey||e.altKey)return;
  const delta=e.shiftKey?page:step;
  const next=e.key==='Home'?min:e.key==='End'?max:e.key==='PageUp'?value+page:e.key==='PageDown'?value-page:['ArrowUp','ArrowRight'].includes(e.key)?value+delta:['ArrowDown','ArrowLeft'].includes(e.key)?value-delta:undefined;
  if(next===undefined)return;
  e.preventDefault();e.stopPropagation();
  const clamped=Math.round(Math.min(max,Math.max(min,next))*100)/100;
  if(!disabled&&clamped!==value)commit(clamped);
}

/** A stereo meter: RMS as the bar, peak as a line, on the faders' scale. The panel's loop writes into it. */
export function Meter({ id }: { id: string }) {
  useEditorLanguage();
  return (
    <div className="meter" data-testid="mixer-meter" data-meter={id} data-peak="-Infinity" data-rms="-Infinity" data-over="false" title={editorText("Level after the fader and pan: RMS bar, peak line; the top lights red when the peak reaches full scale")}>
      {(["l", "r"] as const).map((c) => (
        <div key={c} className="meter-ch">
          <div className="meter-rms" data-ch={c} />
          <div className="meter-peak" data-ch={c} />
        </div>
      ))}
    </div>
  );
}

/** How long a meter's over (clip) light stays on after a peak at or above full scale. */
const OVER_HOLD_MS = 1500;
export interface MeterEl {
  readonly el: HTMLElement;
  readonly rms: ReadonlyArray<HTMLElement>;
  readonly peak: ReadonlyArray<HTMLElement>;
  /** What was last written, to skip unchanged writes. */
  last: string;
  overUntil: number;
}
/**
 * Fill a meter from a level reading (and say it in data-peak / data-rms, dBFS of the louder
 * channel, for the tests); a peak at full scale lights the meter's over light for a moment.
 * Returns whether the meter shows anything (false once it rests at the floor).
 */
export function drawMeter(m: MeterEl, level: StereoLevel, now: number): boolean {
  const chans = [level.left, level.right];
  if (chans.some((c) => c.peak >= 1)) m.overUntil = now + OVER_HOLD_MS;
  const over = now < m.overUntil;
  const ys = chans.map((c) => [levelY(gainToDb(c.rms)), levelY(gainToDb(c.peak))] as const);
  const key = `${ys.map(([r, p]) => `${r.toFixed(3)},${p.toFixed(3)}`).join(";")}${over ? "!" : ""}`;
  if (key === m.last) return key !== FLOOR_KEY;
  m.last = key;
  ys.forEach(([r, p], i) => {
    m.rms[i]!.style.clipPath = `inset(${r * 100}% 0 0 0)`;
    m.peak[i]!.style.bottom = `${(1 - p) * 100}%`;
  });
  m.el.classList.toggle("over", over);
  m.el.dataset.over = over ? "true" : "false";
  m.el.dataset.peak = String(Math.round(gainToDb(Math.max(level.left.peak, level.right.peak)) * 10) / 10);
  m.el.dataset.rms = String(Math.round(gainToDb(Math.max(level.left.rms, level.right.rms)) * 10) / 10);
  return key !== FLOOR_KEY;
}
const FLOOR_KEY = "1.000,1.000;1.000,1.000";


// ---- effects, ducking, the limiter and the loudness target (D84–D88) ----

/** Fill a gain-reduction light from a reading (dB ≤ 0): its bar grows down from the top over 0…−12 dB. Returns whether it shows anything. */
export function drawReduction(el: HTMLElement, gr: number): boolean {
  const v = Math.max(0, Math.min(12, -gr));
  const key = v.toFixed(1);
  if (el.dataset.last === key) return v > 0.05;
  el.dataset.last = key;
  el.dataset.value = String(Math.round(gr * 10) / 10);
  const bar = el.firstElementChild as HTMLElement | null;
  if (bar) bar.style.height = `${(v / 12) * 100}%`;
  return v > 0.05;
}

/** A small popover over the panel, opened from an anchor (fixed, so the panel's scroll doesn't clip it). Escape or a click outside closes it. */
export function Popover({ anchor, onClose, children, testId, wide = false }: { anchor: HTMLElement; onClose: () => void; children: React.ReactNode; testId: string; wide?: boolean }) {
  useEditorLanguage();
  const ref = useRef<HTMLDivElement>(null);
  const r = anchor.getBoundingClientRect();
  const left = Math.max(8, Math.min(window.innerWidth - (wide ? 318 : 268), r.left));
  useEffect(() => {
    const down = (e: PointerEvent) => {
      if (!ref.current?.contains(e.target as Node) && !anchor.contains(e.target as Node)) onClose();
    };
    const key = (e: KeyboardEvent) => {
      if (isTextComposition(e) || e.key !== "Escape") return;
      e.stopPropagation();
      onClose();
    };
    window.addEventListener("pointerdown", down, true);
    editorEvents.addEventListener("keydown", key, true);
    return () => {
      window.removeEventListener("pointerdown", down, true);
      editorEvents.removeEventListener("keydown", key, true);
    };
  }, [anchor, onClose]);
  return (
    <div className={`fx-popover ${wide ? "wide" : ""}`} data-testid={testId} ref={ref} style={{ left, bottom: Math.max(8, window.innerHeight - r.top + 6) }} onPointerDown={(e) => e.stopPropagation()}>
      {children}
    </div>
  );
}
