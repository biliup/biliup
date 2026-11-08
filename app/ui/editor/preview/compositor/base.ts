// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import type { ClipBox } from "@/app/lib/editor/core/visual/box";
import {getState,mediaUrl} from '@/app/lib/editor/state/store';
import {fallbackProxy} from './media';
import {bindEditorMedia,releaseEditorMedia} from '../mediaElement';
import {editorText} from '@/app/lib/editor/i18n';
import {RecordingEffectsRenderer} from '../recordingEffects';
import { type Visual } from "@/app/lib/editor/core/timeline/timeline";
import { SOLID, solidBackground } from "@/app/lib/editor/core/visual/render";
import { Mix, setCurrentMix } from "@/app/ui/editor/audio/audio";
import { playback, getPlayback } from "@/app/ui/editor/preview/playback";
import { type ProjectModel, type Item } from "@/app/lib/editor/state/project";
import { scrubCacheOn } from "@/app/ui/editor/settings/scrub";
import { ScrubCache, convertsPrimaries } from "@/app/ui/editor/preview/scrub";
import { GpuCompositor, plainColour } from "@/app/ui/editor/preview/gpu";
import { type ClipFx, clipFx, fxSkips, needsPicturePass, fxPictureSize, hasOverlay, fxOverlayInner, visibleRect } from "@/app/lib/editor/core/visual/fx";
import { FxRenderer, frameKeyOf, presentedFrames } from "@/app/ui/editor/preview/fx";
import { type ShaderUse, clipShader, shaderTime } from "@/app/lib/editor/core/visual/shaders";
import { ShaderCanvas, whenRunner, shaderProblem } from "@/app/ui/editor/preview/shader";
import { type MediaEl, type ScrubView, type PictureEl, pictureReady, FX_POOL, FX_CONTEXTS, setStyle, placeContent, px, previewSrc, wanted, reachable, SCRUB_SETTLE_MS, renderTimes, previewStats, playSeek } from "@/app/ui/editor/preview/compositor/media";

/**
 * Issue #55: a clip's effects drawn by the shared renderer (past FX_CONTEXTS), copied into its own
 * 2D canvas; `sig` is what it holds (the same frame again isn't drawn twice).
 */
interface FxCopy {
  readonly canvas: HTMLCanvasElement;
  readonly ctx: CanvasRenderingContext2D;
  sig: string;
  drawn: boolean;
}

