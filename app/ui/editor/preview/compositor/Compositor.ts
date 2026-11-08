// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { heldPast, sourceAt } from "@/app/lib/editor/core/audio/audio";
import { syncMediaLoop } from '../mediaLoop';
import { type Visual, visualsAt } from "@/app/lib/editor/core/timeline/timeline";
import { isDrawnComponent, componentHtml } from "@/app/lib/editor/core/visual/components";
import { graphicInfo, graphicLocal, graphicDocument } from "@/app/lib/editor/core/visual/graphics";
import {graphicWithAssets} from '@/app/lib/editor/core/visual/renderAssets';
import {loadRenderAssets,renderAssetsKey} from '@/app/lib/editor/render-assets';
import { overlaps, irisRadius, maskClipPath, maskRings } from "@/app/lib/editor/core/visual/transitions";
import { SOLID, solidBackground, canvasBackground, textHtml } from "@/app/lib/editor/core/visual/render";
import { PREROLL_S } from "@/app/ui/editor/audio/audio";
import { getPreviewPlayback } from "@/app/ui/editor/preview/playback";
import { type Item, layoutOf } from "@/app/lib/editor/state/project";
import { getState, mediaUrl, graphicUrl } from "@/app/lib/editor/state/store";
import {bindGraphicMedia} from '@/app/ui/editor/preview/graphicMedia';
import { scrubCacheOn } from "@/app/ui/editor/settings/scrub";
import { gpuCompositorOn } from "@/app/ui/editor/settings/gpu";
import { GpuCompositor, plainColour, type GpuPicture } from "@/app/ui/editor/preview/gpu";
import { needsPicturePass, fxPictureSize, fxSkips } from "@/app/lib/editor/core/visual/fx";
import { shaderTime } from "@/app/lib/editor/core/visual/shaders";
import { px, transitionCanvas, setBlur, placeBox, placePicture, drive, seek, irisClip, wanted } from "@/app/ui/editor/preview/compositor/media";

import { CompositorBase } from "@/app/ui/editor/preview/compositor/base";
import {RecordingEffectsRenderer} from '../recordingEffects';
import {RECORDING_MASK} from '@/app/lib/editor/recording-effects';

/** Issue #55: the most upcoming clips warmed up at once (a <video> and a decoder each). */
const WARM_AHEAD = 16;

/**
 * Draws the project into the preview frame, imperatively: it runs on every playhead change
 * (60 times a second while playing) without going through React.
 */
