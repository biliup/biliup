// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import type { MultiLayout } from "@/app/lib/editor/contracts";
import { muteOutput } from "@/app/ui/editor/audio/audio";
import { playback, getPlayback } from "@/app/ui/editor/preview/playback";
import { actions, getState, subscribeApp,projectOf } from "@/app/lib/editor/state/store";
import { type TakeMode, type Resolution, type PartKind, type RecorderState, set, stopStream, state, errorText, MIN_TAKE_MS } from "@/app/ui/editor/capture/recorder/state";
import { openParts, openStreams } from "@/app/ui/editor/capture/recorder/streams";
import { closeMonitor, closePreview, watchMeter } from "@/app/ui/editor/capture/recorder/devices";
import {editorText} from '@/app/lib/editor/i18n';
import {saveCaptureDraft,retainCaptureDraft,releaseCaptureDraft,failCaptureDraft} from '../recovery'
import {currentEditorStore} from '@/app/lib/editor/state/store/core'
import type {EditorStore} from '@/app/lib/editor/project-store'

// ---- a take ----

export interface TakeOptions {
  readonly mode: TakeMode;
  readonly mic?: string | undefined;
  /** Camera and screen: record the microphone too. */
  readonly withMic?: boolean;
  readonly camera?: string | undefined;
  readonly resolution?: Resolution;
  /** Voiceover: play the timeline from the playhead while recording, and whether muted. */
  readonly playTimeline?: boolean;
  readonly muteTimeline?: boolean;
  /** Camera and screen: put the take on the timeline at the playhead too. */
  readonly place?: boolean;
  /** Stop by itself after this many seconds of recording (D144's screen_record). */
  readonly seconds?: number;
  /** Seconds of 3-2-1 before recording starts; none (every take since D267, D273) means it starts as soon as the devices are open. */
  readonly countdown?: number;
  /** D144: the agent's screen_record this take answers (card id): uploaded and handed to the agent's turn, not saved as the user's edit. */
  readonly agent?: { readonly id: string };
  /** D211: the sources of a multi-source take (each its own file; with more than one, or mode "multi"). */
  readonly parts?: ReadonlyArray<PartKind>;
  /** D211: where the camera goes over the screen. */
  readonly layout?: MultiLayout;
}

/** D211: one source's recorder in a multi-source take. */
interface PartRec {
  readonly kind: PartKind;
  readonly recorder: MediaRecorder;
  readonly mime: string;
  readonly chunks: Array<Blob>;
  startMs: number | undefined;
  stopMs: number | undefined;
}

/** Whether a take records its sources into separate files (D211). */
export const isMulti = (options: Pick<TakeOptions, "mode" | "parts">) => options.mode === "multi" || (options.parts?.length ?? 0) > 1;

export interface Running {
  readonly owner:EditorStore;
  readonly options: TakeOptions;
  readonly dir: string;
  readonly frame: number;
  readonly fps:number;
  failure?:string;
  readonly streams: Array<MediaStream>;
  cameraStream?: MediaStream;
  readonly cleanups: Array<() => void>;
  recorder: MediaRecorder | undefined;
  chunks: Array<Blob>;
  cancelled: boolean;
  startMs: number;
  /** performance.now() of the current pause, and the milliseconds paused before it. */
  pauseMs: number | undefined;
  pausedTotal: number;
  playing: boolean;
  countdown: ReturnType<typeof setTimeout> | undefined;
  /** Resolves the countdown early (cancel). */
  wake: (() => void) | undefined;
  /** The timed stop (options.seconds). */
  autoStop: ReturnType<typeof setTimeout> | undefined;
  /** The agent got its answer already. */
  answered: boolean;
  /** The server was seen waiting for this take (AgentState.screenTake). */
  awaited: boolean;
  /** D211: a multi-source take's recorders. */
  parts: Array<PartRec> | undefined;
  finished: boolean;
  opening: boolean;
  device?:string;
}

