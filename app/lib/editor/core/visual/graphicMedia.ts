// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Project media in graphics (D290). A graphic draws the project's own footage and stills: in script
// with `await biliup.frame(mediaId, seconds, { width, height })` (an ImageBitmap of that media's frame
// at that moment of the media, for a canvas's drawImage or a WebGL texture), and in markup with a
// marker, `biliup:media/<mediaId>` (a still, or a video's first frame) or `biliup:media/<mediaId>#t=<s>`
// (a video's frame at s seconds), in an <img> src or a CSS url(). Nothing is inlined into the
// document (its size cap stays about the author's file): the pictures come from the server's frame
// source (apps/server/src/export/graphicMedia.ts: ffmpeg, the frame on the media's own grid, at most
// GRAPHIC_MEDIA_MAX_EDGE on its long edge), the same bytes for every drawer:
// - the export and render_frames (a top-level data: page in the rasteriser) fetch them from the
//   rasteriser's own scheme, GRAPHIC_MEDIA_SCHEME, which asks the server over its JSON lines;
// - the preview (a sandboxed iframe, no network) asks its parent by postMessage ({ biliup: "media",
//   seq, id, t, w, h }), which fetches them from the project's graphic-media API and answers
//   { biliup: "media-reply", seq, blob } (or error).
// So preview == export as for the rest of a graphic; only while the preview plays is a picture not
// come yet drawn as the nearest one already there (asks paced, one in flight per media and size),
// and asked with p: a video's may come from its preview proxy, smaller, which the reply says
// ({ full: { width, height } }) and the graphic is given scaled up to that exact size (never handed
// to an exact ask). The CSP allows only that scheme for fetches
// and blob: images (the markers' pictures), still nothing from the network.

/** The rasteriser's scheme for a graphic's media (export and render_frames): biliup-media://media/<id>?t=&w=&h=. */
export const GRAPHIC_MEDIA_SCHEME = "biliup-media";
/** The largest picture of a media a graphic gets (its long edge, px): a 4K clip comes at most this big. */
export const GRAPHIC_MEDIA_MAX_EDGE = 1920;
/** The longest a media moment may be (seconds). */
const MAX_T = 24 * 3600;

