// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// D221: the GPU compositor (issue #12 item 4, behind Settings › App › Preview "Draw the preview on
// the GPU", off by default). One WebGL2 canvas under the preview's media layer draws, in track order,
// the bottom run of layers it supports: video and image clips (their box, content rect and fit,
// rounded corners, opacity, a wipe or an iris) with their picture effects (D136's FxRenderer, run in
// this same context into a framebuffer, instead of a WebGL context per clip), and plain-colour
// backgrounds. From the first layer it doesn't support (text, components, graphics, a shader, a blur,
// an overlay effect, a mask) everything above stays on the DOM path, so z-order and the export's
// order are kept. The <video> and <img> elements stay where they are (sound, the clock, hit-testing),
// made transparent; the canvas reads their current frames.
// Issue #12 (D265): drawn to match the DOM path (measured against it on testsrc2's pixel-sharp
// edges): one backing pixel per device pixel, pictures at core's exact sub-pixel rect (the DOM path
// places a <video> by a transform for the same reason, media.ts placePicture), antialiased picture
// edges, mipmapped stills, and moving pictures sampled over a quarter device pixel more (the
// browser's own video drawing is that much softer than one bilinear tap).
// Export doesn't use it: it stays on ffmpeg (see D221).
import type { ClipBox } from "@/app/lib/editor/core/visual/box";
import type { ClipFx } from "@/app/lib/editor/core/visual/fx";
import { FxRenderer } from "@/app/ui/editor/preview/fx";

const VERTEX = `#version 300 es
layout(location = 0) in vec2 aPos;
uniform vec4 uRect;
uniform vec2 uBacking;
uniform float uK;
out vec2 vP;
void main() {
  vec2 p = uRect.xy + aPos*uRect.zw;
  vP = p;
  vec2 n = p*uK / uBacking*2.0 - 1.0;
  gl_Position = vec4(n.x, -n.y, 0.0, 1.0);
}`;

// vP is in canvas px (top-left origin). The box clips (rounded, antialiased over one device pixel
// like the browser's border-radius), the picture fills uImg (its fit already worked out) with its
// edges antialiased along uEdge, a wipe shows what's left of uWipe, an iris the circle uIris.
// uSoft: how far apart (device px) the four taps of a moving picture are (0: one bilinear tap).
const FRAGMENT = `#version 300 es
precision highp float;
in vec2 vP;
out vec4 outColor;
uniform sampler2D uTex;
uniform bool uSolid;
uniform vec4 uColour;
uniform vec4 uBox;
uniform float uRadius;
uniform vec4 uImg;
uniform vec4 uEdge;
uniform float uOpacity;
uniform float uWipe;
uniform vec3 uIris;
uniform float uPx;
uniform bool uFlipY;
uniform float uSoft;
uniform bool uGamut;
uniform bool uMasked;
uniform sampler2D uMask;
uniform vec2 uBackingF;
// SMPTE 170M primaries to BT.709's in linear light (BT.709's transfer both ways), as Chromium shows
// untagged SD video and the export's colorspace filter converts it.
const mat3 C170 = mat3(0.939542, 0.017772, -0.001622, 0.050181, 0.965793, -0.004370, 0.010277, 0.016435, 1.005991);
vec3 toLinear(vec3 v) { return mix(v / 4.5, pow((v + 0.099) / 1.099, vec3(1.0 / 0.45)), step(0.081, v)); }
vec3 fromLinear(vec3 v) { return mix(v*4.5, 1.099*pow(v, vec3(0.45)) - 0.099, step(0.018, v)); }
void main() {
  vec2 h = uBox.zw*0.5;
  float r = min(uRadius, min(h.x, h.y));
  vec2 q = abs(vP - (uBox.xy + h)) - (h - r);
  float d = length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - r;
  float cover = clamp(0.5 - d / uPx, 0.0, 1.0);
  cover *= clamp((uWipe - vP.x) / uPx + 0.5, 0.0, 1.0);
  if (uIris.z >= 0.0) cover *= clamp((uIris.z - length(vP - uIris.xy)) / uPx + 0.5, 0.0, 1.0);
  if (uMasked) cover *= texture(uMask, vec2(gl_FragCoord.x, uBackingF.y - gl_FragCoord.y) / uBackingF).a;
  vec4 c;
  if (uSolid) c = uColour;
  else {
    vec2 e = min(vP - uEdge.xy, uEdge.xy + uEdge.zw - vP);
    cover *= clamp(0.5 + min(e.x, e.y) / uPx, 0.0, 1.0);
    if (cover <= 0.0) discard;
    vec2 uv = (vP - uImg.xy) / uImg.zw;
    if (uFlipY) uv.y = 1.0 - uv.y;
    if (uSoft > 0.0) {
      vec2 o = uSoft*0.5*uPx / uImg.zw;
      c = 0.25*(texture(uTex, uv - o) + texture(uTex, uv + o) + texture(uTex, uv + vec2(o.x, -o.y)) + texture(uTex, uv + vec2(-o.x, o.y)));
    } else c = texture(uTex, uv);
    if (uGamut && c.a > 0.0) c.rgb = fromLinear(clamp(C170*toLinear(c.rgb / c.a), 0.0, 1.0))*c.a;
  }
  outColor = c*(uOpacity*cover);
}`;