/** The take's recorders (one, or one per source). */
const recordersOf = (r: Running): Array<MediaRecorder> => (r.parts ? r.parts.map((p) => p.recorder) : r.recorder ? [r.recorder] : []);
export let running: Running | undefined;
const ownsTake=(take:Running)=>currentEditorStore()===take.owner&&take.owner.getSnapshot().snapshot?.dir===take.dir;

/** Ends take `r` in the UI; nothing when another take has started since (its state isn't touched). */
function endTake(r: Running, patch: Partial<RecorderState>) {
  clearTimeout(r.autoStop);
  // An agent's take that didn't make it: the agent hears why.
  if (r.options.agent && !r.answered) {
    r.answered = true;
    void actions.agentScreenTake(r.options.agent.id, { problem: patch.message?.kind === "discarded" ? (r.cancelled ? "The user cancelled the recording." : "The take was too short.") : patch.message?.text ?? "The recording didn't work." },r.dir);
  }
  if (running !== r) return;
  running = undefined;
  set({ phase: "idle", mode: undefined, count: undefined, startedAt: undefined, agentId: undefined, paused: false, pausedMs: 0, pausedAt: undefined, parts: undefined, ...patch,...(!ownsTake(r)?{message:undefined,recordedMs:undefined,offsets:undefined}:{}) });
}

const pickMime = (video: boolean) =>
  (video ? ["video/webm;codecs=vp9,opus", "video/webm;codecs=vp8,opus", "video/webm", "video/mp4"] : ["audio/webm;codecs=opus", "audio/webm", "audio/mp4"]).find((m) => MediaRecorder.isTypeSupported(m)) ?? "";

const sleep = (ms: number, r: Running) =>
  new Promise<void>((resolve) => {
    r.wake = resolve;
    r.countdown = setTimeout(resolve, ms);
  });

function release(r: Running) {
  closeMonitor();
  for (const s of r.streams) stopStream(s);
  for (const c of r.cleanups) c();
  // The timeline played for timing: back to where the take starts (saved, discarded or cancelled).
  if (r.playing) {
    if(ownsTake(r)){playback.pause();playback.seek(r.frame);}
    r.playing = false;
  }
  muteOutput(false);
  if (state.screenLive) set({ screenLive: undefined });
}

/** The recorder (the first one of a multi-source take) started: the timer, a timed stop, the timeline playing. */
function recordingStarted(r: Running, startMs: number) {
  if(running!==r||r.cancelled||r.finished)return;
  const { options } = r;
  r.startMs = startMs;
  set({ phase: "recording", count: undefined, startedAt: Date.now() });
  if (options.seconds && options.seconds > 0) r.autoStop = setTimeout(() => void stopTake(), options.seconds * 1000);
  // Play the timeline from where the take goes, for timing (muted unless asked otherwise).
  if (ownsTake(r)&&(options.mode === "voiceover" || options.mode === "system") && options.playTimeline) {
    muteOutput(options.muteTimeline !== false);
    playback.seek(r.frame);
    playback.play();
    r.playing = true;
  }
}

