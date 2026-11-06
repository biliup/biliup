// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The WebGPU side of shader effects (D213), shared by the preview (shader.ts: a <canvas> over the
// clip's picture) and the export (the rasteriser's shader window, apps/desktop/src/render.ts, which
// loads this file bundled on its own as window.BiliupShader): one device, a pipeline per effect's
// module (core shaderModuleCode), the picture uploaded to a texture, one full-screen draw, and either
// a canvas to show or the pixels read back. Both sides run the same code on the same numbers.

// WebGPU's flag values (the spec's; TypeScript's DOM types don't declare the constant objects).
const BUFFER = { MAP_READ: 0x1, COPY_DST: 0x8, UNIFORM: 0x40 } as const;
const TEXTURE = { COPY_SRC: 0x1, COPY_DST: 0x2, TEXTURE_BINDING: 0x4, RENDER_ATTACHMENT: 0x10 } as const;
const MAP_READ = 0x1;

export type ShaderSource =
  | { readonly kind: "external"; readonly source: GPUCopyExternalImageSource; readonly width: number; readonly height: number }
  | { readonly kind: "rgba"; readonly data: Uint8Array<ArrayBuffer>; readonly width: number; readonly height: number };

export interface ShaderDraw {
  /** core shaderModuleCode(def). */
  readonly code: string;
  /** core shaderFrameBytes(w, h, t, premultiplied). */
  readonly frame: ArrayBuffer;
  /** core shaderParamBytes(params, values), when it has params. */
  readonly params?: ArrayBuffer | undefined;
  /** Output size (px). */
  readonly width: number;
  readonly height: number;
}

/** Why there's no WebGPU here, or undefined when there is. */
export async function webGpuProblem(): Promise<string | undefined> {
  const gpu = (globalThis.navigator as Navigator | undefined)?.gpu;
  if (!gpu) return "This renderer has no WebGPU.";
  try {
    const adapter = await gpu.requestAdapter();
    return adapter ? undefined : "No usable WebGPU adapter is available.";
  } catch (error) {
    return `WebGPU didn't start: ${(error as Error).message}`;
  }
}

export class ShaderRunner {
  /** The device went away (a GPU reset): make a new runner. */
  lost = false;
  private readonly pipelines = new Map<string, Promise<GPURenderPipeline | string>>();
  private readonly ready = new Map<string, GPURenderPipeline | string>();
  private source: GPUTexture | undefined;
  private readonly sampler: GPUSampler;
  private readonly frameBuf: GPUBuffer;
  private paramBuf: GPUBuffer | undefined;
  private target: GPUTexture | undefined;
  private readback: GPUBuffer | undefined;

  readonly device: GPUDevice;

  private constructor(device: GPUDevice) {
    this.device = device;
    this.sampler = device.createSampler({ magFilter: "linear", minFilter: "linear", addressModeU: "clamp-to-edge", addressModeV: "clamp-to-edge" });
    this.frameBuf = device.createBuffer({ size: 16, usage: BUFFER.UNIFORM | BUFFER.COPY_DST });
    void device.lost.then(() => (this.lost = true));
  }

  /** A runner on this machine's GPU, or why there can't be one. */
  static async create(): Promise<ShaderRunner | { readonly problem: string }> {
    const problem = await webGpuProblem();
    if (problem) return { problem };
    try {
      const adapter = await navigator.gpu.requestAdapter();
      if (!adapter) return { problem: "No usable WebGPU adapter is available." };
      return new ShaderRunner(await adapter.requestDevice());
    } catch (error) {
      return { problem: `WebGPU didn't start: ${(error as Error).message}` };
    }
  }

  /** The pipeline for a module and output format: compiled once (errors come back as a sentence). */
  pipeline(code: string, format: GPUTextureFormat): Promise<GPURenderPipeline | string> {
    const key = `${format}\n${code}`;
    let p = this.pipelines.get(key);
    if (!p) {
      p = (async () => {
        const shaderModule = this.device.createShaderModule({ code });
        const info = await shaderModule.getCompilationInfo();
        const errors = info.messages.filter((m) => m.type === "error");
        if (errors.length) return errors.map((m) => `line ${m.lineNum}: ${m.message}`).join("; ");
        try {
          return await this.device.createRenderPipelineAsync({
            layout: "auto",
            vertex: { module: shaderModule, entryPoint: "biliupVertex" },
            fragment: { module: shaderModule, entryPoint: "biliupFragment", targets: [{ format }] },
            primitive: { topology: "triangle-list" },
          });
        } catch (error) {
          return (error as Error).message;
        }
      })();
      void p.then((r) => this.ready.set(key, r));
      this.pipelines.set(key, p);
    }
    return p;
  }

