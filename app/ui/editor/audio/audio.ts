// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The preview's playback mix (phase 4 chunk 5, D54). Every item that sounds at the playhead
// (core audibleAt: clips with sound and audio items, the gain from their volume, keys, fades, the
// track's volume and mute, transitions) plays through Web Audio:
//
//   media element → MediaElementAudioSourceNode → GainNode (per item) → track bus → master → analyser → speakers
//
// The track bus (D83) makes the sum stereo (mono at full level on both channels, as the speakers
// up-mixed it before), pans it with core panGains (left and right GainNodes between a splitter and
// a merger: exactly the export's per-channel factors, not a StereoPannerNode, whose law differs)
// and meters it (one AnalyserNode per channel, after the pan). The master GainNode is the project's
// master volume (core masterGain), metered the same way after it; the mono analyser after it stays
// the app tests' level().
//
// A clip that the preview shows already has a <video> (the Compositor's): its sound is routed from
// that element, so picture and sound come from one decoder and stay together. Audio items, and
// clips on a hidden track (hidden hides the picture, not the sound), get an <audio> element here,
// warmed up two seconds ahead like the next video, and kept in step with the playback clock the
// same way (seeked only when it drifts). Gains move by a short linear ramp once per animation
// frame towards the value one ramp ahead, so volume and fades follow smoothly and a cut starts or
// stops with a ~17 ms ramp instead of a click. Paused or scrubbing: silent.
//
// Track effects (D84): each bus runs its track's stages (core trackStages, the export's own: noise
// removal too since D246, an AudioWorklet running core's SpectralDenoiser from the track's measured
// noise floor) between its input and its pan, as the export filters each track's sum: BiquadFilterNodes for the EQ (the cookbook curves core biquadResponseDb states; Web Audio
// takes a high-/low-pass Q in dB) and a DynamicsCompressorNode with a gain after it that cancels
// its built-in make-up gain (core makeupCompensationDb) and adds the effect's own. Ducking (D85) is
// core's duck curve added to each item's gain on the ducked track (ducking.ts duckCurvesOf), the
// export's numbers. The master limiter (D86) follows the master volume: a hard-knee
// DynamicsCompressor (ratio 20, compensated) working LIMITER_MARGIN_DB under the ceiling, as the
// export's alimiter does, and a clip at the ceiling, so the preview never goes over it either;
// the meters, level() and the capture read after it.
//
// Noise removal's latency (D262): its worklet hands the sound back DENOISE_FFT samples (~21 ms)
// late, which the export's afftdn doesn't. So a track with noise removal running is heard that much
// early: its items are looked at `lead` seconds ahead (which item sounds, its gain, its source time),
// and its clips sound through an <audio> element of the mix's (the Compositor's <video> keeps only
// the picture), started `lead` ahead. Out of the worklet the sound lands with the picture and the
// other tracks, as in the export. Every play start, seek and scrub clears the worklets' buffers.
//
// The file:// renderer can only hear an element through Web Audio if the media was fetched in
// CORS mode: elements get crossOrigin="anonymous" before their src, and the server's /media/file
// route allows it.
import { audibleAt, audioSpan, itemGain, soundMedia, sourceAt } from "@/app/lib/editor/core/audio/audio";
import { trackStages } from "@/app/lib/editor/core/audio/effects";
import { duckDbAt } from "@/app/lib/editor/core/audio/ducking";

import { duckCurvesOf } from "@/app/ui/editor/audio/ducking";

import { layoutOf, type Item, type ProjectModel } from "@/app/lib/editor/state/project";
import { mediaUrl } from "@/app/lib/editor/state/store";
import {bindEditorMedia,releaseEditorMedia,fallbackEditorProxy,canSeekEditorMedia} from '@/app/ui/editor/preview/mediaElement';
import {previewSrc} from '@/app/ui/editor/preview/compositor/media';
import {syncMediaLoop} from '@/app/ui/editor/preview/mediaLoop';
import { RAMP_S, wanted, startedAt, WARM_S, PREROLL_S, seek, DRIFT_S } from "@/app/ui/editor/audio/mix/elements";
import type { MixState } from "@/app/ui/editor/audio/mix/levels";
import { MixGraph } from "@/app/ui/editor/audio/mix/graph";
import { stagesLatency } from "@/app/ui/editor/audio/mix/nodes";
export { setCurrentMix, currentMix } from "@/app/ui/editor/audio/mix/hook";
export { PREROLL_S, muteOutput } from "@/app/ui/editor/audio/mix/elements";
export { stageShape, buildStages, buildLimiter } from "@/app/ui/editor/audio/mix/nodes";
export type { StageChain, LimiterNodes } from "@/app/ui/editor/audio/mix/nodes";
export type { Level, StereoLevel, Levels, MixState, CaptureResult } from "@/app/ui/editor/audio/mix/levels";

