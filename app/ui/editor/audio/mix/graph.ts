// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The mix's Web Audio graph (D83, D84, D86): the context, master, limiter, track buses and their
// effects, element routes and gain ramps, the meters and the test-only capture. Mix (audio.ts) drives it.

import { masterGain, panGains, trackPan } from "@/app/lib/editor/core/audio/audio";
import { masterLimiter, trackStages, type Stage } from "@/app/lib/editor/core/audio/effects";

import type { ProjectModel } from "@/app/lib/editor/state/project";
import { type Bus, type Route, outputMuted, liveOutputs, UNWIRE_S, RAMP_S } from "@/app/ui/editor/audio/mix/elements";
import { type LimiterNodes, buildLimiter, buildStages, stageShape, setLimiter, loadDenoise } from "@/app/ui/editor/audio/mix/nodes";
import { getDuckState } from "@/app/ui/editor/audio/ducking";
import { previewLoudnessGain } from "@/app/ui/editor/audio/loudnessPreview";
import { type Capture, type Levels, type StereoLevel, levelOf, SILENT, type CaptureResult, sinkOf } from "@/app/ui/editor/audio/mix/levels";

const newContext = () => new AudioContext({ latencyHint: 'interactive' });

export class MixGraph {
  protected ctx: AudioContext | undefined;
  protected master: GainNode | undefined;
  protected analyser: AnalyserNode | undefined;
  protected masterMeters: readonly [AnalyserNode, AnalyserNode] | undefined;
  protected readonly buses = new Map<string, Bus>();
  /** The pan factors and master gain last asked for (ramped towards). */
  protected readonly panTarget = new WeakMap<Bus, { left: number; right: number }>();
  /** The stages each bus was last set to (JSON), so unchanged ones are skipped. */
  protected readonly stageKeys = new WeakMap<Bus, string>();
  protected masterTarget = 1;
  /** After the limiter: what the speakers, meters and capture get. */
  protected post: GainNode | undefined;
  protected limiter: LimiterNodes | undefined;
  /** Whether the master feeds the limiter (it is wired) and what the project asks for. */
  protected limiterOn = false;
  protected limiterTarget: { readonly ceilingDb: number } | undefined = { ceilingDb: -1 };
  /** Test-only capture of the master output (see capture()). */
  protected tap: Capture | undefined;
  protected marks: Array<{ t: number; frame: number }> = [];
  protected readonly routes = new WeakMap<HTMLMediaElement, Route>();

  /** An AudioContext made ahead of the first play (prepare()), suspended until graph() takes it. */
  protected spare: AudioContext | undefined;

  /**
   * Issue #55: make the AudioContext before the first play. Making one blocks the page for 90 ms
   * (near 300 ms on a busy machine), which was a hitch in the first frame of playback. It waits
   * suspended (no audio device kept busy) and becomes the graph's context on first use.
   */
  prepare() {
    if (this.ctx || this.spare) return;
    try {
      this.spare = newContext();
      void this.spare.suspend().catch(() => {});
    } catch {
      this.spare = undefined;
    }
  }

  /** The audio graph, made on first use (an AudioContext can't be made before the page runs). */
  protected graph() {
    if (!this.ctx) {
      try {
        const ctx = this.spare ?? newContext();
        this.spare = undefined;
        // Noise removal's worklet (D246): its chains pass through until it has loaded, then rebuild.
        void loadDenoise(ctx);
        const master = ctx.createGain();
        master.gain.value = this.masterTarget;
        // After the master volume: the limiter (D86, wired by mixer()), then `post`, what is heard and metered.
        const post = ctx.createGain();
        const limiter = buildLimiter(ctx, this.limiterTarget?.ceilingDb ?? -1);
        limiter.clip.connect(post);
        master.connect(this.limiterTarget ? limiter.input : post);
        const analyser = ctx.createAnalyser();
        analyser.fftSize = 2048;
        post.connect(analyser);
        const out = ctx.createGain();
        out.gain.value = outputMuted ? 0 : 1;
        analyser.connect(out);
        out.connect(ctx.destination);
        liveOutputs.add(out);
        // The master strip's meters: the output split into its two channels.
        const split = ctx.createChannelSplitter(2);
        const meters = [ctx.createAnalyser(), ctx.createAnalyser()] as const;
        for (const m of meters) m.fftSize = 2048;
        post.connect(split);
        split.connect(meters[0], 0);
        split.connect(meters[1], 1);
        this.ctx = ctx;
        this.post = post;
        this.limiter = limiter;
        this.limiterOn = this.limiterTarget !== undefined;
        this.master = master;
        this.analyser = analyser;
        this.masterMeters = meters;
      } catch {
        return undefined;
      }
    }
    return { ctx: this.ctx, master: this.master!, analyser: this.analyser! };
  }

