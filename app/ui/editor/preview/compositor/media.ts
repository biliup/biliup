// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import type { ClipBox } from "@/app/lib/editor/core/visual/box";
import type { Visual } from "@/app/lib/editor/core/timeline/timeline";
import { irisRadius, maskClipPath } from "@/app/lib/editor/core/visual/transitions";
import { proxyOf, getState, proxyUrl, mediaUrl } from "@/app/lib/editor/state/store";
import { proxiesOn } from "@/app/ui/editor/settings/proxies";
import {canSeekEditorMedia,seekEditorMedia as seek} from '../mediaElement';
export {fallbackEditorProxy as fallbackProxy} from '../mediaElement';
export {mediaPosition as wanted,reachableMediaTime as reachable,seekEditorMedia as seek} from '../mediaElement';

/** Renderers kept for reuse when clips with effects leave the canvas (D136). */
export const FX_POOL = 4;
/**
 * Issue #55: the most WebGL contexts the preview's effects canvases hold (in use and pooled).
 * Chromium keeps 16 per page and takes the oldest away past that (its layer flipped to the plain
 * picture, and a restore took another one's); clips past this share one context, copied into a 2D
 * canvas each. 10 + that shared one + the Effects panel's + the GPU compositor's stays under 16.
 */
export const FX_CONTEXTS = 10;
/** D221: how long the playhead rests before a <video> under a scrub canvas is seeked to it. */
export const SCRUB_SETTLE_MS = 200;

/** D221: how long the last renders took (ms), for the compositor's measurements in tests. */
export const renderTimes: Array<number> = [];
/** Issue #55: counters the preview tests read (compositors made, effects contexts lost, seeks while playing). */
export const previewStats = { compositors: 0, fxLost: 0, playSeeks: 0 };
(globalThis as { __biliupPreview?: unknown }).__biliupPreview = { renderMs: () => renderTimes.splice(0), stats: () => ({ ...previewStats }) };

/** D221: a clip's scrub canvas (its cached frame over the hidden <video>). */
export interface ScrubView {
  readonly canvas: HTMLCanvasElement;
  readonly ctx: CanvasRenderingContext2D;
  /** The source and sample drawn ("" for none yet). */
  key: string;
  shown: boolean;
  lastT: number;
  movedAt: number;
}

export type MediaEl = HTMLVideoElement | HTMLImageElement;
/** What a clip's picture is drawn from: its media element, or (D221) its scrub canvas. */
export type PictureEl = MediaEl | HTMLCanvasElement;
/** Whether a picture has something to read. */
export const pictureReady = (el: PictureEl) => (el instanceof HTMLVideoElement ? el.readyState >= 2 : el instanceof HTMLImageElement ? el.complete && el.naturalWidth > 0 : el.width > 0);
// Where a video was last asked to be; while a seek runs only the newest target is kept.
/**
 * D212: what a clip's element plays: the media item's proxy when the server has made one and
 * "Use proxies for preview" is on, else the original. The URL carries a tag of the file's path (or
 * the proxy's key), so a relinked file gets a new element, never a <video> that has buffered the old file.
 */
export function previewSrc(media: { readonly id: string; readonly path: string }, kind: "video" | "image") {
  const proxy = kind === "video" && proxiesOn() ? proxyOf(getState(), media) : undefined;
  return proxy?.phase === "done" && proxy.key ? proxyUrl(media.id, proxy.key) : mediaUrl(media.id, media.path);
}

/**
 * Issue #55: a playing <video>'s drift seeks. A seek while playing lands where the clock was when it
 * started, so it is aimed ahead by how long the last one took (`lead`, source s at 1×), and the next
 * one waits a second (`at`): with many decoders busy a seek can take longer than the drift allowed,
 * and chasing each landing with another seek kept a layer frozen on its last frame. The first one is
 * aimed blind (`blind`: no seek timed yet, so no lead): on a busy machine it lands as far behind as it
 * took (0.3 s with 20 decoders and the CPU loaded), so one landing still off may seek again at once,
 * now aimed by the time it just measured; after that the second's wait applies.
 */
