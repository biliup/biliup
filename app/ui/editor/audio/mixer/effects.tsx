import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The Mixer's effect slots and their popovers (D84–D88; redesigned in D246). Every popover has the
// same header (the effect's name and one On switch: bypass for track effects, on/off for ducking
// and the limiter), fields grouped by what they shape with their units after the value, and a live
// graph: the EQ's response with a draggable handle per band, the compressor's curve with its gain
// reduction, the voice chain's curve, the noise floor noise removal works from, the duck curve
// over the timeline with the playhead, the limiter's ceiling with the master's peak and its
// reduction. Every change is one edit (a handle drag or a field scrub is previewed while it moves).
import { useState, useRef } from "react";
import { duckSettings, duckNote, DUCK_DEFAULTS } from "@/app/lib/editor/core/audio/ducking";
import { EQ, COMPRESSOR, VOICE, DENOISE, eqParams, compressorParams, amountOf, DUCK, effectName, effectSummary, effectStages, voiceStages, masterLimiter, projectTargetLufs, LOUDNESS_PRESETS, LIMITER_CEILING_DB, trackNoiseFloor, type Stage } from "@/app/lib/editor/core/audio/effects";
import { denoiseReductionDb } from "@/app/lib/editor/core/audio/denoise";
import type { ModuleUse } from "@/app/lib/editor/core/timeline/types";
import { planAddEffect, planVoicePreset, planMoveEffect, planBypassEffect, planRemoveEffect, planEffectParams, soundTrack, dbLabel, planDuck, planMasterLimiter, planLoudnessTarget } from "@/app/ui/editor/audio/sound";
import type { ProjectModel, Track } from "@/app/lib/editor/state/project";
import { useDuckCurves, useDuckState } from "@/app/ui/editor/audio/ducking";
import { NumberField } from "@/app/ui/editor/inspector/fields";
import { getPlayback } from "@/app/ui/editor/preview/playback";
import { Popover } from "@/app/ui/editor/audio/mixer/controls";
import {useProjectEdit} from '../../workspace/projectEdit';
import type {GestureOrigin} from '../../workspace/pointerGesture';
import { EqGraph, CompressorGraph, DuckGraph, LimiterGraph, type EqHandle, dbText } from "@/app/ui/editor/audio/mixer/graphs";
import { useLoudnessPreview, setPreviewAtTarget } from "@/app/ui/editor/audio/loudnessPreview";
import {preparingStatus} from '@/app/lib/editor/export/jobs';

/** One On switch for every effect (track effects, ducking, the limiter). */
function OnSwitch({ on, onChange, disabled, testId, label }: { on: boolean; onChange: (on: boolean) => void; disabled: boolean; testId: string; label: string }) {
  useEditorLanguage();
  return (
    <button type="button" role="switch" className={`fxp-switch ${on ? "on" : ""}`} data-testid={testId} aria-checked={on} aria-label={label} title={on ? editorText("{0}: on (click to bypass)", { 0: label }) : editorText("{0}: bypassed (click to turn on)", { 0: label })} disabled={disabled} onClick={() => onChange(!on)}>
      <span className="fxp-switch-knob" />
      <span className="fxp-switch-text">{on ? editorText("On") : editorText("Off")}</span>
    </button>
  );
}

/** A number field with its unit after the value. */
function Field({ unit, ...props }: React.ComponentProps<typeof NumberField> & { unit: string }) {
  useEditorLanguage();
  return (
    <div className="fxp-field" data-unit={unit}>
      <NumberField {...props} />
      <span className="fxp-unit">{unit}</span>
    </div>
  );
}

function Head({ title, sub, children }: { title: string; sub?: string; children?: React.ReactNode }) {
  useEditorLanguage();
  return (
    <div className="fxp-head">
      <div className="fxp-title">
        <b>{title}</b>
        {sub && <span className="muted small">{sub}</span>}
      </div>
      {children}
    </div>
  );
}