const ID = /^[A-Za-z0-9][A-Za-z0-9_-]{0,127}$/;
// A marker in the author's HTML: biliup:media/<id>, optionally #t=<seconds>.
const MARKER = /biliup:media\/([A-Za-z0-9][A-Za-z0-9_-]{0,127})(?:#t=(\d+(?:\.\d+)?|\.\d+))?/g;
// A literal media id handed to the helpers in script: biliup.frame("id", …) / biliup.image('id').
const CALL = /\bbiliup\s*\.\s*(?:frame|image)\s*\(\s*(["'`])([A-Za-z0-9][A-Za-z0-9_-]{0,127})\1/g;

/** One picture a graphic asks for: a media, a moment of it (seconds from the media's start) and the box it should fit (px, optional). */
export interface GraphicMediaRequest {
  readonly id: string;
  readonly t: number;
  readonly w?: number;
  readonly h?: number;
  /** The preview asks while it plays (p=1): a video's picture may come from its preview proxy, smaller (GraphicMediaPicture.full says the exact size). Never in the export. */
  readonly preview?: true;
}

/** A media request from a URL's path id and query (t, w, h), or why not. Sizes are whole pixels 1–8192. */
export function parseGraphicMediaRequest(id: string, query: { readonly get: (name: string) => string | null }): GraphicMediaRequest | { readonly problem: string } {
  if (!ID.test(id)) return { problem: "bad media id" };
  const t = Number(query.get("t") ?? 0);
  if (!Number.isFinite(t) || t < 0 || t > MAX_T) return { problem: "t must be seconds from 0" };
  const size = (name: string) => {
    const raw = query.get(name);
    if (raw === null || raw === "") return undefined;
    const n = Number(raw);
    return Number.isInteger(n) && n >= 1 && n <= 8192 ? n : null;
  };
  const w = size("w");
  const h = size("h");
  if (w === null || h === null) return { problem: "w and h must be whole pixels from 1 to 8192" };
  return { id, t, ...(w !== undefined ? { w } : {}), ...(h !== undefined ? { h } : {}), ...(query.get("p") === "1" ? { preview: true as const } : {}) };
}

/**
 * The size a media's picture is given: its own (width × height, already turned upright), made
 * smaller (never larger) to fit the asked box and GRAPHIC_MEDIA_MAX_EDGE, its aspect kept, in even
 * pixels (what ffmpeg's scale gives every chroma layout).
 */
export function graphicMediaSize(width: number, height: number, ask: { readonly w?: number | undefined; readonly h?: number | undefined } = {}, maxEdge = GRAPHIC_MEDIA_MAX_EDGE): { readonly width: number; readonly height: number } {
  const k = Math.min(1, maxEdge / Math.max(width, height), ask.w ? ask.w / width : 1, ask.h ? ask.h / height : 1);
  const even = (x: number) => Math.max(2, Math.round(x / 2) * 2);
  return { width: even(width * k), height: even(height * k) };
}

/** Which frame of a video is on screen at `t` seconds of it (its own frame rate), within its frames when known. */
export function graphicMediaFrame(t: number, fps: number, frames?: number): number {
  // The frame whose time span holds t; a hair of slack so t = n / fps lands on n despite rounding.
  const n = Math.max(0, Math.floor(t * fps + 1e-6));
  return frames && frames > 0 ? Math.min(n, frames - 1) : n;
}

/** The media ids a graphic's HTML names (markers and literal biliup.frame / biliup.image ids), each once: for graphic_write's note. */
export function graphicMediaUsed(html: string): Array<string> {
  return [...new Set([...[...html.matchAll(MARKER)].map((m) => m[1]!), ...[...html.matchAll(CALL)].map((m) => m[2]!)])];
}

/**
 * The script graphicDocument puts in a graphic's <head> (before its own bootstrap): window.biliup
 * (frame, image, url) and window.__biliupMedia (begin({ playing }): before each preview render; ready: the markers' pictures in place and decoded,
 * awaited before the first frame; hold(promise) / held(): work a library started in render(t) that the
 * frame must show, awaited after it; sweep(fps): called after each frame and each motion-blur
 * sub-frame, keeps the cache bounded by forgetting pictures not asked for lately; stats(): the
 * pictures still pending, which the rasteriser asks for when a frame runs late).
 * A picture handed to the page is the page's: it is never closed here. The page may keep it in its
 * own caches and draw it any number of frames later without asking again (round 2's A2 film kept
 * every thumbnail it got and drew them seconds later; closing pictures unasked for about 2 × fps
 * frames detached them and the export died at frame 511). Forgetting only drops the cache's
 * reference: GC frees the pixels once the page lets go too, and a later ask fetches a fresh bitmap.
 */
export const GRAPHIC_MEDIA_BOOT = `(() => {
  const MARK = ${MARKER.toString()};
  const BLANK = "data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7";
  const KEEP = 24;
  const waiting = new Map();
  const fulls = new WeakMap();
  let seq = 0;
  // Pictures asked of the host and not answered yet (the rasteriser asks when a frame is late).
  let pending = 0;
  // The preview (an iframe): its parent answers. The export: the rasteriser's own scheme.
  addEventListener("message", (e) => {
    const d = e.data;
    if (e.source !== parent || !d || d.biliup !== "media-reply" || !waiting.has(d.seq)) return;
    const w = waiting.get(d.seq);
    waiting.delete(d.seq);
    if (d.blob instanceof Blob) {
      // A proxy's picture (asked while playing): given at the exact one's size.
      const f = d.full;
      if (f && f.width >= 1 && f.height >= 1 && f.width <= 8192 && f.height <= 8192) fulls.set(d.blob, { width: Math.round(f.width), height: Math.round(f.height) });
      w.done(d.blob);
    }
    else w.fail(new Error(String(d.error || "the media couldn't be loaded")));
  });
  const load = (id, t, w, h, preview) => {
    pending++;
    const p = ask(id, t, w, h, preview);
    p.then(() => pending--, () => pending--);
    return p;
  };
  const ask = (id, t, w, h, preview) => {
    if (parent !== window) return new Promise((done, fail) => {
      const s = ++seq;
      waiting.set(s, { done, fail });
      parent.postMessage({ biliup: "media", seq: s, id, t, w, h, ...(preview ? { p: true } : {}) }, "*");
    });
    const q = "t=" + t + (w ? "&w=" + w : "") + (h ? "&h=" + h : "");
    return fetch("${GRAPHIC_MEDIA_SCHEME}://media/" + encodeURIComponent(id) + "?" + q).then((r) => (r.ok ? r.blob() : r.text().then((m) => Promise.reject(new Error(m || "the media couldn't be loaded")))));
  };
  // Bounded: at most KEEP pictures beyond the ones asked for in the last frame. Never closed.
  const cache = new Map();
  let gen = 0;
  // The preview while it plays (its parent says so with each render): a picture not here yet is
  // drawn as the nearest one of the same media and size already here (within NEAR seconds), and
  // asked for paced: one ask in flight per media and size, only the newest waiting behind it. The
  // preview asks 60 frames a second and footage takes longer than a frame to come (a 1920-px frame
  // is 6 MB), so waiting for each exact frame left render(t) running for hundreds of ms with the
  // canvas half drawn on screen (a black card where the footage goes) and the asks piling up.
  // Stopped (and always in the export) every picture is the exact one, awaited.
  let playing = false;
  // Stopped in the preview, a render is drawn twice when it must: first with the nearest pictures
  // while the exact ones are asked for (warming), then, once they're here, again with them; so the
  // frame it stops or scrubs to never shows half drawn either.
  let warming = false;
  let misses = [];
  const NEAR = 1;
  const runs = new Map();
  // An ask that never comes back frees its run after PACE_BOUND ms (it's still answered if it comes).
  const PACE_BOUND = 5000;
  const issue = (r, job) => {
    r.busy = true;
    const p = load(job.id, job.s, job.w, job.h, true);
    p.then(job.done, job.fail);
    let freed = false;
    const timer = setTimeout(() => free(), PACE_BOUND);
    const free = () => {
      if (freed) return;
      freed = true;
      clearTimeout(timer);
      r.busy = false;
      const n = r.next;
      r.next = undefined;
      if (n) {
        n.entry.job = undefined;
        issue(r, n);
      }
    };
    p.then(free, free);
  };
  const size = (v) => (typeof v === "number" && v >= 1 ? Math.min(8192, Math.round(v)) : 0);
  // A blob's ImageBitmap; a proxy's picture scaled up to the exact one's size on the GPU (an
  // OffscreenCanvas: createImageBitmap's own resize runs on the CPU and halved the preview's frame rate).
  const picture = async (b) => {
    const v = await createImageBitmap(b);
    const f = fulls.get(b);
    if (!f || typeof OffscreenCanvas === "undefined" || (v.width === f.width && v.height === f.height)) return v;
    const c = new OffscreenCanvas(f.width, f.height);
    const g = c.getContext("2d");
    if (!g) return v;
    g.drawImage(v, 0, 0, f.width, f.height);
    v.close();
    return c.transferToImageBitmap();
  };
  const entry = (id, t, opts, paced) => {
    if (typeof id !== "string") throw new TypeError("biliup.frame(mediaId, seconds): mediaId is the media's id (project_read lists them)");
    const s = Math.max(0, Number(t) || 0);
    const w = size(opts && opts.width);
    const h = size(opts && opts.height);
    const run = id + "|" + w + "|" + h;
    // A paced ask's picture may be its proxy's (smaller, then given at the exact size): never
    // handed to an exact ask (stopped, a blob: URL), only drawn as the nearest one while it comes.
    const k = run + "|" + s + (paced ? "|p" : "");
    let e = cache.get(k);
    if (!e) {
      if (paced) {
        let r = runs.get(run);
        if (!r) runs.set(run, (r = { busy: false }));
        let done, fail;
        e = { blob: new Promise((a, b) => ((done = a), (fail = b))), s, run };
        const job = { id, s, w, h, k, done, fail, entry: e };
        if (!r.busy) issue(r, job);
        else {
          // Only the newest waits: the one it replaces is never asked (whoever awaits it gets this one's picture).
          const old = r.next;
          if (old) {
            old.entry.job = undefined;
            if (cache.get(old.k) === old.entry) cache.delete(old.k);
            e.blob.then(old.done, old.fail);
          }
          e.job = r.next = job;
        }
      } else e = { blob: load(id, s, w, h), s, run };
      cache.set(k, e);
      e.blob.catch(() => cache.get(k) === e && cache.delete(k));
    }
    e.gen = gen;
    return e;
  };
  // The picture of the same media and size already here nearest to e (the latest at or before it first).
  const nearest = (e) => {
    let before, after;
    for (const c of cache.values()) {
      if (c === e || c.run !== e.run || !c.value || !c.value.width || Math.abs(c.s - e.s) > NEAR) continue;
      if (c.s <= e.s) { if (!before || c.s > before.s) before = c; }
      else if (!after || c.s < after.s) after = c;
    }
    return before || after;
  };
  const biliup = {
    /** The media's picture at \`t\` seconds of it (a still: any t), as an ImageBitmap fitting { width, height } (at most ${GRAPHIC_MEDIA_MAX_EDGE} px long). */
    frame(id, t, opts) {
      const preview = parent !== window;
      const paced = playing && preview;
      const e = entry(id, t, opts, paced);
      const bitmap = (e.bitmap ??= e.blob.then(picture).then((v) => (e.value = v)));
      if (!(paced || (warming && preview)) || e.value) return bitmap;
      const near = nearest(e);
      if (!near) return bitmap;
      if (!paced) misses.push(bitmap.catch(() => {}));
      near.gen = gen;
      return Promise.resolve(near.value);
    },
    image(id, opts) {
      return biliup.frame(id, 0, opts);
    },
    /** The same picture as a blob: URL, for an <img> or a CSS url() set from script (kept for the page's life). */
    url(id, t, opts) {
      const e = entry(id, t, opts, false);
      e.keep = true;
      return (e.url ??= e.blob.then((b) => URL.createObjectURL(b)));
    },
  };
  window.biliup = Object.freeze(biliup);
  let marks;
  const swap = async (text, used) => {
    let out = text;
    for (const ref of new Set(text.match(MARK) || [])) {
      const m = new RegExp(MARK.source).exec(ref);
      const u = await biliup.url(m[1], m[2] ? Number(m[2]) : 0).catch((error) => (console.error("biliup:media/" + m[1] + ": " + error.message), BLANK));
      used.add(u);
      out = out.split(ref).join(u);
    }
    return out;
  };
  // Work a library started during render(t) that the frame must show (biliupUI's stage preview
  // drawing a clip's frame): the bootstrap awaits it before the capture.
  let holds = [];
  window.__biliupMedia = {
    stats: () => ({ pending }),
    /** Before a render: whether the preview is playing (nearest pictures, paced asks) or not (exact ones). */
    begin: (o) => {
      playing = !!(o && o.playing);
      warming = !playing && !!(o && o.warm);
      misses = [];
    },
    /** After a warming render: a promise for the exact pictures it drew the nearest ones for, else undefined. */
    missed: () => (misses.length ? Promise.all(misses.splice(0)) : undefined),
    hold: (p) => { holds.push(Promise.resolve(p).catch(() => {})); },
    held: async () => {
      while (holds.length) {
        const now = holds;
        holds = [];
        await Promise.all(now);
      }
    },
    ready: () => (marks ??= (async () => {
      const used = new Set();
      const jobs = [];
      for (const el of document.querySelectorAll("*")) {
        for (const a of ["src", "href", "xlink:href", "poster", "style"]) {
          const v = el.getAttribute(a);
          if (v && v.includes("biliup:media/")) jobs.push(swap(v, used).then((nv) => el.setAttribute(a, nv)));
        }
        if (el.tagName === "STYLE" && el.textContent.includes("biliup:media/")) jobs.push(swap(el.textContent, used).then((nv) => (el.textContent = nv)));
      }
      await Promise.all(jobs);
      // Decoded before the first frame (a CSS background too: the same blob: URL, decoded once).
      await Promise.all([...used].map((u) => { const i = new Image(); i.src = u; return i.decode().catch(() => {}); }));
      await Promise.all([...document.images].map((i) => i.decode().catch(() => {})));
    })()),
    sweep: () => {
      gen++;
      for (const [k, e] of cache) {
        if (cache.size <= KEEP) break;
        // Asked for in the last frame (or kept for a blob: URL): stays. Else forgotten, oldest first
        // (a Map iterates in insertion order), but never closed: the page may still hold it, and
        // drawImage of a closed bitmap throws "The image source is detached".
        if (e.keep || e.gen >= gen - 1) continue;
        cache.delete(k);
      }
    },
  };
})();`;