// Issue #12 (D265): one direction of a Gaussian blur (sigma in backing px) over a whole-canvas
// texture, premultiplied, transparent outside it (CSS filter: blur()'s edge).
const BLUR_VERTEX = `#version 300 es
layout(location = 0) in vec2 aPos;
out vec2 vUv;
void main() {
  vUv = aPos;
  gl_Position = vec4(aPos*2.0 - 1.0, 0.0, 1.0);
}`;
const BLUR_FRAGMENT = `#version 300 es
precision highp float;
in vec2 vUv;
out vec4 outColor;
uniform sampler2D uTex;
uniform vec2 uDir;
uniform float uSigma;
vec4 at(vec2 uv) {
  return (uv.x < 0.0 || uv.y < 0.0 || uv.x > 1.0 || uv.y > 1.0) ? vec4(0.0) : texture(uTex, uv);
}
void main() {
  vec2 px = uDir / vec2(textureSize(uTex, 0));
  int r = int(min(ceil(uSigma*3.0), 120.0));
  float k = -0.5 / (uSigma*uSigma);
  vec4 sum = at(vUv);
  float total = 1.0;
  // Two taps at once, between texels, weighted by their sum (linear filtering does the split).
  for (int i = 1; i <= 120; i += 2) {
    if (i > r) break;
    float w0 = exp(float(i*i)*k);
    float w1 = i + 1 <= r ? exp(float((i + 1)*(i + 1))*k) : 0.0;
    float w = w0 + w1;
    float o = float(i) + w1 / w;
    sum += (at(vUv + px*o) + at(vUv - px*o))*w;
    total += 2.0*w;
  }
  outColor = sum / total;
}`;

type Rect = { readonly x: number; readonly y: number; readonly w: number; readonly h: number };
/** Where a picture is drawn, and (when it differs) the rect its antialiased edges follow. */
type Pic = Rect & { readonly edge?: Rect };
/** A layer's picture: its texture, where it lands, rows bottom-up (a target), sampled soft (moving), SMPTE 170M primaries to convert. */
type Src = { readonly tex: WebGLTexture; readonly img: Pic; readonly flip: boolean; readonly soft: boolean; readonly gamut: boolean };
const EVERYWHERE: Rect = { x: -1e5, y: -1e5, w: 2e5, h: 2e5 };
/** A stand-in layer covering the canvas (a blurred target drawn back). */
const WHOLE = (size: { w: number; h: number }): GpuPicture => ({ id: "", source: undefined as never, box: { x: 0, y: 0, w: size.w, h: size.h, fit: "fill", scale: 1, opacity: 1, radius: 0, content: { x: 0, y: 0, w: size.w, h: size.h } }, opacity: 1 });

/** Issue #12 (D265): how far apart (device px) a moving picture's four taps are; measured best against the DOM path at 0.25 (0.15 and 0.35 both worse). */
const SOFT = 0.25;

/** Where the canvas (w × h canvas px) lands on screen: `k` device px per canvas px. */
export interface GpuPlace {
  readonly w: number;
  readonly h: number;
  readonly k: number;
}

/** One layer the compositor draws. */
export interface GpuPicture {
  readonly id: string;
  readonly source: HTMLVideoElement | HTMLImageElement | HTMLCanvasElement;
  readonly box: ClipBox;
  readonly opacity: number;
  /** The share of the canvas width a wipe shows (undefined: no wipe). */
  readonly wipe?: number | undefined;
  /** An iris's circle in canvas px. */
  readonly iris?: { readonly x: number; readonly y: number; readonly r: number } | undefined;
  /** Picture effects: drawn at `size` (core's fxPictureSize), `t` s from the clip's start. */
  readonly fx?: { readonly fx: ClipFx; readonly size: { readonly w: number; readonly h: number }; readonly t: number } | undefined;
  /** D265: a <video> Chromium shows with SMPTE 170M primaries converted to BT.709's (untagged SD); its upload has the values unconverted. */
  readonly gamut?: boolean | undefined;
  /** D265: the source is a canvas at the picture's effects size (a shader's), stretched over the content rect. */
  readonly stretch?: boolean | undefined;
  /** D265: a reveal shape (D133): its outline's rings in canvas px, filled even-odd, clipping the box. */
  readonly mask?: ReadonlyArray<ReadonlyArray<readonly [number, number]>> | undefined;
  /** D265: a blur effect (D136): Gaussian sigma in canvas px on the picture, inside its box. */
  readonly pictureBlur?: number | undefined;
  /** D265: a blur dissolve (D133): Gaussian sigma in canvas px on the whole box, its edges too. */
  readonly blur?: number | undefined;
}

export interface GpuSolid {
  readonly id: string;
  /** Premultiplied RGBA, 0–1. */
  readonly colour: readonly [number, number, number, number];
  readonly opacity: number;
}