/** D211: one MediaRecorder per source, all started in the same tick; the take ends when every one has stopped. */
function startParts(r: Running, streams: Map<PartKind, MediaStream>) {
  const { options } = r;
  const recs: Array<PartRec> = [];
  try {
    for (const [kind, stream] of streams) {
      const video = kind === "screen" || kind === "camera";
      const mime = pickMime(video);
      const recorder = new MediaRecorder(stream, { ...(mime ? { mimeType: mime } : {}), ...(video ? { videoBitsPerSecond: kind === "camera" && options.resolution === "720p" ? 6_000_000 : 10_000_000 } : {}), audioBitsPerSecond: 192_000 });
      recs.push({ kind, recorder, mime, chunks: [], startMs: undefined, stopMs: undefined });
    }
  } catch (error) {
    release(r);
    return endTake(r, { message: { kind: "error", text: editorText("Recording couldn't start: {0}",{0:errorText(error)}) } });
  }
  r.parts = recs;
  for (const p of recs) {
    p.recorder.ondataavailable = (e) => {
      if (e.data.size) p.chunks.push(e.data);
    };
    p.recorder.onstart = (e) => {
      // The event's own time (the page's clock): when this recorder's file begins.
      p.startMs = e.timeStamp > 0 ? e.timeStamp : performance.now();
      if (state.phase === "countdown" && running === r) recordingStarted(r, p.startMs);
    };
    p.recorder.onstop = () => {
      p.stopMs = performance.now();
      if (recs.every((x) => x.stopMs !== undefined)) void finishParts(r);
    };
    p.recorder.onerror=event=>{
      r.failure=errorText((event as Event&{error?:unknown}).error??new Error('Recording interrupted'))
      for(const other of recs)if(other.recorder.state!=='inactive')other.recorder.stop()
    }
  }
  try {
    for (const p of recs) p.recorder.start(250);
  } catch (error) {
    for (const p of recs) if (p.recorder.state !== "inactive") p.recorder.stop();
    r.parts = undefined;
    release(r);
    endTake(r, { message: { kind: "error", text: editorText("Recording couldn't start: {0}",{0:errorText(error)}) } });
  }
}

/** Record: opens the devices, then records (no 3-2-1, D267, D273) until stopTake or cancelTake. */
export async function startTake(options: TakeOptions) {
  if(typeof MediaRecorder==='undefined'){
    const text=editorText('This browser does not support recording.');
    if(options.agent)void actions.agentScreenTake(options.agent.id,{problem:text});
    return set({message:{kind:'error',text}});
  }
  const owner=currentEditorStore(),snapshot = owner?.getSnapshot().snapshot;
  const refuse = (text: string) => {
    if (options.agent) void actions.agentScreenTake(options.agent.id, { problem: text });
  };
  if(!getState().biliupCanEdit||getState().connection!=='connected'){
    const text=editorText('This project is not available for recording.');
    refuse(text);return set({message:{kind:'error',text}});
  }
  if (running || state.phase !== "idle") return refuse(editorText("Another recording is running."));
  if (!owner||!snapshot) {
    refuse("No project is open.");
    return set({ message: { kind: "error", text: "Open a project first: takes go into its media folder." } });
  }
  const r: Running = { owner, options, dir: snapshot.dir, frame: Math.round(getPlayback().frame),fps:projectOf(snapshot).canvas.fps, streams: [], cleanups: [], recorder: undefined, chunks: [], cancelled: false, startMs: 0, pauseMs: undefined, pausedTotal: 0, playing: false, countdown: undefined, wake: undefined, autoStop: undefined, answered: false, awaited: false, parts: undefined, finished: false, opening:true };
  running = r;
  if(options.agent)r.cleanups.push(subscribeApp(watchAgentTake));
  closeMonitor();
  const multi = isMulti(options);
  set({ phase: "countdown", mode: options.mode, count: options.countdown || undefined, message: undefined, recordedMs: undefined, startedAt: undefined, agentId: options.agent?.id, paused: false, pausedMs: 0, pausedAt: undefined, parts: multi ? options.parts : undefined });
  const stream = multi ? await openParts(options, r) : await openStreams(options, r);
  if(!ownsTake(r))r.cancelled=true;
  if (typeof stream === "string" || r.cancelled) {
    r.opening=false;
    release(r);
    return endTake(r, typeof stream === "string" ? { message: { kind: "error", text: stream } } : {});
  }
  const screen=stream instanceof Map?stream.get('screen'):options.mode==='screen'?stream:undefined;
  const label=screen?.getVideoTracks()[0]?.label?.replace(/[\u0000-\u001f\u007f-\u009f]/g,' ').trim();
  r.device=label?[...label].slice(0,120).join(''):undefined;
  const sound=stream instanceof Map?(stream.get('voiceover')??stream.get('system')):stream.getAudioTracks().length?stream:undefined;
  if(sound)watchMeter(sound);
  if(options.agent){
    const accepted=await actions.agentApprove(options.agent.id,true);
    if(!accepted||r.cancelled||!ownsTake(r)){r.opening=false;release(r);return endTake(r,{message:{kind:'discarded',text:editorText('Recording request ended.')}})}
  }
  r.opening=false;
  // D267, D273: every take records straight away; "countdown" without a count is the devices opening.
  for (let n = Math.max(0, Math.floor(options.countdown ?? 0)); n > 0; n--) {
    set({ count: n });
    await sleep(1000, r);
    if (!ownsTake(r)){cancelTake();return;}
    if (r.cancelled) return;
  }
  if (stream instanceof Map) return startParts(r, stream);
  const video = options.mode !== "voiceover" && options.mode !== "system";
  const mimeType = pickMime(video);
  let recorder: MediaRecorder;
  try {
    recorder = new MediaRecorder(stream, { ...(mimeType ? { mimeType } : {}), ...(video ? { videoBitsPerSecond: options.resolution === "720p" ? 6_000_000 : 10_000_000 } : {}), audioBitsPerSecond: 192_000 });
  } catch (error) {
    release(r);
    return endTake(r, { message: { kind: "error", text: editorText("Recording couldn't start: {0}",{0:errorText(error)}) } });
  }
  r.recorder = recorder;
  recorder.ondataavailable = (e) => {
    if (e.data.size) r.chunks.push(e.data);
  };
  recorder.onstart = () => recordingStarted(r, performance.now());
  recorder.onerror=event=>{r.failure=errorText((event as Event&{error?:unknown}).error??new Error('Recording interrupted'));if(recorder.state!=='inactive')recorder.stop()}
  recorder.onstop = () => {
    // A take stopped while paused: the pause up to now isn't in the file either.
    if (r.pauseMs !== undefined) (r.pausedTotal += performance.now() - r.pauseMs, (r.pauseMs = undefined));
    void finish(r, performance.now() - r.startMs - r.pausedTotal, recorder.mimeType || mimeType);
  };
  try {
    recorder.start(250);
  } catch (error) {
    r.recorder = undefined;
    release(r);
    endTake(r, { message: { kind: "error", text: editorText("Recording couldn't start: {0}",{0:errorText(error)}) } });
  }
}