export class Compositor extends CompositorBase {
  protected draw() {
    const project = this.shown();
    if (!project) return;
    if (getState().snapshot?.dir !== this.dir) {
      for (const id of [...this.els.keys()]) this.drop(id);
      this.clearOverlays();
      this.mix.stop();
      return;
    }
    // Issue #12 (D265): device px per canvas px, read before this render writes any style (no forced
    // layout): the GPU compositor gives the canvas one backing pixel per device pixel.
    const gpuScale = gpuCompositorOn() && !this.gpuBroken ? this.screenScale(project.canvas.width) : 1;
    const background = canvasBackground(project);
    if (this.frameEl.style.background !== background) this.frameEl.style.background = background;
    const { frame: playhead, playing: running, rate, selected, hover } = getPreviewPlayback();
    // Issue #118: while paused, the frame skimmed on the timeline is drawn instead of the playhead's.
    const frame = !running && hover !== undefined ? hover : playhead;
    // Shuttling (J/L at another speed, G3): the videos are seeked per frame, like scrubbing, and silent.
    const playing = running && rate === 1;
    const fps = project.canvas.fps;
    const sources = this.sources();
    const { starts } = layoutOf(project);
    const visible = visualsAt(project, starts, frame);
    const mediaKind = (item: Item) => project.media.find((m) => m.id === item.media);
    const clips = visible.filter((v) => v.item.type === "clip" && mediaKind(v.item) && mediaKind(v.item)!.kind !== "audio");
    // Warm up the next clip two seconds ahead so the cut doesn't wait for a decoder: on the main
    // track, and (issue #55) on every other video track too, else an overlay's next clip started
    // from a cold <video> and its box showed nothing (what's under it) until the decoder had a frame.
    // How early a main clip starts: half an overlapping transition into it (a dip doesn't overlap).
    const main = project.tracks.find((t) => t.role === "main");
    const preroll = (it: Item) => (main?.items.includes(it) && it.transitionIn && overlaps(it.transitionIn.module) ? it.transitionIn.duration / 2 : 0);
    const aheads: Array<Item> = [];
    for (const track of project.tracks) {
      if (track.kind !== "video" || track.hidden) continue;
      const next = track.items.find((it) => {
        const start = starts.get(it.id)! - preroll(it);
        return start > frame && start - frame < 2 * fps && mediaKind(it)?.kind === "video";
      });
      if (next) aheads.push(next);
    }
    // The nearest first, at most WARM_AHEAD of them (each is a decoder).
    aheads.sort((a, b) => starts.get(a.id)! - starts.get(b.id)!).splice(WARM_AHEAD);
    const keep = new Set([...clips.map((c) => c.item.id), ...aheads.map((a) => a.id)]);
    for (const [id, el] of [...this.els]) {
      if (keep.has(id)) continue;
      // No longer shown (it ended, its track was hidden): the mix may hold the <video> a moment,
      // out of sight, while its sound ramps down or hands over to an <audio>, so nothing clicks.
      if (el instanceof HTMLVideoElement) {
        this.mix.unshow(id, el);
        if (this.mix.holds(el)) {
          el.style.opacity = "0";
          continue;
        }
      }
      this.drop(id);
    }

    // Everything is stacked in track order (visualsAt's, the order export composites in, D39):
    // media and backgrounds in the media layer, text in the text layer, interleaved by z-index
    // (the layers don't form stacking contexts of their own), so a background on a track above a
    // text track covers it here exactly as it will in the export.
    const drawn = visible.filter(
      (v) =>
        (v.item.type === "clip" && mediaKind(v.item) && mediaKind(v.item)!.kind !== "audio") ||
        v.item.type === "danmaku" ||
        (v.item.type === "component" && v.item.component === SOLID) ||
        isDrawnComponent(v.item) ||
        (v.item.type === "graphic" && typeof v.item.src === "string") ||
        (v.item.type === "text" && typeof v.item.text === "string"),
    );
    const z = new Map(drawn.map((v, i) => [v.item.id, i + 1]));
    // D221: the bottom run of layers the GPU compositor draws (behind its setting); the rest stays DOM.
    const gpuOn = gpuCompositorOn() && !this.gpuBroken;
    // A context the browser took back (too many WebGL contexts on the page) is made again.
    if (gpuOn && this.gpu?.lost) {
      this.gpu.dispose();
      this.gpu = undefined;
    }
    // The DOM path's spare effects renderers hold a WebGL context each: let them go while the GPU draws.
    if (gpuOn) for (const r of this.fxPool.splice(0)) r.dispose();
    if (gpuOn && !this.gpu) {
      try {
        this.gpu = new GpuCompositor(this.mediaLayer);
        this.gpu.onRestored = () => this.render();
      } catch {
        this.gpuBroken = true;
      }
    }
    const onGpu = new Set<string>();
    this.onGpu = onGpu;
    if (gpuOn && this.gpu && !this.gpu.lost) {
      // D265: every layer it can draw, in runs between the layers it can't (each run its own canvas).
      for (const v of drawn) if (this.gpuCan(v, project)) onGpu.add(v.item.id);
    }
    const gpuPictures = new Map<string, GpuPicture>();
    const comments=drawn.filter(v=>v.item.type==='danmaku'&&v.item.danmaku?.settings.enabled);
    for(const [id,layer] of this.danmakuLayers){if(!comments.some(v=>v.item.id===id)){layer.renderer.dispose();layer.wrap.remove();this.danmakuLayers.delete(id)}}
    for(const v of comments){
      if(!v.box||!v.item.danmaku)continue;
      let layer=this.danmakuLayers.get(v.item.id);
      if(!layer){const renderer=new RecordingEffectsRenderer(()=>this.render()),blank=document.createElement('canvas'),wrap=document.createElement('div');
        wrap.className='pv-box';wrap.dataset.item=v.item.id;wrap.append(renderer.canvas);this.mediaLayer.append(wrap);
        layer={renderer,blank,wrap};this.danmakuLayers.set(v.item.id,layer)}
      if(layer.blank.width!==v.item.danmaku.width)layer.blank.width=v.item.danmaku.width;
      if(layer.blank.height!==v.item.danmaku.height)layer.blank.height=v.item.danmaku.height;
      layer.renderer.draw(project,sources,v.item,layer.blank,v.source,(frame-(starts.get(v.item.id)??0))/project.canvas.fps);
      placeBox(layer.wrap,layer.renderer.canvas,v.box,project.canvas,v.wipe,v.iris,v.mask,v.blur);
      layer.wrap.style.opacity=String(v.opacity);layer.wrap.style.zIndex=String(z.get(v.item.id));
    }
    const layered = drawn.filter((v) => v.item.type !== "text");
    const solids = layered.filter((v) => v.item.type === "component" && v.item.component === SOLID);
    for (const [id, el] of [...this.solids]) {
      if (solids.some((v) => v.item.id === id)) continue;
      el.remove();
      this.solids.delete(id);
    }
    for (const v of solids) {
      let el = this.solids.get(v.item.id);
      if (!el) {
        el = document.createElement("div");
        el.className = "pv-solid";
        // A dip to black's black layer isn't an item: clicking it selects nothing.
        if (v.item.id.startsWith("dip:")) el.dataset.dip = v.item.id;
        else el.dataset.item = v.item.id;
        this.solids.set(v.item.id, el);
        this.mediaLayer.append(el);
      }
      const bg = solidBackground(v.item, project);
      if (el.dataset.bg !== bg) {
        el.dataset.bg = bg;
        el.style.background = bg;
      }
      // On the GPU its div stays transparent: an inline opacity would beat styles.css's
      // `.pv-solid[data-gpu] { opacity: 0 }`, and a dip would be drawn twice (twice as dark mid-way).
      el.style.opacity = onGpu.has(v.item.id) ? "0" : String(v.opacity);
      el.style.zIndex = String(z.get(v.item.id));
      transitionCanvas(el, v, project.canvas);
      if (onGpu.has(v.item.id)) el.dataset.gpu = "";
      else delete el.dataset.gpu;
    }

    // Components (D76): core's HTML at this frame, remade only when it changes (a terminal typing,
    // a fade); media references go through the server's /media/file route.
    const target = {
      ...project,
      mediaUrl: (id: string) => {
        const m = project.media.find((x) => x.id === id);
        return m && m.kind === "image" ? mediaUrl(m.id, m.path) : undefined;
      },
    };
    const comps = drawn.filter((v) => isDrawnComponent(v.item));
    for (const [id, c] of [...this.comps]) {
      if (comps.some((v) => v.item.id === id)) continue;
      c.el.remove();
      this.comps.delete(id);
    }
    const whole = Math.floor(frame);
    for (const v of comps) {
      // A push or zoom transition is in its box (D79), drawn into the HTML.
      // Only a moved one gets visualsAt's box (taken at the fractional playhead): otherwise its own at the
      // whole frame, so a keyed component isn't rebuilt on every animation frame while playing.
      const html = componentHtml(v.item, target, whole - starts.get(v.item.id)!, { opacity: v.opacity, motion: v.motion, fps, ...(v.box && (v.shift !== undefined || v.shiftY !== undefined || v.zoom !== undefined) ? { box: v.box } : {}) });
      const key = `${html}|${v.wipe ?? ""}|${v.iris ?? ""}|${v.mask ? `${v.mask.shape}:${v.mask.p}` : ""}`;
      let c = this.comps.get(v.item.id);
      if (!c || c.key !== key) {
        const holder = document.createElement("div");
        holder.innerHTML = html;
        const el = holder.firstElementChild as HTMLElement | null;
        if (!el) continue;
        // A wipe reveals the canvas from the left (as the export crops it): its edge in the element's
        // own pixels, through the scale about the box's centre and the move's offset; combined with
        // the element's own reveal (an in/out wipe) as the larger of the two.
        if (v.wipe !== undefined && v.box) {
          const b = v.box;
          const centre = b.x + b.w / 2 + (v.motion.dx ?? 0);
          const edge = b.w / 2 + (project.canvas.width * v.wipe - centre) / b.scale;
          const reveal = v.motion.reveal;
          const right = Math.min(b.w, Math.max(0, b.w - edge, reveal !== undefined && reveal < 1 ? (1 - reveal) * b.w : 0));
          el.style.clipPath = reveal !== undefined && reveal < 1 ? `inset(${px(-0.2 * b.h)} ${px(right)} ${px(-0.2 * b.h)} ${px(-0.05 * b.w)})` : `inset(0 ${px(right)} 0 0)`;
        } else if (v.iris !== undefined && v.box) {
          // An iris (D79): the canvas's circle in the element's own pixels, through its scale about the box's centre and the move's offset.
          const b = v.box;
          const ox = b.x + b.w / 2 + (v.motion.dx ?? 0);
          const oy = b.y + b.h / 2 + (v.motion.dy ?? 0);
          const r = irisRadius(v.iris, project.canvas) / b.scale;
          el.style.clipPath = `circle(${px(r)} at ${px(b.w / 2 + (project.canvas.width / 2 - ox) / b.scale)} ${px(b.h / 2 + (project.canvas.height / 2 - oy) / b.scale)})`;
        } else if (v.mask && v.box) {
          // A reveal shape (D133): the canvas's outline in the element's own pixels, as for the iris.
          const b = v.box;
          const ox = b.x + b.w / 2 + (v.motion.dx ?? 0);
          const oy = b.y + b.h / 2 + (v.motion.dy ?? 0);
          el.style.clipPath = maskClipPath(v.mask.shape, v.mask.p, project.canvas, (x, y) => [b.w / 2 + (x - ox) / b.scale, b.h / 2 + (y - oy) / b.scale]);
        }
        if (c) c.el.replaceWith(el);
        else this.mediaLayer.append(el);
        c = { el, key };
        this.comps.set(v.item.id, c);
      }
      const zi = String(z.get(v.item.id));
      if (c.el.style.zIndex !== zi) c.el.style.zIndex = zi;
      setBlur(c.el, v.blur && v.blur / (v.box?.scale ?? 1));
      c.el.classList.toggle("sel-clip", v.item.id === selected);
    }
    // Graphics (D78): the server's document in a sandboxed iframe; render(t) by postMessage. One
    // starting within two seconds is loaded ahead, out of sight at its first frame, so it doesn't
    // start blank (like the next clip's warm-up).
    type Shown = Pick<Visual, "wipe" | "iris" | "mask" | "shift" | "shiftY" | "zoom" | "blur"> & { item: Item; opacity: number; visible: boolean };
    const graphics: Array<Shown> = drawn
      .filter((v) => v.item.type === "graphic" && typeof v.item.src === "string")
      .map((v) => ({ item: v.item, opacity: v.opacity, ...(v.wipe !== undefined ? { wipe: v.wipe } : {}), ...(v.iris !== undefined ? { iris: v.iris } : {}), ...(v.mask ? { mask: v.mask } : {}), ...(v.shift !== undefined ? { shift: v.shift } : {}), ...(v.shiftY !== undefined ? { shiftY: v.shiftY } : {}), ...(v.zoom !== undefined ? { zoom: v.zoom } : {}), ...(v.blur !== undefined ? { blur: v.blur } : {}), visible: true }));
    for (const track of project.tracks) {
      if (track.hidden || track.kind !== "video") continue;
      for (const it of track.items) {
        const s = starts.get(it.id)!;
        if (it.type === "graphic" && typeof it.src === "string" && s > frame && s - frame <= 2 * fps && !graphics.some((g) => g.item.id === it.id)) graphics.push({ item: it, opacity: 0, visible: false });
      }
    }
    for (const [id, g] of [...this.graphics]) {
      if (graphics.some((v) => v.item.id === id && graphicUrl(v.item.src!,project) === g.src)) continue;
      g.releaseMedia();
      g.wrap.remove();
      this.graphics.delete(id);
    }
    for (const v of graphics) {
      const src = graphicUrl(v.item.src!,project);
      if (!src) continue;
      let g = this.graphics.get(v.item.id);
      if (!g) {
        // Asked first: a 404 page in the iframe would draw "Not found" over the canvas, where the
        // export draws nothing. A missing file is asked for again 3 s later.
        const check = this.graphicCheck.get(src);
        if (check !== "ok") {
          if (check === "pending" || (typeof check === "number" && Date.now() < check)) continue;
          this.graphicCheck.set(src, "pending");
          const assets=loadRenderAssets(this.dir,renderAssetsKey(this.dir,getState().brand,getState().projectFonts));
          const signal=AbortSignal.any([this.lifetime.signal,AbortSignal.timeout(30000)]);
          void Promise.all([fetch(src,{credentials:'include',signal}).then(r=>r.ok?r.json():undefined),assets])
            .then(([value,resources]) => {if(typeof value?.html!=='string'||!this.graphicCheck.has(src))return false;this.graphicDocuments.set(src,graphicDocument(graphicWithAssets(value.html,resources),project.canvas));return true}).catch(()=>false)
            .then((ok) => {
              if(!this.graphicCheck.has(src))return;
              this.graphicCheck.set(src, ok ? "ok" : Date.now() + 3000);
              if (!ok) setTimeout(() => this.render(), 3100);
              this.render();
            });
          continue;
        }
        const wrap = document.createElement("div");
        wrap.className = "pv-graphic";
        wrap.dataset.item = v.item.id;
        const iframe = document.createElement("iframe");
        iframe.setAttribute("sandbox", "allow-scripts");
        iframe.setAttribute("tabindex", "-1");
        iframe.title = v.item.name ?? v.item.id;
        iframe.style.width = `${project.canvas.width}px`;
        iframe.style.height = `${project.canvas.height}px`;
        const entry = { wrap, frame: iframe, src, loaded: false, sent: "", releaseMedia: bindGraphicMedia(iframe,this.dir,project) };
        iframe.addEventListener("load", () => {
          if(this.graphics.get(v.item.id)!==entry)return;
          entry.loaded = true;
          entry.sent = "";
          this.render();
        });
        iframe.srcdoc = this.graphicDocuments.get(src)!;
        wrap.append(iframe);
        this.mediaLayer.append(wrap);
        g = entry;
        this.graphics.set(v.item.id, g);
      }
      g.wrap.style.opacity = String(v.opacity);
      g.wrap.style.zIndex = v.visible ? String(z.get(v.item.id)) : "0";
      g.wrap.style.visibility = v.visible ? "" : "hidden";
      transitionCanvas(g.wrap, v, project.canvas);
      if (g.loaded) {
        const local = graphicLocal(v.item, whole - starts.get(v.item.id)!);
        const info = graphicInfo(v.item, project.canvas, local);
        // Anything render(t, info) is given: a new length or canvas draws again too. And whether it
        // plays: a graphic's footage (biliup.frame) is its nearest decoded frame while playing, the
        // exact one when stopped, so the frame playback stops on is drawn again exactly.
        const sent = JSON.stringify([local, info, running]);
        if (g.sent !== sent) {
          g.sent = sent;
          g.frame.contentWindow?.postMessage({ biliup: "render", seq: ++this.graphicSeq, t: local / fps, info, playing: running }, "*");
        }
      }
    }

    if (!scrubCacheOn() && this.scrub.lru.size) this.scrub.clear();
    this.scrubbed.clear();
    this.scrubAimed.clear();
    clips.forEach((c) => {
      const media = mediaKind(c.item)!;
      const el = this.elementFor(c.item, media.kind === "image" ? "image" : "video");
      if (!el) return;
      const wrap = this.boxes.get(c.item.id)!;
      wrap.classList.toggle("sel-clip", c.item.id === selected);
      // D221: paused or shuttling, a video's picture may be its cached frame (a canvas over it).
      const scrubbed = el instanceof HTMLVideoElement ? this.scrubPicture(c.item.id, el, c.source + 0.0001, playing, running && rate !== 1 ? rate : 0) : { pic: el, seek: true };
      let pic = scrubbed.pic;
      if(c.item.effects?.some(effect=>!effect.bypass&&effect.module===RECORDING_MASK)){
        let processor=this.recordingEffects.get(c.item.id);
        if(!processor){processor=new RecordingEffectsRenderer(()=>this.render());this.recordingEffects.set(c.item.id,processor);wrap.append(processor.canvas)}
        pic=processor.draw(project,sources,c.item,pic,c.source,(frame-(starts.get(c.item.id)??0))/project.canvas.fps);
      }else{this.recordingEffects.get(c.item.id)?.dispose();this.recordingEffects.delete(c.item.id)}
      // The opacity stays on the media element (the wrapper only places, clips and rounds it).
      el.style.opacity = String(c.opacity);
      wrap.style.zIndex = String(z.get(c.item.id));
      if (c.box) placeBox(wrap, el, c.box, project.canvas, c.wipe, c.iris, c.mask, c.blur);
      if (pic !== el) {
        if (c.box) placePicture(pic, c.box);
        pic.style.opacity = el.style.opacity;
        el.style.visibility = "hidden";
      } else if (el instanceof HTMLVideoElement && !wrap.querySelector("[data-missing]")) el.style.visibility = "";
      if (onGpu.has(c.item.id) && c.box) {
        // D221: drawn by the GPU compositor (its effects too); the DOM elements stay, transparent.
        wrap.dataset.gpu = "";
        const use = fxSkips(media) ? undefined : this.shaderUseOf(c.item);
        this.dropFx(c.item.id, !!use);
        const fx = this.effectsOf(c.item);
        // D265: a shader (D213) still runs on its own canvas (out of sight in the transparent box), the picture the compositor reads.
        const shaded = use ? this.drawShader(c.item, pic, wrap, use, fxPictureSize(c.box, media.probe), shaderTime(frame, starts.get(c.item.id)!, fps)) : undefined;
        gpuPictures.set(c.item.id, {
          id: c.item.id,
          source: shaded ?? pic,
          stretch: !!shaded,
          box: c.box,
          opacity: c.opacity,
          wipe: c.wipe,
          iris: c.iris !== undefined ? { x: project.canvas.width / 2, y: project.canvas.height / 2, r: irisRadius(c.iris, project.canvas) } : undefined,
          fx: fx && needsPicturePass(fx) && !this.fxBroken ? { fx, size: fxPictureSize(c.box, media.probe), t: (Math.floor(frame) - starts.get(c.item.id)!) / fps } : undefined,
          gamut: !shaded && pic instanceof HTMLVideoElement && this.gamutVideo(pic),
          mask: c.mask ? maskRings(c.mask.shape, c.mask.p, project.canvas.width, project.canvas.height) : undefined,
          blur: c.blur,
          pictureBlur: fx?.blur,
        });
      } else {
        delete wrap.dataset.gpu;
        if (c.box) this.drawFx(c.item, pic, wrap, c.box, c.opacity, frame, fps, starts.get(c.item.id)!);
      }
      if (!(el instanceof HTMLVideoElement)) return;
      const loopChanged = syncMediaLoop(el, project, c.item, frame - starts.get(c.item.id)!);
      if (media.probe?.hasAudio) this.mix.attach(c.item.id, el);
      // A held clip (D94) past its media's end sits paused on the last frame.
      if (scrubbed.seek) drive(el, c.source, playing && !heldPast(project, c.item, frame - starts.get(c.item.id)!), c.item.speed ?? 1, c.item.keepPitch !== false, loopChanged);
      else if (!el.paused) el.pause();
    });
    this.warmBudget = 1;
    for (const id of this.fxWarmed) if (!aheads.some((a) => a.id === id)) this.fxWarmed.delete(id);
    for (const ahead of aheads) {
      if (clips.some((c) => c.item.id === ahead.id)) continue;
      if (!gpuOn) this.warmFx(ahead);
      const el = this.elementFor(ahead, "video");
      if (el instanceof HTMLVideoElement) {
        const wrap = this.boxes.get(ahead.id)!;
        el.style.opacity = "0";
        wrap.style.zIndex = "0";
        if (mediaKind(ahead)?.probe?.hasAudio) this.mix.attach(ahead.id, el);
        const from = starts.get(ahead.id)! - preroll(ahead);
        const speed = ahead.speed ?? 1;
        // Just before it starts (while playing) it already runs, out of sight and silent, so the
        // cut doesn't wait for the decoder to start (picture and sound).
        const early = (ahead.in ?? 0) + (frame - starts.get(ahead.id)!) * speed;
        const earlySource = sourceAt(project, ahead, frame - starts.get(ahead.id)!);
        const loopChanged = syncMediaLoop(el, project, ahead, frame - starts.get(ahead.id)!);
        // Issue #12 (D265): starting to run is never a seek back. Paused, it waits at its first frame
        // (stepping onto the cut shows it at once); made while playing, it waits where its run starts
        // (PREROLL_S before that), so the run just starts. One waiting at its first frame when playback
        // starts stays there and starts running at its cut (its decoder already holds that frame). Before,
        // the run began with a seek PREROLL_S back: a second decoder start on a long-GOP file right
        // before the cut, which stalled the clip playing on screen into a drift seek.
        const first = sourceAt(project, ahead, -preroll(ahead)) + 0.0001;
        const parked = el.paused && !el.seeking && el.readyState >= 2;
        if (playing && from - frame <= PREROLL_S * fps && early >= 0) {
          const lead = el.currentTime - earlySource;
          if (!loopChanged && parked && lead > -0.1 && lead <= 0.5 / fps) {
            if (el.playbackRate !== speed || el.preservesPitch !== (ahead.keepPitch !== false)) {
              el.playbackRate = speed;
              el.preservesPitch = ahead.keepPitch !== false;
            }
            el.play().catch(() => {});
          } else if (loopChanged || !(parked && lead > 0 && el.currentTime <= first + 0.05)) drive(el, earlySource, true, speed, ahead.keepPitch !== false, loopChanged);
        } else {
          if (!el.paused) el.pause();
          if (!playing) seek(el, first);
          else if (!wanted.has(el)) seek(el, Math.max(0, sourceAt(project, ahead, -(preroll(ahead) + PREROLL_S * fps))) + 0.0001);
        }
      }
    }

    // Text: each one rebuilt only when what it shows changes, not on every frame. Issue #55: per
    // text, so one fading or typing text no longer rebuilds (and lays out) all the others with it.
    const shownTexts = visible
      .filter((v) => v.item.type === "text" && typeof v.item.text === "string")
      .map(({ item, opacity, motion }) => ({ id: item.id, html: textHtml(item, project, { opacity, motion, selected: item.id === selected }) }));
    this.syncTexts(shownTexts);
    const texts = new Map(visible.filter((v) => v.item.type === "text").map((v) => [v.item.id, v]));
    for (const child of this.textLayer.children) {
      const el = child as HTMLElement;
      const zi = String(z.get(el.dataset.item ?? "") ?? 0);
      if (el.style.zIndex !== zi) el.style.zIndex = zi;
      // A text on the main track through an iris, push or zoom (D79): as the canvas, like the
      // export's raster. Its own transform-origin is its centre, so a zoom about the canvas's
      // centre is that scale plus a shift; the iris's circle is in its own pixels.
      const v = texts.get(el.dataset.item ?? "");
      const zoom = v?.zoom ?? 1;
      const moved = !!v && (!!v.shift || !!v.shiftY || zoom !== 1);
      // Layout is read only while one is moved (not on every frame of every text).
      const ox = moved ? el.offsetLeft + el.offsetWidth / 2 : 0;
      const oy = moved ? el.offsetTop + el.offsetHeight / 2 : 0;
      const translate = moved ? `${px((v!.shift ?? 0) + (1 - zoom) * (project.canvas.width / 2 - ox))} ${px((v!.shiftY ?? 0) + (1 - zoom) * (project.canvas.height / 2 - oy))}` : "";
      if (el.style.translate !== translate) el.style.translate = translate;
      const scale = zoom !== 1 ? String(zoom) : "";
      if (el.style.scale !== scale) el.style.scale = scale;
      // The text's own clip-path (an in/out wipe move) is kept aside while an iris replaces it.
      setBlur(el, v?.blur);
      if (v?.iris !== undefined || v?.mask) {
        if (el.dataset.clip === undefined) el.dataset.clip = el.style.clipPath;
        // Its own pixels: the layout box moved by the in/out move's translate (motion), as for components.
        const left = el.offsetLeft + (v.motion.dx ?? 0);
        const top = el.offsetTop + (v.motion.dy ?? 0);
        el.style.clipPath = v.iris !== undefined ? irisClip(v.iris, project.canvas, left, top) : maskClipPath(v.mask!.shape, v.mask!.p, project.canvas, (x, y) => [x - left, y - top]);
      } else if (el.dataset.clip !== undefined) {
        el.style.clipPath = el.dataset.clip;
        delete el.dataset.clip;
      }
    }
    this.scrub.sweep();
    if (this.gpu && !this.gpu.lost) {
      if (onGpu.size) {
        const place = { w: project.canvas.width, h: project.canvas.height, k: gpuScale };
        // Issue #82 (D241): a layer with nothing to draw yet (the next clip's <video> still seeking at
        // a cut) keeps the last frame up a moment instead of a transparent hole over the black frame.
        if (!this.gpu.holds(gpuPictures.values(), place)) {
          this.gpu.begin(place);
          let last: number | undefined;
          for (const v of drawn) {
            if (!onGpu.has(v.item.id)) {
              if (last !== undefined) this.gpu.flush(last);
              last = undefined;
              continue;
            }
            const p = gpuPictures.get(v.item.id);
            if (p) this.gpu.drawPicture(p);
            else if (v.item.type === "component") this.gpu.drawSolid({ id: v.item.id, colour: plainColour(solidBackground(v.item, project))!, opacity: v.opacity });
            last = z.get(v.item.id)!;
          }
          if (last !== undefined) this.gpu.flush(last);
          // Issue #12 (D265): the next clips' pictures, so a cut has one to hold while its <video> seeks.
          for (const ahead of aheads) {
            const el = this.els.get(ahead.id);
            if (el instanceof HTMLVideoElement && !clips.some((c) => c.item.id === ahead.id)) this.gpu.prewarm(ahead.id, el);
          }
          this.gpu.end();
        } else this.frameEl.dataset.gpuHeld = String(Number(this.frameEl.dataset.gpuHeld ?? 0) + 1);
      } else this.gpu.clear();
      this.frameEl.dataset.gpuLayers = String(onGpu.size);
    }
    // A clip that isn't drawn this time (warming up as the next clip, fading out of the mix): no canvas over it.
    for (const [id, view] of this.scrubViews) {
      if (this.scrubbed.has(id) || !view.shown) continue;
      view.shown = false;
      view.canvas.style.visibility = "hidden";
    }
    // Sound after picture: the mix uses the <video> elements made above.
    this.mix.render(project, frame, playing);
    this.frameEl.dataset.frame = String(Math.floor(frame));
    if (!running && hover !== undefined) this.frameEl.dataset.hover = "true";
    else delete this.frameEl.dataset.hover;
    this.frameEl.dataset.playing = playing ? "true" : "false";
  }