/** A source's natural size (0 when it has no picture yet). */
const naturalSize = (s: GpuPicture["source"]) =>
  s instanceof HTMLVideoElement ? { w: s.videoWidth, h: s.videoHeight } : s instanceof HTMLImageElement ? { w: s.naturalWidth, h: s.naturalHeight } : { w: s.width, h: s.height };

/**
 * What a source shows now, so a paused one isn't uploaded again. A video's key names its file too:
 * a relinked file (same item and media ids, another src) paused at the same time is another picture.
 */
const frameKey = (s: GpuPicture["source"]) =>
  s instanceof HTMLVideoElement ? `v${s.currentSrc}|${s.currentTime}|${s.paused ? "p" : performance.now()}` : s instanceof HTMLImageElement ? `i${s.currentSrc}` : s.classList.contains("pv-scrub") ? `c${s.dataset.scrub}|${s.dataset.sample}` : `c${performance.now()}`;

/** Where a picture of size n lands in rect r with an object-fit (centred, as CSS's default object-position). */
export function fitInto(r: Rect, n: { w: number; h: number }, fit: string): Rect {
  if (fit === "fill" || !n.w || !n.h) return r;
  const k = fit === "cover" ? Math.max(r.w / n.w, r.h / n.h) : Math.min(r.w / n.w, r.h / n.h);
  const w = n.w * k;
  const h = n.h * k;
  return { x: r.x + (r.w - w) / 2, y: r.y + (r.h - h) / 2, w, h };
}

