// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The mix's timing constants, route and bus shapes, element seeking, and the output mute (D114).
import type { StageChain } from "@/app/ui/editor/audio/mix/nodes";

/** How long a gain change takes (one display frame): smooth, and short enough to feel immediate. */
export const RAMP_S = 1 / 60;
/** Seek a playing element when it is this far from where the clock says it should be. */
export const DRIFT_S = 0.12;
/** Audio items starting this soon get their element ready. */
export const WARM_S = 2;
/** ...and this soon (while playing) it already runs, silent, so a cut doesn't wait for it to start. */
export const PREROLL_S = 0.3;
// When an element was first seen playing (its currentTime then), to tell when it really runs.
export const startedAt = new WeakMap<HTMLMediaElement, number>();

export interface Route {
  readonly source: MediaElementAudioSourceNode;
  readonly gain: GainNode;
  /** The gain last asked for (the AudioParam ramps towards it). */
  target: number;
  /** The track bus its gain feeds (items move between tracks). */
  bus?: Bus;
  /** Moving to another bus: ramping down until this context time, then rewired (no click). */
  rewiring?: number;
}

/** A reroute or a dropped bus first ramps to silence over this long, so it doesn't click. */
export const UNWIRE_S = 0.01;

/** A track's bus (D83): stereo up-mix → its effects (D84) → pan (a gain per channel) → master, with a meter per channel. */
export interface Bus {
  readonly input: GainNode;
  /** The effect stages' nodes between input and splitter (empty: input feeds the splitter). */
  chain: StageChain;
  readonly splitter: ChannelSplitterNode;
  readonly left: GainNode;
  readonly right: GainNode;
  readonly merger: ChannelMergerNode;
  readonly meters: readonly [AnalyserNode, AnalyserNode];
}

export {mediaPosition as wanted,seekEditorMedia as seek} from '../../preview/mediaElement';

// D114: the Capture tab mutes what the preview plays while a voiceover records with the timeline
// playing (so the microphone doesn't pick it up). The meters still move; only the speakers go quiet.
export let outputMuted = false;
export const liveOutputs = new Set<GainNode>();
export function muteOutput(muted: boolean) {
  outputMuted = muted;
  for (const out of liveOutputs) out.gain.value = muted ? 0 : 1;
}