/** The compositor's state and per-layer helpers; Compositor (Compositor.ts) draws a frame with them. */
export abstract class CompositorBase {
  protected readonly lifetime = new AbortController();
  protected danmakuLayers = new Map<string,{renderer:RecordingEffectsRenderer;blank:HTMLCanvasElement;wrap:HTMLDivElement}>();
  protected recordingEffects = new Map<string,RecordingEffectsRenderer>();
  protected project: ProjectModel | undefined;
  protected readonly els = new Map<string, MediaEl>();
  // Each media element sits in a box wrapper (D75), keyed like `els`.
  protected readonly boxes = new Map<string, HTMLDivElement>();
  protected readonly solids = new Map<string, HTMLDivElement>();
  // Components (D76): each one's element and the HTML (plus wipe) it was made from.
  protected readonly comps = new Map<string, { el: HTMLElement; key: string }>();
  // Graphics (D78): a wrapper and its sandboxed iframe per item; `sent` is the last render asked for.
  protected readonly graphics = new Map<string, { wrap: HTMLDivElement; frame: HTMLIFrameElement; src: string; loaded: boolean; sent: string; releaseMedia: () => void }>();
  // Whether the server has each graphic's document: "ok", being asked, or when to ask again (D78).
  protected readonly graphicCheck = new Map<string, "ok" | "pending" | number>();
  protected readonly graphicDocuments = new Map<string,string>();
  // Clip looks and effects (D136): each clip's compiled effects (by its effects' JSON), its WebGL
  // canvas over the hidden picture element, and its overlay (vignette, grain, bars).
  protected readonly fxOf = new Map<string, { key: string; fx: ClipFx | undefined }>();
  protected readonly fxCanvas = new Map<string, FxRenderer>();
  protected readonly fxOverlay = new Map<string, { el: HTMLDivElement; html: string }>();
  protected readonly fxPool: Array<FxRenderer> = [];
  // Issue #55: clips past FX_CONTEXTS: one renderer for all of them, a 2D canvas each.
  protected readonly fxCopies = new Map<string, FxCopy>();
  protected fxShared: FxRenderer | undefined;
  // Issue #55: clips about to start whose effects programs have been compiled ahead.
  protected readonly fxWarmed = new Set<string>();
  // How many clips' effects may still be warmed up in this render (one a frame: a compile blocks it).
  protected warmBudget = 0;
  protected fxBroken = false;
  // Shader effects (D213): a WebGPU canvas per clip that has one, and the "not drawn here" badges.
  protected readonly shaderCanvas = new Map<string, ShaderCanvas>();
  protected readonly shaderOf = new Map<string, { key: string; use: ShaderUse | undefined }>();
  protected readonly shaderBadge = new Map<string, HTMLDivElement>();
  protected readonly offRunner: () => void;
  protected graphicSeq = 0;
  // Issue #55: each text's element and the HTML it was made from.
  protected readonly texts = new Map<string, { el: HTMLElement; html: string }>();
  // The text being edited in place: hidden while the editor sits over it.
  protected editing: string | undefined;
  protected readonly unsubscribe: () => void;
  protected readonly frameEl: HTMLDivElement;
  protected readonly mediaLayer: HTMLDivElement;
  protected readonly textLayer: HTMLDivElement;
  // The project folder this preview draws. Another folder open means this preview is on its way
  // out: it draws nothing more (its media ids could name the new project's files).
  protected readonly dir: string;
  // The playback mix (D54): clips' sound comes from the <video> elements made here.
  protected readonly mix = new Mix();
  // D221: exact frames for scrubbing, decoded ahead; a canvas per clip that shows one.
  protected readonly scrub = new ScrubCache(() => this.render());
  protected readonly scrubViews = new Map<string, ScrubView>();
  protected settleTimer = 0;
  // The clips whose scrub canvas this render placed (any other one is hidden).
  protected readonly scrubbed = new Set<string>();
  // Issue #55: the files whose decode this render has aimed (one clip each; see scrubPicture).
  protected readonly scrubAimed = new Set<object>();
  // D221: the GPU compositor (behind its setting), made on first use; broken when WebGL2 isn't there.
  protected gpu: GpuCompositor | undefined;
  protected gpuBroken = false;
  // The layers the GPU compositor drew last (a new frame of theirs needs a render, as for effects).
  protected onGpu = new Set<string>();

  constructor(dir: string, frameEl: HTMLDivElement, mediaLayer: HTMLDivElement, textLayer: HTMLDivElement) {
    this.dir = dir;
    this.frameEl = frameEl;
    this.mediaLayer = mediaLayer;
    this.textLayer = textLayer;
    this.unsubscribe = playback.subscribe(() => this.render());
    // Shader effects (D213) draw once the WebGPU runner is known (or the badge says why not).
    this.offRunner = whenRunner(() => this.render());
    playback.setFollower((f) => this.follow(f));
    setCurrentMix(this.mix);
    // Issue #55: the mix's AudioContext made while idle, not in the first frame of playback.
    this.prepareIdle = requestIdleCallback(() => this.mix.prepare(), { timeout: 3000 });
    previewStats.compositors++;
  }
  private prepareIdle: number;

  destroy() {
    this.lifetime.abort();
    this.unsubscribe();
    this.offRunner();
    for(const layer of this.danmakuLayers.values()){layer.renderer.dispose();layer.wrap.remove()}
    this.danmakuLayers.clear();
    for (const id of [...this.shaderCanvas.keys()]) this.dropShader(id);
    playback.setFollower(undefined);
    for (const id of [...this.els.keys()]) this.drop(id);
    for (const r of this.fxPool.splice(0)) r.dispose();
    this.fxShared?.dispose();
    this.fxShared = undefined;
    for (const el of this.solids.values()) el.remove();
    this.solids.clear();
    this.clearOverlays();
    this.graphicCheck.clear();
    this.graphicDocuments.clear();
    cancelIdleCallback(this.prepareIdle);
    this.mix.destroy();
    setCurrentMix(undefined);
    clearTimeout(this.settleTimer);
    this.scrub.destroy();
    this.gpu?.dispose();
    this.gpu = undefined;
  }