/** Stop: the take is saved (or discarded when under a second). */
export async function stopTake() {
  const r = running;
  if (!r || r.finished || state.phase==='saving') return;
  const live = recordersOf(r).filter((x) => x.state !== "inactive");
  // MediaRecorder becomes inactive before its final data/stop events. A second device ending
  // must not turn that pending save into a cancellation.
  if (!live.length) {if(!recordersOf(r).length)cancelTake();return}
  for (const x of live) x.stop();
}

/** D209: Pause the running take (the timer and the file stop; the timeline it plays pauses too). */
export function pauseTake() {
  const r = running;
  const recs = r ? recordersOf(r) : [];
  if (!r || !recs.length || recs.some((x) => x.state !== "recording")) return;
  for (const x of recs) x.pause();
  r.pauseMs = performance.now();
  clearTimeout(r.autoStop);
  if (r.playing&&ownsTake(r)) playback.pause();
  set({ paused: true, pausedAt: Date.now() });
}

/** D209: Resume a paused take. */
export function resumeTake() {
  const r = running;
  const recs = r ? recordersOf(r) : [];
  if (!r || !recs.length || recs.some((x) => x.state !== "paused") || r.pauseMs === undefined) return;
  r.pausedTotal += performance.now() - r.pauseMs;
  r.pauseMs = undefined;
  for (const x of recs) x.resume();
  if (r.playing&&ownsTake(r)) playback.play();
  if (r.options.seconds && r.options.seconds > 0) r.autoStop = setTimeout(() => void stopTake(), Math.max(0, r.options.seconds * 1000 - (performance.now() - r.startMs - r.pausedTotal)));
  set({ paused: false, pausedMs: state.pausedMs + (Date.now() - (state.pausedAt ?? Date.now())), pausedAt: undefined });
}

