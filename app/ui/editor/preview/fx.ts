// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// A clip's picture effects in the preview (D136): a WebGL2 pass over the clip's picture, at the size
// the effects work at (core fxPictureSize: the picture at its fit, before any zoom), drawn into a
// <canvas> that sits where the <video> or <img> would (which stays, hidden, as the source: the mix,
// the clock and the tests keep reading it). The maths follow the export's ffmpeg filters
// (exportPlan.ts fxGraph) step for step, in the same order: mirror, shake (zoomed in, a whole-pixel
// window moved by core shakeWindow), pixelate (block averages from the top-left, partial blocks
// averaged over what they cover, as pixelize), colour (core's table read with trilinear
// interpolation, as lut3d interp=trilinear), posterize (floor), RGB split (red moved right, blue
// left, edges smeared, as rgbashift), sharpen (core's 3×3 kernel over 16, edges mirrored, as
// convolution), glow (a thresholded copy, Gaussian-blurred, screened on). Blur is CSS on the canvas
// (D133's maths); vignette, grain and letterbox are core's HTML over it.
import { colourLut, shakeWindow, shakeZoom, type ClipFx, type ColourLut } from "@/app/lib/editor/core/visual/fx";

const VERTEX = `#version 300 es
in vec2 aPos;
void main() { gl_Position = vec4(aPos, 0.0, 1.0); }`;

// The picture as the effects see it: where output pixel q (centre, picture px) samples the source,
// after the shake's window and the mirror.
const SOURCE = `
uniform sampler2D uSrc;
uniform vec2 uSize;
uniform vec2 uZoomed;
uniform vec2 uWin;
uniform bool uMirror;
vec4 picture(vec2 q) {
  vec2 uv = (q + uWin) / uZoomed;
  if (uMirror) uv.x = 1.0 - uv.x;
  return texture(uSrc, uv);
}`;

/** Block averages for pixelate: one texel per block. */
const PIXELATE = `#version 300 es
precision highp float;
${SOURCE}
uniform int uBlock;
out vec4 outColor;
void main() {
  ivec2 b = ivec2(gl_FragCoord.xy);
  vec4 sum = vec4(0.0);
  float count = 0.0;
  for (int y = 0; y < 256; y++) {
    if (y >= uBlock) break;
    for (int x = 0; x < 256; x++) {
      if (x >= uBlock) break;
      vec2 q = vec2(b*uBlock + ivec2(x, y)) + 0.5;
      if (q.x > uSize.x || q.y > uSize.y) continue;
      sum += picture(q);
      count += 1.0;
    }
  }
  outColor = sum / max(count, 1.0);
}`;

/** The picture, pixelated or not, through the colour table and posterize. */
const COLOUR = `#version 300 es
precision highp float;
precision highp sampler3D;
${SOURCE}
uniform sampler2D uBlocks;
uniform int uBlock;
uniform sampler3D uLut;
uniform int uLutN;
uniform float uLevels;
uniform bool uFlipY;
out vec4 outColor;
vec3 lut(vec3 c) {
  float n = float(uLutN - 1);
  vec3 x = clamp(c, 0.0, 1.0)*n;
  ivec3 i = min(ivec3(floor(x)), ivec3(uLutN - 2));
  vec3 f = x - vec3(i);
  vec3 c000 = texelFetch(uLut, i, 0).rgb;
  vec3 c100 = texelFetch(uLut, i + ivec3(1, 0, 0), 0).rgb;
  vec3 c010 = texelFetch(uLut, i + ivec3(0, 1, 0), 0).rgb;
  vec3 c110 = texelFetch(uLut, i + ivec3(1, 1, 0), 0).rgb;
  vec3 c001 = texelFetch(uLut, i + ivec3(0, 0, 1), 0).rgb;
  vec3 c101 = texelFetch(uLut, i + ivec3(1, 0, 1), 0).rgb;
  vec3 c011 = texelFetch(uLut, i + ivec3(0, 1, 1), 0).rgb;
  vec3 c111 = texelFetch(uLut, i + ivec3(1, 1, 1), 0).rgb;
  vec3 c00 = mix(c000, c100, f.r);
  vec3 c10 = mix(c010, c110, f.r);
  vec3 c01 = mix(c001, c101, f.r);
  vec3 c11 = mix(c011, c111, f.r);
  return mix(mix(c00, c10, f.g), mix(c01, c11, f.g), f.b);
}
void main() {
  // Rows are numbered from the top, as the export's are.
  vec2 q = vec2(gl_FragCoord.x, uFlipY ? uSize.y - gl_FragCoord.y : gl_FragCoord.y);
  vec4 c = uBlock > 1 ? texelFetch(uBlocks, ivec2(floor(q / float(uBlock))), 0) : picture(q);
  vec3 rgb = c.rgb;
  if (uLutN > 1) rgb = lut(rgb);
  if (uLevels > 1.0) rgb = min(floor(clamp(rgb, 0.0, 1.0)*uLevels), uLevels - 1.0) / (uLevels - 1.0);
  outColor = vec4(rgb, c.a);
}`;