  /** Components and graphics off the canvas. */
  protected clearOverlays() {
    for (const c of this.comps.values()) c.el.remove();
    this.comps.clear();
    for (const g of this.graphics.values()) { g.releaseMedia(); g.wrap.remove(); }
    this.graphics.clear();
  }

  setEditing(id: string | undefined) {
    this.editing = id;
    this.render();
  }

  /** The project on screen: the open one, or a local draft of an edit in progress on top of it. */
  shown(): ProjectModel | undefined {
    const draft = getPlayback().draft;
    return draft && draft.base === this.project ? draft.project : this.project;
  }

  protected sources() {
    const draft = getPlayback().draft;
    return draft?.kind === 'history' && draft.base === this.project ? draft.sources : getState().biliupSources ?? {};
  }

  setProject(project: ProjectModel) {
    this.project = project;
    this.render();
  }

  protected drop(id: string) {
    this.recordingEffects.get(id)?.dispose();this.recordingEffects.delete(id);
    const el = this.els.get(id);
    if (!el) return;
    if (el instanceof HTMLVideoElement) {
      this.mix.detach(id, el);
      releaseEditorMedia(el);
    }
    el.remove();
    this.scrubViews.get(id)?.canvas.remove();
    this.scrubViews.delete(id);
    this.boxes.get(id)?.remove();
    this.boxes.delete(id);
    this.els.delete(id);
    this.dropFx(id);
  }

  /** A clip's effects canvas and overlay off the canvas (its picture element shown again). */
  protected dropFx(id: string, keepShader = false) {
    if (!keepShader) this.dropShader(id);
    if (this.fxCanvas.has(id) || this.fxCopies.has(id)) {
      this.releaseFx(id);
      const el = this.els.get(id);
      if (el) el.style.visibility = this.recordingEffects.has(id) ? "hidden" : "";
    }
    this.fxOverlay.get(id)?.el.remove();
    this.fxOverlay.delete(id);
  }

  /** Whether a clip's picture is drawn by a canvas of ours (effects D136, a shader D213, the GPU compositor D221): a new frame needs a render. */
  protected drawsPicture(id: string) {
    return this.recordingEffects.has(id) || this.fxCanvas.has(id) || this.fxCopies.has(id) || this.shaderCanvas.has(id) || this.onGpu.has(id);
  }

  /** A clip's shader canvas and badge off the canvas. */
  protected dropShader(id: string) {
    this.shaderCanvas.get(id)?.dispose();
    this.shaderCanvas.delete(id);
    this.shaderBadge.get(id)?.remove();
    this.shaderBadge.delete(id);
  }

  /** A clip's shader effect (D213), remade only when its effects change. */
  protected shaderUseOf(item: Item): ShaderUse | undefined {
    const key = item.effects?.length ? JSON.stringify(item.effects) : "";
    const known = this.shaderOf.get(item.id);
    if (known && known.key === key) return known.use;
    const use = key ? clipShader(item) : undefined;
    this.shaderOf.set(item.id, { key, use });
    return use;
  }

