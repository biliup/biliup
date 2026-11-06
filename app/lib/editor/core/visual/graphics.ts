// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// `graphic` items (D78): the escape hatch for anything the components don't cover. The agent (with
// graphic_write) or the user writes an HTML file in the project's graphics/ folder that defines
// `render(t, info)`; Biliup calls it for each frame (t = seconds from the item's start). The preview
// shows it in a sandboxed iframe (served by the local server with its own CSP), the export draws it
// frame by frame in an offscreen Chromium window; both load the same document (graphicDocument) and
// call render the same way, so preview == export.
//
// Limits: full canvas (no box; opacity, fades and transitions apply); no network, no Node, no
// other files (CSP: inline scripts and styles, data: images and fonts only); fonts: the bundled DM
// Sans and DM Mono, the brand kit's font files (both declared inside the document when it names
// them, D198, fonts.ts resolveGraphicFonts) and the fonts installed on this Mac; every face is
// loaded before the first render(t), so canvas text isn't drawn in a fallback; render(t) must draw synchronously or return a promise
// that settles within GRAPHIC_FRAME_MS, or longer only while it is fetching project media
// (biliup.frame: the rasteriser waits while pictures keep coming, up to about 27 s, under the
// server's 30 s; desktop render.ts GRAPHIC_CAP_MS); the export draws every frame (D291: streamed raw to ffmpeg,
// with motion blur's sub-frames when it's on; the preview draws no blur); the file is read
// when the export starts or the preview loads it (write a new file to change a graphic: graphic_write
// never overwrites, so earlier revisions keep their look).
//
// Bundled libraries (D193): a graphic loads one with a marker, <script src="biliup:lib/three.js">
// </script> (also confetti.js, globe.js, spring.js, random.js, motion.js, ui.js). resolveGraphicLibs replaces each
// marker with the library's source as an inline script (so the CSP stays as it is and nothing loads
// from anywhere), right after the graphic file is read (the server's readGraphics, which the preview
// route, the export and render_frames all go through), so preview == export by construction. Core
// can't read files: the server passes the sources (assets/graphic-libs, ServerConfig
// graphicLibsDir). MAX_GRAPHIC_BYTES caps the author's file, not the libraries; with every library
// in, a document stays under MAX_GRAPHIC_DOC_BYTES (the export loads it as a data: URL, which
// Chromium caps at 2 MB of URL, base64).
import type { Canvas, Item } from "@/app/lib/editor/core/timeline/types";
import { GRAPHIC_MEDIA_BOOT, GRAPHIC_MEDIA_SCHEME } from "@/app/lib/editor/core/visual/graphicMedia";
import {GRAPHIC_CENSUS} from './census';

/** Where graphic files live in a project, and what their names look like (project.schema.json `src`). */
export const GRAPHIC_SRC = /^graphics\/[A-Za-z0-9][A-Za-z0-9._-]{0,80}\.html$/;
/** The most a graphic file may hold (bytes). */
export const MAX_GRAPHIC_BYTES = 256 * 1024;
/** How long one render(t) may take before the frame fails (ms). */
export const GRAPHIC_FRAME_MS = 5000;
/** The CSP of a graphic's document: its own inline script and styles, data: images and fonts, nothing else; D290: the project's media through the rasteriser's own scheme (blob: images of them). */
export const GRAPHIC_CSP = `default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; img-src data: blob:; font-src data:; connect-src ${GRAPHIC_MEDIA_SCHEME}:`;

/** What render(t, info) gets besides t. */
export interface GraphicInfo {
  readonly frame: number;
  readonly fps: number;
  readonly duration: number;
  readonly width: number;
  readonly height: number;
  readonly props: { readonly [key: string]: unknown };
}

/**
 * The graphic's own frame shown `local` frames into its item: from the item's `in` (frames of the
 * graphic before what the item shows: a cut-off start, a split's second half, a range export's
 * slice), held at the item's first and last frame before and after it.
 */
export const graphicLocal = (item: Pick<Item, "duration" | "in">, local: number): number => (item.in ?? 0) + Math.max(0, Math.min(item.duration - 1, local));

/** What render(t, info) gets at the graphic's own frame `local` (graphicLocal); its duration counts from the graphic's start, `in` included. */
export const graphicInfo = (item: Pick<Item, "duration" | "props" | "in">, canvas: Pick<Canvas, "width" | "height"> & { readonly fps: number }, local: number): GraphicInfo => ({
  frame: local,
  fps: canvas.fps,
  duration: (item.in ?? 0) + item.duration,
  width: canvas.width,
  height: canvas.height,
  props: item.props ?? {},
});

