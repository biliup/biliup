// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The text census of a drawn graphic frame (render_frames' and review_submit's legibility note):
// every word the DOM shows, how tall its letters stand on the frame, the small ones listed. The
// films' agent-step lines came out 2 to 2.5 % tall (a piece typed under its default, a thread's
// step rows smaller than its message) and nothing measured them before the critic, which judged
// them by eye. Canvas-drawn text isn't in the DOM and isn't measured.

/** Words a viewer must read stand at least this share of the frame's height (the motion films' readability floor). */
export const SMALL_TEXT = 0.03;
/** A letter's height (cap and ascender height) as a share of its font size, about DM Sans' and SF Pro's. */
export const LETTER_SHARE = 0.72;

/** One text run of a census: its words (cut at 60 characters) and its letters' height, % of the frame's height. */
export interface CensusRun {
  readonly text: string;
  readonly share: number;
}
/**
 * A frame's census: the runs under the floor (the biggest first, at most 40), how many runs were
 * measured, and `unseen`: canvases, images and videos on screen (each over 1 % of the frame) whose
 * words, if any, the census can't read, so "no small text" can't be claimed for them.
 */
export interface Census {
  readonly small: ReadonlyArray<CensusRun>;
  readonly measured: number;
  readonly unseen?: number;
  /** Words big enough to read but blurred, faded or dimmed, with what does it (the biggest first, at most 8). */
  readonly soft?: ReadonlyArray<SoftRun>;
  /** An biliupUI stage's preview showing nothing (black): the share of the frame it covers, % (the largest). */
  readonly emptyPreview?: number;
}

/**
 * A word the census skipped as softened: its total blur (px), opacity and brightness, and the one
 * style that weighs most: by "stage" (biliupUI.mount's blur/dim props on part), "swap" (biliupUI.sequence
 * morphing into or out of piece, its key at at s for dur s) or "graphic" (the graphic's own el's prop).
 * T4-keynote: the Export moment was judged at 22.6 s, 0.08 s into a 0.2 s swap into export.high, for
 * three rounds; the agent read it as dim styling and shortened the swap each time.
 */
export interface SoftRun extends CensusRun {
  readonly blur: number;
  readonly alpha: number;
  readonly bright: number;
  readonly by?: "stage" | "swap" | "graphic";
  readonly prop?: "blur" | "opacity" | "brightness";
  readonly part?: string;
  readonly piece?: string;
  readonly at?: number;
  readonly dur?: number;
  readonly side?: string;
  readonly el?: string;
}

/**
 * The census as a function's source, called in the graphic's window after render(t) as
 * `(${GRAPHIC_CENSUS})(width, height, floor)`. A run counts when it is on screen (half of it inside
 * the frame), not hidden, not faded under 35 % and not under a blur over 1 px (the stage blurred
 * behind a hero is texture, not words to read). Its size is its font in frame px (every transform
 * on the way, the camera's included) times LETTER_SHARE, as biliupUI.frameText measures.
 */