  /**
   * A clip's shader effect (D213): drawn into its WebGPU canvas at the effects' size, which then is
   * the picture (shown, or the source of the other picture effects). Without WebGPU: a badge on the
   * clip saying it isn't drawn here and that the export will refuse.
   */
  protected drawShader(item: Item, el: PictureEl, wrap: HTMLDivElement, use: ShaderUse | undefined, size: { w: number; h: number }, t: number): HTMLCanvasElement | undefined {
    if (!use) {
      if (this.shaderCanvas.has(item.id) || this.shaderBadge.has(item.id)) this.dropShader(item.id);
      return undefined;
    }
    const problem = shaderProblem();
    let badge = this.shaderBadge.get(item.id);
    if (problem) {
      this.shaderCanvas.get(item.id)?.dispose();
      this.shaderCanvas.delete(item.id);
      if (!badge) {
        badge = document.createElement("div");
        badge.className = "pv-shader-badge";
        badge.dataset.shaderBadge = item.id;
        this.shaderBadge.set(item.id, badge);
      }
      // Sized in canvas px, so it reads the same at any preview size.
      badge.style.fontSize = `${Math.max(12, Math.round((this.project?.canvas.height ?? 1080) / 36))}px`;
      badge.textContent = `${use.def.name} isn't drawn here: ${problem.replace(/\.$/, "")}. The export needs WebGPU too.`;
      if (badge.parentElement !== wrap) wrap.append(badge);
      return undefined;
    }
    if (badge) {
      badge.remove();
      this.shaderBadge.delete(item.id);
    }
    let s = this.shaderCanvas.get(item.id);
    if (!s) {
      s = new ShaderCanvas();
      s.canvas.className = "pv-fx-canvas";
      s.canvas.dataset.shader = item.id;
      this.shaderCanvas.set(item.id, s);
      el.after(s.canvas);
    }
    const ok = pictureReady(el) && s.render(el, use, size.w, size.h, t, () => this.render());
    return ok || s.drawn ? s.canvas : undefined;
  }

  /** A clip's effects canvas off the canvas, its renderer kept for the next clip (a few: a context and shaders each). */
  protected releaseFx(id: string) {
    const copy = this.fxCopies.get(id);
    if (copy) {
      this.fxCopies.delete(id);
      copy.canvas.remove();
      return;
    }
    const r = this.fxCanvas.get(id)!;
    this.fxCanvas.delete(id);
    r.canvas.remove();
    r.onRestored = undefined;
    r.canvas.style.filter = "";
    if (this.fxPool.length < FX_POOL && this.fxCanvas.size + this.fxPool.length < FX_CONTEXTS && !r.lost) this.fxPool.push(r);
    else r.dispose();
  }

  /** Issue #55: a renderer of the preview's own (one WebGL context); a lost one counts for the tests and draws again when it's back. */
  protected newFxRenderer(): FxRenderer {
    const r = new FxRenderer();
    r.canvas.addEventListener("webglcontextlost", () => !r.disposed && previewStats.fxLost++);
    return r;
  }

  /**
   * Issue #55: a clip with picture effects about to start (warmed up 2 s ahead): a renderer for it
   * made now, its programs compiled (a WebGL compile blocks the frame; 20 clips starting at once at
   * a cut took several), pooled for the clip to take; past FX_CONTEXTS the shared one compiles them.
   */
  protected warmFx(item: Item) {
    if (this.fxBroken || this.fxWarmed.has(item.id) || this.warmBudget <= 0) return;
    const media = this.project?.media.find((m) => m.id === item.media);
    const fx = fxSkips(media) ? undefined : this.effectsOf(item);
    if (!fx || !needsPicturePass(fx)) return;
    this.fxWarmed.add(item.id);
    this.warmBudget--;
    try {
      for (const r of this.fxPool) r.warm(fx);
      if (this.fxCanvas.size + this.fxPool.length < FX_CONTEXTS) {
        const r = this.newFxRenderer();
        r.warm(fx);
        this.fxPool.push(r);
      } else {
        if (!this.fxShared) {
          this.fxShared = this.newFxRenderer();
          this.fxShared.onRestored = () => this.render();
        }
        this.fxShared.warm(fx);
      }
    } catch {
      this.fxBroken = true;
    }
  }