/** Seconds the running take has recorded (pauses left out), for the timer. */
export const takeSeconds = (s: Pick<RecorderState, "startedAt" | "pausedMs" | "pausedAt">, now = Date.now()) =>
  s.startedAt === undefined ? 0 : Math.max(0, (s.pausedAt ?? now) - s.startedAt - s.pausedMs) / 1000;

/** Cancel: nothing is kept. */
export function cancelTake() {
  const r = running;
  if (!r || state.phase==='saving') return;
  r.cancelled = true;
  // Keep ownership until a pending native picker returns, then release its tracks. A new take
  // must not start while the old picker can still hand back a camera or display stream.
  if(r.opening){for(const stream of r.streams)stopStream(stream);return}
  clearTimeout(r.countdown);
  r.wake?.();
  const live = recordersOf(r).filter((x) => x.state !== "inactive");
  if (live.length) for (const x of live) x.stop();
  else {
    // The devices may still be opening: startTake releases what they give once it sees `cancelled`.
    release(r);
    endTake(r, { message: { kind: "discarded", text: "Cancelled. Nothing was kept." } });
  }
}

async function finish(r: Running, ms: number, mimeType: string) {
  if(r.finished)return
  r.finished=true
  release(r);
  const recordedMs = Math.round(ms);
  if (r.cancelled) return endTake(r, { message: { kind: "discarded", text: "Cancelled. Nothing was kept." } });
  if ((!r.failure&&recordedMs < MIN_TAKE_MS) || !r.chunks.length) return endTake(r, { recordedMs, message: { kind:r.failure?'error':'discarded', text:r.failure??"That take was under a second, so it was discarded." } });
  set({ phase: "saving", startedAt: undefined, recordedMs });
  const { options } = r;
  const audioOnly = options.mode === "voiceover" || options.mode === "system";
  const blob = new Blob(r.chunks, { type: mimeType.split(";")[0] || (audioOnly ? "audio/webm" : "video/webm") });
  if (options.agent) {
    const takeId=options.agent.id.replace(/^(.{8})(.{4})(.{4})(.{4})(.{12})$/,'$1-$2-$3-$4-$5');
    const recovery=await retainCaptureDraft({multi:false,fps:r.fps,dir:r.dir,name:r.device,parts:[{blob,kind:'screen',device:r.device,offsetMs:0,recordedMs}]},takeId,true);
    const uploaded=await actions.uploadTake(blob,"screen",options.agent.id,r.dir,r.device);
    if("message" in uploaded){failCaptureDraft(recovery,uploaded.message);return endTake(r,{message:{kind:"error",text:uploaded.message}});}
    const answer=await actions.agentScreenTake(options.agent.id,{path:uploaded.path},r.dir);
    if(!answer.ok){failCaptureDraft(recovery,answer.error.message);return endTake(r,{message:{kind:"error",text:answer.error.message}});}
    r.answered=true;
    await releaseCaptureDraft(recovery);
    return endTake(r,{message:{kind:"saved",text:editorText("Recording uploaded to the assistant.")}});
  }
  const kind = options.mode === "multi" ? "screen" : options.mode;
  const device = kind === "screen" ? r.device : kind === "camera" ? state.cameras.find((c) => c.id === options.camera)?.label : kind === "system" ? undefined : state.mics.find((m) => m.id === options.mic)?.label;
  const place = audioOnly || options.place === true;
  const result = await saveCaptureDraft({
    multi:false,fps:r.fps,parts:[{blob,kind,device,offsetMs:0,recordedMs}],
    dir: r.dir,
    ...(kind === "screen" && r.device ? { name: r.device } : {}),
    ...(place ? { placeAt: r.frame } : {}),
  });
  if ("problem" in result) return endTake(r, { message: { kind: "error", text: result.problem } });
  const { saved } = result;
  endTake(r, { message: { kind: "saved", text: editorText(r.failure?'Recording interrupted; the available recording was saved.':place?"{0} saved to Media and timeline ({1} s).":"{0} saved to Media ({1} s).",{0:saved.name,1:saved.seconds.toFixed(1)}) } });
}