type Patch = Record<string, number | null>;
/** EQ band handles: which params a handle drags. */
const EQ_HANDLES: ReadonlyArray<{ id: string; label: string; freq: string; gain?: string; min: number; max: number }> = [
  { id: "lowcut", get label() { return editorText("LC"); }, freq: "lowCutHz", min: 20, max: 1000 },
  { id: "low", get label() { return editorText("L"); }, freq: "lowHz", gain: "lowGainDb", min: 20, max: 1000 },
  { id: "mid", get label() { return editorText("M"); }, freq: "midHz", gain: "midGainDb", min: 100, max: 10000 },
  { id: "high", get label() { return editorText("H"); }, freq: "highHz", gain: "highGainDb", min: 1000, max: 16000 },
  { id: "highcut", get label() { return editorText("HC"); }, freq: "highCutHz", min: 1000, max: 20000 },
];

/** The settings of one track effect: header with its On switch, graph, grouped fields, order and remove. */
function EffectEditor({ project, track, index, fx, online, onMoved, onRemoved }: { project: ProjectModel; track: Track; index: number; fx: ModuleUse; online: boolean; onMoved: (to: number, saved:Promise<boolean>) => void; onRemoved: () => void }) {
  useEditorLanguage();
  const {commit,preview,cancel}=useProjectEdit();
  const [drag, setDrag] = useState<Patch | undefined>(undefined);
  const dragRef = useRef<Patch | undefined>(undefined);
  const paramsKey = JSON.stringify(fx.params ?? {});
  // The saved edit has come back: the drag's local values give way to it.
  const [dragParamsKey,setDragParamsKey]=useState(paramsKey);
  if(dragParamsKey!==paramsKey){setDragParamsKey(paramsKey);setDrag(undefined);}
  const shown: ModuleUse = drag ? { ...fx, params: { ...fx.params, ...Object.fromEntries(Object.entries(drag).map(([k, v]) => [k, v ?? undefined])) } } : fx;
  const set = (patch: Patch,origin?:GestureOrigin) => commit((now) => planEffectParams(now, track.id, index, patch, fx.module),origin);
  const bypassed = fx.bypass === true;
  const count = (track.mixer?.effects ?? []).length;
  const field = (key: string, label: string, unit: string, step: number, min: number, max: number, value: number | undefined, optional = false) => (
    <Field key={key} unit={unit} label={label} testId={`fx-param-${key}`} value={optional && shown.params?.[key] === undefined ? undefined : value} placeholder={optional ? editorText("off") : undefined} step={step} min={min} max={max} disabled={!online} onCommit={(v,origin) => set({ [key]: v ?? null },origin)} />
  );
  let body: React.ReactNode = null;
  if (fx.module === EQ) {
    const p = eqParams(shown.params);
    const stages = effectStages({ ...shown, bypass: false });
    const handles: Array<EqHandle> = EQ_HANDLES.filter((h) => h.gain || shown.params?.[h.freq] !== undefined).map((h) => ({ id: h.id, label: h.label, freq: p[h.freq as keyof typeof p] as number, gainDb: h.gain ? (p[h.gain as keyof typeof p] as number) : 0, vertical: h.gain !== undefined, min: h.min, max: h.max }));
    body = (
      <>
        <EqGraph
          stages={stages}
          handles={handles}
          bypassed={bypassed}
          disabled={!online}
          onDrag={(id, freq, gainDb) => {
            const h = EQ_HANDLES.find((x) => x.id === id)!;
            const patch: Patch = { ...dragRef.current, [h.freq]: freq, ...(h.gain ? { [h.gain]: gainDb } : {}) };
            dragRef.current = patch;
            setDrag(patch);
            preview(project,planEffectParams(project,track.id,index,patch,fx.module));
          }}
          onDrop={(origin) => {
            const d = dragRef.current;
            dragRef.current = undefined;
            if (!d) return;
            void set(d,origin).finally(()=>setDrag(current=>current===d?undefined:current));
          }}
          onCancel={() => {
            // Cancelled or closed mid-drag (D262): no edit, and the scrub preview goes.
            dragRef.current = undefined;
            setDrag(undefined);
            cancel();
          }}
        />
        <div className="fxp-bands">
          <div className="fxp-band">
            <span className="fxp-band-name">{editorText("Low cut")}</span>
            {field("lowCutHz", "Freq", "Hz", 10, 20, 1000, p.lowCutHz, true)}
          </div>
          <div className="fxp-band">
            <span className="fxp-band-name">{editorText("Low shelf")}</span>
            {field("lowGainDb", "Gain", "dB", 0.5, -24, 24, p.lowGainDb)}
            {field("lowHz", "Freq", "Hz", 10, 20, 1000, p.lowHz)}
          </div>
          <div className="fxp-band">
            <span className="fxp-band-name">{editorText("Mid peak")}</span>
            {field("midGainDb", "Gain", "dB", 0.5, -24, 24, p.midGainDb)}
            {field("midHz", "Freq", "Hz", 50, 100, 10000, p.midHz)}
            {field("midQ", "Q", "", 0.1, 0.1, 10, p.midQ)}
          </div>
          <div className="fxp-band">
            <span className="fxp-band-name">{editorText("High shelf")}</span>
            {field("highGainDb", "Gain", "dB", 0.5, -24, 24, p.highGainDb)}
            {field("highHz", "Freq", "Hz", 100, 1000, 16000, p.highHz)}
          </div>
          <div className="fxp-band">
            <span className="fxp-band-name">{editorText("High cut")}</span>
            {field("highCutHz", "Freq", "Hz", 100, 1000, 20000, p.highCutHz, true)}
          </div>
        </div>
        <div className="muted small">{editorText("Drag a handle: sideways for the frequency, up and down for the gain.")}</div>
      </>
    );
  } else if (fx.module === COMPRESSOR) {
    const c = compressorParams(shown.params);
    body = (
      <>
        <CompressorGraph stage={{ kind: "compressor", ...c }} trackId={track.id} bypassed={bypassed} />
        <div className="fxp-grid">
          {field("thresholdDb", "Threshold", "dB", 1, -60, 0, c.thresholdDb)}
          {field("ratio", "Ratio", ": 1", 0.5, 1, 20, c.ratio)}
          {field("attackMs", "Attack", "ms", 1, 1, 500, c.attackMs)}
          {field("releaseMs", "Release", "ms", 10, 10, 1000, c.releaseMs)}
          {field("makeupDb", "Make-up", "dB", 0.5, 0, 24, c.makeupDb)}
        </div>
      </>
    );
  } else if (fx.module === VOICE || fx.module === DENOISE) {
    const amount = amountOf(shown.params);
    const pct = <Field unit="%" label={editorText("Amount")} testId="fx-param-amount" value={Math.round(amount * 100)} step={5} min={0} max={100} disabled={!online} onCommit={(v,origin) => set({ amount: v === undefined ? null : Math.round(v) / 100 },origin)} />;
    if (fx.module === VOICE) {
      const stages = voiceStages(amount);
      const comp = stages.find((s): s is Extract<Stage, { kind: "compressor" }> => s.kind === "compressor")!;
      const peak = stages.find((s) => s.kind === "biquad" && s.type === "peaking") as Extract<Stage, { kind: "biquad" }>;
      body = (
        <>
          <EqGraph stages={stages} handles={[]} bypassed={bypassed} testId="fx-voice-graph" />
          {pct}
          <div className="muted small" data-testid="fx-voice-summary">
            {editorText("Low cut 80 Hz, presence ")}{dbText(peak.gainDb)} {editorText(" at 3.5 kHz, compression ")}{comp.ratio}{editorText(":1 from ")}{dbText(comp.thresholdDb)} {editorText(" with ")}{dbText(comp.makeupDb)} {editorText(" make-up.")}</div>
        </>
      );
    } else {
      body = <DenoiseInfo project={project} track={track} amount={amount} bypassed={bypassed} amountField={pct} />;
    }
  }
  return (
    <>
      <Head title={editorText(effectName(fx.module))}>
        <OnSwitch on={!bypassed} label={editorText(effectName(fx.module))} testId="fx-bypass" disabled={!online} onChange={(on) => commit((now) => planBypassEffect(now, track.id, index, !on, fx.module))} />
      </Head>
      {body}
      <div className="fxp-foot">
        <button type="button" className="btn ghost" data-testid="fx-up" disabled={!online || index === 0} title={editorText("Earlier in the chain")} onClick={() => onMoved(index-1,commit((now) => planMoveEffect(now, track.id, index, -1, fx.module)))}>
          {editorText("↑ Earlier")}</button>
        <button type="button" className="btn ghost" data-testid="fx-down" disabled={!online || index >= count - 1} title={editorText("Later in the chain")} onClick={() => onMoved(index+1,commit((now) => planMoveEffect(now, track.id, index, 1, fx.module)))}>
          {editorText("↓ Later")}</button>
        <button type="button" className="btn ghost danger" data-testid="fx-remove" disabled={!online} onClick={() => void commit((now) => planRemoveEffect(now, track.id, index, fx.module)).then(saved=>{if(saved)onRemoved()})}>
          {editorText("Remove")}</button>
      </div>
    </>
  );
}