  /**
   * Issue #55: the text layer as `texts` (core's textHtml each, in stacking order): a text whose HTML
   * is unchanged keeps its element, a changed one gets a new one in its place, gone ones leave. The
   * text being edited in place is hidden while the editor sits over it.
   */
  protected syncTexts(texts: ReadonlyArray<{ readonly id: string; readonly html: string }>) {
    const layer = this.textLayer;
    const ids = new Set(texts.map((t) => t.id));
    for (const [id, t] of [...this.texts]) {
      if (ids.has(id) && t.el.parentElement === layer) continue;
      t.el.remove();
      this.texts.delete(id);
    }
    // Anything else in the layer goes.
    const kept = new Set<Element>([...this.texts.values()].map((t) => t.el));
    for (const child of [...layer.children]) if (!kept.has(child)) child.remove();
    let at: Element | null = layer.firstElementChild;
    for (const { id, html } of texts) {
      let t = this.texts.get(id);
      if (!t || t.html !== html) {
        const holder = document.createElement("div");
        holder.innerHTML = html;
        const el = holder.firstElementChild as HTMLElement | null;
        if (!el) continue;
        if (t) {
          if (at === t.el) at = el;
          t.el.replaceWith(el);
        }
        t = { el, html };
        this.texts.set(id, t);
      }
      if (t.el !== at) layer.insertBefore(t.el, at);
      else at = at.nextElementSibling;
      const hide = id === this.editing ? "hidden" : "";
      if (t.el.style.visibility !== hide) t.el.style.visibility = hide;
    }
  }