/** RGB split then sharpen, on whole pixels (rows from the top, like everything after COLOUR). */
const SPATIAL = `#version 300 es
precision highp float;
uniform sampler2D uImg;
uniform ivec2 uDim;
uniform int uSplit;
uniform int uSharp;
out vec4 outColor;
vec3 at(ivec2 p) {
  int xr = clamp(p.x - uSplit, 0, uDim.x - 1);
  int xb = clamp(p.x + uSplit, 0, uDim.x - 1);
  return vec3(texelFetch(uImg, ivec2(xr, p.y), 0).r, texelFetch(uImg, p, 0).g, texelFetch(uImg, ivec2(xb, p.y), 0).b);
}
// Mirrored at the edges (-1 reads 1), as ffmpeg's convolution.
ivec2 mirror(ivec2 p) {
  ivec2 m = p;
  if (m.x < 0) m.x = -m.x;
  if (m.y < 0) m.y = -m.y;
  if (m.x >= uDim.x) m.x = 2*uDim.x - 2 - m.x;
  if (m.y >= uDim.y) m.y = 2*uDim.y - 2 - m.y;
  return clamp(m, ivec2(0), uDim - 1);
}
void main() {
  ivec2 p = ivec2(gl_FragCoord.xy);
  vec3 c = at(p);
  if (uSharp > 0) {
    float a = float(uSharp) / 16.0;
    vec3 n = at(mirror(p + ivec2(-1, 0))) + at(mirror(p + ivec2(1, 0))) + at(mirror(p + ivec2(0, -1))) + at(mirror(p + ivec2(0, 1)));
    c = clamp(c*(1.0 + 4.0*a) - a*n, 0.0, 1.0);
  }
  // Alpha stays as it is (rgbashift and the convolution's alpha plane leave it).
  outColor = vec4(c, texelFetch(uImg, p, 0).a);
}`;

/** Glow, one direction of its Gaussian at a reduced size (the first pass thresholds too). */
const GLOW = `#version 300 es
precision highp float;
uniform sampler2D uImg;
uniform vec2 uImgSize;
uniform vec2 uOut;
uniform vec2 uDir;
uniform float uSigma;
uniform bool uThreshold;
uniform float uLow;
uniform float uAmount;
out vec4 outColor;
vec3 src(vec2 p) {
  vec3 c = texture(uImg, p / uOut).rgb;
  return uThreshold ? clamp((c - uLow) / (1.0 - uLow), 0.0, 1.0)*uAmount : c;
}
void main() {
  vec2 p = gl_FragCoord.xy;
  int r = int(ceil(3.0*uSigma));
  vec3 sum = vec3(0.0);
  float wsum = 0.0;
  for (int i = -192; i <= 192; i++) {
    if (i < -r || i > r) continue;
    float w = exp(-0.5*float(i*i) / (uSigma*uSigma));
    sum += src(p + uDir*float(i))*w;
    wsum += w;
  }
  outColor = vec4(sum / wsum, 1.0);
}`;

/** The result, the glow screened on, onto the canvas (flipped back: the canvas's rows go up). */
const FINAL = `#version 300 es
precision highp float;
uniform sampler2D uImg;
uniform sampler2D uGlow;
uniform bool uUseGlow;
uniform vec2 uDim;
out vec4 outColor;
void main() {
  vec2 q = vec2(gl_FragCoord.x, uDim.y - gl_FragCoord.y);
  vec4 px = texelFetch(uImg, ivec2(q), 0);
  vec3 c = px.rgb;
  if (uUseGlow) {
    vec3 g = texture(uGlow, q / uDim).rgb;
    c = 1.0 - (1.0 - c)*(1.0 - g);
  }
  // The source's alpha kept (a still's transparency), premultiplied for the canvas.
  outColor = vec4(c*px.a, px.a);
}`;

/**
 * Issue #55: frames each <video> has presented (counted by the preview's requestVideoFrameCallback),
 * so a playing clip's effects are drawn once per new frame, not on every animation frame.
 */
export const presentedFrames = new WeakMap<HTMLVideoElement, number>();