/** Noise removal: the floor it works from (measured or assumed) and how far it goes down. */
function DenoiseInfo({ project, track, amount, bypassed, amountField }: { project: ProjectModel; track: Track; amount: number; bypassed: boolean; amountField: React.ReactNode }) {
  useEditorLanguage();
  const envelopes = useDuckState((s) => s.envelopes);
  const { db: floor, measured } = trackNoiseFloor(project, track, envelopes);
  const reduction = denoiseReductionDb(amount);
  const LO = -80;
  const X = (db: number) => ((Math.max(LO, Math.min(0, db)) - LO) / -LO) * 268 + 4;
  return (
    <>
      <svg className={`fxg denoise ${bypassed ? "bypassed" : ""}`} viewBox="0 0 276 46" data-testid="fx-denoise-graph" data-floor={floor} data-measured={measured} role="img" aria-label={editorText("Noise floor and reduction")}>
        <title>{editorText(measured ? 'Noise floor estimated from the audio played on this track.' : 'No played audio measured yet. Using −40 dBFS.')}</title>
        <rect className="fxg-meter" x={4} y={8} width={268} height={10} rx={2} />
        <rect className="fxg-noise" x={4} y={8} width={X(floor) - 4} height={10} rx={2} />
        <rect className="fxg-gr" x={Math.max(4, X(floor - reduction))} y={8} width={X(floor) - Math.max(4, X(floor - reduction))} height={10} />
        <line className="fxg-threshold" x1={X(floor)} x2={X(floor)} y1={4} y2={22} />
        <text className="fxg-tick" x={Math.min(176, X(floor) + 3)} y={34}>
          {editorText("floor ")}{dbText(floor, "dBFS")}
        </text>
        <text className="fxg-tick" x={4} y={44}>
          {editorText("sound under the floor goes down by up to ")}{Math.round(reduction)} {editorText(" dB")}</text>
      </svg>
      {amountField}
    </>
  );
}