export class Mix extends MixGraph {
  /** Elements the Compositor shows, by item id (registered with attach). */
  private readonly shown = new Map<string, HTMLVideoElement>();
  /** <audio> elements this mix owns, by item id. */
  private readonly own = new Map<string, HTMLAudioElement>();
  /** The element each sounding item is heard through. */
  private readonly active = new Map<string, { el: HTMLMediaElement }>();
  /** Items waiting to hand over to another element, since when (performance.now()). */
  private readonly handover = new Map<string, number>();
  /** Elements ramping to 0 after they stopped sounding, kept until this context time (no click at an end or a cut). */
  private readonly fading = new Map<HTMLMediaElement, number>();
  private last: MixState = { state: "none", items: [] };
  /** The last render's frame and whether it played: a play start or a jump clears noise removal (D262). */
  private was: { frame: number; playing: boolean } | undefined;
  /** The lead (s) each own element was last started with: a change seeks it (D262). */
  private readonly ledBy = new WeakMap<HTMLMediaElement, number>();

  /** Whether an element is still in use: sounding for an item, or ramping down after it. */
  holds(el: HTMLMediaElement | undefined): boolean {
    if (!el) return false;
    for (const a of this.active.values()) if (a.el === el) return true;
    if (!this.ctx) return false;
    const until = this.fading.get(el);
    if (until === undefined) return false;
    if (this.ctx.currentTime < until) return true;
    this.fading.delete(el);
    return false;
  }

  private unroute(el: HTMLMediaElement) {
    this.fading.delete(el);
    const route = this.routes.get(el);
    if (!route) return;
    route.source.disconnect();
    route.gain.disconnect();
    this.routes.delete(el);
  }

  /** The Compositor shows (or warms up) a <video> for a clip with sound: its sound can come through here. */
  attach(itemId: string, el: HTMLVideoElement) {
    this.shown.set(itemId, el);
  }

  /**
   * The Compositor stopped showing a clip's <video> (it ended, or its track was hidden). The mix
   * may still hold it for a moment (see holds) while its sound hands over or ramps down.
   */
  unshow(itemId: string, el: HTMLVideoElement) {
    if (this.shown.get(itemId) === el) this.shown.delete(itemId);
  }

  /** The Compositor is dropping a <video>. */
  detach(itemId: string, el: HTMLVideoElement) {
    this.unshow(itemId, el);
    for (const [id, a] of this.active) if (a.el === el) this.active.delete(id);
    this.unroute(el);
  }

  private ownElement(item: Item, project: ProjectModel): HTMLAudioElement | undefined {
    const media = item.media ? project.media.find((m) => m.id === item.media) : undefined;
    const src = media ? media.kind==='video'?previewSrc(media,'video'):mediaUrl(media.id,media.path) : undefined;
    if (!src || !media) return undefined;
    let el = this.own.get(item.id);
    if (el && (el.dataset.src === src || el.dataset.failedProxy === src)) return el;
    if (el) this.dropOwn(item.id);
    el = document.createElement("audio");
    el.crossOrigin = "anonymous";
    el.preload = "auto";
    el.dataset.item = item.id;
    el.dataset.src = src;
    const audio=el,original=mediaUrl(media.id,media.path);
    audio.addEventListener('error',()=>fallbackEditorProxy(audio,src,original));
    bindEditorMedia(audio,media,src,()=>wanted.get(audio)??audio.currentTime);
    this.own.set(item.id, el);
    return el;
  }

  private dropOwn(id: string) {
    const el = this.own.get(id);
    if (!el) return;
    for (const [item, a] of this.active) if (a.el === el) this.active.delete(item);
    this.unroute(el);
    releaseEditorMedia(el);
    this.own.delete(id);
  }

  /** Ramp an element down; it is kept (holds) until the ramp has landed. */
  private retire(el: HTMLMediaElement) {
    const route = this.routes.get(el);
    if (route && this.ctx) {
      this.ramp(route, 0);
      this.fading.set(el, this.ctx.currentTime + RAMP_S + 0.015);
    }
  }