/**
 * A graphic file's HTML as the document both the preview and the export load: canvas-sized,
 * transparent, the strict CSP, and a bootstrap after the author's markup. window.__biliupRender(t,
 * info) calls their render and resolves two animation frames later (so what's captured is drawn);
 * the preview's iframe gets the same call by postMessage ({ biliup: "render", seq, t, info }) and
 * answers { biliup: "rendered", seq }.
 */
/** The libraries a graphic may load with <script src="biliup:lib/<name>"></script> (D193). */
export const GRAPHIC_LIBS = ["three.js", "confetti.js", "globe.js", "spring.js", "random.js", "motion.js", "ui.js"] as const;
export type GraphicLib = (typeof GRAPHIC_LIBS)[number];

/** The most a graphic's whole document may hold, libraries in (bytes): base64 of it must stay under Chromium's 2 MB URL limit. */
export const MAX_GRAPHIC_DOC_BYTES = 1_400_000;

// A marker: a script element whose only attribute is src="biliup:lib/<name>", empty.
const LIB_MARKER = /<script\s+src\s*=\s*(["'])biliup:lib\/([^"'<>\s]*)\1\s*>\s*<\/script\s*>/gi;

/** Which libraries a graphic's HTML asks for, known and unknown (each once, in order); for graphic_write's note. */
export function graphicLibsUsed(html: string): { readonly known: Array<string>; readonly unknown: Array<string> } {
  const known: Array<string> = [];
  const unknown: Array<string> = [];
  for (const m of html.matchAll(LIB_MARKER)) {
    const name = m[2]!;
    const list = (GRAPHIC_LIBS as ReadonlyArray<string>).includes(name) ? known : unknown;
    if (!list.includes(name)) list.push(name);
  }
  return { known, unknown };
}

/**
 * The graphic's HTML with each library marker replaced by the library's source (`libs`: name →
 * source) as an inline classic script, once per library (a repeated marker becomes a comment); a
 * marker for a library that isn't there becomes a comment saying so (the graphic then fails where
 * it uses it, visibly). Nothing else in the HTML changes.
 */
export function resolveGraphicLibs(html: string, libs: Readonly<Record<string, string>>): string {
  // Each library once: a second marker for it is a comment (so a document never holds two copies,
  // which could take it past the export's data: URL limit while the preview still loaded it).
  const seen = new Set<string>();
  return html.replace(LIB_MARKER, (_marker, _q, name: string) => {
    const source = (GRAPHIC_LIBS as ReadonlyArray<string>).includes(name) && Object.hasOwn(libs, name) ? libs[name] : undefined;
    const safe = name.replace(/--/g, "- -");
    if (source === undefined) return `<!-- biliup:lib/${safe} is not a bundled library (bundled: ${GRAPHIC_LIBS.join(", ")}) -->`;
    if (seen.has(name)) return `<!-- biliup:lib/${safe} is already loaded above -->`;
    seen.add(name);
    // An inline script ends at the first </script: keep any inside the source from closing it.
    return `<script>/* biliup:lib/${safe} */\n${source.replace(/<\/(script)/gi, "<\\/$1")}\n</script>`;
  });
}

// D198: biliup:brand/logo-light (also logo-dark, logo-mark), in an attribute or a CSS url().
const BRAND_MARKER = /biliup:brand\/logo-(light|dark|mark)\b/g;
const BLANK = "data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7";

/** Which kit logos a graphic asks for (biliup:brand/logo-…), each once. */
export function graphicLogosUsed(html: string): Array<"light" | "dark" | "mark"> {
  return [...new Set([...html.matchAll(BRAND_MARKER)].map((m) => m[1] as "light" | "dark" | "mark"))];
}

/** The graphic's HTML with each biliup:brand/logo-… marker replaced by the kit's logo as a data: URL (a missing one: a blank pixel). */
export function resolveGraphicBrand(html: string, logos: Readonly<Partial<Record<"light" | "dark" | "mark", string>>>): string {
  return html.replace(BRAND_MARKER, (_m, slot: "light" | "dark" | "mark") => logos[slot] ?? BLANK);
}

/** For the agent's system prompt (D193): the render(t) contract and the bundled libraries. */
export const GRAPHIC_LIBS_DOC = [
  "Graphics (graphic_write) draw everything from t alone: render(t, info) is called for each frame in any order, in the preview and again in the export, so no timers, no requestAnimationFrame loops, no Date.now or performance.now, no Math.random (use biliupRandom(seed)), no state carried from one frame to the next; nothing loads from the network.",
  'Bundled libraries load with a marker in the HTML, inlined by Biliup (the same in the preview and the export): <script src="biliup:lib/three.js"></script>, and likewise biliup:lib/confetti.js, biliup:lib/globe.js, biliup:lib/spring.js, biliup:lib/random.js, biliup:lib/motion.js, biliup:lib/ui.js; any other name is not loaded.',
  "three.js (0.169, window.THREE): create the renderer once at load, new THREE.WebGLRenderer({ canvas, alpha: true, antialias: true, preserveDrawingBuffer: true }) with a canvas of info.width by info.height (renderer.setSize(info.width, info.height, false), pixel ratio 1), then inside render(t) set the camera and every object from t and call renderer.render(scene, camera).",
  "biliupConfetti.draw(ctx, t, { seed, count, at, origin: { x, y } as canvas fractions, angle, spread, velocity, gravity, drag, size, life, colors, shapes }) draws a confetti burst on a 2D context as a pure function of t (clear the canvas first).",
  "biliupGlobe.draw(ctx, t, { x, y, radius, dots, rotation, speed in degrees per second, tilt, dotSize, ocean, markers: [{ lat, lon, color, size }], colors: { dot, marker, sphere, rim } }) draws a dotted spinning globe; biliupGlobe.project(ctx, t, lat, lon, options) gives { x, y, visible }.",
  'biliupSpring(t, "spring.land" or { stiffness, damping, mass, velocity }) is the spring Biliup uses for eases (0 at t = 0 seconds, settling at 1; presets spring.snap, spring.settle, spring.land, spring.bounce), with biliupSpring.ease(preset)(p) fitted to p from 0 to 1.',
  'biliupMotion (motion.js) is for one-take motion films, all pure functions of t: springs(t, [{ at, to }], "spring.snap") sums one spring per target change (a value retargeted many times stays continuous); edges(t, [{ at, left, right }], { lead, trail }) gives a liquid tab indicator\'s two edges, the leading one on the faster spring; rise(t, { at, out, height }) gives the y of text rising out of a mask line (in an overflow: hidden box); flood(t, { at, x, y, r, width, height, dur: 0.4, until, to: { x, y, r } }) grows a circle past the frame\'s corners and contracts it into the next shape; iris(open, { x, y, r, blades: 6 }) gives blade polygons; squeeze(xs, widths, anchor, f) pulls a wordmark\'s letters into its period; cursor(t, [{ at, x, y }]) gives a drawn cursor\'s path; click(t, { from: [x, y], x, y, fire }) one believable click (a list chains them): it glides in, rests 0.1 s, presses (scale the cursor by .scale, the button by .button) and releases, and the change it causes is keyed at its fire, 0.03 s into the press, never on the cursor\'s arrival; camera(t, [{ at, x, y, zoom, dur }]) a screen-studio camera ((x, y) the world point at the centre, zoom in log space; each move a glide of 0.75 s or more, longer for a big push), frameRect(left, top, w, h, { width, height, fit, pad, align }) the camera key that makes a UI rect fill the frame (fit: true shows all of it), apply(ctx, cam, width, height) its transform and toScreen(cam, x, y, width, height) a world point on screen; shape(t, [{ at, x, y, w, h, r } or { at, bleed: true }]) one rounded rect morphing key to key, morph(t, from, to, { at, dur: 0.4 }) rect to rect in dur s (to "bleed" fills past every edge), coverage(s, width, height) the share of the frame inside it, swap(t, at) the old content\'s exit then the new one\'s entry inside a morphing shape (alpha, a short blur, dy), shapePath(ctx, s) its path, footageRect(srcW, srcH, width, height, s) where a clip covers it (the whole frame\'s crop: a shot already on screen at another rect, a preview fitted to a card, never jumps to it on one frame, it eases there or uses it from the start); riseText(ctx, text, x, baseline, t, { at, out, size }) a line rising out of its mask; beat(n, { bpm, offset }), loop(t, duration), ease.inOut/out/in/glide (glide: the calm move, easing off and slowing down longer than it sped up), mix, mixColor.',
  "Pace (measured at 60 fps on the films that worked): moves at normal speed, about 0.8 s with a soft settle (slowing down longer than it sped up, peaking about 1.2 frame heights/s), one main move at a time (the next may start as the last settles; a cursor may ride along); something happens on nearly every beat, about one big state change per bar; after a change, about half a beat to a beat to read it, then the next: never 2 s of one state, the end card about 1.5 s. The defaults of camera (0.75 s), biliupUI.sequence (0.9 s) and biliupUI.fly (0.9 s) move this way.",
  "biliupUI (ui.js) is Biliup's own editor UI as live DOM generated from the app's real components and CSS (parts: editor, topbar, agent, message, turn, review, timeline, preview, composer, storyboard, export, exportProgress, inspector, effects, menu: a timeline clip\'s right-click menu with Add to chat; the composer and a message take chips, the clip\'s \"@name\" drawn as the app\'s chip). For a film about Biliup itself, build from these parts instead of redrawing its panels. Readable first: every word the viewer must read stands at least 3 % of the frame\'s height (its letters, a 4.2 % font); render_frames lists every word under that in each frame it draws, and two calls fix it: the line as a piece at its default type (a smaller type makes smaller words, and type sizes the piece\'s first line, so a thread\'s 12 px step rows come out at 12/13 of its message: give steps their own \"turn.steps\" key), or the camera key biliupUI.frameText(el, world, { width, height }) on the stage. Build as pieces: biliupUI.sequence(host, t, keys) carries ONE shell through keys, each a single element (\"composer\", \"turn.steps\", \"timeline.main\", \"inspector.amount\", \"preview.frame\", \"topbar.export\", \"topbar.saved\") set large on the canvas by default (centred, 86 % of the width, words 4.8 % of the height) with its props (typed text, steps, clips, a filter's amount, your canvas in the preview, progress) and a look (theme \"light\" or dark, radius, color), and morphs each into the next (centre, size, radius, colour; the new content pushing in from its side out of a blur, carry elements such as the prompt travelling into the next piece); biliupUI.piece shows one alone, biliupUI.mount a whole part (rarely: a desktop-sized panel reads as a screen recording), biliupUI.cursor the pointer. The editor mounted every frame is a live stage (its panels follow props: clips, steps, playhead, the Inspector, the export dialog; blur and dim put it behind a big hero piece), and biliupUI.fly carries an element into its place in it (the hero composer docking into the agent panel, a media tile becoming its timeline clip, the clip becoming the preview) while a camera (biliupMotion.camera) pushes into where the work happens. biliupUI.frameText(el or [els], world, { width, height }) is the camera key that makes those words read: measured on a hidden mount with them filled in, it zooms until their font is 4.8 % of the frame\'s height (a piece\'s size), keeps them whole inside the safe inset, and returns text (the share they get) and fits (false: too wide to set that large, so set them as a piece); for example keys.push({ at: send, ...biliupUI.frameText([root.querySelector(\".msg-user\"), root.querySelector(\".worklog\")], world, { width: 1080, height: 1350 }), dur: 0.9 }). By default a part states no run facts (the review says \"Reviewing\" with empty bars, no round, model, time or time left; no steps, reply, edit count, media tiles or export progress; ui.js's header below lists the sample labels that stay): pass a result, score, round or timing only from this run's real results.",
  'The default story for a film about Biliup is the live stage; a worked example to adapt: every frame, root = biliupUI.mount(ed, "editor", { width: 1440, height: 1000, media, time: t, blur, dim, message, turn: { steps, done }, timeline: { clips, playhead }, side, inspector: { filter: "Warm", amount }, export, exportProgress }) in a world div whose transform is biliupMotion.cameraCSS(cam, 1080, 1350). The preview shows the cut by itself: give each clip its media id and in point (timeline: { clips: [{ track: "Main", start, duration, name, image, media: id, in: 2.5 }], playhead }) and the stage draws that clip\'s real frame under the playhead (a gap is black, as in the app); preview: { frame: canvas } replaces it with your own canvas (a flood, a look being applied), passed from frame 0; render_frames says when a preview on screen is empty. Its beats, about one a bar: (1) hero = biliupUI.fly(fx, t, { id: "composer", at: send, to: biliupUI.find(root, "composer"), from: { piece: "composer", y: 640, props: { text: prompt, caret: true }, typing: { at: 0.05, dur: 3 } } }) (typing types the whole prompt in and grows the card smoothly at each new line; never slice the text yourself), the big composer typed over the editor at blur 6, dim 0.16 (call every biliupUI.fly every frame, before and after its flight too: guarding it until its target exists leaves stale copies); (2) Send flies the prompt into its message bubble (to: root.querySelector(".msg-user"), fit: "contain"; fly keeps one copy on screen: its target is hidden until the copy lands, from frame 0 when from is a piece key, and the typed words leave the flying card before the bubble\'s come in, so don\'t hide or fade the bubble yourself; message: { text: prompt } and turn: { steps: [first], done: 0 } from send, so a step is already running) while the camera glides over the same 0.9 s from the composer onto the message and steps, its key from biliupUI.frameText (above; a guessed rect or a wide shot left the films\' 12 px steps 1.5 to 2.5 % tall in a dark column), and blur and dim ease to 0 only as it lands: the bubble lands large over a ticking step, never docked into the small composer, over an empty panel, with the preview\'s picture dropped (clips with media ids under the playhead, or preview.frame from frame 0: an empty preview is a black hole on the stage) or with the dark program filling the frame; (3) the steps tick in, one a beat, the camera on them; when frameText says fits: false they are the moment as a piece instead (biliupUI.sequence key "turn.steps" at its default size) over the stage kept blurred, the panel only once they\'re done; (4) per clip, biliupUI.fly(fx, t, { id: "clip" + i, at, from: tiles[i], to: lane[i], arc: -70 }) with tiles = root.querySelectorAll(\'[data-testid="media-tile"] .thumb\'), lane = root.querySelectorAll(".tl-lane.main .clip") (a clip is in timeline.clips from its flight\'s start, shown when it lands), one after another, then the camera pushes in on the lane until the clip names read (4.5 % of the frame); (5) into the preview, the cut playing, flooding on the drop; (6) side: "inspector", the cursor drags the Warm amount and the preview (your canvas) warms with it; (7) the cursor clicks Export: export, then exportProgress filling to done; (8) back to the composer for the loop. cam = biliupMotion.camera(t, keys) with keys from biliupMotion.frameRect on rects measured once (biliupUI.rectIn on a hidden mount): a push in for every key moment, a wide shot only as a brief pull-back between them. Pieces (biliupUI.sequence) work as close-ups or a whole film; never steps floating over an editor left blurred.',
  "biliupRandom(seed) returns a seeded generator (rnd() in [0, 1), rnd.range(a, b), rnd.int(a, b), rnd.pick(list), rnd.normal()); biliupRandom.noise(seed, x) is smooth noise.",
  'Fonts: name a family in CSS font-family or in ctx.font (for example ctx.font = "600 64px \'DM Sans\', sans-serif") and Biliup declares it inside the graphic when it is DM Sans, DM Mono, a font file of the brand kit or one in the project\'s fonts folder (media_download a WOFF2/TTF/OTF of an OFL font such as Geist or Archivo puts it there), loaded before the first frame; fonts installed on this Mac work by name, and so does a family the graphic declares itself (@font-face with a data: URL). Any other family is a project_check error (it would fall back to another font), so never ask for one that isn\'t there.',
  'Brand logos: src="biliup:brand/logo-light" (also logo-dark, logo-mark) in an <img> or a CSS url() is the kit\'s logo file as it is (an SVG stays vector), inlined by Biliup. The kit\'s other SVGs (such as claude-mark.svg, the Claude mark the app\'s composer shows in its model picker) have no marker: read the file from the kit (brand_read lists its path) and paste its <svg> into the graphic as it is, never redrawn.',
  'Project media (ids from project_read): const f = await biliup.frame(mediaId, seconds, { width, height }) is that video\'s frame at that moment of it (a still: any seconds) as an ImageBitmap for ctx.drawImage or a three.js texture, exact in the export and the paused preview (while the preview plays, a frame not decoded yet is the nearest one that is); for footage that plays inside the graphic, await biliup.frame(id, start + t) inside an async render (return its promise). In markup, src="biliup:media/<id>" (a still or a video\'s first frame) or "biliup:media/<id>#t=2.5" in an <img> or a CSS url(). Pictures come at most 1920 px on their long edge (smaller when you ask for a smaller width or height), never inlined, so the 256 KB limit is about your code only.',
].join(" ");

export function graphicDocument(html: string, canvas: Pick<Canvas, "width" | "height">, timeoutMs=5000): string {
  const bound=Number.isFinite(timeoutMs)?Math.max(1000,Math.min(30000,Math.round(timeoutMs))):5000;
  const size = `width:${canvas.width}px;height:${canvas.height}px`;
  // The bootstrap goes in <head>, before the author's markup: an unclosed <script>, <!--, <textarea>
  // or <title> in their HTML can't swallow it. It only looks up window.render when called.
  return `<!doctype html><html><head><meta charset="utf-8"><meta http-equiv="Content-Security-Policy" content="${GRAPHIC_CSP}">
<style>html,body{margin:0;padding:0;background:transparent;overflow:hidden;${size}}</style>
<script>${GRAPHIC_MEDIA_BOOT}</script>
<script>(() => {
  const frames = () => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(() => done())));
  // D198: every declared face loaded once before the first frame (canvas text doesn't wait for a font).
  let fonts;
  const fontsReady = () => (fonts ??= Promise.all([...document.fonts].map((f) => f.load().catch(() => {}))).then(() => document.fonts.ready));
  // dry (a rehearsal, nothing captured): no frames waited for after render(t).
  window.__biliupRender = async (t, info, dry) => {
    await fontsReady();
    await window.__biliupMedia.ready();
    // Whatever render throws comes back as an Error with its name and message: a DOMException
    // (drawImage of a closed bitmap, a failed decode) or a thrown string would reach the export as
    // a value without a message.
    if (typeof window.render === "function") {
      try {
        await window.render(t, info);
      } catch (e) {
        const text = e && typeof e.message === "string" ? (e.name && e.name !== "Error" ? e.name + ": " : "") + e.message : String(e);
        throw new Error(text || "render(t) failed");
      }
    }
    await window.__biliupMedia.held();
    if (!dry) await frames();
    window.__biliupMedia.sweep();
    return true;
  };
  // The preview asks once per frame while it plays and never waits. One render(t) at a time: a
  // graphic awaiting footage (biliup.frame) for longer than a frame would otherwise have many
  // render(t) calls interleaved on one canvas (their transforms and save/restore stacks mixed, an
  // older t finishing after a newer one and staying on screen, the media source asked out of order).
  // While one runs only the newest ask waits; the ones it replaced are answered as skipped.
  let busy = false;
  let next;
  // Bounded: a render(t) (or the exact pictures a stopped one waits for) that never settles frees
  // the preview after BOUND ms, answered as an error, so the asks after it still draw.
  const BOUND = ${bound};
  const bounded = (p) => new Promise((done, fail) => {
    const timer = setTimeout(() => fail(new Error("render(t) took over " + BOUND / 1000 + " s")), BOUND);
    p.then(done, fail).finally(() => clearTimeout(timer));
  });
  const pump = async () => {
    busy = true;
    while (next) {
      const d = next;
      next = undefined;
      const playing = d.playing === true;
      window.__biliupMedia.begin({ playing, warm: !playing });
      // Playing: nothing waits for frames after render(t) (that's the rasteriser's capture), the
      // next ask comes with the next frame; two animation frames a render capped the graphic at 30 fps.
      await bounded(window.__biliupRender(d.t, d.info, playing))
        .then(async () => {
          // Stopped: drawn with the nearest pictures while the exact ones came; now with them.
          const missed = window.__biliupMedia.missed();
          if (!missed) return;
          await bounded(missed);
          // A newer ask (a scrub on) draws next instead.
          if (next) return;
          window.__biliupMedia.begin({ playing: false });
          await bounded(window.__biliupRender(d.t, d.info));
        })
        .then(() => {
          let census;
          if(typeof d.census === "number" && d.census > 0 && d.census < 1){
            try{census=(${GRAPHIC_CENSUS})(${canvas.width},${canvas.height},d.census)}catch{census={small:[],measured:0,unseen:1}}
          }
          parent.postMessage({biliup:"rendered",seq:d.seq,...(census?{census}:{})},"*");
        }, (error) => parent.postMessage({ biliup: "rendered", seq: d.seq, error: String(error) }, "*"));
    }
    busy = false;
  };
  addEventListener("message", (e) => {
    const d = e.data;
    if (!d || d.biliup !== "render") return;
    if (next) parent.postMessage({ biliup: "rendered", seq: next.seq, skipped: true }, "*");
    next = d;
    if (!busy) void pump();
  });
})();</script></head><body>
${html}
</body></html>`;
}