/** A track's effect slots (ducking has its own row) and the add menu. */
export function EffectSlots({ project, track, online }: { project: ProjectModel; track: Track; online: boolean }) {
  useEditorLanguage();
  const {commit}=useProjectEdit();
  const [open, setOpen] = useState<{ index: number; module: string; anchor: HTMLElement } | undefined>(undefined);
  const [menu, setMenu] = useState<HTMLElement | undefined>(undefined);
  const effects = track.mixer?.effects ?? [];
  const hasCompressor = effects.some((fx) => !fx.bypass && (fx.module === COMPRESSOR || fx.module === VOICE));
  // The popover belongs to that entry: if the chain changes under it (an agent turn, an undo), it closes.
  const shownFx = open && effects[open.index]?.module === open.module ? effects[open.index] : undefined;
  // A move (↑ / ↓) follows the entry to its new place; the popover waits for that edit to land.
  const [moving, setMoving] = useState(false);
  if(open&&!shownFx&&!moving)setOpen(undefined);
  return (
    <div className="fx-list" data-testid="fx-list">
      <div className="fx-slots">
        {effects.map((fx, index) =>
          fx.module === DUCK ? null : (
            <button
              key={index}
              type="button"
              className={`fx-slot ${fx.bypass ? "bypassed" : ""} ${open?.index === index ? "open" : ""}`}
              data-testid="fx-slot"
              data-module={fx.module}
              data-index={index}
              data-bypass={fx.bypass === true}
              title={`${editorText(effectName(fx.module))}: ${effectSummary(fx)}${fx.bypass ? ` (${editorText("off")})` : ""}`}
              onClick={(e) => setOpen(open?.index === index ? undefined : { index, module: fx.module, anchor: e.currentTarget })}
            >
              <span className="fx-dot" />
              <span className="ellipsis">{editorText(effectName(fx.module))}</span>
              {fx.bypass && <span className="fx-off">{editorText("off")}</span>}
            </button>
          ),
        )}
      </div>
      <div className="fx-foot">
        <button type="button" className="fx-add" data-testid="fx-add" disabled={!online} title={editorText("Add an effect")} onClick={(e) => setMenu(menu ? undefined : e.currentTarget)}>
          {editorText("+ FX")}</button>
        {hasCompressor && (
          <div className="gr" data-testid="fx-gr" data-gr={track.id} data-value="0" title={editorText("Compressor gain reduction")}>
            <div className="gr-bar" />
          </div>
        )}
      </div>
      {menu && (
        <Popover anchor={menu} onClose={() => setMenu(undefined)} testId="fx-menu">
          <div className="fx-pop-title">{editorText("Add to ")}{track.name}</div>
          {(
            [
              ["fx-add-audio.eq", "EQ", "Low cut, low / mid / high", (now: ProjectModel) => planAddEffect(now, track.id, EQ)],
              ["fx-add-audio.compressor", "Compressor", "Evens out the level", (now: ProjectModel) => planAddEffect(now, track.id, COMPRESSOR)],
              ["fx-add-voice-preset", "Voice preset", "EQ (low cut, presence) and a gentle compressor", (now: ProjectModel) => planVoicePreset(now, track.id)],
              ["fx-add-audio.voice-enhance", "Voice", "The voice chain in one slot", (now: ProjectModel) => planAddEffect(now, track.id, VOICE)],
              ["fx-add-audio.denoise", "Noise removal", "Takes out steady hiss and hum", (now: ProjectModel) => planAddEffect(now, track.id, DENOISE)],
            ] as const
          ).map(([id, name, hint, make]) => (
            <button
              key={id}
              type="button"
              className="fx-menu-item"
              data-testid={id}
              disabled={!online}
              onClick={() => {
                setMenu(undefined);
                commit(make);
              }}
            >
              <b>{editorText(name)}</b>
            </button>
          ))}
        </Popover>
      )}
      {open && shownFx && (
        <Popover anchor={open.anchor} onClose={() => setOpen(undefined)} testId="fx-popover" wide>
          <EffectEditor
            project={project}
            track={track}
            index={open.index}
            fx={shownFx}
            online={online&&!moving}
            onMoved={(to,saved) => {
              setMoving(true);
              void saved.then(ok=>{
                setMoving(false);
                if(ok)setOpen(current=>current===open?{...current,index:to}:current);
              },()=>setMoving(false));
            }}
            onRemoved={() => setOpen(current=>current===open?undefined:current)}
          />
        </Popover>
      )}
    </div>
  );
}