/** D211: every recorder of a multi-source take stopped: the parts go up and in as one edit. */
async function finishParts(r: Running) {
  if (r.finished) return;
  r.finished = true;
  if (r.pauseMs !== undefined) (r.pausedTotal += performance.now() - r.pauseMs, (r.pauseMs = undefined));
  release(r);
  const parts = (r.parts ?? []).filter(part=>part.chunks.length);
  const first = Math.min(...parts.map((p) => p.startMs ?? Infinity));
  const lengthOf = (p: PartRec) => Math.max(0,(p.stopMs ?? 0) - (p.startMs ?? p.stopMs ?? 0) - r.pausedTotal);
  const recordedMs = parts.length?Math.round(Math.min(...parts.map(lengthOf))):0;
  if (r.cancelled) return endTake(r, { message: { kind: "discarded", text: "Cancelled. Nothing was kept." } });
  if (!parts.length || (!r.failure&&recordedMs < MIN_TAKE_MS)) return endTake(r, { recordedMs, message: { kind:r.failure?'error':'discarded', text:r.failure??"That take was under a second, so it was discarded." } });
  const offsets: Partial<Record<PartKind, number>> = {};
  for (const p of parts) offsets[p.kind] = Math.round(((p.startMs ?? first) - first) * 10) / 10;
  set({ phase: "saving", startedAt: undefined, recordedMs, offsets });
  const { options } = r;
  const deviceOf = (kind: PartKind) =>
    kind === "screen" ? r.device : kind === "camera" ? state.cameras.find((c) => c.id === options.camera)?.label : kind === "voiceover" ? state.mics.find((m) => m.id === options.mic)?.label : undefined;
  const result = await saveCaptureDraft({
    multi:true,fps:r.fps,
    parts: parts.map((p) => ({ blob: new Blob(p.chunks, { type: (p.recorder.mimeType || p.mime).split(";")[0] || (p.kind === "screen" || p.kind === "camera" ? "video/webm" : "audio/webm") }), kind: p.kind, device: deviceOf(p.kind), offsetMs: offsets[p.kind] ?? 0, recordedMs: lengthOf(p) })),
    name: parts.some((p) => p.kind === "screen") ? r.device : undefined,
    placeAt: r.frame,
    layout: options.layout,
    dir: r.dir,
  });
  if ("problem" in result) return endTake(r, { message: { kind: "error", text: result.problem } });
  endTake(r, { message: { kind: "saved", text: editorText(r.failure?'Recording interrupted; the available recording was saved.':"{0} saved to Media and timeline ({1} s).",{0:result.saved.name,1:result.saved.seconds.toFixed(1)}) } });
}

/**
 * D144: an agent's take is cancelled (streams stopped, nothing kept) once nobody waits for it: the
 * project changed, the turn stopped or finished, or the server stopped waiting (timed out).
 */
function watchAgentTake() {
  const r = running;
  if (!r?.options.agent || r.cancelled) return;
  const app = getState();
  const id = r.options.agent.id;
  if (app.agent?.screenTake === id) r.awaited = true;
  const gone = !ownsTake(r) || !app.agent || app.agent.dir !== r.dir || app.agent.running === null || (r.awaited && app.agent.screenTake !== id);
  if (gone) {
    // Cancel an unfinished take; a completed upload has its own durable server receipt.
    r.answered = true;
    cancelTake();
  }
}

export const clock = (seconds: number) => {
  const s = Math.max(0, Math.floor(seconds));
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
};

/** The Capture tab went away: devices close; a running take keeps going until Stop. */
export function closeAll() {
  closeMonitor(true);
  closePreview();
}