  /** Playing for real: started, loaded, not seeking, and its time has moved since it started. */
  private ready(el: HTMLMediaElement) {
    if (el.paused || el.seeking || el.readyState < 3) {
      startedAt.delete(el);
      return false;
    }
    const first = startedAt.get(el);
    if (first === undefined) {
      startedAt.set(el, el.currentTime);
      return false;
    }
    return el.currentTime > first + 0.02;
  }

  /** Bring the mix to frame `frame` of `project` (called by the Compositor on every redraw). */
  render(project: ProjectModel, frame: number, playing: boolean) {
    const fps = project.canvas.fps;
    const { starts } = layoutOf(project);
    const g = playing ? this.graph() : this.ctx ? { ctx: this.ctx } : undefined;
    if (playing && g?.ctx.state === "suspended") void g.ctx.resume().catch(() => {});
    // D262: tracks whose noise removal delays their sound are looked at that much ahead.
    const leads = new Map<string, number>();
    for (const t of project.tracks) {
      const lead = stagesLatency(trackStages(t), this.ctx);
      if (lead > 0) leads.set(t.id, lead);
    }
    const leadOf = (trackId: string) => leads.get(trackId) ?? 0;
    let audible = audibleAt(project, starts, frame);
    if (leads.size) {
      audible = audible.filter((a) => !leads.has(a.track.id));
      for (const lead of new Set(leads.values())) audible.push(...audibleAt(project, starts, frame + lead * fps).filter((a) => leads.get(a.track.id) === lead));
    }
    this.clearOnJump(frame, playing, fps);
    // Ducking (D85): each ducked track's curve, added to its items' gain like the export's expression.
    const ducks = duckCurvesOf(project);
    const audibleIds = new Set(audible.map((a) => a.item.id));
    const keepOwn = new Set<string>();
    const items: Array<{ id: string; gain: number; playing: boolean; own: boolean }> = [];
    // Each gain heads for its value one ramp from now, when the ramp lands.
    const ahead = frame + (playing ? RAMP_S * fps : 0);
    const nowMs = performance.now();

    // Items that stopped sounding (ended, deleted, muted out of the list): ramp down.
    for (const [id, a] of [...this.active]) {
      if (audibleIds.has(id)) continue;
      this.active.delete(id);
      this.handover.delete(id);
      this.retire(a.el);
    }

    for (const a of audible) {
      const id = a.item.id;
      const index = a.track.items.indexOf(a.item);
      const start = starts.get(id) ?? 0;
      const lead = leadOf(a.track.id);
      const local = Math.min(ahead + lead * fps, audioSpan(project, a.track, index, start).to - 1e-6) - start;
      const gain = a.muted ? 0 : itemGain(project, a.track, index, local, duckDbAt(ducks.get(a.track.id)?.keys, start + local));
      // A led track's clip sounds through an <audio> of ours, started early (D262); its <video> shows the picture.
      const video = lead > 0 ? undefined : this.shown.get(id);
      const preferred: HTMLMediaElement | undefined = video ?? this.ownElement(a.item, project);
      if (!preferred) continue;
      if (!video) {
        keepOwn.add(id);
        this.drive(preferred, a.item, a.source, playing, lead, syncMediaLoop(preferred, project, a.item, frame + lead * fps - start));
      }
      let cur = this.active.get(id)?.el;
      if (cur && cur !== preferred) {
        // Another element takes over (a track hidden or shown mid-play): the old one keeps
        // sounding until the new one is really playing, then they swap with a ramp. Paused, or
        // after a second of waiting, they swap at once.
        const since = this.handover.get(id) ?? nowMs;
        this.handover.set(id, since);
        if (!playing || this.ready(preferred) || nowMs - since > 1000) {
          this.retire(cur);
          this.handover.delete(id);
          cur = undefined;
        } else {
          // Keep the outgoing one going: an <audio> of ours is still driven; a <video> the
          // Compositor no longer shows plays on by itself for these few frames.
          if (this.own.get(id) === cur) {
            keepOwn.add(id);
            this.drive(cur, a.item, a.source, playing, lead, syncMediaLoop(cur, project, a.item, frame + lead * fps - start));
          }
          const route = this.route(preferred);
          if (route) this.ramp(route, 0);
        }
      }
      if (!cur) {
        cur = preferred;
        this.active.set(id, { el: cur });
        this.fading.delete(cur);
      }
      items.push({ id, gain: playing ? gain : 0, playing: playing && !cur.paused, own: this.own.get(id) === cur });
      if (!this.ctx && !playing) continue;
      const route = this.route(cur);
      if (route) {
        const bus = this.bus(a.track.id);
        if (bus && route.bus !== bus) this.rewire(route, bus);
        if (route.rewiring === undefined) this.ramp(route, playing ? gain : 0);
      }
    }
    this.mixer(project);
    if (this.tap && this.ctx) this.marks.push({ t: this.ctx.currentTime, frame });

    // Audio items (and clips on hidden tracks) about to start: ready at their first frame and
    // silent. Just before they start (while playing) they already run, so a cut overlaps two
    // playing elements instead of waiting for one to start.
    for (const track of project.tracks) {
      track.items.forEach((item, index) => {
        const lead = leadOf(track.id);
        if (keepOwn.has(item.id) || (this.shown.has(item.id) && lead === 0) || !soundMedia(project, item)) return;
        // Clips on a visible track get their <video> from the Compositor, which warms it up (a led track's: ours, D262).
        if (item.type === "clip" && !track.hidden && lead === 0) return;
        const start = starts.get(item.id) ?? 0;
        const from = audioSpan(project, track, index, start).from;
        const at = frame + lead * fps;
        if (from <= at || from - at > WARM_S * fps) return;
        const el = this.ownElement(item, project);
        if (!el) return;
        keepOwn.add(item.id);
        const early = sourceAt(project, item, at - start);
        if (playing && from - at <= PREROLL_S * fps && (item.in ?? 0) + (at - start) * (item.speed ?? 1) >= 0) {
          this.drive(el, item, early, true, lead, syncMediaLoop(el, project, item, at - start));
          const route = this.route(el);
          if (route) this.ramp(route, 0);
        } else {
          if (!el.paused) el.pause();
          seek(el, sourceAt(project, item, from - start));
        }
      });
    }

    // Compositor <video>s held for a handover or a ramp don't get paused by it: pause them here.
    if (!playing) for (const [el] of this.fading) if (!el.paused && !(el instanceof HTMLAudioElement)) el.pause();
    for (const [id, el] of [...this.own]) if (!keepOwn.has(id) && !this.holds(el)) this.dropOwn(id);
    this.last = { state: this.ctx?.state ?? "none", items };
  }