/** The Duck row (D85): under which track, and the settings, in a popover; what it waits for when the key isn't measured. */
export function DuckRow({ project, track, online }: { project: ProjectModel; track: Track; online: boolean }) {
  useEditorLanguage();
  const {commit}=useProjectEdit();
  const [anchor, setAnchor] = useState<HTMLElement | undefined>(undefined);
  const curves = useDuckCurves(project);
  useDuckState((s) => s.measuring);
  const fxIndex = (track.mixer?.effects ?? []).findIndex((e) => e.module === DUCK);
  const fx = fxIndex >= 0 ? track.mixer!.effects![fxIndex] : undefined;
  const settings = fx ? duckSettings(fx.params) : undefined;
  const bypassed = fx?.bypass === true;
  const keys = project.tracks.filter((t) => t.id !== track.id && soundTrack(project, t));
  const note = duckNote(project, track, curves.get(track.id));
  const name = (id: string) => project.tracks.find((t) => t.id === id)?.name ?? id;
  const label = settings ? `${editorText("Ducking")}${bypassed ? ` (${editorText("off")})` : ""}: ${settings.under.map(name).join(", ")} ${dbLabel(settings.amountDb)} dB` : editorText("Ducking");
  const curve = curves.get(track.id);
  return (
    <div className="duck-row">
      <button type="button" className={`duck-btn ${settings && !bypassed ? "on" : ""}`} data-testid="duck-btn" data-under={settings?.under.join(",") ?? ""} data-bypass={bypassed} disabled={!online || (!keys.length && !settings)} title={note ?? (settings ? editorText("Ducks under {0} by {1} dB", { 0: settings.under.map(name).join(", "), 1: dbLabel(settings.amountDb) }) : editorText("Duck this track while another has speech"))} onClick={(e) => setAnchor(anchor ? undefined : e.currentTarget)}>
        <span className="ellipsis">{label}</span>
        {note && !bypassed && (
          <span className="duck-wait" data-testid="duck-note" title={note}>
            …
          </span>
        )}
      </button>
      {anchor && (
        <Popover anchor={anchor} onClose={() => setAnchor(undefined)} testId="duck-popover" wide>
          <Head title={editorText("Ducking")}>
            {settings && <OnSwitch on={!bypassed} label={editorText("Ducking")} testId="duck-on" disabled={!online} onChange={(on) => commit((now) => planBypassEffect(now, track.id, fxIndex, !on, DUCK))} />}
          </Head>
          <label className="field">
            <span className="k">{editorText("Under")}</span>
            <select
              className="v"
              data-testid="duck-under"
              value={settings?.under[0] ?? ""}
              disabled={!online}
              onChange={(e) => {
                const id = e.target.value;
                commit((now) => planDuck(now, track.id, id ? { under: [id] } : null));
              }}
            >
              <option value="">{editorText("Nothing (off)")}</option>
              {keys.map((t) => (
                <option key={t.id} value={t.id}>
                  {t.name}
                </option>
              ))}
            </select>
          </label>
          {settings && <DuckGraph keys={bypassed ? [] : (curve?.keys ?? [])} frames={getPlayback().frames} amountDb={settings.amountDb} bypassed={bypassed} />}
          {settings && (
            <div className="fxp-grid">
              <Field unit="dB" label={editorText("Amount")} testId="duck-amountDb" value={settings.amountDb} step={1} min={-40} max={0} disabled={!online} onCommit={(v,origin) => commit((now) => planDuck(now, track.id, { amountDb: v ?? DUCK_DEFAULTS.amountDb }),origin)} />
              <Field unit="dB" label={editorText("Threshold")} testId="duck-thresholdDb" value={settings.thresholdDb} placeholder={editorText("auto")} step={1} min={-80} max={0} disabled={!online} onCommit={(v,origin) => commit((now) => planDuck(now, track.id, { thresholdDb: v ?? null }),origin)} />
              <Field unit="ms" label={editorText("Attack")} testId="duck-attackMs" value={settings.attackMs} step={50} min={0} max={2000} disabled={!online} onCommit={(v,origin) => commit((now) => planDuck(now, track.id, { attackMs: v ?? DUCK_DEFAULTS.attackMs }),origin)} />
              <Field unit="ms" label={editorText("Release")} testId="duck-releaseMs" value={settings.releaseMs} step={50} min={0} max={5000} disabled={!online} onCommit={(v,origin) => commit((now) => planDuck(now, track.id, { releaseMs: v ?? DUCK_DEFAULTS.releaseMs }),origin)} />
            </div>
          )}
          {note && (
            <div className="small duck-note-text" data-testid="duck-popover-note">
              {note}
            </div>
          )}
        </Popover>
      )}
    </div>
  );
}