  /** The compiled pipeline when it's there already (no waiting), else undefined. */
  pipelineNow(code: string, format: GPUTextureFormat): GPURenderPipeline | string | undefined {
    const key = `${format}\n${code}`;
    if (!this.pipelines.has(key)) void this.pipeline(code, format);
    return this.ready.get(key);
  }

  private upload(src: ShaderSource): GPUTexture {
    if (!this.source || this.source.width !== src.width || this.source.height !== src.height) {
      this.source?.destroy();
      this.source = this.device.createTexture({ size: [src.width, src.height], format: "rgba8unorm", usage: TEXTURE.TEXTURE_BINDING | TEXTURE.COPY_DST | TEXTURE.RENDER_ATTACHMENT });
    }
    if (src.kind === "rgba") this.device.queue.writeTexture({ texture: this.source }, src.data, { bytesPerRow: src.width * 4 }, [src.width, src.height]);
    else this.device.queue.copyExternalImageToTexture({ source: src.source }, { texture: this.source }, [src.width, src.height]);
    return this.source;
  }

  private encode(pipeline: GPURenderPipeline, src: ShaderSource, draw: ShaderDraw, view: GPUTextureView): GPUCommandEncoder {
    const texture = this.upload(src);
    this.device.queue.writeBuffer(this.frameBuf, 0, draw.frame);
    const entries: Array<GPUBindGroupEntry> = [
      { binding: 0, resource: texture.createView() },
      { binding: 1, resource: this.sampler },
      { binding: 2, resource: { buffer: this.frameBuf } },
    ];
    if (draw.params) {
      if (!this.paramBuf || this.paramBuf.size < draw.params.byteLength) {
        this.paramBuf?.destroy();
        this.paramBuf = this.device.createBuffer({ size: Math.max(16, draw.params.byteLength), usage: BUFFER.UNIFORM | BUFFER.COPY_DST });
      }
      this.device.queue.writeBuffer(this.paramBuf, 0, draw.params);
      entries.push({ binding: 3, resource: { buffer: this.paramBuf, size: draw.params.byteLength } });
    }
    const group = this.device.createBindGroup({ layout: pipeline.getBindGroupLayout(0), entries });
    const encoder = this.device.createCommandEncoder();
    const pass = encoder.beginRenderPass({ colorAttachments: [{ view, loadOp: "clear", storeOp: "store", clearValue: { r: 0, g: 0, b: 0, a: 0 } }] });
    pass.setPipeline(pipeline);
    pass.setBindGroup(0, group);
    pass.draw(3);
    pass.end();
    return encoder;
  }

  /** Draw into a configured canvas context (the preview). */
  drawToCanvas(pipeline: GPURenderPipeline, src: ShaderSource, draw: ShaderDraw, context: GPUCanvasContext): void {
    this.device.queue.submit([this.encode(pipeline, src, draw, context.getCurrentTexture().createView()).finish()]);
  }

  /** Draw and read the pixels back: width × height RGBA, rows from the top, alpha straight (the export). */
  async drawToPixels(src: ShaderSource, draw: ShaderDraw): Promise<Uint8Array> {
    const pipeline = await this.pipeline(draw.code, "rgba8unorm");
    if (typeof pipeline === "string") throw new Error(`The shader doesn't compile: ${pipeline}`);
    const { width: w, height: h } = draw;
    if (!this.target || this.target.width !== w || this.target.height !== h) {
      this.target?.destroy();
      this.target = this.device.createTexture({ size: [w, h], format: "rgba8unorm", usage: TEXTURE.RENDER_ATTACHMENT | TEXTURE.COPY_SRC });
    }
    const row = Math.ceil((w * 4) / 256) * 256;
    if (!this.readback || this.readback.size !== row * h) {
      this.readback?.destroy();
      this.readback = this.device.createBuffer({ size: row * h, usage: BUFFER.COPY_DST | BUFFER.MAP_READ });
    }
    this.device.pushErrorScope("validation");
    const encoder = this.encode(pipeline, src, draw, this.target.createView());
    encoder.copyTextureToBuffer({ texture: this.target }, { buffer: this.readback, bytesPerRow: row }, [w, h]);
    this.device.queue.submit([encoder.finish()]);
    const error = await this.device.popErrorScope();
    if (error) throw new Error(`WebGPU: ${error.message}`);
    await this.readback.mapAsync(MAP_READ);
    const mapped = new Uint8Array(this.readback.getMappedRange());
    const out = new Uint8Array(w * h * 4);
    for (let y = 0; y < h; y++) out.set(mapped.subarray(y * row, y * row + w * 4), y * w * 4);
    this.readback.unmap();
    return out;
  }

  dispose() {
    this.source?.destroy();
    this.target?.destroy();
    this.readback?.destroy();
    this.device.destroy();
  }
}