export const GRAPHIC_CENSUS = `function (W, H, floor) {
  var LETTER = ${LETTER_SHARE};
  var state = new Map();
  function look(el) {
    if (state.has(el)) return state.get(el);
    var cs = getComputedStyle(el);
    var up = el.parentElement ? look(el.parentElement) : { alpha: 1, blur: 0, bright: 1, shown: true, ok: true, causes: [] };
    var own = parseFloat(cs.opacity);
    if (!(own >= 0)) own = 0;
    var alpha = up.alpha * own;
    var m = /blur\\(([\\d.]+)px\\)/.exec(cs.filter || "");
    var b = /brightness\\(([\\d.]+)\\)/.exec(cs.filter || "");
    var ownBlur = m ? parseFloat(m[1]) : 0, ownBright = b ? parseFloat(b[1]) : 1;
    var blur = Math.max(up.blur, ownBlur), bright = up.bright * ownBright;
    var causes = ownBlur > 0.05 || own < 0.999 || ownBright < 0.999 ? up.causes.concat([{ el: el, blur: ownBlur, alpha: own, bright: ownBright }]) : up.causes;
    var shown = up.shown && cs.display !== "none" && alpha >= 0.02;
    // Under 35 % opacity, a blur over 1 px or darkened under 70 %: texture (the stage behind a hero), not words to read.
    var ok = shown && alpha >= 0.35 && blur <= 1 && bright >= 0.7;
    var s = { alpha: alpha, blur: blur, bright: bright, shown: shown, ok: ok, causes: causes };
    state.set(el, s);
    return s;
  }
  // What softens a word: the element whose blur, opacity or brightness weighs most, and who set it:
  // biliupUI.mount's blur/dim props (the stage), biliupUI.sequence's swap, or the graphic's own style.
  function why(st) {
    var best = null, score = 0;
    for (var i = 0; i < st.causes.length; i++) {
      var c = st.causes[i];
      var v = Math.max(c.blur > 1 ? c.blur / 4 : 0, (1 - c.alpha) * 1.5, (1 - c.bright) * 1.5);
      if (v > score) { score = v; best = c; }
    }
    var out = { blur: Math.round(st.blur * 10) / 10, alpha: Math.round(st.alpha * 100) / 100, bright: Math.round(st.bright * 100) / 100 };
    if (!best) return out;
    var e = best.el;
    // The blur on screen (a filter's px are the element's own, before its scale: mount's blur prop is screen px).
    if (best.blur > 1) out.blur = Math.round(best.blur * scaleOf(e) * 10) / 10;
    var hd = e.closest && e.closest("[data-biliup-piece]");
    if (hd && e.closest(".biliup-ui-shell")) {
      out.by = "swap"; out.piece = hd.getAttribute("data-biliup-piece"); out.at = parseFloat(hd.getAttribute("data-biliup-at")) || 0;
      out.dur = parseFloat(hd.getAttribute("data-biliup-dur")) || 0; out.side = hd.getAttribute("data-biliup-swap") || "";
    } else if (e.classList && e.classList.contains("biliup-ui") && e.getAttribute("data-part")) {
      out.by = "stage"; out.part = e.getAttribute("data-part");
    } else {
      out.by = "graphic";
      var cls = typeof e.className === "string" ? e.className.trim().split(/\\s+/).filter(Boolean).slice(0, 2) : [];
      out.el = e.tagName.toLowerCase() + (e.id ? "#" + e.id : "") + (cls.length ? "." + cls.join(".") : "");
    }
    out.prop = best.blur > 1 && best.blur / 4 >= Math.max(1 - best.alpha, 1 - best.bright) * 1.5 ? "blur" : best.alpha <= best.bright ? "opacity" : "brightness";
    return out;
  }
  function scaleOf(el) {
    // SVG text: its viewBox scales it too, which offsetWidth doesn't see.
    if (el instanceof SVGElement && el.getScreenCTM) { var c = el.getScreenCTM(); if (c) return Math.hypot(c.a, c.b); }
    for (var e = el; e && e.nodeType === 1; e = e.parentElement) {
      if (e.offsetWidth > 0) { var b = e.getBoundingClientRect(); if (b.width > 0) return b.width / e.offsetWidth; }
    }
    return 1;
  }
  var runs = new Map();
  var soft = new Map();
  var measured = 0;
  var walk = document.createTreeWalker(document.body || document.documentElement, 4);
  for (var n = walk.nextNode(); n; n = walk.nextNode()) {
    var text = (n.nodeValue || "").replace(/\\s+/g, " ").trim();
    var p = n.parentElement;
    if (!p || text.length < 2 || !/[\\p{L}\\p{N}]/u.test(text)) continue;
    var tag = p.tagName;
    if (tag === "SCRIPT" || tag === "STYLE" || tag === "TITLE" || tag === "NOSCRIPT") continue;
    var st = look(p);
    if (!st.shown) continue;
    var cs = getComputedStyle(p);
    if (cs.visibility === "hidden") continue;
    var range = document.createRange();
    range.selectNodeContents(n);
    var rects = range.getClientRects();
    var area = 0, inside = 0;
    for (var i = 0; i < rects.length; i++) {
      var r = rects[i];
      var a = r.width * r.height;
      if (!a) continue;
      area += a;
      var w = Math.max(0, Math.min(r.right, W) - Math.max(r.left, 0));
      var h = Math.max(0, Math.min(r.bottom, H) - Math.max(r.top, 0));
      inside += w * h;
    }
    if (!area || inside < area / 2) continue;
    var share = ((parseFloat(cs.fontSize) || 0) * scaleOf(p) * LETTER) / H;
    if (!(share > 0)) continue;
    if (!st.ok) {
      // Words big enough to read but blurred, faded or dimmed: listed with what softens them.
      // A swap fits the piece smaller while it morphs, so its words count from 0.6 of the floor; a
      // stage's from the floor (its desktop-sized chrome is texture even when sharp).
      var sk = text.slice(0, 60);
      if (share >= floor * 0.6 && !soft.has(sk)) {
        var w = why(st);
        if (w.by !== "stage" || share >= floor) soft.set(sk, Object.assign({ text: sk, share: Math.round(share * 1000) / 10 }, w));
      }
      continue;
    }
    measured++;
    if (share >= floor) continue;
    var key = text.slice(0, 60);
    var was = runs.get(key);
    if (was === undefined || share < was) runs.set(key, share);
  }
  var unseen = 0;
  var pics = document.querySelectorAll("canvas, img, video, image, object, embed, iframe");
  for (var k = 0; k < pics.length; k++) {
    var q = pics[k];
    var b = q.getBoundingClientRect();
    var vis = Math.max(0, Math.min(b.right, W) - Math.max(b.left, 0)) * Math.max(0, Math.min(b.bottom, H) - Math.max(b.top, 0));
    if (vis < W * H * 0.01) continue;
    var ps = look(q);
    if (ps.ok && getComputedStyle(q).visibility !== "hidden") unseen++;
  }
  // An biliupUI preview with nothing in it (no preview.frame, no clip under the playhead, or its media unreadable): a black
  // hole on the stage. The films' editor wide shots showed one for seconds and nothing said so.
  var empty = 0;
  var frames = document.querySelectorAll(".biliup-ui .frame");
  for (var j = 0; j < frames.length; j++) {
    var fr = frames[j];
    if (!fr.closest(".stage") || fr.closest(".biliup-ui-measure")) continue;
    if (fr.querySelector("canvas:not([data-biliup-footage=failed]), img, video, svg, iframe")) continue;
    var fb = fr.getBoundingClientRect();
    var fv = Math.max(0, Math.min(fb.right, W) - Math.max(fb.left, 0)) * Math.max(0, Math.min(fb.bottom, H) - Math.max(fb.top, 0));
    if (fv < W * H * 0.02) continue;
    var fs = look(fr);
    if (!fs.shown || fs.alpha < 0.35 || getComputedStyle(fr).visibility === "hidden") continue;
    empty = Math.max(empty, fv / (W * H));
  }
  var small = [];
  runs.forEach(function (share, text) { small.push({ text: text, share: Math.round(share * 1000) / 10 }); });
  small.sort(function (a, b) { return b.share - a.share; });
  var softs = [];
  soft.forEach(function (v) { softs.push(v); });
  softs.sort(function (a, b) { return b.share - a.share; });
  return { small: small.slice(0, 40), measured: measured, unseen: unseen, soft: softs.slice(0, 8), emptyPreview: Math.round(empty * 100) };
}`;

