import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The effect popovers' live graphs (D246): the EQ's response (core's cookbook curves, the ones the
// preview's BiquadFilterNodes and the export's filters are tested against) with draggable bands,
// the compressor's transfer curve with its gain reduction, the duck curve over the timeline with
// the playhead, and the limiter's ceiling with the master's peak and its reduction. Everything is
// drawn from core's maths; only the live readings (gain reduction, peak, playhead) come from the mix.
import { useEffect, useRef, useState } from "react";
import { stagesResponseDb, compressorCurveDb, type Stage } from "@/app/lib/editor/core/audio/effects";
import type { DuckKey } from "@/app/lib/editor/core/audio/ducking";
import { gainToDb } from "@/app/lib/editor/core/audio/audio";

import { currentMix } from "@/app/ui/editor/audio/audio";
import { getPlayback } from "@/app/ui/editor/preview/playback";
import {beginEditorGesture,type GestureOrigin} from '../../workspace/pointerGesture';

const W = 276;
const H = 112;
const F_LO = 20;
const F_HI = 20000;
const fx = (f: number) => (Math.log10(f / F_LO) / Math.log10(F_HI / F_LO)) * W;
const xf = (x: number) => F_LO * (F_HI / F_LO) ** Math.min(1, Math.max(0, x / W));
/** Frequencies and gains in words: 80 Hz, 3.5 kHz; +4 dB, −2.5 dB. */
export const hzText = (f: number) => (f >= 1000 ? `${Math.round(f / 100) / 10} kHz` : `${Math.round(f)} Hz`);
export const dbText = (db: number, unit = "dB") => `${db > 0 ? "+" : db < 0 ? "−" : ""}${Math.abs(Math.round(db * 10) / 10)} ${unit}`;
/** A frequency rounded to what a person would type: 2 significant digits under 1 kHz' tens, 3 above. */
export const roundHz = (f: number) => {
  const step = f < 100 ? 1 : f < 1000 ? 5 : f < 10000 ? 50 : 100;
  return Math.round(f / step) * step;
};

/** Every animation frame while mounted: `read` returns what to draw (compared as a string, so idle frames cost nothing). */
export function useLive<T>(read: () => T): T {
  const reader=useRef(read);
  reader.current=read;
  const [value, setValue] = useState<T>(read);
  const last = useRef(JSON.stringify(value));
  useEffect(() => {
    let raf = 0;
    const loop = () => {
      const v = reader.current();
      const key = JSON.stringify(v);
      if (key !== last.current) {
        last.current = key;
        setValue(v);
      }
      raf = requestAnimationFrame(loop);
    };
    raf = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(raf);
  }, []);
  return value;
}

export interface EqHandle {
  readonly id: string;
  readonly label: string;
  readonly freq: number;
  readonly gainDb: number;
  /** Whether dragging up/down changes the gain (cuts move sideways only). */
  readonly vertical: boolean;
  readonly min: number;
  readonly max: number;
}

const EQ_RANGE = 18;
const gy = (db: number) => H / 2 - (Math.max(-EQ_RANGE, Math.min(EQ_RANGE, db)) / EQ_RANGE) * (H / 2 - 6);

/** The EQ's response 20 Hz–20 kHz on a log axis, ±18 dB, with a handle per band (drag: frequency sideways, gain up and down). */
export function EqGraph({ stages, handles, bypassed, onDrag, onDrop, onCancel, disabled, testId = "fx-eq-graph" }: { stages: ReadonlyArray<Stage>; handles: ReadonlyArray<EqHandle>; bypassed: boolean; onDrag?: (id: string, freq: number, gainDb: number) => void; onDrop?: (origin:GestureOrigin) => void; onCancel?: () => void; disabled?: boolean; testId?: string }) {
  useEditorLanguage();
  const svg = useRef<SVGSVGElement>(null);
  // A drag's window listeners, removed when it ends, is cancelled (pointercancel), or the graph goes
  // away mid-drag (the popover closed: D262), so no listener or scrub preview is left behind.
  const dragging = useRef<(() => void) | undefined>(undefined);
  useEffect(()=>()=>dragging.current?.(),[]);
  useEffect(()=>{if(disabled)dragging.current?.()},[disabled]);
  const points: Array<string> = [];
  for (let x = 0; x <= W; x += 2) points.push(`${x},${gy(stagesResponseDb(stages, 48000, xf(x))).toFixed(1)}`);
  const at1k = stagesResponseDb(stages, 48000, 1000);
  const down = (h: EqHandle) => (e: React.PointerEvent) => {
    if (disabled || !onDrag || e.button !== 0) return;
    dragging.current?.();
    e.preventDefault();
    e.stopPropagation();
    const box = svg.current!.getBoundingClientRect();
    const sx = box.width / W;
    const sy = box.height / H;
    const move = (ev: PointerEvent) => {
      const x = (ev.clientX - box.left) / sx;
      const y = (ev.clientY - box.top) / sy;
      const freq = Math.min(h.max, Math.max(h.min, roundHz(xf(x))));
      const gain = h.vertical ? Math.round(Math.max(-24, Math.min(24, ((H / 2 - y) / (H / 2 - 6)) * EQ_RANGE)) * 2) / 2 : h.gainDb;
      onDrag(h.id, freq, gain);
    };
    dragging.current=beginEditorGesture(e,{
      move,
      commit:origin=>{dragging.current=undefined;onDrop?.(origin)},
      cancel:()=>{dragging.current=undefined;onCancel?.()},
    });
  };
  return (
    <svg ref={svg} className={`fxg ${bypassed ? "bypassed" : ""}`} viewBox={`0 0 ${W} ${H}`} data-testid={testId} data-at1k={at1k.toFixed(2)} role="img" aria-label={editorText("EQ response{0}", { 0: bypassed ? " (bypassed)" : "" })}>
      {[100, 1000, 10000].map((f) => (
        <g key={f}>
          <line className="fxg-grid" x1={fx(f)} x2={fx(f)} y1={0} y2={H} />
          <text className="fxg-tick" x={fx(f) + 3} y={H - 3}>
            {f >= 1000 ? editorText("{0}k", { 0: f / 1000 }) : f}
          </text>
        </g>
      ))}
      {[-12, 12].map((d) => (
        <g key={d}>
          <line className="fxg-grid" x1={0} x2={W} y1={gy(d)} y2={gy(d)} />
          <text className="fxg-tick" x={3} y={gy(d) - 2}>
            {d > 0 ? `+${d}` : `−${-d}`}
          </text>
        </g>
      ))}
      <line className="fxg-zero" x1={0} x2={W} y1={gy(0)} y2={gy(0)} />
      <polyline className="fxg-fill" points={`0,${gy(0)} ${points.join(" ")} ${W},${gy(0)}`} />
      <polyline className="fxg-curve" points={points.join(" ")} />
      {handles.map((h) => (
        <g key={h.id} className={`fxg-handle ${disabled ? "" : "live"}`} data-testid={`fx-eq-handle-${h.id}`} data-freq={h.freq} data-gain={h.gainDb} transform={`translate(${fx(h.freq).toFixed(1)},${gy(h.vertical ? h.gainDb : stagesResponseDb(stages, 48000, h.freq)).toFixed(1)})`} onPointerDown={down(h)}>
          <circle r={7} />
          <text y={3}>{editorText(h.label)}</text>
          <title>{editorText("{0}: {1}{2} (drag)", { 0: h.id, 1: hzText(h.freq), 2: h.vertical ? `, ${dbText(h.gainDb)}` : "" })}</title>
        </g>
      ))}
    </svg>
  );
}

const C_LO = -60;
const cx = (db: number) => ((Math.max(C_LO, Math.min(0, db)) - C_LO) / -C_LO) * W * 0.62;
const cy = (db: number) => H - 4 - ((Math.max(C_LO, Math.min(12, db)) - C_LO) / (12 - C_LO)) * (H - 8);

/** The compressor's static curve (input → output, with make-up), the threshold, and its live gain reduction as a bar. */
export function CompressorGraph({ stage, trackId, bypassed }: { stage: Extract<Stage, { kind: "compressor" }>; trackId: string; bypassed: boolean }) {
  useEditorLanguage();
  const gr = useLive(() => Math.round((bypassed ? 0 : (currentMix()?.levels().reduction[trackId] ?? 0)) * 10) / 10);
  const pts: Array<string> = [];
  for (let d = C_LO; d <= 0; d += 1) pts.push(`${cx(d).toFixed(1)},${cy(compressorCurveDb(stage, d)).toFixed(1)}`);
  const meterX = W * 0.62 + 46;
  const meterH = H - 22;
  const grH = (Math.min(24, -gr) / 24) * meterH;
  return (
    <svg className={`fxg ${bypassed ? "bypassed" : ""}`} viewBox={`0 0 ${W} ${H}`} data-testid="fx-comp-graph" role="img" aria-label={editorText("Compressor curve and gain reduction")}>
      {[-48, -36, -24, -12].map((d) => (
        <line key={d} className="fxg-grid" x1={cx(d)} x2={cx(d)} y1={0} y2={H} />
      ))}
      <line className="fxg-zero" x1={cx(C_LO)} y1={cy(C_LO)} x2={cx(0)} y2={cy(0)} strokeDasharray="2 3" />
      <line className="fxg-threshold" x1={cx(stage.thresholdDb)} x2={cx(stage.thresholdDb)} y1={0} y2={H} />
      <text className="fxg-tick" x={cx(stage.thresholdDb) + 3} y={10}>
        {dbText(stage.thresholdDb)}
      </text>
      <polyline className="fxg-curve" points={pts.join(" ")} />
      <text className="fxg-tick" x={cx(0)} y={H - 3} textAnchor="end">
        {editorText("in −60…0 dB")}</text>
      <g transform={`translate(${meterX},4)`}>
        <text className="fxg-tick" x={-2} y={8}>
          {editorText("GR")}</text>
        <rect className="fxg-meter" x={0} y={12} width={10} height={meterH} rx={1} />
        <rect className="fxg-gr" data-testid="fx-pop-gr" data-value={gr} x={0} y={12} width={10} height={grH} rx={1} />
        <text className="fxg-value" x={16} y={22}>
          {gr < -0.05 ? dbText(gr) : editorText("0 dB")}
        </text>
      </g>
    </svg>
  );
}

/** The duck curve over the whole timeline (0 dB at the top, the amount at the bottom) with the playhead. */
export function DuckGraph({ keys, frames, amountDb, bypassed }: { keys: ReadonlyArray<DuckKey>; frames: number; amountDb: number; bypassed: boolean }) {
  useEditorLanguage();
  const frame = useLive(() => Math.round(getPlayback().frame));
  const span = Math.max(1, frames);
  const depth = Math.min(-1, amountDb);
  const y = (db: number) => 8 + (db / depth) * (H - 24);
  const x = (f: number) => (f / span) * W;
  const pts = [`0,${y(0)}`, ...keys.map((k) => `${x(k.frame).toFixed(1)},${y(k.db).toFixed(1)}`), `${W},${y(0)}`];
  return (
    <svg className={`fxg duck ${bypassed ? "bypassed" : ""}`} viewBox={`0 0 ${W} ${H}`} data-testid="duck-graph" data-keys={keys.length} role="img" aria-label={editorText("Duck curve over the timeline")}>
      <line className="fxg-grid" x1={0} x2={W} y1={y(depth)} y2={y(depth)} />
      <text className="fxg-tick" x={3} y={y(depth) - 3}>
        {dbText(depth)}
      </text>
      <line className="fxg-zero" x1={0} x2={W} y1={y(0)} y2={y(0)} />
      <polyline className="fxg-curve duck" points={pts.join(" ")} />
      <line className="fxg-playhead" data-testid="duck-graph-playhead" x1={x(frame)} x2={x(frame)} y1={0} y2={H - 12} />
      <text className="fxg-tick" x={3} y={H - 2}>
        {keys.length ? editorText("the timeline, start to end") : editorText("no speech under the key yet")}
      </text>
    </svg>
  );
}

/** The master's peak against the limiter's ceiling, and the limiter's gain reduction. */
export function LimiterGraph({ ceilingDb, on }: { ceilingDb: number; on: boolean }) {
  useEditorLanguage();
  const live = useLive(() => {
    const l = currentMix()?.levels();
    const peak = l ? gainToDb(Math.max(l.master.left.peak, l.master.right.peak)) : -Infinity;
    return { peak: Number.isFinite(peak) ? Math.round(peak * 10) / 10 : -60, gr: on ? Math.round((l?.limiter ?? 0) * 10) / 10 : 0 };
  });
  const LO = -36;
  const X = (db: number) => ((Math.max(LO, Math.min(0, db)) - LO) / -LO) * (W - 8) + 4;
  return (
    <svg className={`fxg limiter ${on ? "" : "bypassed"}`} viewBox={`0 0 ${W} 46`} data-testid="limiter-graph" role="img" aria-label={editorText("Master peak, ceiling and limiting")}>
      <rect className="fxg-meter" x={4} y={6} width={W - 8} height={10} rx={2} />
      <rect className="fxg-peak" data-testid="limiter-pop-peak" data-value={live.peak} x={4} y={6} width={Math.max(0, X(live.peak) - 4)} height={10} rx={2} />
      <line className="fxg-threshold" x1={X(ceilingDb)} x2={X(ceilingDb)} y1={2} y2={20} />
      <text className="fxg-tick" x={X(ceilingDb) > W / 2 ? X(ceilingDb) - 3 : X(ceilingDb) + 3} y={30} textAnchor={X(ceilingDb) > W / 2 ? "end" : "start"}>
        {editorText("ceiling ")}{dbText(ceilingDb, "dBTP")}
      </text>
      <rect className="fxg-meter" x={4} y={34} width={W * 0.4} height={6} rx={2} />
      <rect className="fxg-gr" data-testid="limiter-pop-gr" data-value={live.gr} x={4} y={34} width={(Math.min(12, -live.gr) / 12) * W * 0.4} height={6} rx={2} />
      <text className="fxg-value" x={W * 0.4 + 10} y={41}>
        {on ? editorText("limiting {0}", { 0: live.gr < -0.05 ? dbText(live.gr) : "0 dB" }) : editorText("limiter off")}
      </text>
    </svg>
  );
}