/** What a source shows now: a <video>'s presented frame (or its time), an <img>'s file; undefined for a canvas. */
export const frameKeyOf = (source: TexImageSource): string | undefined =>
  source instanceof HTMLVideoElement ? (presentedFrames.get(source) ? `p${presentedFrames.get(source)}` : `v${source.currentTime}`) : source instanceof HTMLImageElement ? `i${source.currentSrc}` : undefined;

type Program = { readonly program: WebGLProgram; readonly uniforms: Map<string, WebGLUniformLocation | null> };
type Target = { readonly tex: WebGLTexture; readonly fbo: WebGLFramebuffer; w: number; h: number };

/** One WebGL2 context drawing a clip's picture with its effects into its canvas. */
export class FxRenderer {
  readonly canvas: HTMLCanvasElement;
  private readonly gl: WebGL2RenderingContext;
  private readonly programs = new Map<string, Program>();
  private src: WebGLTexture;
  private lutTex: WebGLTexture;
  private lutKey = "";
  private readonly targets = new Map<string, Target>();
  private readonly float: boolean;
  /** Whether the last draw had a source frame to read. */
  drawn = false;

  /** The context is lost (the GPU reset, or too many contexts): nothing draws until it's restored. */
  lost = false;
  /** Called when a lost context comes back (everything has been rebuilt): draw again. */
  onRestored: (() => void) | undefined;
  // What the last draw was made of: the same again is skipped.
  private last = "";
  private buf: WebGLBuffer | null = null;
  /** D221: where the last pass draws: the canvas (null), or a framebuffer of the GPU compositor's (rows bottom-up, as on a canvas). */
  private output: WebGLFramebuffer | null = null;
  /** D221: drawing in the GPU compositor's context (into its framebuffers), not a canvas of its own. */
  private readonly shared: boolean;

  constructor(canvas = document.createElement("canvas"), shared?: WebGL2RenderingContext) {
    this.canvas = canvas;
    this.shared = !!shared;
    const gl = shared ?? canvas.getContext("webgl2", { preserveDrawingBuffer: true, premultipliedAlpha: true, antialias: false, alpha: true });
    if (!gl) throw new Error("WebGL2 isn't available");
    this.gl = gl;
    this.float = !!gl.getExtension("EXT_color_buffer_float");
    this.src = this.lutTex = gl.createTexture()!;
    this.init();
    canvas.addEventListener("webglcontextlost", (e) => {
      e.preventDefault();
      this.lost = true;
      this.drawn = false;
    });
    canvas.addEventListener("webglcontextrestored", () => {
      this.lost = false;
      this.init();
      this.onRestored?.();
    });
  }