/** The master's limiter (on / ceiling, gain reduction) and loudness target (D86, D87), and how the preview plays it (D246). */
export function MasterControls({ project, online }: { project: ProjectModel; online: boolean }) {
  useEditorLanguage();
  const {commit}=useProjectEdit();
  const [anchor, setAnchor] = useState<HTMLElement | undefined>(undefined);
  const [customTarget,setCustomTarget]=useState(false);
  const limiter = masterLimiter(project);
  const target = projectTargetLufs(project);
  const preset = LOUDNESS_PRESETS.find((p) => p.lufs === target);
  const loud = useLoudnessPreview();
  const fresh = loud.fresh&&loud.targetLufs === target;
  const status = !loud.atTarget
    ? editorText("As mixed")
    : loud.problem
      ? loud.problem
      : !loud.online ? editorText('Disconnected')
      : loud.waitingForSave ? editorText('Waiting for saved changes')
      : loud.measuring ? (loud.step?preparingStatus({step:loud.step}).text:editorText('Waiting to measure loudness'))
      : !fresh ? editorText('Waiting to measure loudness')
      : loud.gainDb === undefined ? editorText('Silent mix')
      : loud.lufs===undefined ? editorText('Gain {0} dB',{0:dbLabel(loud.gainDb)})
      : editorText('{0} LUFS · gain {1} dB',{0:dbLabel(loud.lufs),1:dbLabel(loud.gainDb)});
  return (
    <div className="fx-list master-fx">
      <div className="fx-foot">
        <button type="button" className={`fx-slot ${limiter ? "" : "bypassed"}`} data-testid="limiter-btn" data-on={limiter !== undefined} data-ceiling={limiter?.ceilingDb ?? ""} title={limiter ? editorText("Limiter on: peaks stay under {0} dBTP", { 0: dbLabel(limiter.ceilingDb) }) : editorText("Limiter off: loud peaks can clip")} onClick={(e) => setAnchor(anchor ? undefined : e.currentTarget)}>
          <span className="fx-dot" />
          <span className="ellipsis">{editorText("Limiter")}</span>
        </button>
        {limiter && (
          <div className="gr" data-testid="limiter-gr" data-gr="master" data-value="0" title={editorText("Limiter gain reduction")}>
            <div className="gr-bar" />
          </div>
        )}
      </div>
      <button type="button" className={`fx-slot target ${loud.atTarget ? "heard" : ""}`} data-testid="target-btn" data-lufs={target} title={loud.atTarget ? editorText("Loudness target, heard in the preview") : editorText("Loudness target: the export dialog starts from it")} onClick={(e) => setAnchor(anchor ? undefined : e.currentTarget)}>
        <span className="ellipsis">{preset ? editorText(preset.name) : editorText("{0} LUFS", { 0: dbLabel(target) })}</span>
      </button>
      {anchor && (
        <Popover anchor={anchor} onClose={() => setAnchor(undefined)} testId="master-popover" wide>
          <Head title={editorText("Limiter")}>
            <OnSwitch on={limiter !== undefined} label={editorText("Limiter")} testId="limiter-toggle" disabled={!online} onChange={(on) => commit((now) => planMasterLimiter(now, on, masterLimiter(now)?.ceilingDb ?? LIMITER_CEILING_DB))} />
          </Head>
          <LimiterGraph ceilingDb={limiter?.ceilingDb ?? LIMITER_CEILING_DB} on={limiter !== undefined} />
          {limiter && <Field unit="dBTP" label={editorText("Ceiling")} testId="limiter-ceiling" value={limiter.ceilingDb} step={0.5} min={-12} max={0} disabled={!online} onCommit={(v,origin) => commit((now) => planMasterLimiter(now, true, v ?? LIMITER_CEILING_DB),origin)} />}
          <div className="fxp-sep" />
          <Head title={editorText("Loudness target")} />
          <div className="fxp-grid">
            <label className="field">
              <select className="v" data-testid="target-select" value={!customTarget&&preset ? String(target) : "custom"} disabled={!online} onChange={(e) => {
                const value=e.target.value;
                setCustomTarget(value==='custom');
                if(value!=='custom')void commit(now=>planLoudnessTarget(now,Number(value)));
              }}>
                {LOUDNESS_PRESETS.map((p) => (
                  <option key={p.id} value={String(p.lufs)}>
                    {editorText(p.name)} ({dbLabel(p.lufs)} LUFS)</option>
                ))}
                <option value="custom">{editorText("Custom…")}</option>
              </select>
            </label>
            <Field unit="LUFS" label={editorText("Target")} testId="target-lufs" value={target} step={1} min={-30} max={-5} disabled={!online} onCommit={(v,origin) => commit((now) => planLoudnessTarget(now, v ?? -14),origin)} />
          </div>
          <div className="fxp-seg" role="radiogroup" aria-label={editorText("The preview plays")}>
            <span className="muted small">{editorText("Preview")}</span>
            <button type="button" role="radio" aria-checked={!loud.atTarget} className={!loud.atTarget ? "on" : ""} data-testid="preview-loudness-mixed" onClick={() => setPreviewAtTarget(false)}>
              {editorText("As mixed")}</button>
            <button type="button" role="radio" aria-checked={loud.atTarget} className={loud.atTarget ? "on" : ""} data-testid="preview-loudness-target" disabled={!loud.online} onClick={() => setPreviewAtTarget(true)}>
              {editorText("At target")}</button>
          </div>
          <div className="muted small" data-testid="preview-loudness-status" data-measuring={loud.atTarget && loud.online && !loud.problem && !loud.waitingForSave && (loud.measuring || !fresh)} data-gain={loud.atTarget && fresh && loud.gainDb !== undefined ? loud.gainDb : ""} data-lufs={loud.lufs ?? ""}>
            {status}
            {loud.atTarget&&loud.problem&&<button type="button" className="btn small" disabled={!loud.online||loud.waitingForSave} onClick={loud.retry}>{editorText('Retry')}</button>}
          </div>
        </Popover>
      )}
    </div>
  );
}

