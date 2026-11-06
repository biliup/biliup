// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { LIMITER_MARGIN_DB, type Stage, DENOISE_FLOOR_DB } from "@/app/lib/editor/core";
import { n } from "./num.ts";
import type { ExportPlan } from "./types.ts";
import { constantGain, AUDIO_BLOCK, gainExpr } from "./gain.ts";

const atempo = (speed: number) => {
  const out: Array<string> = [];
  let s = speed;
  // Keep each stage within the interpolation range instead of dropping samples at high rates.
  while (s > 2) (out.push("atempo=2"), (s /= 2));
  while (s < 0.5) (out.push("atempo=0.5"), (s /= 0.5));
  if (Math.abs(s - 1) > 1e-9) out.push(`atempo=${n(s)}`);
  return out;
};

export interface AudioGraph {
  /** Input arguments for the audio layers, starting at input index `first`. */
  readonly inputs: Array<string>;
  /** Filter chains ending in the label [aout] (48 kHz stereo, `frames` long). */
  readonly filters: Array<string>;
}

/** What follows the mix (D86, D87): a loudness filter or gain, the limiter, a measuring filter at the very end. */
export interface AudioOptions {
  /** A loudnorm filter (the measuring pass's, or the second pass's when the limiter is bypassed). */
  readonly loudnorm?: string | undefined;
  /** The loudness gain in dB (the second pass with the limiter on). */
  readonly gainDb?: number | undefined;
  /** Leave the limiter out (the first measuring pass measures the mix as it goes into it). */
  readonly noLimiter?: boolean | undefined;
  /** A filter after everything (a measuring pass's loudnorm print or ebur128). */
  readonly tail?: string | undefined;
  /** Report original mix time before discarding a range's prefix (audio-only passes). */
  readonly progress?: boolean | undefined;
}

/** True-peak margin (dB) under the ceiling the limiter works to (core, shared with the preview's limiter, D86). */
export { LIMITER_MARGIN_DB };
/** The limiter's lookahead / attack and release (ms): alimiter's defaults. */
export const LIMITER_ATTACK_MS = 5;
export const LIMITER_RELEASE_MS = 50;

/**
 * The master limiter as ffmpeg filters (D86): alimiter at 4× (192 kHz, so inter-sample peaks are
 * seen: "true-peak-ish") with the limit LIMITER_MARGIN_DB under the ceiling, auto-level off and
 * its lookahead delay compensated, then back to 48 kHz.
 */
export const limiterFilter = (ceilingDb: number) =>
  `aresample=192000,alimiter=limit=${n(Math.max(0.0625, 10 ** ((ceilingDb - LIMITER_MARGIN_DB) / 20)))}:attack=${LIMITER_ATTACK_MS}:release=${LIMITER_RELEASE_MS}:level=0:latency=1,aresample=48000`;

const lin = (db: number) => 10 ** (db / 20);
/** One effect stage as an ffmpeg filter (D84; the curves core biquadResponseDb states, the compressor's static curve). */
export function stageFilter(stage: Stage): string {
  switch (stage.kind) {
    case "biquad":
      switch (stage.type) {
        case "highpass":
        case "lowpass":
          return `${stage.type}=f=${n(stage.freq)}:p=2:t=q:w=${n(stage.q)}`;
        case "peaking":
          return `equalizer=f=${n(stage.freq)}:t=q:w=${n(stage.q)}:g=${n(stage.gainDb)}`;
        case "lowshelf":
        case "highshelf":
          return `${stage.type}=f=${n(stage.freq)}:t=s:w=1:g=${n(stage.gainDb)}`;
      }
      break;
    case "compressor":
      return `acompressor=threshold=${n(Math.max(0.000976563, lin(stage.thresholdDb)))}:ratio=${n(stage.ratio)}:attack=${n(stage.attackMs)}:release=${n(stage.releaseMs)}:makeup=${n(lin(stage.makeupDb))}:knee=1:link=maximum:detection=peak`;
    case "denoise":
      return `afftdn=nr=${n(6 + 24 * stage.amount)}:nf=${n(Math.min(-20, Math.max(-80, stage.floorDb ?? DENOISE_FLOOR_DB)))}`;
  }
  return "anull";
}