  /** Keep an element this mix owns in step with the clock (the Compositor does this for video). */
  private drive(el: HTMLMediaElement, item: Item, source: number, playing: boolean, lead: number, loopChanged: boolean) {
    const rate = item.speed ?? 1;
    if (el.playbackRate !== rate || el.preservesPitch !== (item.keepPitch !== false)) {
      el.playbackRate = rate;
      el.preservesPitch = item.keepPitch !== false;
    }
    const target = source + 0.0001;
    // A shorter audio stream stays silent until the registered source cycle wraps.
    if (playing && el.ended && Number.isFinite(el.duration) && source >= el.duration) return;
    if(!canSeekEditorMedia(el,target)){
      if(!el.paused)el.pause();
      seek(el,target);
      return;
    }
    // Noise removal came or went mid-play (D262): its new lead is a seek, too small to count as drift.
    const relead = (this.ledBy.get(el) ?? 0) !== lead;
    this.ledBy.set(el, lead);
    if (playing) {
      if (el.paused) {
        seek(el, target);
        el.play().catch(() => {});
      } else if (relead || loopChanged) seek(el, target);
      else if (Math.abs(el.currentTime - target) > DRIFT_S * Math.max(1, rate)) {
        seek(el, target);
      }
    } else {
      if (!el.paused) el.pause();
      seek(el, target);
    }
  }

  /**
   * Clear noise removal's buffers (D262) when the playhead starts, jumps or is scrubbed: what they
   * hold belongs to the old position. Playing on from one frame to the next keeps them.
   */
  private clearOnJump(frame: number, playing: boolean, fps: number) {
    const was = this.was;
    this.was = { frame, playing };
    if (!was) return;
    const started = playing && !was.playing;
    const jumped = playing ? frame < was.frame || frame - was.frame > fps / 2 : frame !== was.frame;
    if (started || jumped) this.resetDenoise();
  }

  /** Silence everything (another project, the preview going away). */
  stop() {
    for (const id of [...this.own.keys()]) this.dropOwn(id);
    this.shown.clear();
    this.active.clear();
    this.handover.clear();
    this.fading.clear();
    this.last = { state: this.ctx?.state ?? "none", items: [] };
  }

  destroy() {
    this.stop();
    for (const id of [...this.buses.keys()]) this.dropBus(id);
    void this.ctx?.close().catch(() => {});
    this.ctx = undefined;
    void this.spare?.close().catch(() => {});
    this.spare = undefined;
  }

  state(): MixState {
    return this.last;
  }
}