  /** Buffers, textures and programs (again after a lost context). */
  private init() {
    const gl = this.gl;
    this.programs.clear();
    this.targets.clear();
    this.lutKey = "";
    this.last = "";
    const buf = gl.createBuffer();
    this.buf = buf;
    gl.bindBuffer(gl.ARRAY_BUFFER, buf);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 3, -1, -1, 3]), gl.STATIC_DRAW);
    this.src = this.texture2d(gl.LINEAR);
    this.lutTex = gl.createTexture()!;
  }

  /** Issue #55: compiles the programs `fx` draws with now (a clip about to start), not in the frame it starts in. */
  warm(fx: ClipFx) {
    if (this.lost || this.gl.isContextLost()) return;
    if (fx.pixelate) this.program("pixelate", PIXELATE);
    this.program("colour", COLOUR);
    if (fx.rgbSplit || fx.sharpen) this.program("spatial", SPATIAL);
    if (fx.glow) this.program("glow", GLOW);
    this.program("final", FINAL);
  }

  /** Whether WebGL2 can be had here (the panel and preview fall back to the plain picture without it). */
  static available(): boolean {
    try {
      return !!document.createElement("canvas").getContext("webgl2");
    } catch {
      return false;
    }
  }

  /** Disposed on purpose (its context lost by us, not taken away). */
  disposed = false;

  dispose() {
    this.disposed = true;
    // A shared context is the compositor's to lose.
    if (!this.shared) this.gl.getExtension("WEBGL_lose_context")?.loseContext();
  }

  private texture2d(filter: number) {
    const gl = this.gl;
    const tex = gl.createTexture()!;
    gl.activeTexture(gl.TEXTURE7);
    gl.bindTexture(gl.TEXTURE_2D, tex);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, filter);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, filter);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    return tex;
  }

  private program(name: string, fragment: string): Program {
    const known = this.programs.get(name);
    if (known) return known;
    const gl = this.gl;
    const shader = (type: number, source: string) => {
      const s = gl.createShader(type)!;
      gl.shaderSource(s, source);
      gl.compileShader(s);
      if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) throw new Error(`${name}: ${gl.getShaderInfoLog(s)}`);
      return s;
    };
    const program = gl.createProgram()!;
    gl.attachShader(program, shader(gl.VERTEX_SHADER, VERTEX));
    gl.attachShader(program, shader(gl.FRAGMENT_SHADER, fragment));
    gl.bindAttribLocation(program, 0, "aPos");
    gl.linkProgram(program);
    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) throw new Error(`${name}: ${gl.getProgramInfoLog(program)}`);
    const p = { program, uniforms: new Map<string, WebGLUniformLocation | null>() };
    this.programs.set(name, p);
    return p;
  }

  private use(p: Program, values: Record<string, number | boolean | ReadonlyArray<number> | { int: number | ReadonlyArray<number> }>) {
    const gl = this.gl;
    gl.useProgram(p.program);
    for (const [k, v] of Object.entries(values)) {
      if (!p.uniforms.has(k)) p.uniforms.set(k, gl.getUniformLocation(p.program, k));
      const at = p.uniforms.get(k)!;
      if (at === null) continue;
      if (typeof v === "boolean") gl.uniform1i(at, v ? 1 : 0);
      else if (typeof v === "number") gl.uniform1f(at, v);
      else if ("int" in v) {
        if (typeof v.int === "number") gl.uniform1i(at, v.int);
        else gl.uniform2i(at, v.int[0]!, v.int[1]!);
      } else if (v.length === 2) gl.uniform2f(at, v[0]!, v[1]!);
    }
  }

  private target(name: string, w: number, h: number, filter: number): Target {
    const gl = this.gl;
    let t = this.targets.get(name);
    if (!t) {
      t = { tex: this.texture2d(filter), fbo: gl.createFramebuffer()!, w: 0, h: 0 };
      this.targets.set(name, t);
    }
    if (t.w !== w || t.h !== h) {
      t.w = w;
      t.h = h;
      // On a spare unit: binding it where a pass samples would read the target it writes (a feedback loop).
      gl.activeTexture(gl.TEXTURE7);
      gl.bindTexture(gl.TEXTURE_2D, t.tex);
      if (this.float) gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA16F, w, h, 0, gl.RGBA, gl.HALF_FLOAT, null);
      else gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, w, h, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
      gl.bindFramebuffer(gl.FRAMEBUFFER, t.fbo);
      gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, t.tex, 0);
    }
    return t;
  }

  private setLut(lut: ColourLut) {
    const gl = this.gl;
    gl.bindTexture(gl.TEXTURE_3D, this.lutTex);
    if (this.lutKey === lut.key) return;
    this.lutKey = lut.key;
    gl.texParameteri(gl.TEXTURE_3D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_3D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
    gl.texImage3D(gl.TEXTURE_3D, 0, gl.RGB32F, lut.size, lut.size, lut.size, 0, gl.RGB, gl.FLOAT, lut.data);
  }

  private draw(target: Target | undefined, w: number, h: number) {
    const gl = this.gl;
    gl.bindFramebuffer(gl.FRAMEBUFFER, target ? target.fbo : this.output);
    gl.viewport(0, 0, w, h);
    // Its own triangle (another user of a shared context may have bound another buffer).
    gl.bindBuffer(gl.ARRAY_BUFFER, this.buf);
    gl.enableVertexAttribArray(0);
    gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
  }

  private bind(unit: number, tex: WebGLTexture, kind: number = this.gl.TEXTURE_2D) {
    this.gl.activeTexture(this.gl.TEXTURE0 + unit);
    this.gl.bindTexture(kind, tex);
  }

  /**
  *Draws `source` (a <video> or <img>) with `fx` into the canvas at w × h (whole px). `k` is px
  *per canvas px (1 in the preview; less in a thumbnail), `t` the moment (s) from the clip's start.
   */
  render(source: TexImageSource, fx: ClipFx, w: number, h: number, t: number, k = 1, key?: string, output: WebGLFramebuffer | null = null): boolean {
    const gl = this.gl;
    this.output = output;
    if (this.lost || gl.isContextLost()) {
      this.drawn = false;
      return false;
    }
    // The same frame with the same effects at the same size (a paused clip re-rendered): nothing to do.
    const frame = frameKeyOf(source);
    const sig = key !== undefined && frame !== undefined ? `${key}|${frame}|${w}x${h}|${k}|${fx.shake ? t : ""}` : "";
    if (sig && sig === this.last && this.drawn) return true;
    this.last = "";
    if (!this.shared && this.canvas.width !== w) this.canvas.width = w;
    if (!this.shared && this.canvas.height !== h) this.canvas.height = h;
    this.bind(0, this.src);
    gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, false);
    gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, false);
    gl.pixelStorei(gl.UNPACK_COLORSPACE_CONVERSION_WEBGL, gl.BROWSER_DEFAULT_WEBGL);
    try {
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, source);
    } catch {
      // A source it may not read (another origin without CORS) or not decoded yet.
      this.drawn = false;
      return false;
    }
    // The shake's window in the zoomed picture (whole px), as the export's crop.
    let zoomed = [w, h];
    let win = [0, 0];
    if (fx.shake) {
      const z = shakeZoom(fx.shake.amount*k, w, h);
      const zw = Math.max(w, Math.round(w*z));
      const zh = Math.max(h, Math.round(h*z));
      const o = shakeWindow(fx.shake, t, w, h, zw, zh, k);
      zoomed = [zw, zh];
      win = [o.x, o.y];
    }
    const pictureUniforms = { uSrc: { int: 0 }, uSize: [w, h], uZoomed: zoomed, uWin: win, uMirror: fx.mirror === true };
    // Pixelate: block averages first.
    const block = fx.pixelate ? Math.max(1, Math.round(fx.pixelate*k)) : 1;
    let blocks: Target | undefined;
    if (block > 1) {
      blocks = this.target("blocks", Math.ceil(w / block), Math.ceil(h / block), gl.NEAREST);
      this.bind(0, this.src);
      this.use(this.program("pixelate", PIXELATE), { ...pictureUniforms, uBlock: { int: block } });
      this.draw(blocks, blocks.w, blocks.h);
    }
    // Colour and posterize, rows from the top.
    const lut = fx.colour ? colourLut(fx.colour) : undefined;
    if (lut) {
      this.bind(2, this.lutTex, gl.TEXTURE_3D);
      this.setLut(lut);
    }
    let img = this.target("colour", w, h, gl.NEAREST);
    if (blocks) this.bind(1, blocks.tex);
    this.bind(0, this.src);
    this.use(this.program("colour", COLOUR), { ...pictureUniforms, uBlocks: { int: 1 }, uBlock: { int: block }, uLut: { int: 2 }, uLutN: { int: lut ? lut.size : 0 }, uLevels: fx.posterize ?? 0, uFlipY: false });
    // Targets hold rows from the top (texImage2D puts the image's first row at t = 0); FINAL flips them onto the canvas.
    this.draw(img, w, h);
    const split = fx.rgbSplit ? Math.round(fx.rgbSplit*k) : 0;
    const sharp = fx.sharpen ? Math.round(fx.sharpen*16) : 0;
    if (split > 0 || sharp > 0) {
      const out = this.target("spatial", w, h, gl.NEAREST);
      this.bind(0, img.tex);
      this.use(this.program("spatial", SPATIAL), { uImg: { int: 0 }, uDim: { int: [w, h] }, uSplit: { int: split }, uSharp: { int: sharp } });
      this.draw(out, w, h);
      img = out;
    }
    let glow: Target | undefined;
    if (fx.glow) {
      // Blurred at a reduced size when it's wide (the blur hides the difference), then screened on.
      const sigma = Math.max(0.5, fx.glow.radius*k);
      const s = Math.max(1, Math.floor(sigma / 4));
      const gw = Math.max(1, Math.ceil(w / s));
      const gh = Math.max(1, Math.ceil(h / s));
      const a = this.target("glow-a", gw, gh, gl.LINEAR);
      const b = this.target("glow-b", gw, gh, gl.LINEAR);
      this.bind(0, img.tex);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
      this.use(this.program("glow", GLOW), { uImg: { int: 0 }, uImgSize: [w, h], uOut: [gw, gh], uDir: [1, 0], uSigma: sigma / s, uThreshold: true, uLow: fx.glow.threshold, uAmount: fx.glow.amount });
      this.draw(a, gw, gh);
      this.bind(0, a.tex);
      this.use(this.program("glow", GLOW), { uImg: { int: 0 }, uImgSize: [gw, gh], uOut: [gw, gh], uDir: [0, 1], uSigma: sigma / s, uThreshold: false, uLow: 0, uAmount: 1 });
      this.draw(b, gw, gh);
      this.bind(0, img.tex);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
      glow = b;
    }
    this.bind(0, img.tex);
    if (glow) this.bind(1, glow.tex);
    this.use(this.program("final", FINAL), { uImg: { int: 0 }, uGlow: { int: 1 }, uUseGlow: !!glow, uDim: [w, h] });
    this.draw(undefined, w, h);
    this.drawn = true;
    this.last = sig;
    return true;
  }
}