export const playSeek = new WeakMap<HTMLVideoElement, { at: number; lead: number; from?: number; blind?: boolean }>();

/**
 * Keep a <video> at `source` seconds: playing at the clip's speed (pitch kept), seeked only when
 * it drifts; paused, seeked to the frame.
 */
export function drive(el: HTMLVideoElement, source: number, playing: boolean, speed: number, keepPitch = true, loopChanged = false) {
  if (el.playbackRate !== speed || el.preservesPitch !== keepPitch) {
    el.playbackRate = speed;
    el.preservesPitch = keepPitch;
  }
  const target = source + 0.0001;
  if (playing && el.ended && Number.isFinite(el.duration) && source >= el.duration) return;
  if(!canSeekEditorMedia(el,target)){
    if(!el.paused)el.pause();
    playSeek.delete(el);
    seek(el,target);
    return;
  }
  if (playing) {
    if (loopChanged) {
      playSeek.delete(el);
      seek(el, target);
      if (el.paused) el.play().catch(() => {});
      return;
    }
    if (el.paused) {
      seek(el, target);
      el.play().catch(() => {});
    } else if (!el.seeking && Math.abs(el.currentTime - target) > 0.12 * Math.max(1, speed)) {
      const now = performance.now();
      const last = playSeek.get(el);
      // Not a jump (the playhead moved while playing): that seeks at once.
      if (last && !last.blind && now - last.at < 1000 && Math.abs(el.currentTime - target) < 1) return;
      // After a blind one, no further ahead than it fell behind: its time can include a playhead
      // that hadn't started moving yet (playback starting), which would aim this one past it.
      const lead = last?.blind ? Math.min(last.lead, Math.max(0, target - el.currentTime) / Math.max(speed, 1e-3)) : (last?.lead ?? 0);
      playSeek.set(el, { at: now, lead, from: now, blind: !last });
      previewStats.playSeeks++;
      seek(el, target + lead * speed);
    }
  } else {
    // A drift seek still in flight when playback stops would time a scrub's seek as its own.
    playSeek.delete(el);
    if (!el.paused) el.pause();
    seek(el, target);
  }
}

export const px = (n: number) => `${Math.round(n * 1000) / 1000}px`;
type Canvas = { readonly width: number; readonly height: number };

/** An iris transition (D79): the circle core's `iris` shows, centred on the canvas, in an element's own px (its top-left at `left`, `top` on the canvas). */
export const irisClip = (iris: number, canvas: Canvas, left = 0, top = 0) => `circle(${px(irisRadius(iris, canvas))} at ${px(canvas.width / 2 - left)} ${px(canvas.height / 2 - top)})`;

/**
 * A full-canvas element (a background, a graphic) through a transition: a wipe shows the left share
 * of it (core's `wipe`), an iris the circle at the centre (D79), and a push or zoom moves it as the
 * canvas (D79: shift right, zoom about the canvas's centre, its own transform-origin).
 */
export function transitionCanvas(el: HTMLElement, v: Pick<Visual, "wipe" | "iris" | "mask" | "shift" | "shiftY" | "zoom" | "blur">, canvas: Canvas) {
  const clip = v.wipe !== undefined ? `inset(0 ${((1 - v.wipe) * 100).toFixed(4)}% 0 0)` : v.iris !== undefined ? irisClip(v.iris, canvas) : v.mask ? maskClipPath(v.mask.shape, v.mask.p, canvas) : "";
  if (el.style.clipPath !== clip) el.style.clipPath = clip;
  setBlur(el, v.blur);
  const translate = v.shift || v.shiftY ? `${px(v.shift ?? 0)} ${px(v.shiftY ?? 0)}` : "";
  const scale = v.zoom !== undefined ? String(v.zoom) : "";
  if (el.style.translate !== translate) el.style.translate = translate;
  if (el.style.scale !== scale) el.style.scale = scale;
}
/** A blur dissolve (D133): CSS blur at core's sigma (canvas px), as the export's gblur. */
export const setBlur = (el: HTMLElement, blur: number | undefined) => {
  const filter = blur ? `blur(${px(blur)})` : "";
  if (el.style.filter !== filter) el.style.filter = filter;
};
export const setStyle = (el: HTMLElement, key: "left" | "top" | "width" | "height" | "borderRadius" | "objectFit", value: string) => {
  if (el.style[key] !== value) el.style[key] = value;
};