/** A census back from the rasteriser, checked (it crossed a process boundary). */
export function censusOf(v: unknown): Census | undefined {
  const c = v as { small?: unknown; measured?: unknown; unseen?: unknown } | null;
  if (!c || typeof c !== "object" || !Array.isArray(c.small) || typeof c.measured !== "number") return undefined;
  const small = c.small
    .filter((x): x is CensusRun => !!x && typeof (x as CensusRun).text === "string" && typeof (x as CensusRun).share === "number" && Number.isFinite((x as CensusRun).share))
    .slice(0, 40)
    .map((x) => ({ text: x.text.slice(0, 60), share: x.share }));
  const str = (v: unknown, n = 60) => (typeof v === "string" && v ? v.slice(0, n) : undefined);
  const fin = (v: unknown) => (typeof v === "number" && Number.isFinite(v) ? v : undefined);
  const soft = (Array.isArray((c as { soft?: unknown }).soft) ? ((c as { soft: Array<unknown> }).soft) : [])
    .filter((x): x is Record<string, unknown> => !!x && typeof x === "object" && typeof (x as CensusRun).text === "string" && fin((x as CensusRun).share) !== undefined)
    .slice(0, 8)
    .map((x): SoftRun => {
      const by = x.by === "stage" || x.by === "swap" || x.by === "graphic" ? x.by : undefined;
      const prop = x.prop === "blur" || x.prop === "opacity" || x.prop === "brightness" ? x.prop : undefined;
      const o: Record<string, unknown> = { text: (x.text as string).slice(0, 60), share: x.share as number, blur: fin(x.blur) ?? 0, alpha: fin(x.alpha) ?? 1, bright: fin(x.bright) ?? 1, by, prop, part: str(x.part, 40), piece: str(x.piece, 60), at: fin(x.at), dur: fin(x.dur), side: str(x.side, 4), el: str(x.el, 80) };
      for (const k of Object.keys(o)) if (o[k] === undefined) delete o[k];
      return o as unknown as SoftRun;
    });
  const empty = fin((c as { emptyPreview?: unknown }).emptyPreview);
  return { small, measured: c.measured, ...(typeof c.unseen === "number" && c.unseen > 0 ? { unseen: c.unseen } : {}), ...(soft.length ? { soft } : {}), ...(empty !== undefined && empty > 0 ? { emptyPreview: Math.min(100, Math.round(empty)) } : {}) };
}