  /**
   * Issue #55: a clip past FX_CONTEXTS: its effects drawn by the one shared renderer, then copied
   * into the clip's 2D canvas (the Effects panel's thumbnails do the same). The copy keeps the last
   * picture, so a paused clip whose frame hasn't changed isn't drawn again.
   */
  protected drawShared(copy: FxCopy, source: PictureEl, fx: ClipFx, w: number, h: number, t: number, key: string | undefined): boolean {
    const r = this.fxShared;
    if (!r || r.lost) return false;
    const frame = frameKeyOf(source);
    const sig = key !== undefined && frame !== undefined ? `${key}|${frame}|${w}x${h}|${fx.shake ? t : ""}` : "";
    if (sig && sig === copy.sig && copy.drawn) return true;
    if (!r.render(source, fx, w, h, t, 1)) return false;
    const c = copy.canvas;
    if (c.width !== w) c.width = w;
    if (c.height !== h) c.height = h;
    copy.ctx.globalCompositeOperation = "copy";
    copy.ctx.drawImage(r.canvas, 0, 0);
    copy.sig = sig;
    copy.drawn = true;
    return true;
  }

  /** A clip's compiled effects (D136), remade only when its effects change. */
  protected effectsOf(item: Item): ClipFx | undefined {
    const key = item.effects?.length ? JSON.stringify(item.effects) : "";
    const known = this.fxOf.get(item.id);
    if (known && known.key === key) return known.fx;
    const fx = key ? clipFx(item) : undefined;
    this.fxOf.set(item.id, { key, fx });
    return fx;
  }

  /**
   * Draws a clip's looks and effects (D136): picture effects through its WebGL canvas (laid over the
   * picture element, which is hidden but keeps playing), blur as CSS on what's shown, overlays on top.
   */
  protected drawFx(item: Item, el: PictureEl, wrap: HTMLDivElement, box: ClipBox, opacity: number, frame: number, fps: number, start: number): void {
    const media = this.project?.media.find((m) => m.id === item.media);
    // SVG stills don't take looks or effects (D138; the planners refuse them too).
    const fx = fxSkips(media) ? undefined : this.effectsOf(item);
    const picture = !this.fxBroken && needsPicturePass(fx);
    let shown: HTMLElement = el;
    // A shader effect (D213) runs first, on the picture; its canvas is then the picture.
    const use = fxSkips(media) ? undefined : this.shaderUseOf(item);
    const shaded = this.drawShader(item, el, wrap, use, fxPictureSize(box, media?.probe), shaderTime(frame, start, fps));
    const shaderCanvas = this.shaderCanvas.get(item.id)?.canvas;
    if (shaderCanvas) {
      placeContent(shaderCanvas, box, shaderCanvas.style.objectFit);
      shaderCanvas.style.opacity = el.style.opacity;
      shaderCanvas.style.visibility = shaded && !picture ? "" : "hidden";
    }
    if (shaded) {
      el.style.visibility = "hidden";
      shown = shaded;
    }
    if (picture && fx) {
      let r = this.fxCanvas.get(item.id);
      let copy = this.fxCopies.get(item.id);
      if (!r && !copy) {
        try {
          // Issue #55: a context of its own while there are few; past FX_CONTEXTS the shared one.
          if (this.fxPool.length || this.fxCanvas.size + this.fxPool.length < FX_CONTEXTS) {
            r = this.fxPool.pop() ?? this.newFxRenderer();
            r.canvas.className = "pv-fx-canvas";
            r.canvas.dataset.fx = item.id;
            r.onRestored = () => this.render();
            r.drawn = false;
            this.fxCanvas.set(item.id, r);
            el.after(r.canvas);
          } else {
            const canvas = document.createElement("canvas");
            canvas.className = "pv-fx-canvas";
            canvas.dataset.fx = item.id;
            canvas.dataset.fxShared = "";
            canvas.style.visibility = "hidden";
            if (!this.fxShared) {
              this.fxShared = this.newFxRenderer();
              this.fxShared.onRestored = () => this.render();
            }
            copy = { canvas, ctx: canvas.getContext("2d")!, sig: "", drawn: false };
            this.fxCopies.set(item.id, copy);
            el.after(canvas);
          }
        } catch {
          this.fxBroken = true;
          return this.drawFx(item, el, wrap, box, opacity, frame, fps, start);
        }
      }
      const c = r ? r.canvas : copy!.canvas;
      placeContent(c, box, c.style.objectFit);
      c.style.opacity = el.style.opacity;
      const size = fxPictureSize(box, media?.probe);
      const t = (Math.floor(frame) - start) / fps;
      const key = shaded ? undefined : this.fxOf.get(item.id)?.key;
      const ok = pictureReady(el) && (r ? r.render(shaded ?? el, fx, size.w, size.h, t, 1, key) : this.drawShared(copy!, shaded ?? el, fx, size.w, size.h, t, key));
      // Until the canvas has a picture (loading, a lost context, a source it can't read) the plain picture shows.
      const drawn = ok || (r ? r.drawn && !r.lost : copy!.drawn && !this.fxShared?.lost);
      el.style.visibility = drawn || shaded ? "hidden" : "";
      c.style.visibility = drawn ? "" : "hidden";
      if (shaded) shaded.style.visibility = drawn ? "hidden" : "";
      if (drawn) shown = c;
    } else if (this.fxCanvas.has(item.id) || this.fxCopies.has(item.id)) {
      this.releaseFx(item.id);
      el.style.visibility = shaded ? "hidden" : "";
    }
    // Blur (an effect) on what's shown, as the export blurs the picture before its box.
    // D265: in the shown element's own px (a <video> placed by a transform is scaled to its box).
    const blur = fx?.blur ? `blur(${px(fx.blur / (Number(shown.dataset.scale) || 1))})` : "";
    if (shown.style.filter !== blur) shown.style.filter = blur;
    if (shown !== el && el.style.filter) el.style.filter = "";
    // Vignette, grain and bars: core's HTML over the picture's visible part, with the clip's opacity.
    const html = fx && hasOverlay(fx) ? fxOverlayInner(fx, visibleRect(box), Math.floor(frame) - start) : "";
    let o = this.fxOverlay.get(item.id);
    if (!html) {
      if (o) {
        o.el.remove();
        this.fxOverlay.delete(item.id);
      }
      return;
    }
    if (!o) {
      const div = document.createElement("div");
      div.className = "pv-fx-overlay";
      div.dataset.fxOverlay = item.id;
      o = { el: div, html: "" };
      this.fxOverlay.set(item.id, o);
    }
    if (o.el.parentElement !== wrap || o.el !== wrap.lastElementChild) wrap.append(o.el);
    if (o.html !== html) {
      o.html = html;
      o.el.innerHTML = html;
    }
    o.el.style.opacity = String(opacity);
  }