  /** Route an element's sound through its own gain (once per element: Web Audio allows no more). */
  protected route(el: HTMLMediaElement): Route | undefined {
    const known = this.routes.get(el);
    if (known) return known;
    const g = this.graph();
    if (!g) return undefined;
    try {
      const source = g.ctx.createMediaElementSource(el);
      const gain = g.ctx.createGain();
      gain.gain.value = 0;
      source.connect(gain);
      // Until render() wires it to its track's bus, straight to the master.
      gain.connect(g.master);
      // From now on the element is heard only through the graph; muted would silence its input.
      el.muted = false;
      const route = { source, gain, target: 0 };
      this.routes.set(el, route);
      return route;
    } catch {
      return undefined;
    }
  }

  /** The bus of a track, made on first use. */
  protected bus(trackId: string): Bus | undefined {
    const known = this.buses.get(trackId);
    if (known) return known;
    const g = this.graph();
    if (!g) return undefined;
    const ctx = g.ctx;
    // Explicit stereo with speaker up-mixing: a mono source plays at full level on both channels.
    const input = ctx.createGain();
    input.channelCount = 2;
    input.channelCountMode = "explicit";
    input.channelInterpretation = "speakers";
    const splitter = ctx.createChannelSplitter(2);
    const left = ctx.createGain();
    const right = ctx.createGain();
    const merger = ctx.createChannelMerger(2);
    const meters = [ctx.createAnalyser(), ctx.createAnalyser()] as const;
    for (const m of meters) m.fftSize = 2048;
    input.connect(splitter);
    splitter.connect(left, 0);
    splitter.connect(right, 1);
    left.connect(merger, 0, 0);
    right.connect(merger, 0, 1);
    left.connect(meters[0]);
    right.connect(meters[1]);
    merger.connect(g.master);
    const bus: Bus = { input, chain: buildStages(ctx, []), splitter, left, right, merger, meters };
    this.buses.set(trackId, bus);
    return bus;
  }

  protected dropBus(trackId: string) {
    const bus = this.buses.get(trackId);
    if (!bus) return;
    this.buses.delete(trackId);
    // The chain's own nodes go with dispose (D262: noise removal's processors end, not just disconnect).
    const nodes = [bus.input, bus.splitter, bus.left, bus.right, bus.merger];
    const letGo = () => {
      bus.chain.dispose();
      nodes.forEach((n) => n.disconnect());
    };
    const ctx = this.ctx;
    if (!ctx || ctx.state !== "running") return letGo();
    // Ramp its output down first, then let go of it.
    for (const g of [bus.left.gain, bus.right.gain]) {
      g.cancelScheduledValues(ctx.currentTime);
      g.setValueAtTime(g.value, ctx.currentTime);
      g.linearRampToValueAtTime(0, ctx.currentTime + UNWIRE_S);
    }
    setTimeout(letGo, UNWIRE_S * 1000 + 15);
  }

  /** Clear every noise removal's buffers (D262): a seek, a scrub or a play start leaves no stale tail. */
  protected resetDenoise() {
    for (const bus of this.buses.values()) bus.chain.reset();
  }

  /** Feed a route into another bus: at once when it is silent, else after a short ramp down (the next render ramps it back up). */
  protected rewire(route: Route, bus: Bus) {
    const ctx = this.ctx!;
    if (route.rewiring !== undefined) return;
    const connect = () => {
      route.gain.disconnect();
      route.gain.connect(bus.input);
      route.bus = bus;
      route.rewiring = undefined;
    };
    if (route.gain.gain.value === 0 || ctx.state !== "running") return connect();
    const param = route.gain.gain;
    param.cancelScheduledValues(ctx.currentTime);
    param.setValueAtTime(param.value, ctx.currentTime);
    param.linearRampToValueAtTime(0, ctx.currentTime + UNWIRE_S);
    route.target = 0;
    route.rewiring = ctx.currentTime + UNWIRE_S;
    setTimeout(connect, UNWIRE_S * 1000 + 5);
  }

  /** Move an AudioParam towards a value with the usual short ramp. */
  protected glide(param: AudioParam, target: number) {
    const ctx = this.ctx!;
    const now = ctx.currentTime;
    param.cancelScheduledValues(now);
    param.setValueAtTime(param.value, now);
    param.linearRampToValueAtTime(target, now + RAMP_S);
  }

  /** Pan every sounding track's bus and set the master volume (D83); drop buses of tracks that went. */
  protected mixer(project: ProjectModel) {
    this.limiterTarget = masterLimiter(project);
    // The master volume, and At target's loudness gain (D246) where the export puts it: before the limiter.
    const master = masterGain(project) * previewLoudnessGain(project);
    if (!this.ctx) {
      this.masterTarget = master;
      return;
    }
    this.wireLimiter();
    const ids = new Set(project.tracks.map((t) => t.id));
    for (const id of [...this.buses.keys()]) if (!ids.has(id)) this.dropBus(id);
    for (const track of project.tracks) {
      const bus = this.buses.get(track.id);
      if (!bus) continue;
      // The export's stages (D246: noise removal too, from the same measured floor).
      this.effects(bus, trackStages(track, project, getDuckState().envelopes));
      const want = panGains(trackPan(track));
      const had = this.panTarget.get(bus);
      if (had && had.left === want.left && had.right === want.right) continue;
      this.panTarget.set(bus, { ...want });
      if (had) {
        this.glide(bus.left.gain, want.left);
        this.glide(bus.right.gain, want.right);
      } else {
        bus.left.gain.value = want.left;
        bus.right.gain.value = want.right;
      }
    }
    if (master !== this.masterTarget && this.master) {
      this.masterTarget = master;
      this.glide(this.master.gain, master);
    }
  }