/** The audio half of the graph, with one shared set of mix and measurement options. */
export function audioGraph(plan: ExportPlan, first: number, opts: AudioOptions = {}): AudioGraph {
  const total = n((plan.audio.length ? plan.audioFrames : plan.frames) / plan.fps);
  const firstSample = plan.audio.length ? Math.round(plan.audioWindow.from * 48000 / plan.fps) : 0;
  const samples = Math.max(1, Math.round(plan.frames * 48000 / plan.fps));
  const inputs: Array<string> = [];
  const filters: Array<string> = [`anullsrc=r=48000:cl=stereo,atrim=duration=${total}[asil]`];
  const labels = ["[asil]"];
  const buses = plan.buses ?? [];
  const busOf = new Map(buses.map((b) => [b.track, [] as Array<string>]));
  plan.audio.forEach((layer, k) => {
    const index = first + k;
    const seconds = (layer.to - layer.from) / plan.fps;
    const samples = Math.max(1, Math.round(seconds * 48000));
    const sourceSamples = Math.max(1, Math.round(seconds * layer.speed * 48000));
    if (layer.cycle) inputs.push("-t", n(layer.cycle.samples / 48000 + 0.5), "-i", layer.file);
    else inputs.push("-ss", n(layer.source), "-t", n(seconds * layer.speed + 0.05), "-i", layer.file);
    const constant = constantGain(layer.gain);
    const volume = constant !== undefined ? `volume=${n(constant)}` : `asetnsamples=n=${AUDIO_BLOCK}:p=0,volume=eval=frame:volume='${gainExpr(layer.gain, plan.fps, layer.localStart)}'`;
    const delay = Math.round((layer.from / plan.fps) * 48000);
    const bus = busOf.get(layer.track);
    filters.push(
      [
        `[${index}:a:0]aresample=48000`,
        // Mono plays at full level in both ears, as Web Audio up-mixes it in the preview.
        "pan=stereo|FL<FL+FC+BL+SL|FR<FR+FC+BR+SR",
        "aformat=sample_fmts=fltp:channel_layouts=stereo",
        // The track's pan (D83): one factor per channel, the law in core panGains. A bussed track pans after its effects.
        ...(!bus && (layer.pan.left !== 1 || layer.pan.right !== 1) ? [`pan=stereo|c0=${n(layer.pan.left)}*c0|c1=${n(layer.pan.right)}*c1`] : []),
        ...(layer.cycle
          ? [
              // Exactly the media's length in project frames (padded if its sound is shorter), repeated.
              `apad=whole_len=${layer.cycle.samples}`,
              `atrim=end_sample=${layer.cycle.samples}`,
              `aloop=loop=-1:size=${layer.cycle.samples}`,
              `atrim=start_sample=${layer.cycle.offset}:end_sample=${layer.cycle.offset + sourceSamples}`,
            ]
          : [`atrim=end_sample=${sourceSamples}`]),
        "asetpts=PTS-STARTPTS",
        // keepPitch false (D125): resampled like tape (the pitch follows the speed), else atempo keeps it.
        ...(layer.item.keepPitch === false && Math.abs(layer.speed - 1) > 1e-9 ? [`asetrate=${n(48000 * layer.speed)}`, "aresample=48000"] : atempo(layer.speed)),
        // Tempo/resample filters can leave a partial final block: keep the planned span exact.
        `apad=whole_len=${samples}`,
        `atrim=end_sample=${samples}`,
        "asetpts=N/SR/TB",
        volume,
        `adelay=delays=${delay}S:all=1`,
      ].join(",") + `[a${k}]`,
    );
    (bus ?? labels).push(`[a${k}]`);
  });
  // Tracks with effects (D84): their items summed over a silent base, the stages in order, then the pan.
  buses.forEach((b, j) => {
    const items = busOf.get(b.track)!;
    if (!items.length) return;
    filters.push(`anullsrc=r=48000:cl=stereo,atrim=duration=${total}[tsil${j}]`);
    const chain = [
      `[tsil${j}]${items.join("")}amix=inputs=${items.length + 1}:duration=first:dropout_transition=0:normalize=0`,
      ...b.stages.map(stageFilter),
      "aformat=sample_fmts=fltp:channel_layouts=stereo",
      ...(b.pan.left !== 1 || b.pan.right !== 1 ? [`pan=stereo|c0=${n(b.pan.left)}*c0|c1=${n(b.pan.right)}*c1`] : []),
    ];
    filters.push(`${chain.join(",")}[t${j}]`);
    labels.push(`[t${j}]`);
  });
  const mix =
    labels.length === 1
      ? `[asil]anull`
      : `${labels.join("")}amix=inputs=${labels.length}:duration=first:dropout_transition=0:normalize=0`;
  // The master volume after the sum (D83), before loudness normalising (which would undo it).
  const master = plan.masterGain !== 1 && labels.length > 1 ? `,volume=${n(plan.masterGain)}` : "";
  const processing = [
    ...(opts.gainDb !== undefined && opts.gainDb !== 0 ? [`volume=${n(opts.gainDb)}dB`] : []),
    // The master limiter, last (D86). Nothing to limit in a silent mix.
    ...(plan.limiter && !opts.noLimiter && labels.length > 1 ? [limiterFilter(plan.limiter.ceilingDb)] : []),
  ];
  const output = [
    // Report at most once per second of source time, after stateful processing.
    // Regrouping here does not change the blocks seen by the track effects or limiter.
    ...(opts.progress && firstSample > 0 ? ["asetnsamples=n=48000:p=0", "ashowinfo@biliup_audio_progress"] : []),
    // Buses, tempo filters and the limiter have seen the original history and future
    // samples before the output window is selected. Measurement never counts that history.
    `atrim=start_sample=${firstSample}:end_sample=${firstSample + samples}`,
    "asetpts=N/SR/TB",
    // Bypassed-limiter loudnorm targets this exported interval, not the discarded prefix.
    ...(opts.loudnorm ? [opts.loudnorm, "aresample=48000"] : []),
    `apad=whole_len=${samples}`,
    `atrim=end_sample=${samples}`,
    ...(opts.tail ? [opts.tail] : []),
  ];
  filters.push(`${mix}${master},atrim=duration=${total}${[...processing,...output].map((f) => `,${f}`).join("")},aformat=sample_fmts=fltp:channel_layouts=stereo[aout]`);
  return { inputs, filters };
}

/** Audio-only work shares one command shape; it never needs video encoder arguments. */
export function audioCommand(plan: ExportPlan, options: AudioOptions, outputArgs: Array<string>) {
  const audio = audioGraph(plan, 0, { ...options, progress: true });
  return {
    inputs: audio.inputs,
    graph: audio.filters.join(";\n"),
    outputArgs,
    totalMs: Math.max(1, Math.round(((plan.audio.length ? plan.audioWindow.from : 0) + plan.frames) * 1000 / plan.fps)),
  };
}