  /** The wrapper of a clip's media element (its box). */
  boxOf(id: string) {
    return this.boxes.get(id);
  }

  /** The element for a clip, made on first use; replaced if the clip now shows other media. */
  protected elementFor(item: Item, kind: "video" | "image"): MediaEl | undefined {
    const media = item.media ? this.project?.media.find((m) => m.id === item.media) : undefined;
    const src = media ? previewSrc(media, kind) : undefined;
    if (!src || !media) return undefined;
    let el = this.els.get(item.id);
    if (el && (el.dataset.src === src || el.dataset.failedProxy === src)) return el;
    if (el) this.drop(item.id);
    if (kind === "image") {
      el = document.createElement("img");
      el.alt = "";
      // Issue #61 (D233): not the browser's image drag (a ghost copy): a press drags the layer.
      el.draggable = false;
      // CORS mode (as the <video>), so a look's WebGL pass may read it (D136).
      el.crossOrigin = "anonymous";
      el.addEventListener("load", () => this.drawsPicture(item.id) && this.render());
    } else {
      const video = document.createElement("video");
      // Muted until the mix routes its sound (a clip with sound); CORS mode so Web Audio may hear it.
      video.muted = true;
      video.crossOrigin = "anonymous";
      video.playsInline = true;
      video.preload = "auto";
      video.addEventListener("seeked", () => {
        // Issue #55: a drift seek while playing has landed: how long it took aims the next one.
        const ps = playSeek.get(video);
        if (ps?.from !== undefined) playSeek.set(video, { at: ps.at, lead: Math.min(0.5, (performance.now() - ps.from) / 1000), blind: ps.blind });
        // A paused clip with effects (D136) or a shader (D213) draws the frame the seek landed on;
        // under a scrub canvas waiting for its frame (D221), the <video> may have it first.
        if (this.drawsPicture(item.id) || this.scrubViews.get(item.id)?.shown) this.render();
      });
      video.addEventListener("loadeddata", () => this.drawsPicture(item.id) && this.render());
      // Issue #55: count presented frames (effects redraw per new frame); a paused clip's new frame draws.
      presentedFrames.set(video, 0);
      const onFrame = () => {
        if (!video.isConnected && this.els.get(video.dataset.item ?? "") !== video) return;
        presentedFrames.set(video, (presentedFrames.get(video) ?? 0) + 1);
        if (video.paused && this.drawsPicture(video.dataset.item ?? "")) this.render();
        video.requestVideoFrameCallback(onFrame);
      };
      video.requestVideoFrameCallback(onFrame);
      // D222: its size is known now (whether macOS lifts it, until the scrub cache reads its colours).
      video.addEventListener("loadedmetadata", () => this.render());
      el = video;
    }
    el.dataset.item = item.id;
    el.dataset.src = src;
    // Something Chromium can't decode (HEIC stills, odd codecs) or a file that went missing: say
    // so on the canvas instead of showing black.
    const shown = el;
    const original = mediaUrl(media.id,media.path);
    const wrap = document.createElement("div");
    wrap.className = "pv-box";
    wrap.dataset.item = item.id;
    shown.addEventListener("error", () => {
      if (this.els.get(item.id) !== shown || wrap.querySelector("[data-missing]")) return;
      if(shown instanceof HTMLVideoElement&&fallbackProxy(shown,src,original))return;
      shown.style.visibility = "hidden";
      const note = document.createElement("div");
      note.className = "pv-missing";
      note.dataset.missing = item.id;
      note.textContent = editorText("Can't show {0}",{0:media.name});
      note.title = shown.dataset.sourceError ?? '';
      wrap.append(note);
    });
    if(el instanceof HTMLVideoElement){const video=el;bindEditorMedia(video,media,src,()=>wanted.get(video)??video.currentTime)}
    else el.src = src;
    if (el instanceof HTMLVideoElement && media.probe?.hasAudio) this.mix.attach(item.id, el);
    this.els.set(item.id, el);
    this.boxes.set(item.id, wrap);
    wrap.append(el);
    this.mediaLayer.append(wrap);
    return el;
  }