  /** A bus's effect chain to `stages` (D84): parameters glide when its shape is the same, else it is rebuilt between input and splitter. */
  protected effects(bus: Bus, stages: ReadonlyArray<Stage>) {
    const shape = stageShape(stages, this.ctx);
    const key = `${shape}|${JSON.stringify(stages)}`;
    if (this.stageKeys.get(bus) === key) return;
    const had = this.stageKeys.has(bus);
    this.stageKeys.set(bus, key);
    if (bus.chain.shape === shape) {
      bus.chain.set(stages, had ? RAMP_S : 0);
      return;
    }
    const old = bus.chain;
    const chain = buildStages(this.ctx!, stages);
    bus.input.disconnect();
    // D262: dispose, not just disconnect: a noise removal's processor would otherwise run on forever.
    old.dispose();
    if (chain.first && chain.last) {
      bus.input.connect(chain.first);
      chain.last.connect(bus.splitter);
    } else bus.input.connect(bus.splitter);
    bus.chain = chain;
  }

  /** Wire the master limiter in or out (D86) and set its ceiling. */
  protected wireLimiter() {
    const want = this.limiterTarget;
    const l = this.limiter;
    if (!l || !this.master || !this.post) return;
    if (want && want.ceilingDb !== l.ceilingDb) setLimiter(l, want.ceilingDb);
    if ((want !== undefined) === this.limiterOn) return;
    this.master.disconnect(want ? this.post : l.input);
    this.master.connect(want ? l.input : this.post);
    this.limiterOn = want !== undefined;
  }

  protected ramp(route: Route, target: number) {
    const ctx = this.ctx!;
    if (route.target === target && route.gain.gain.value === target) return;
    route.target = target;
    const now = ctx.currentTime;
    const param = route.gain.gain;
    param.cancelScheduledValues(now);
    param.setValueAtTime(param.value, now);
    param.linearRampToValueAtTime(target, now + RAMP_S);
  }

  /** Left and right levels of the master and of every track bus (the Mixer's meters), from the last ~43 ms. */
  levels(): Levels {
    const read = (meters: readonly [AnalyserNode, AnalyserNode] | undefined): StereoLevel => (meters ? { left: levelOf(meters[0]), right: levelOf(meters[1]) } : SILENT);
    const tracks: Record<string, StereoLevel> = {};
    const reduction: Record<string, number> = {};
    for (const [id, bus] of this.buses) {
      tracks[id] = read(bus.meters);
      reduction[id] = bus.chain.reduction();
    }
    return { master: read(this.masterMeters), tracks, limiter: this.limiterOn && this.limiter ? this.limiter.input.reduction : 0, reduction };
  }

  /**
   * Test-only: record what the master puts out (after the master volume), sample for sample, with
   * the context time of each block and, from render(), which frame was drawn at which context
   * time. The app tests compare it with the export's mix (D83). Nothing else uses it.
   */
  capture(): { stop: () => CaptureResult } | undefined {
    const g = this.graph();
    if (!g) return undefined;
    const node = g.ctx.createScriptProcessor(4096, 2, 2);
    const sink = g.ctx.createGain();
    sink.gain.value = 0;
    const tap: Capture = { node, sink, blocks: [] };
    node.onaudioprocess = (e) => {
      tap.blocks.push({ t: e.playbackTime, left: e.inputBuffer.getChannelData(0).slice(), right: e.inputBuffer.getChannelData(1).slice() });
    };
    // After the limiter (D86): what is heard.
    const out = this.post ?? g.master;
    out.connect(node);
    node.connect(sink);
    sink.connect(g.ctx.destination);
    this.tap = tap;
    this.marks = [];
    return {
      stop: () => {
        out.disconnect(node);
        node.disconnect();
        sink.disconnect();
        node.onaudioprocess = null;
        if (this.tap === tap) this.tap = undefined;
        return { sampleRate: g.ctx.sampleRate, blocks: tap.blocks, marks: this.marks, latency: (g.ctx.outputLatency || 0) + (g.ctx.baseLatency || 0), sink: sinkOf(g.ctx) };
      },
    };
  }

  /** The master output's level right now: RMS and peak of the last ~43 ms (0 when nothing plays). */
  level(): { rms: number; peak: number } {
    return this.analyser ? levelOf(this.analyser) : { rms: 0, peak: 0 };
  }
}