/** Parses a CSS colour (any form the browser takes) to premultiplied RGBA, or undefined for anything that isn't one plain colour. */
let probe: CanvasRenderingContext2D | null | undefined;
export function plainColour(css: string): readonly [number, number, number, number] | undefined {
  if (/gradient|url\(|,\s*(?:radial|linear|repeating)/i.test(css)) return undefined;
  probe ??= document.createElement("canvas").getContext("2d");
  if (!probe) return undefined;
  probe.fillStyle = "#010203";
  probe.fillStyle = css;
  const v = String(probe.fillStyle);
  if (v === "#010203" && !/^#010203$/i.test(css.trim())) return undefined;
  let rgba: Array<number>;
  const hex = /^#([0-9a-f]{6})$/i.exec(v);
  if (hex) rgba = [0, 2, 4].map((i) => parseInt(hex[1]!.slice(i, i + 2), 16) / 255).concat(1);
  else {
    const m = /^rgba?\(([^)]+)\)$/.exec(v);
    if (!m) return undefined;
    const parts = m[1]!.split(/[,\s/]+/).filter(Boolean).map(Number);
    rgba = [parts[0]! / 255, parts[1]! / 255, parts[2]! / 255, parts[3] ?? 1];
  }
  const a = rgba[3]!;
  return [rgba[0]! * a, rgba[1]! * a, rgba[2]! * a, a];
}

/**
 * Issue #82 (D241): the longest the GPU compositor keeps the frame on screen for a layer with nothing
 * to draw. Issue #12 (D265): down from 500 ms, now that the next clip's picture is uploaded while it
 * warms up (`prewarm`), so a hold is only the rare clip that had no frame by its cut.
 */
export const HOLD_MS = 150;

type Uniforms = Record<string, WebGLUniformLocation | null>;

export class GpuCompositor {
  /**
   * D265: the GL context draws on an offscreen surface; each run of consecutive GPU layers (in track
   * order, between layers the DOM draws: text, components, graphics, overlays) is handed to a canvas
   * of its own at that run's place in the stacking order (`flush`), so a layer the GPU can't draw
   * only interrupts the run instead of sending every layer above it to the DOM path. The first run's
   * canvas is `canvas`.
   */
  readonly canvas: HTMLCanvasElement;
  private readonly surface: OffscreenCanvas;
  private readonly runs: Array<HTMLCanvasElement> = [];
  private run = 0;
  private readonly gl: WebGL2RenderingContext;
  private program!: WebGLProgram;
  private vao!: WebGLVertexArrayObject;
  private quad!: WebGLBuffer;
  private u: Uniforms = {};
  // Each layer's last uploaded picture (`n`: its natural size), kept while the layer is drawn.
  private readonly textures = new Map<string, { tex: WebGLTexture; key: string; source?: GpuPicture["source"] | undefined; n: { w: number; h: number }; mip: boolean }>();
  // Each effects layer's last drawn picture (`drawn` once it holds one).
  private readonly fxTargets = new Map<string, { tex: WebGLTexture; fbo: WebGLFramebuffer; w: number; h: number; drawn?: boolean }>();
  // Issue #82 (D241): since when the frame on screen has been held (a layer had nothing to draw).
  private heldSince: number | undefined;
  private fx: FxRenderer | undefined;
  private used = new Set<string>();
  private canvasSize = { w: 1, h: 1 };
  // Canvas px per device px, and its inverse.
  private px = 1;
  private k = 1;
  private placeKey = "";
  private blurProgram!: WebGLProgram;
  private bu: Uniforms = {};
  // Whole-canvas targets for a layer drawn in steps (blurs), made at the backing size when needed.
  private targets: Array<{ tex: WebGLTexture; fbo: WebGLFramebuffer; w: number; h: number }> = [];
  // A reveal shape's coverage at the backing size (drawn by a 2D canvas: antialiased as the browser's clip-path).
  private maskTex: WebGLTexture | undefined;
  private maskCanvas: OffscreenCanvas | undefined;
  /** The context is lost: the preview draws on the DOM path until it's back. */
  lost = false;
  onRestored: (() => void) | undefined;

  private readonly parent: HTMLElement;

  constructor(parent: HTMLElement) {
    this.parent = parent;
    this.surface = new OffscreenCanvas(1, 1);
    this.canvas = this.runCanvas(0);
    const gl = this.surface.getContext("webgl2", { alpha: true, premultipliedAlpha: true, antialias: false, preserveDrawingBuffer: false });
    if (!gl) throw new Error("WebGL2 isn't available");
    this.gl = gl;
    this.init();
    this.surface.addEventListener("webglcontextlost", (e) => {
      e.preventDefault();
      this.lost = true;
    });
    this.surface.addEventListener("webglcontextrestored", () => {
      this.lost = false;
      this.textures.clear();
      this.fxTargets.clear();
      this.init();
      this.onRestored?.();
    });
  }

  private init() {
    const gl = this.gl;
    const shader = (type: number, src: string) => {
      const s = gl.createShader(type)!;
      gl.shaderSource(s, src);
      gl.compileShader(s);
      if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) throw new Error(`compositor: ${gl.getShaderInfoLog(s)}`);
      return s;
    };
    const program = gl.createProgram()!;
    gl.attachShader(program, shader(gl.VERTEX_SHADER, VERTEX));
    gl.attachShader(program, shader(gl.FRAGMENT_SHADER, FRAGMENT));
    gl.linkProgram(program);
    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) throw new Error(`compositor: ${gl.getProgramInfoLog(program)}`);
    this.program = program;
    this.u = {};
    for (const name of ["uRect", "uBacking", "uK", "uTex", "uSolid", "uColour", "uBox", "uRadius", "uImg", "uEdge", "uOpacity", "uWipe", "uIris", "uPx", "uFlipY", "uSoft", "uGamut", "uMasked", "uMask", "uBackingF"]) this.u[name] = gl.getUniformLocation(program, name);
    const blur = gl.createProgram()!;
    gl.attachShader(blur, shader(gl.VERTEX_SHADER, BLUR_VERTEX));
    gl.attachShader(blur, shader(gl.FRAGMENT_SHADER, BLUR_FRAGMENT));
    gl.linkProgram(blur);
    if (!gl.getProgramParameter(blur, gl.LINK_STATUS)) throw new Error(`compositor: ${gl.getProgramInfoLog(blur)}`);
    this.blurProgram = blur;
    this.bu = {};
    for (const name of ["uTex", "uDir", "uSigma"]) this.bu[name] = gl.getUniformLocation(blur, name);
    this.targets = [];
    this.maskTex = undefined;
    this.vao = gl.createVertexArray()!;
    gl.bindVertexArray(this.vao);
    this.quad = gl.createBuffer()!;
    gl.bindBuffer(gl.ARRAY_BUFFER, this.quad);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([0, 0, 1, 0, 0, 1, 1, 1]), gl.STATIC_DRAW);
    gl.enableVertexAttribArray(0);
    gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);
    gl.bindVertexArray(null);
    this.fx = undefined;
  }

  static available(): boolean {
    try {
      return !!document.createElement("canvas").getContext("webgl2");
    } catch {
      return false;
    }
  }

  /** The backing size for a placement (k capped: a zoomed-in preview isn't drawn past 2 device px per canvas px). */
  private static layout(place: GpuPlace) {
    const k = Math.max(0.1, Math.min(2, Number.isFinite(place.k) ? place.k : 1));
    return { k, bw: Math.max(1, Math.round(place.w * k)), bh: Math.max(1, Math.round(place.h * k)) };
  }

  /** Starts a frame (see GpuPlace). */
  begin(place: GpuPlace) {
    const gl = this.gl;
    const { k, bw, bh } = GpuCompositor.layout(place);
    if (this.surface.width !== bw) this.surface.width = bw;
    if (this.surface.height !== bh) this.surface.height = bh;
    this.placeKey = `${k}|${bw}|${bh}`;
    this.canvasSize = { w: place.w, h: place.h };
    this.run = 0;
    this.k = k;
    this.px = 1 / k;
    this.used = new Set();
    this.bindOwn();
    gl.clearColor(0, 0, 0, 0);
    gl.clear(gl.COLOR_BUFFER_BIT);
  }

  /** The compositor's own state (after FxRenderer used the context). */
  private bindOwn() {
    const gl = this.gl;
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    gl.viewport(0, 0, this.surface.width, this.surface.height);
    gl.useProgram(this.program);
    gl.bindVertexArray(this.vao);
    gl.enable(gl.BLEND);
    gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA);
    gl.uniform2f(this.u.uBacking!, this.surface.width, this.surface.height);
    gl.uniform1f(this.u.uK!, this.k);
    gl.uniform1f(this.u.uPx!, this.px);
    gl.uniform1i(this.u.uTex!, 0);
    gl.uniform1i(this.u.uMask!, 1);
    gl.uniform2f(this.u.uBackingF!, this.surface.width, this.surface.height);
    gl.uniform1i(this.u.uMasked!, 0);
  }

  private texture(id: string) {
    const gl = this.gl;
    let t = this.textures.get(id);
    if (!t) {
      const tex = gl.createTexture()!;
      gl.bindTexture(gl.TEXTURE_2D, tex);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
      t = { tex, key: "", n: { w: 0, h: 0 }, mip: false };
      this.textures.set(id, t);
    }
    return t;
  }

  /**
   * Uploads a source's current picture into layer `id`'s texture (when it changed). Stills get
   * mipmaps (the DOM path rasterises an <img> with them; measured SSIM 0.997 vs 0.988 without);
   * moving pictures don't (the browser's video drawing has none, and they'd cost every frame).
   */
  private upload(id: string, source: GpuPicture["source"]) {
    const gl = this.gl;
    const t = this.texture(id);
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, t.tex);
    const key = frameKey(source);
    // Another element under the same layer id (another project's clip with the same ids) is uploaded
    // even when its key matches the old one's.
    if (t.key === key && t.source === source) return t;
    gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, false);
    gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, true);
    gl.pixelStorei(gl.UNPACK_COLORSPACE_CONVERSION_WEBGL, gl.BROWSER_DEFAULT_WEBGL);
    try {
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, source);
    } catch {
      return undefined;
    }
    const mip = source instanceof HTMLImageElement;
    if (mip) gl.generateMipmap(gl.TEXTURE_2D);
    if (mip !== t.mip) gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, mip ? gl.LINEAR_MIPMAP_LINEAR : gl.LINEAR);
    t.mip = mip;
    t.key = key;
    t.source = source;
    t.n = naturalSize(source);
    return t;
  }

  private fxTarget(id: string, w: number, h: number) {
    const gl = this.gl;
    let t = this.fxTargets.get(id);
    if (!t) {
      const tex = gl.createTexture()!;
      t = { tex, fbo: gl.createFramebuffer()!, w: 0, h: 0 };
      this.fxTargets.set(id, t);
    }
    if (t.w !== w || t.h !== h) {
      t.w = w;
      t.h = h;
      t.drawn = false;
      gl.activeTexture(gl.TEXTURE0);
      gl.bindTexture(gl.TEXTURE_2D, t.tex);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, w, h, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
      gl.bindFramebuffer(gl.FRAMEBUFFER, t.fbo);
      gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, t.tex, 0);
    }
    return t;
  }

  /** A canvas coordinate on the nearest whole device pixel. */
  private snap(v: number) {
    return Math.round(v * this.k) / this.k;
  }

  private clip(rect: Rect & { radius?: number }, wipe: number | undefined, iris: GpuPicture["iris"]) {
    const gl = this.gl;
    // The box's straight edges on whole device pixels, hard, as the browser clips a box (measured).
    const x0 = this.snap(rect.x);
    const y0 = this.snap(rect.y);
    const box = { x: x0, y: y0, w: this.snap(rect.x + rect.w) - x0, h: this.snap(rect.y + rect.h) - y0 };
    gl.uniform4f(this.u.uRect!, box.x, box.y, box.w, box.h);
    gl.uniform4f(this.u.uBox!, box.x, box.y, box.w, box.h);
    gl.uniform1f(this.u.uRadius!, rect.radius ?? 0);
    gl.uniform1f(this.u.uWipe!, wipe === undefined ? 1e9 : this.canvasSize.w * wipe);
    gl.uniform3f(this.u.uIris!, iris?.x ?? 0, iris?.y ?? 0, iris ? iris.r : -1);
  }

  /** Whether a source has a picture to read now (a seeking <video> drops below HAVE_CURRENT_DATA). */
  private static ready(s: GpuPicture["source"]) {
    const n = naturalSize(s);
    return (s instanceof HTMLVideoElement ? s.readyState >= 2 : true) && n.w > 0 && n.h > 0;
  }

  /** Issue #82 (D241): the layer's last picture, when it has one, for while its source has none (a <video> seeking). */
  private held(p: GpuPicture): { tex: WebGLTexture; img: Pic; flip: boolean; soft: boolean; gamut: boolean } | undefined {
    const f = p.fx ? this.fxTargets.get(p.id) : undefined;
    if (f?.drawn) return { tex: f.tex, img: p.box.content, flip: true, soft: true, gamut: false };
    const t = this.textures.get(p.id);
    if (!t?.key || !t.n.w) return undefined;
    return { tex: t.tex, img: p.stretch ? p.box.content : this.pictureRect(p, t.n), flip: false, soft: !t.mip, gamut: true };
  }

  /**
   * Where a plain picture of natural size n lands: its fit in the content rect, at its exact
   * sub-pixel place (D265: snapping it to device or canvas pixels measured worse against the DOM
   * path). A picture edge within a device pixel of its box's edge reaches it, so the box clips it
   * there, hard, as the browser does (only the antialiased edge moves, not the picture).
   */
  private pictureRect(p: GpuPicture, n: { w: number; h: number }): Pic {
    const img = fitInto(p.box.content, n, p.box.fit);
    const b = p.box;
    const near = (a: number, c: number) => Math.abs(a - c) < this.px;
    let x0 = img.x;
    let y0 = img.y;
    let x1 = img.x + img.w;
    let y1 = img.y + img.h;
    if (near(x0, b.x)) x0 = Math.min(x0, b.x - this.px);
    if (near(y0, b.y)) y0 = Math.min(y0, b.y - this.px);
    if (near(x1, b.x + b.w)) x1 = Math.max(x1, b.x + b.w + this.px);
    if (near(y1, b.y + b.h)) y1 = Math.max(y1, b.y + b.h + this.px);
    return { ...img, edge: { x: x0, y: y0, w: x1 - x0, h: y1 - y0 } };
  }

  /**
   * Issue #12 (D265): the next clip's picture, uploaded while it warms up out of sight (not drawn),
   * so at its cut the layer has a picture to hold while its <video> still seeks: before, a layer
   * with nothing of its own held the whole frame (every GPU layer frozen) up to 500 ms.
   */
  prewarm(id: string, source: HTMLVideoElement) {
    if (this.lost || !GpuCompositor.ready(source)) return;
    this.used.add(id);
    // Paused at its first frame: uploaded once. Running (just before its cut): every frame, so the
    // picture held at the cut is its latest.
    this.upload(id, source);
  }

  /**
   * Issue #82 (D241): whether to leave the frame on screen as it is instead of drawing this one: a
   * layer's source has no picture yet and the layer none of its own to hold (a clip at a cut whose
   * <video> never had a frame while it warmed up). The DOM path shows a seeking <video>'s last frame
   * and the clip under it; drawn here it was a transparent hole: black. Held at most HOLD_MS, and
   * never across a size change (that clears the canvas anyway).
   */
  holds(pictures: Iterable<GpuPicture>, place: GpuPlace): boolean {
    if (this.lost) return false;
    const l = GpuCompositor.layout(place);
    const same = this.placeKey === `${l.k}|${l.bw}|${l.bh}`;
    let stuck = false;
    for (const p of pictures)
      if (!GpuCompositor.ready(p.source) && !this.held(p)) {
        stuck = true;
        // Which layer and why, for the tests (issue #82).
        const s = p.source;
        this.canvas.dataset.heldBy = `${p.id}:${s instanceof HTMLVideoElement ? `rs${s.readyState}${s.seeking ? "s" : ""}@${s.currentTime.toFixed(3)}` : s.tagName}`;
      }
    const now = performance.now();
    if (!stuck || !same || this.used.size === 0) {
      this.heldSince = undefined;
      return false;
    }
    this.heldSince ??= now;
    if (now - this.heldSince > HOLD_MS) return false;
    return true;
  }

  /** A clip's picture (with its effects when it has some); its last one while its source has none. False when there's nothing to draw. */
  drawPicture(p: GpuPicture): boolean {
    const gl = this.gl;
    if (this.lost) return false;
    if (!GpuCompositor.ready(p.source)) {
      const h = this.held(p);
      if (!h) return false;
      this.used.add(p.id);
      this.layer(p, { tex: h.tex, img: h.img, flip: h.flip, soft: h.soft, gamut: h.gamut && !!p.gamut });
      return true;
    }
    this.used.add(p.id);
    if (p.fx) {
      this.fx ??= new FxRenderer(this.surface as unknown as HTMLCanvasElement, gl);
      const { w, h } = p.fx.size;
      const target = this.fxTarget(p.id, w, h);
      gl.disable(gl.BLEND);
      gl.bindVertexArray(null);
      const ok = this.fx.render(p.source, p.fx.fx, w, h, p.fx.t, 1, undefined, target.fbo);
      this.bindOwn();
      if (!ok) return this.drawPicture({ ...p, fx: undefined });
      target.drawn = true;
      // The effects canvas is stretched over the content rect (as the DOM path's canvas, object-fit fill).
      this.layer(p, { tex: target.tex, img: p.box.content, flip: true, soft: true, gamut: false });
      return true;
    }
    const t = this.upload(p.id, p.source);
    if (!t) return false;
    this.layer(p, { tex: t.tex, img: p.stretch ? p.box.content : this.pictureRect(p, t.n), flip: false, soft: !t.mip, gamut: !!p.gamut });
    return true;
  }

  /**
   * Draws a layer from its picture: straight onto the canvas, or (D265) in steps through whole-canvas
   * targets when it's blurred, as the DOM path stacks its CSS: a blur effect on the picture, then the
   * box's clip (corners, wipe, iris, reveal shape) and opacity, then a blur dissolve on all of it.
   */
  private layer(p: GpuPicture, src: Src) {
    const gl = this.gl;
    const inner = (p.pictureBlur ?? 0) * this.k;
    const outer = (p.blur ?? 0) * this.k;
    if (p.mask) this.drawMask(p.mask);
    if (inner < 0.3 && outer < 0.3) return this.blit(p, src, true, p.opacity, !!p.mask);
    let from = src;
    if (inner >= 0.3) {
      // Transparent past the picture's own edges (not reaching its box's), as the browser's CSS blur: its edges fade.
      this.bindTarget(this.target(0));
      const { edge: _, ...img } = src.img;
      this.blit(p, { ...src, img }, false, 1, false);
      this.blurTarget(0, inner);
      from = this.whole(0);
    }
    if (outer >= 0.3) {
      const i = inner >= 0.3 ? 1 : 0;
      this.bindTarget(this.target(i));
      this.blit(p, from, true, p.opacity, !!p.mask);
      this.blurTarget(i, outer);
      this.bindTarget(undefined);
      gl.uniform1i(this.u.uMasked!, 0);
      return this.blit(WHOLE(this.canvasSize), this.whole(i), false, 1, false);
    }
    this.bindTarget(undefined);
    this.blit(p, from, true, p.opacity, !!p.mask);
  }

  /** Target i's picture as a source covering the canvas, one texel per backing pixel. */
  private whole(i: number, edge: Rect = EVERYWHERE): Src {
    const t = this.targets[i]!;
    return { tex: t.tex, img: { x: 0, y: 0, w: t.w / this.k, h: t.h / this.k, edge }, flip: true, soft: false, gamut: false };
  }

  /** A whole-canvas target at the backing size. */
  private target(i: number) {
    const gl = this.gl;
    const w = this.surface.width;
    const h = this.surface.height;
    let t = this.targets[i];
    if (!t) {
      t = { tex: gl.createTexture()!, fbo: gl.createFramebuffer()!, w: 0, h: 0 };
      this.targets[i] = t;
    }
    if (t.w !== w || t.h !== h) {
      t.w = w;
      t.h = h;
      gl.activeTexture(gl.TEXTURE0);
      gl.bindTexture(gl.TEXTURE_2D, t.tex);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, w, h, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
      gl.bindFramebuffer(gl.FRAMEBUFFER, t.fbo);
      gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, t.tex, 0);
    }
    return t;
  }

  /** Draws into a target (cleared) or, undefined, the canvas. */
  private bindTarget(t: { fbo: WebGLFramebuffer } | undefined) {
    const gl = this.gl;
    gl.bindFramebuffer(gl.FRAMEBUFFER, t?.fbo ?? null);
    gl.viewport(0, 0, this.surface.width, this.surface.height);
    if (t) {
      gl.clearColor(0, 0, 0, 0);
      gl.clear(gl.COLOR_BUFFER_BIT);
    }
  }

  /** Target i blurred (Gaussian, sigma in backing px): two passes through target 2, repeated at sigma/√n past what one pass covers (40). */
  private blurTarget(i: number, sigma: number) {
    const n = Math.ceil((sigma / 40) ** 2);
    for (let j = 0; j < n; j++) this.blurOnce(i, sigma / Math.sqrt(n));
  }

  private blurOnce(i: number, sigma: number) {
    const gl = this.gl;
    const a = this.targets[i]!;
    const tmp = this.target(2);
    gl.useProgram(this.blurProgram);
    gl.disable(gl.BLEND);
    gl.uniform1i(this.bu.uTex!, 0);
    gl.uniform1f(this.bu.uSigma!, sigma);
    gl.activeTexture(gl.TEXTURE0);
    for (const [from, to, dir] of [[a, tmp, [1, 0]], [tmp, a, [0, 1]]] as const) {
      gl.bindFramebuffer(gl.FRAMEBUFFER, to.fbo);
      gl.bindTexture(gl.TEXTURE_2D, from.tex);
      gl.uniform2f(this.bu.uDir!, dir[0], dir[1]);
      gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
    }
    this.bindOwn();
  }

  /** A reveal shape's coverage into the mask texture (unit 1), even-odd and antialiased. */
  private drawMask(rings: NonNullable<GpuPicture["mask"]>) {
    const gl = this.gl;
    const w = this.surface.width;
    const h = this.surface.height;
    if (!this.maskCanvas || this.maskCanvas.width !== w || this.maskCanvas.height !== h) this.maskCanvas = new OffscreenCanvas(w, h);
    const ctx = this.maskCanvas.getContext("2d")!;
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.clearRect(0, 0, w, h);
    ctx.setTransform(this.k, 0, 0, this.k, 0, 0);
    ctx.beginPath();
    for (const ring of rings) {
      ring.forEach(([x, y], i) => (i ? ctx.lineTo(x, y) : ctx.moveTo(x, y)));
      ctx.closePath();
    }
    ctx.fillStyle = "#fff";
    ctx.fill("evenodd");
    if (!this.maskTex) {
      this.maskTex = gl.createTexture()!;
      gl.activeTexture(gl.TEXTURE1);
      gl.bindTexture(gl.TEXTURE_2D, this.maskTex);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    }
    gl.activeTexture(gl.TEXTURE1);
    gl.bindTexture(gl.TEXTURE_2D, this.maskTex);
    gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, false);
    gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, false);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, this.maskCanvas);
    gl.activeTexture(gl.TEXTURE0);
  }

  /** Draws a picture at `src.img`, clipped to its box (or not, for a blur effect's first step). */
  private blit(p: GpuPicture, src: Src, clipped: boolean, opacity: number, masked: boolean) {
    const gl = this.gl;
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, src.tex);
    if (clipped) this.clip(p.box, p.wipe, p.iris);
    else this.clip({ x: -1e5, y: -1e5, w: 2e5, h: 2e5 }, undefined, undefined);
    gl.uniform1i(this.u.uMasked!, masked ? 1 : 0);
    gl.uniform1i(this.u.uSolid!, 0);
    const img = src.img;
    gl.uniform4f(this.u.uImg!, img.x, img.y, img.w, img.h);
    const e = img.edge ?? img;
    gl.uniform4f(this.u.uEdge!, e.x, e.y, e.w, e.h);
    gl.uniform1f(this.u.uOpacity!, opacity);
    gl.uniform1i(this.u.uFlipY!, src.flip ? 1 : 0);
    gl.uniform1f(this.u.uSoft!, src.soft ? SOFT : 0);
    gl.uniform1i(this.u.uGamut!, src.gamut ? 1 : 0);
    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
    if (masked) gl.uniform1i(this.u.uMasked!, 0);
  }

  /** A plain-colour background over the whole canvas. */
  drawSolid(s: GpuSolid) {
    const gl = this.gl;
    if (this.lost) return;
    this.clip({ x: 0, y: 0, w: this.canvasSize.w, h: this.canvasSize.h }, undefined, undefined);
    gl.uniform1i(this.u.uMasked!, 0);
    gl.uniform1i(this.u.uSolid!, 1);
    gl.uniform4f(this.u.uColour!, ...s.colour);
    gl.uniform1f(this.u.uOpacity!, s.opacity);
    gl.uniform1i(this.u.uFlipY!, 0);
    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
  }

  /** A run's canvas (made on first use, at the bottom of the parent until placed). */
  private runCanvas(i: number) {
    let c = this.runs[i];
    if (!c) {
      c = document.createElement("canvas");
      c.className = "pv-gpu";
      c.dataset.gpuCompositor = String(i);
      c.style.visibility = "hidden";
      this.runs[i] = c;
      this.parent.prepend(c);
    }
    return c;
  }

  /**
   * Shows what was drawn since `begin` (or the last flush) as the next run, its canvas at z-index
   * `z` (any of its layers' own: the DOM elements of a GPU layer are transparent), and starts the
   * next run on a clear surface.
   */
  flush(z: number) {
    if (this.lost) return;
    const c = this.runCanvas(this.run++);
    const w = this.surface.width;
    const h = this.surface.height;
    if (c.width !== w) c.width = w;
    if (c.height !== h) c.height = h;
    // In the canvas's own px (the layer is scaled with it): one backing pixel per device pixel, not
    // 100 % of the canvas (1080 × k is rarely a whole number of rows, and a canvas stretched by a
    // fraction of a pixel is resampled once more by the browser, softening every edge).
    const st = c.style;
    const cw = `${w / this.k}px`;
    const ch = `${h / this.k}px`;
    if (st.width !== cw) st.width = cw;
    if (st.height !== ch) st.height = ch;
    const zi = String(z);
    if (st.zIndex !== zi) st.zIndex = zi;
    if (st.visibility) st.visibility = "";
    // Handing the bitmap over clears the surface for the next run.
    (c.getContext("bitmaprenderer") as ImageBitmapRenderingContext).transferFromImageBitmap(this.surface.transferToImageBitmap());
    this.bindOwn();
  }

  /** Ends a frame: run canvases not used this time are hidden, textures of layers no longer drawn (or warmed up) go. */
  end() {
    const gl = this.gl;
    for (let i = this.run; i < this.runs.length; i++) this.hideRun(i);
    for (const [id, t] of [...this.textures]) {
      if (this.used.has(id)) continue;
      gl.deleteTexture(t.tex);
      this.textures.delete(id);
    }
    for (const [id, t] of [...this.fxTargets]) {
      if (this.used.has(id)) continue;
      gl.deleteTexture(t.tex);
      gl.deleteFramebuffer(t.fbo);
      this.fxTargets.delete(id);
    }
  }

  private hideRun(i: number) {
    const c = this.runs[i]!;
    if (c.style.visibility === "hidden") return;
    c.style.visibility = "hidden";
    (c.getContext("bitmaprenderer") as ImageBitmapRenderingContext).transferFromImageBitmap(null);
  }

  /** Nothing drawn (every layer back on the DOM path). */
  clear() {
    if (this.lost) return;
    this.run = 0;
    this.used = new Set();
    this.end();
  }

  dispose() {
    this.gl.getExtension("WEBGL_lose_context")?.loseContext();
    for (const c of this.runs) c.remove();
  }
}