  /**
   * D221: a video clip's picture while paused or shuttling: its exact cached frame drawn into a canvas
   * over the hidden <video>, or the <video> itself (cache off, a file the cache can't read, no frame
   * yet, or the playhead at rest with the <video> caught up). On a miss the last drawn frame stays up while the next decodes, as a seeking <video> keeps
   * its last frame; at 1× the canvas stays until the <video> has caught up, then the <video> shows.
   * `seek`: whether the <video> may be seeked now. Light footage (≤ 1080p: proxies, HD) seeks as
   * before (on a miss whichever has the frame first shows it); heavier footage under a scrub canvas
   * waits until the playhead rests, as two 4K decoders slow each other down (measured).
   */
  protected scrubPicture(id: string, el: HTMLVideoElement, t: number, playing: boolean, shuttle: number): { pic: PictureEl; seek: boolean } {
    let view = this.scrubViews.get(id);
    const src = scrubCacheOn() && el.dataset.src && !el.error ? this.scrub.source(el.dataset.src) : undefined;
    if (!src || src.state !== "ready") {
      if (view?.shown) {
        view.shown = false;
        view.canvas.style.visibility = "hidden";
      }
      return { pic: el, seek: true };
    }
    if (!view) {
      const canvas = document.createElement("canvas");
      canvas.className = "pv-scrub";
      canvas.dataset.scrub = id;
      canvas.style.visibility = "hidden";
      el.after(canvas);
      view = { canvas, ctx: canvas.getContext("2d")!, key: "", shown: false, lastT: t, movedAt: 0 };
      this.scrubViews.set(id, view);
    }
    const now = performance.now();
    // Stepping or dragging leads the window that way; a jump (a click far off) keeps it around the frame.
    const step = Math.abs(t - view.lastT) <= 0.25;
    const dir: -1 | 0 | 1 = shuttle ? (shuttle > 0 ? 1 : -1) : !step ? 0 : t > view.lastT + 1e-6 ? 1 : t < view.lastT - 1e-6 ? -1 : 0;
    if (t !== view.lastT) {
      view.lastT = t;
      view.movedAt = now;
    }
    const resting = now - view.movedAt >= SCRUB_SETTLE_MS;
    // Issue #55: one decode per file, aimed by the first clip of it in a render. Several clips of one
    // file at other times each re-aimed it (an abort and a fresh read per clip per render: 20 clips
    // made thousands of range requests a second and Chromium refused them); the others show their
    // cached frame when there is one, else their <video> as with the cache off.
    if (!playing && !this.scrubAimed.has(src)) {
      this.scrubAimed.add(src);
      src.want(t, dir);
    }
    const hit = src.frame(t);
    const key = hit ? `${src.id}|${hit.sample}` : "";
    // The canvas only covers the time the <video> takes to get to the frame: once it's there, it shows
    // again (as before D221). On a miss the last frame drawn stays up until the playhead rests.
    const videoThere = !el.seeking && el.readyState >= 2 && Math.abs(el.currentTime - reachable(el, t)) < 0.002;
    let show = playing ? view.shown && (el.seeking || el.readyState < 2) : hit ? !(videoThere && !this.scrub.keep) : view.shown && !resting && !videoThere;
    if (show && hit && view.key !== key) {
      const c = view.canvas;
      if (c.width !== hit.bitmap.width) c.width = hit.bitmap.width;
      if (c.height !== hit.bitmap.height) c.height = hit.bitmap.height;
      view.ctx.drawImage(hit.bitmap, 0, 0);
      view.key = key;
      c.dataset.time = String(hit.pts);
      c.dataset.sample = String(hit.sample);
    }
    if (!view.key) show = false;
    view.shown = show;
    view.canvas.style.visibility = show ? "" : "hidden";
    this.scrubbed.add(id);
    const seek = src.light || !show || playing || resting;
    if (show && !resting) {
      clearTimeout(this.settleTimer);
      this.settleTimer = window.setTimeout(() => this.render(), SCRUB_SETTLE_MS + 10);
    }
    return { pic: show ? view.canvas : el, seek };
  }