/** A picture's content rect (core's, its fit worked out) in its box wrapper's own px. */
export function placeContent(el: HTMLElement, box: ClipBox, fit: string = box.fit) {
  if (el.style.transform) el.style.transform = "";
  delete el.dataset.scale;
  setStyle(el, "left", px(box.content.x - box.x));
  setStyle(el, "top", px(box.content.y - box.y));
  setStyle(el, "width", px(box.content.w));
  setStyle(el, "height", px(box.content.h));
  setStyle(el, "objectFit", fit);
}

/**
 * Issue #12 (D265): a <video> (or its scrub canvas) at its natural size, moved and scaled onto the
 * content rect by a transform. Laid out at the content rect, the browser snaps a video's layer in
 * ways that move the picture up to a third of a canvas pixel depending on where the box sits; the
 * transform places it exactly where core's maths (and the export, and the GPU compositor) put it.
 * Anything else, or a picture whose shape isn't the content rect's (a probe that disagrees with the
 * file), is laid out at the content rect with its object-fit as before.
 */
export function placePicture(el: HTMLElement, box: ClipBox) {
  const n = el instanceof HTMLVideoElement ? { w: el.videoWidth, h: el.videoHeight } : el instanceof HTMLCanvasElement && el.classList.contains("pv-scrub") ? { w: el.width, h: el.height } : { w: 0, h: 0 };
  const c = box.content;
  const fits = n.w > 0 && n.h > 0 && c.w > 0 && c.h > 0 && (box.fit === "fill" || Math.abs((c.w / c.h) * (n.h / n.w) - 1) < 2e-3);
  if (!fits) return placeContent(el, box);
  setStyle(el, "left", "0px");
  setStyle(el, "top", "0px");
  setStyle(el, "width", `${n.w}px`);
  setStyle(el, "height", `${n.h}px`);
  setStyle(el, "objectFit", "fill");
  if (el.style.transformOrigin !== "0px 0px") el.style.transformOrigin = "0 0";
  const t = `translate(${px(c.x - box.x)}, ${px(c.y - box.y)}) scale(${Math.round((c.w / n.w) * 1e6) / 1e6}, ${Math.round((c.h / n.h) * 1e6) / 1e6})`;
  // A CSS filter (an effect's blur) works in the element's own px, before this scale: drawFx divides by it.
  el.dataset.scale = String(Math.sqrt((c.w / n.w) * (c.h / n.h)));
  if (el.style.transform !== t) el.style.transform = t;
}

/**
 * A clip's box (D75): the wrapper at core's box in canvas pixels (it clips the picture and rounds
 * its corners), the picture inside at core's content rect (fit and zoom; a push or zoom transition
 * is already in the box, D79), and a wipe revealing the canvas from the left (the share of the
 * canvas width shown, in the wrapper's own pixels) or an iris the circle at the canvas's centre.
 */
export function placeBox(wrap: HTMLElement, el: HTMLElement, box: ClipBox, canvas: Canvas, shown: number | undefined, iris: number | undefined, mask?: Visual["mask"], blur?: number) {
  setStyle(wrap, "left", px(box.x));
  setStyle(wrap, "top", px(box.y));
  setStyle(wrap, "width", px(box.w));
  setStyle(wrap, "height", px(box.h));
  setStyle(wrap, "borderRadius", box.radius > 0 ? px(box.radius) : "");
  placePicture(el, box);
  const hidden = shown === undefined ? 0 : Math.min(box.w, Math.max(0, box.x + box.w - canvas.width * shown));
  const clip = hidden > 0 ? `inset(0 ${px(hidden)} 0 0)` : iris !== undefined ? irisClip(iris, canvas, box.x, box.y) : mask ? maskClipPath(mask.shape, mask.p, canvas, (x, y) => [x - box.x, y - box.y]) : "";
  if (wrap.style.clipPath !== clip) wrap.style.clipPath = clip;
  setBlur(wrap, blur);
}
