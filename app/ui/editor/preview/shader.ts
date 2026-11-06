// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// A clip's shader effect in the preview (D213): a WebGPU <canvas> at the effects' size (core
// fxPictureSize) over the clip's picture, drawn by the same runner the export's rasteriser uses
// (shaderRunner.ts) from the <video>/<img> (which stays, hidden, as the source), with t from the
// clip's start (core shaderTime). Without WebGPU the clip shows its plain picture and the preview
// puts a badge on it; the export refuses (never silently different).
import { shaderFrameBytes, shaderModuleCode, shaderParamBytes, type ShaderUse } from "@/app/lib/editor/core/visual/shaders";
import { ShaderRunner } from "@/app/ui/editor/preview/shaderRunner";

let shared: Promise<ShaderRunner | { readonly problem: string }> | undefined;
let known: ShaderRunner | { readonly problem: string } | undefined;

/** The preview's one runner (one device for every clip), made on first use; again after a lost device. */
export function previewRunner(): ShaderRunner | { readonly problem: string } | undefined {
  if (known && "lost" in known && known.lost) {
    known = undefined;
    shared = undefined;
  }
  if (!shared) {
    shared = ShaderRunner.create();
    void shared.then((r) => {
      known = r;
      for (const f of [...listeners]) f();
    });
  }
  return known;
}
/** Called once the runner (or why there's none) is known: draw again. Returns the unsubscribe. */
const listeners = new Set<() => void>();
export function whenRunner(f: () => void): () => void {
  listeners.add(f);
  return () => listeners.delete(f);
}

/** Why shader effects can't be drawn here, once known (undefined while unknown or when they can). */
export function shaderProblem(): string | undefined {
  const r = previewRunner();
  return r && "problem" in r ? r.problem : undefined;
}

export class ShaderCanvas {
  readonly canvas = document.createElement("canvas");
  private context: GPUCanvasContext | undefined;
  private configured: GPUDevice | undefined;
  private readonly format: GPUTextureFormat = navigator.gpu?.getPreferredCanvasFormat?.() ?? "bgra8unorm";
  /** Whether the canvas has the last asked frame. */
  drawn = false;
  private last = "";

  /**
   * Draw the effect on `source`'s current picture at w×h, t seconds from the clip's start. False
   * when it can't be yet (no runner, still compiling, the source not ready): the caller shows the
   * plain picture; `onReady` is called when a compile finishes.
   */
  render(source: HTMLVideoElement | HTMLImageElement | HTMLCanvasElement, use: ShaderUse, w: number, h: number, t: number, onReady: () => void): boolean {
    // Where it got to (data-state on the canvas: starting, no-picture, compiling, error, drawn).
    const state = (s: string) => (this.canvas.dataset.state = s);
    const runner = previewRunner();
    if (!runner || "problem" in runner || runner.lost) return state("starting"), (this.drawn = false);
    const sw = source instanceof HTMLVideoElement ? source.videoWidth : source instanceof HTMLImageElement ? source.naturalWidth : source.width;
    const sh = source instanceof HTMLVideoElement ? source.videoHeight : source instanceof HTMLImageElement ? source.naturalHeight : source.height;
    if (!sw || !sh) return state("no-picture"), (this.drawn = false);
    const code = shaderModuleCode(use.def);
    const pipeline = runner.pipelineNow(code, this.format);
    if (pipeline === undefined) {
      state("compiling");
      void runner.pipeline(code, this.format).then(onReady);
      return (this.drawn = false);
    }
    if (typeof pipeline === "string") {
      // It doesn't compile here (the checks passed): said on the canvas for whoever looks.
      this.canvas.dataset.error = pipeline.slice(0, 300);
      state("error");
      return (this.drawn = false);
    }
    // D221: a scrub canvas names the frame it holds.
    const frame = source instanceof HTMLVideoElement ? `v${source.currentTime}` : source instanceof HTMLImageElement ? `i${source.currentSrc}` : `c${source.dataset.scrub}|${source.dataset.sample}`;
    const sig = `${code}|${JSON.stringify(use.values)}|${frame}|${w}x${h}|${t}`;
    if (sig === this.last && this.drawn) return true;
    if (this.canvas.width !== w) this.canvas.width = w;
    if (this.canvas.height !== h) this.canvas.height = h;
    if (!this.context) this.context = (this.canvas.getContext("webgpu") as GPUCanvasContext | null) ?? undefined;
    if (!this.context) return (this.drawn = false);
    if (this.configured !== runner.device) {
      this.context.configure({ device: runner.device, format: this.format, alphaMode: "premultiplied" });
      this.configured = runner.device;
    }
    try {
      runner.drawToCanvas(pipeline, { kind: "external", source, width: sw, height: sh }, { code, frame: shaderFrameBytes(w, h, t, true), params: shaderParamBytes(use.def.params, use.values), width: w, height: h }, this.context);
    } catch (error) {
      // A source it can't read yet (not decoded) or a lost device: the plain picture shows.
      this.canvas.dataset.error = String((error as Error).message).slice(0, 300);
      state("error");
      this.last = "";
      return (this.drawn = false);
    }
    this.last = sig;
    state("drawn");
    return (this.drawn = true);
  }

  dispose() {
    this.context?.unconfigure();
    this.canvas.remove();
  }
}