  /** Issue #12 (D265): whether this <video> is shown with SMPTE 170M primaries converted to BT.709's (scrub/colour.ts). */
  protected gamutVideo(el: HTMLVideoElement): boolean {
    const src = el.dataset.src ? this.scrub.peek(el.dataset.src) : undefined;
    return convertsPrimaries(src?.index?.colour, el.videoHeight);
  }

  /**
   * D221: whether the GPU compositor can draw a layer as the DOM path does: a video or image clip
   * (box, fit, radius, opacity, a wipe, an iris or a reveal shape, a blur dissolve; picture effects and
   * blur and a shader, not overlays) or a plain-colour background with no transition styling.
   */
  protected gpuCan(v: Visual, project: ProjectModel): boolean {
    const item = v.item;
    if (item.type === "component" && item.component === SOLID) return !v.mask && !v.blur && v.wipe === undefined && v.iris === undefined && !v.shift && !v.shiftY && v.zoom === undefined && !!plainColour(solidBackground(item, project));
    if (item.type !== "clip" || !v.box) return false;
    const media = project.media.find((m) => m.id === item.media);
    if (!media || media.kind === "audio" || fxSkips(media)) return false;
    const el = this.els.get(item.id);
    if (el instanceof HTMLVideoElement && el.error) return false;
    if (this.boxes.get(item.id)?.querySelector("[data-missing]")) return false;
    // D265: a shader draws on its own WebGPU canvas, which the compositor then reads as the picture.
    if (this.shaderUseOf(item) && shaderProblem()) return false;
    const fx = this.effectsOf(item);
    return !fx || !hasOverlay(fx);
  }

  render() {
    if (this.lifetime.signal.aborted) return;
    const t0 = performance.now();
    this.draw();
    if (renderTimes.push(performance.now() - t0) > 2000) renderTimes.splice(0, 1000);
  }

  protected abstract draw(): void;
  protected abstract follow(f: number): number | undefined;
}