  /** Device px per canvas px (the scaled canvas layer's width on screen, not the frame's rounded clientWidth). */
  protected screenScale(width: number) {
    const dpr = window.devicePixelRatio || 1;
    const r = (this.mediaLayer.parentElement ?? this.mediaLayer).getBoundingClientRect();
    return r.width > 0 ? (r.width * dpr) / width : (this.frameEl.clientWidth / width) * dpr;
  }

  /** Where the top fully visible video really is, so the clock can follow a late decoder. */
  protected follow(f: number) {
    const project = this.shown();
    if (!project) return undefined;
    const { starts } = layoutOf(project);
    const top = visualsAt(project, starts, f)
      .filter((v) => v.item.type === "clip" && v.opacity === 1)
      .at(-1);
    if (!top) return undefined;
    const el = this.els.get(top.item.id);
    if (!(el instanceof HTMLVideoElement) || el.paused || el.seeking || el.readyState < 3) return undefined;
    const start=starts.get(top.item.id)!;
    if(heldPast(project,top.item,f-start))return undefined;
    // A looping source time belongs to the cycle nearest this timeline frame. Mapping it
    // straight back from the clip's In point would rewind the entire project on each loop.
    let drift=(el.currentTime-top.source)*project.canvas.fps;
    const length=top.item.loop?project.media.find(media=>media.id===top.item.media)?.probe?.frames:undefined;
    if(length&&length>0)drift-=Math.round(drift/length)*length;
    return f+drift/(top.item.speed??1);
  }
}