const secs = (v: number) => `${Math.round(v * 100) / 100} s`;

/** How a softened word looks: "blurred 6 px, at 40 % opacity". */
function softLook(r: SoftRun): string {
  const parts = [...(r.blur > 1 ? [`blurred ${r.blur} px`] : []), ...(r.alpha < 0.999 ? [`at ${Math.round(r.alpha * 100)} % opacity`] : []), ...(r.bright < 0.999 ? [`darkened to ${Math.round(r.bright * 100)} %`] : [])];
  return parts.join(", ") || "softened";
}

/** Who softens it and the one change that makes it read, said exactly (the prop, the key, the element). */
export function softCause(r: SoftRun): string {
  if (r.by === "swap" && r.piece) {
    const end = (r.at ?? 0) + (r.dur ?? 0);
    return r.side === "out"
      ? `biliupUI.sequence's swap out of "${r.piece}" (the next key's push blurs and fades the old piece away; it isn't the moment's subject there)`
      : `biliupUI.sequence's swap into "${r.piece}" (its key at ${secs(r.at ?? 0)}, dur ${secs(r.dur ?? 0)}): the new piece pushes in through a 14 px blur and a fade over about the first 60 % of its dur, so this frame is mid-swap, not a styling problem. It reads from about ${secs((r.at ?? 0) + 0.6 * (r.dur ?? 0))} and is at rest at ${secs(end)}: start that key at least its dur before the moment's time (or judge the moment at ${secs(end)} or later), and don't shorten dur or restyle the piece for it`;
  }
  if (r.by === "stage" && r.part) {
    const props = [...(r.blur > 1 ? ["blur"] : []), ...(r.bright < 0.999 ? ["dim"] : [])];
    return `biliupUI.mount("${r.part}")'s ${props.join(" and ") || "blur/dim"} prop${props.length > 1 ? "s" : ""}: the stage behind a hero is meant to be soft; a word the moment is about must not sit in the stage while it is blurred or dimmed: set it as its own piece (biliupUI.sequence or biliupUI.piece) on a layer above the stage, or bring blur and dim to 0 while the stage is the subject`;
  }
  if (r.by === "graphic" && r.el) return `the graphic's own ${r.prop === "blur" ? "filter: blur" : r.prop === "brightness" ? "filter: brightness" : "opacity"} on ${r.el}: raise it (opacity 1, no blur, brightness 1) for the frames the word must read`;
  return "a blur, opacity or brightness on the way to it";
}

/** Softened words as a short list with their cause: at most `max`, one cause said once. */
export function softList(soft: ReadonlyArray<SoftRun>, max = 3): string {
  const shown = soft.slice(0, max);
  const said = new Set<string>();
  return shown
    .map((r) => {
      const cause = softCause(r);
      const word = `"${r.text.length > 40 ? `${r.text.slice(0, 39)}…` : r.text}" ${r.share} % (${softLook(r)}`;
      if (said.has(cause)) return `${word}, the same)`;
      said.add(cause);
      return `${word}: ${cause})`;
    })
    .join("; ");
}

/** Runs as a short list: the biggest first (the moment's lines before the chrome), at most `max`, then how many more. */
export function censusList(small: ReadonlyArray<CensusRun>, max = 5): string {
  const shown = small.slice(0, max).map((x) => `"${x.text.length > 40 ? `${x.text.slice(0, 39)}…` : x.text}" ${x.share} %`);
  return `${shown.join(", ")}${small.length > max ? ` and ${small.length - max} more${small.length >= 40 ? " (at least)" : ""}` : ""}`;
}
