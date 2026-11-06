// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Shader effects (D213, issue #20): GPU effects on a clip's picture, written as TypeGPU functions
// (typed WebGPU shaders in TypeScript) and drawn with WebGPU by both the preview
// (apps/web/src/preview/shader.ts) and the export (the rasteriser's shader window, frames piped
// through ffmpeg). Browser-safe and pure: no GPU here, only the definition both sides share.
//
// An effect is a `ShaderDef`: its params (numbers, colours, switches with ranges) and its WGSL, the
// TypeGPU function resolved by `tgpu.resolve` (plugin:check for plugins, shader_write for the
// agent's). The WGSL declares
//   struct Params { ... }                                        (when it has params)
//   fn effect(uv: vec2f, t: f32, size: vec2f[, p: Params]) -> vec4f
// and may call `frame(uv)`: the clip's picture at uv (0–1 from the top-left, edges clamped, linear
// between pixels; one pixel is 1 / size). Biliup wraps it (shaderModuleCode): the bindings, a full-screen
// triangle and a fragment that calls effect() at every pixel's centre. Its colour is clamped to 0–1;
// its alpha is ignored: the picture keeps its own (a still's transparency), so a shader recolours
// and moves pixels, it doesn't cut holes (v1).
//
// Deterministic: the only inputs are the picture, t (seconds from the clip's start), the size and the
// params; WGSL has no clock, no randomness and no I/O. Bounded: shaderProblems refuses `loop` and
// `while`, `for` loops without small literal bounds, more than MAX_LOOPS loops, storage, atomics,
// attributes, `enable`/`requires`, and anything named biliup*. A clip carries the whole definition
// (effect.shader's params), so it draws the same after the plugin that made it is gone (as D137).
import type { Item, ModuleUse } from "@/app/lib/editor/core/timeline/types";

export const SHADER = "effect.shader";
/** The WGSL's size limit (after comments are removed). */
export const MAX_SHADER_WGSL = 16 * 1024;
export const MAX_SHADER_PARAMS = 16;
/** At most this many `for` loops, each at most MAX_LOOP_STEPS steps, nested at most two deep. */
export const MAX_LOOPS = 4;
export const MAX_LOOP_STEPS = 64;

export type ShaderParamType = "number" | "color" | "bool";
export type ShaderValue = number | string | boolean;

export interface ShaderParam {
  /** A WGSL field name in Params: a letter, then letters and digits. */
  readonly key: string;
  readonly label: string;
  readonly type: ShaderParamType;
  readonly min?: number;
  readonly max?: number;
  readonly step?: number;
  readonly default: ShaderValue;
}

export interface ShaderDef {
  /** `<plugin id>.<name>` for a plugin's, `project.<name>` for one written into the project. */
  readonly id: string;
  readonly name: string;
  readonly description?: string;
  readonly params: ReadonlyArray<ShaderParam>;
  readonly wgsl: string;
}

/** A clip's shader effect as drawn: its definition and its values (every param, in range). */
export interface ShaderUse {
  readonly def: ShaderDef;
  readonly values: Readonly<Record<string, ShaderValue>>;
}

const KEY = /^[a-z][a-zA-Z0-9]{0,30}$/;
const HEX = /^#[0-9a-fA-F]{6}$/;
const finite = (v: unknown): v is number => typeof v === "number" && Number.isFinite(v);

/** The WGSL without comments (line and nested block comments), as it is checked and compiled. */
export function stripWgslComments(src: string): string {
  let out = "";
  let i = 0;
  while (i < src.length) {
    if (src.startsWith("//", i)) {
      const end = src.indexOf("\n", i);
      i = end < 0 ? src.length : end;
      continue;
    }
    if (src.startsWith("/*", i)) {
      let depth = 1;
      i += 2;
      while (i < src.length && depth > 0) {
        if (src.startsWith("/*", i)) {
          depth++;
          i += 2;
        } else if (src.startsWith("*/", i)) {
          depth--;
          i += 2;
        } else i++;
      }
      out += " ";
      continue;
    }
    out += src[i];
    i++;
  }
  return out;
}

/** Problems with a param list, one sentence each (empty when fine). */
export function shaderParamProblems(params: ReadonlyArray<ShaderParam>): Array<string> {
  const problems: Array<string> = [];
  if (params.length > MAX_SHADER_PARAMS) problems.push(`A shader takes at most ${MAX_SHADER_PARAMS} params.`);
  const seen = new Set<string>();
  for (const p of params) {
    const name = `The param "${String(p.key)}"`;
    if (typeof p.key !== "string" || !KEY.test(p.key)) problems.push(`${name} needs a name of letters and digits that starts with a lower-case letter.`);
    else if (seen.has(p.key)) problems.push(`${name} is there twice.`);
    seen.add(p.key);
    if (typeof p.label !== "string" || !p.label.trim() || p.label.length > 40) problems.push(`${name} needs a label of 1–40 characters.`);
    if (p.type === "number") {
      if (!finite(p.min) || !finite(p.max) || !(p.min < p.max)) problems.push(`${name} needs a min below its max.`);
      else if (!finite(p.default) || p.default < p.min || p.default > p.max) problems.push(`${name}'s default must be a number from ${p.min} to ${p.max}.`);
      if (p.step !== undefined && (!finite(p.step) || p.step <= 0 || (finite(p.min) && finite(p.max) && p.step > p.max - p.min))) problems.push(`${name}'s step must be above 0 and fit its range.`);
    } else if (p.type === "color") {
      if (typeof p.default !== "string" || !HEX.test(p.default)) problems.push(`${name}'s default must be a colour like #ff8800.`);
    } else if (p.type === "bool") {
      if (typeof p.default !== "boolean") problems.push(`${name}'s default must be true or false.`);
    } else problems.push(`${name} has a type Biliup doesn't know (number, color or bool).`);
  }
  return problems;
}

/** The WGSL type of a param in Params (switches are f32 0 or 1: WGSL has no bool in uniforms). */
export const wgslTypeOf = (type: ShaderParamType) => (type === "color" ? "vec3f" : "f32");

/**
 * Problems with an effect's WGSL (comments already removed), one sentence each: the shape Biliup
 * calls, and the rules that keep it bounded and away from anything but its picture.
 */
export function shaderWgslProblems(wgsl: string, params: ReadonlyArray<ShaderParam>): Array<string> {
  const problems: Array<string> = [];
  if (wgsl.length > MAX_SHADER_WGSL) problems.push(`The shader is ${Math.round(wgsl.length / 1024)} KB; at most ${MAX_SHADER_WGSL / 1024} KB.`);
  const sig = params.length ? /\bfn\s+effect\s*\(\s*\w+\s*:\s*vec2<?f(?:32>)?\s*,\s*\w+\s*:\s*f32\s*,\s*\w+\s*:\s*vec2<?f(?:32>)?\s*,\s*\w+\s*:\s*Params\s*,?\s*\)\s*->\s*vec4<?f(?:32>)?/ : /\bfn\s+effect\s*\(\s*\w+\s*:\s*vec2<?f(?:32>)?\s*,\s*\w+\s*:\s*f32\s*,\s*\w+\s*:\s*vec2<?f(?:32>)?\s*,?\s*\)\s*->\s*vec4<?f(?:32>)?/;
  if (!sig.test(wgsl)) problems.push(params.length ? "The shader needs fn effect(uv: vec2f, t: f32, size: vec2f, p: Params) -> vec4f." : "The shader needs fn effect(uv: vec2f, t: f32, size: vec2f) -> vec4f.");
  if (params.length) {
    const struct = /\bstruct\s+Params\s*\{([^}]*)\}/.exec(wgsl);
    if (!struct) problems.push("The shader needs struct Params with its params.");
    else {
      const fields = struct[1]!
        .split(",")
        .map((f) => f.trim())
        .filter(Boolean)
        .map((f) => /^(\w+)\s*:\s*(\w+(?:<\w+>)?)$/.exec(f));
      const want = params.map((p) => `${p.key}: ${wgslTypeOf(p.type)}`).join(", ");
      const got = fields.map((m) => (m ? `${m[1]}: ${m[2]!.replace(/^vec3<f32>$/, "vec3f")}` : "?")).join(", ");
      if (got !== want) problems.push(`struct Params must be { ${want} } in that order (numbers and switches f32, colours vec3f); it is { ${got} }.`);
    }
  }
  const banned: Array<[RegExp, string]> = [
    [/\bloop\b/, "loop"],
    [/\bwhile\b/, "while"],
    [/@/, "attributes (@...)"],
    [/\benable\b|\brequires\b|\bdiagnostic\b/, "enable, requires and diagnostic"],
    [/\bvar\s*<\s*(storage|uniform|workgroup|handle)\b/, "bindings and storage"],
    [/\btexture_storage|\btextureStore\b|\batomic/, "storage textures and atomics"],
    [/\bptr\s*</, "pointers"],
    [/\boverride\b/, "override"],
    [/\bbiliup\w*/i, "names starting with biliup (Biliup's own)"],
    [/\b(fn|var|let|const|struct|alias)\s+frame\b/, "its own frame (Biliup's)"],
  ];
  for (const [re, what] of banned) if (re.test(wgsl)) problems.push(`Shaders can't use ${what}.`);
  // for loops: `for (var i = a; i < b; i++)` (or <=, i += 1) with literal bounds, at most
  // MAX_LOOP_STEPS steps, the counter never assigned in the body; at most MAX_LOOPS, two deep.
  const fors = [...wgsl.matchAll(/\bfor\s*\(/g)];
  if (fors.length > MAX_LOOPS) problems.push(`A shader has at most ${MAX_LOOPS} for loops.`);
  for (const m of fors) {
    const head = /^for\s*\(\s*var\s+(\w+)\s*(?::\s*[iu]32\s*)?=\s*(-?\d+)[iu]?\s*;\s*(\w+)\s*(<=?)\s*(-?\d+)[iu]?\s*;\s*(\w+)\s*(\+\+|\+=\s*1[iu]?)\s*\)/.exec(wgsl.slice(m.index));
    if (!head || head[1] !== head[3] || head[1] !== head[6]) {
      problems.push("Each for loop must count a var up by one between two whole-number literals: for (var i = 0; i < 8; i++).");
      continue;
    }
    const steps = Number(head[5]) - Number(head[2]) + (head[4] === "<=" ? 1 : 0);
    if (steps > MAX_LOOP_STEPS) problems.push(`A for loop runs ${steps} times; at most ${MAX_LOOP_STEPS}.`);
    const counter = head[1]!;
    const body = wgsl.slice(m.index + head[0].length);
    const assigned = new RegExp(`\\b${counter}\\s*(?:[-+*/%&|^]?=(?!=)|\\+\\+|--)`);
    if (assigned.test(body.slice(0, blockEnd(body)))) problems.push(`The for loop's counter ${counter} is changed inside the loop.`);
  }
  if (loopDepth(wgsl) > 2) problems.push("Shaders nest for loops at most two deep.");
  return problems;
}

/** Where the block that starts at the first `{` of `s` ends (its length when unbalanced). */
function blockEnd(s: string): number {
  const open = s.indexOf("{");
  if (open < 0) return s.length;
  let depth = 0;
  for (let i = open; i < s.length; i++) {
    if (s[i] === "{") depth++;
    else if (s[i] === "}" && --depth === 0) return i + 1;
  }
  return s.length;
}

/** How deep for loops nest. */
function loopDepth(wgsl: string): number {
  let max = 0;
  const stack: Array<boolean> = [];
  let pendingFor = false;
  let loops = 0;
  for (let i = 0; i < wgsl.length; i++) {
    if (/\bfor\s*\(/y.test(wgsl.slice(i, i + 8)) && (i === 0 || !/\w/.test(wgsl[i - 1]!))) pendingFor = true;
    const c = wgsl[i];
    if (c === "{") {
      stack.push(pendingFor);
      if (pendingFor) max = Math.max(max, ++loops);
      pendingFor = false;
    } else if (c === "}") {
      if (stack.pop()) loops--;
    }
  }
  return max;
}

/** Every problem with a definition (params, then WGSL); empty when Biliup can draw it. */
export function shaderDefProblems(def: Pick<ShaderDef, "params" | "wgsl">): Array<string> {
  return [...shaderParamProblems(def.params), ...shaderWgslProblems(def.wgsl, def.params)];
}

// ---- values ----

/** The values a clip draws with: each param's value when it's of the right type and in range (clamped), else its default. */
export function shaderValues(params: ReadonlyArray<ShaderParam>, given: unknown): Record<string, ShaderValue> {
  const g = (given && typeof given === "object" ? given : {}) as Record<string, unknown>;
  const out: Record<string, ShaderValue> = {};
  for (const p of params) {
    const v = Object.hasOwn(g, p.key) ? g[p.key] : undefined;
    if (p.type === "number") out[p.key] = finite(v) ? Math.min(p.max ?? v, Math.max(p.min ?? v, v)) : (p.default as number);
    else if (p.type === "color") out[p.key] = typeof v === "string" && HEX.test(v) ? v.toLowerCase() : String(p.default).toLowerCase();
    else out[p.key] = typeof v === "boolean" ? v : Boolean(p.default);
  }
  return out;
}

/** Values given that aren't params, aren't of the param's type or are out of its range: sentences. */
export function shaderValueProblems(params: ReadonlyArray<ShaderParam>, given: Readonly<Record<string, unknown>>): Array<string> {
  const problems: Array<string> = [];
  for (const [key, v] of Object.entries(given)) {
    const p = params.find((x) => x.key === key);
    if (!p) problems.push(`The shader has no param "${key}" (it has ${params.map((x) => x.key).join(", ") || "none"}).`);
    else if (p.type === "number" && (!finite(v) || v < p.min! || v > p.max!)) problems.push(`"${key}" takes a number from ${p.min} to ${p.max}.`);
    else if (p.type === "color" && (typeof v !== "string" || !HEX.test(v))) problems.push(`"${key}" takes a colour like #ff8800.`);
    else if (p.type === "bool" && typeof v !== "boolean") problems.push(`"${key}" takes true or false.`);
  }
  return problems;
}

/** A clip's shader effect (the first that isn't bypassed), or undefined. Its definition must still pass the checks. */
export function clipShader(item: Pick<Item, "type" | "effects">): ShaderUse | undefined {
  if (item.type !== "clip" || !Array.isArray(item.effects)) return undefined;
  const e = item.effects.find((x) => x && x.module === SHADER && !x.bypass);
  return e ? shaderUseOf(e) : undefined;
}

/** An effect.shader entry's definition and values, or undefined when it isn't one Biliup can draw. */
export function shaderUseOf(e: Pick<ModuleUse, "params">): ShaderUse | undefined {
  const p = (e.params ?? {}) as Record<string, unknown>;
  if (typeof p.shader !== "string" || typeof p.wgsl !== "string" || !Array.isArray(p.fields)) return undefined;
  const def: ShaderDef = { id: p.shader, name: typeof p.name === "string" ? p.name : p.shader, params: p.fields as ReadonlyArray<ShaderParam>, wgsl: p.wgsl };
  if (shaderDefProblems(def).length) return undefined;
  return { def, values: shaderValues(def.params, p.values) };
}

/** The effect.shader entry that draws `def` with `values` (missing ones at their defaults). */
export function shaderEntry(def: ShaderDef, values: Readonly<Record<string, ShaderValue>> = {}): ModuleUse {
  return { module: SHADER, params: { shader: def.id, name: def.name, wgsl: def.wgsl, fields: def.params.map((p) => ({ ...p })), values: shaderValues(def.params, values) } };
}

// ---- the GPU side's shared parts ----

/**
 * The whole WGSL module both drawers compile: the effect, Biliup's bindings and helpers, a
 * full-screen triangle and the fragment. `premultiplied`: the colour goes out multiplied by the
 * picture's alpha (a WebGPU canvas); else straight (read back for the export).
 */
export function shaderModuleCode(def: Pick<ShaderDef, "params" | "wgsl">): string {
  const has = def.params.length > 0;
  return `${def.wgsl}

struct BiliupFrame { size: vec2f, t: f32, premultiplied: f32 }
@group(0) @binding(0) var biliupSource: texture_2d<f32>;
@group(0) @binding(1) var biliupSampler: sampler;
@group(0) @binding(2) var<uniform> biliupFrame: BiliupFrame;
${has ? "@group(0) @binding(3) var<uniform> biliupParams: Params;" : ""}

fn frame(uv: vec2f) -> vec4f { return textureSampleLevel(biliupSource, biliupSampler, uv, 0.0); }

@vertex fn biliupVertex(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f {
  let p = vec2f(f32((i << 1u) & 2u), f32(i & 2u));
  return vec4f(p * 2.0 - 1.0, 0.0, 1.0);
}

@fragment fn biliupFragment(@builtin(position) pos: vec4f) -> @location(0) vec4f {
  let uv = pos.xy / biliupFrame.size;
  let a = frame(uv).a;
  let c = clamp(effect(uv, biliupFrame.t, biliupFrame.size${has ? ", biliupParams" : ""}).rgb, vec3f(0.0), vec3f(1.0));
  return vec4f(select(c, c * a, biliupFrame.premultiplied > 0.5), a);
}
`;
}

/** BiliupFrame's bytes: size, t, premultiplied (16 bytes). */
export function shaderFrameBytes(w: number, h: number, t: number, premultiplied: boolean): ArrayBuffer {
  return new Float32Array([w, h, t, premultiplied ? 1 : 0]).buffer;
}

/** Where each param sits in Params (WGSL's uniform layout: f32 4/4, vec3f aligned 16, size 12), and the struct's size (a multiple of 16). */
export function shaderParamLayout(params: ReadonlyArray<ShaderParam>): { readonly offsets: ReadonlyArray<number>; readonly size: number } {
  let end = 0;
  let align = 4;
  const offsets = params.map((p) => {
    const a = p.type === "color" ? 16 : 4;
    align = Math.max(align, a);
    const at = Math.ceil(end / a) * a;
    end = at + (p.type === "color" ? 12 : 4);
    return at;
  });
  return { offsets, size: Math.max(16, Math.ceil(end / 16) * 16) };
}

/** Colours go in as sRGB-encoded 0–1 values, as the picture's are. */
const rgbOf = (hex: string): [number, number, number] => [0, 2, 4].map((i) => parseInt(hex.slice(1 + i, 3 + i), 16) / 255) as [number, number, number];

/** Params' bytes for these values (undefined without params). */
export function shaderParamBytes(params: ReadonlyArray<ShaderParam>, values: Readonly<Record<string, ShaderValue>>): ArrayBuffer | undefined {
  if (!params.length) return undefined;
  const { offsets, size } = shaderParamLayout(params);
  const f = new Float32Array(size / 4);
  const v = shaderValues(params, values);
  params.forEach((p, i) => {
    const at = offsets[i]! / 4;
    if (p.type === "color") f.set(rgbOf(String(v[p.key])), at);
    else f[at] = p.type === "bool" ? (v[p.key] ? 1 : 0) : Number(v[p.key]);
  });
  return f.buffer;
}

/** A shader effect's moment: seconds from the clip's start at timeline frame `frame` (its start `start`), as both drawers use it. */
export const shaderTime = (frame: number, start: number, fps: number) => (Math.floor(frame) - start) / fps;

// ---- edits ----

type Plan = { readonly ops: ReadonlyArray<{ op: "update_item"; item: string; set: Record<string, unknown> }>; readonly summary: string } | { readonly problem: string };

interface Target {
  readonly tracks: ReadonlyArray<{ readonly locked?: boolean; readonly items: ReadonlyArray<Pick<Item, "id" | "type" | "effects" | "media" | "locked" | "name">> }>;
  readonly media?: ReadonlyArray<{ readonly id: string; readonly kind: string; readonly name?: string; readonly path?: string }>;
}

/** Put a shader effect on a clip (one per clip: it replaces the one it had), or with undefined take it off. */
export function planShader(project: Target, itemId: string, def: ShaderDef | undefined, values: Readonly<Record<string, ShaderValue>> = {}): Plan {
  let found: { item: Pick<Item, "id" | "effects" | "name">; name: string } | undefined;
  for (const t of project.tracks)
    for (const it of t.items) {
      if (it.id !== itemId) continue;
      const media = it.media ? project.media?.find((m) => m.id === it.media) : undefined;
      if (it.type !== "clip" || !media || media.kind === "audio") return { problem: "Shader effects go on video clips and stills." };
      if (media.kind === "image" && /\.svg$/i.test(media.path ?? "")) return { problem: "Shader effects don't apply to SVG images." };
      if (t.locked || it.locked) return { problem: "That clip is locked: unlock it to change its effects." };
      found = { item: it, name: it.name ?? media.name ?? it.id };
    }
  if (!found) return { problem: "Pick a clip first." };
  const list = (found.item.effects ?? []).filter((e) => e.module !== SHADER);
  if (!def) return { ops: [{ op: "update_item", item: itemId, set: { effects: list.length ? list : null } }], summary: `Removed the shader effect from ${found.name}` };
  const problems = [...shaderDefProblems(def), ...shaderValueProblems(def.params, values)];
  if (problems.length) return { problem: problems.join(" ") };
  return { ops: [{ op: "update_item", item: itemId, set: { effects: [...list, shaderEntry(def, values)] } }], summary: `${def.name} on ${found.name}` };
}

/** Change some of a clip's shader effect's values (the others kept): one edit. */
export function planShaderValues(project: Target, itemId: string, values: Readonly<Record<string, ShaderValue>>): Plan {
  for (const t of project.tracks)
    for (const it of t.items) {
      if (it.id !== itemId) continue;
      if (t.locked || it.locked) return { problem: "That clip is locked: unlock it to change its effects." };
      const list = [...(it.effects ?? [])];
      const at = list.findIndex((e) => e.module === SHADER);
      const use = at >= 0 ? shaderUseOf(list[at]!) : undefined;
      if (!use) return { problem: "That clip has no shader effect." };
      const problems = shaderValueProblems(use.def.params, values);
      if (problems.length) return { problem: problems.join(" ") };
      const p = list[at]!.params as Record<string, unknown>;
      list[at] = { ...list[at]!, params: { ...p, values: shaderValues(use.def.params, { ...use.values, ...values }) } };
      const changed = Object.keys(values).map((k) => use.def.params.find((x) => x.key === k)?.label ?? k).join(", ");
      return { ops: [{ op: "update_item", item: itemId, set: { effects: list } }], summary: `${use.def.name}: ${changed} on ${it.name ?? it.id}` };
    }
  return { problem: "Pick a clip first." };
}

/**
 * The agent's edit ops with each effect.shader entry made whole (D213): the agent names the effect
 * (`shader`: a project definition's id from shader_write) and its `values`; the editor fills in
 * the name, the WGSL and the params from `defs` (wgsl the agent wrote itself is replaced when the
 * id is known). An unknown id or values out of range are refused with a sentence.
 */
export function expandShaderEffects(ops: ReadonlyArray<unknown>, defs: ReadonlyMap<string, ShaderDef>, preserveEmbedded = false): { ops: Array<unknown> } | { problem: string } {
  let problem: string | undefined;
  const fill = (effects: unknown): unknown => {
    if (!Array.isArray(effects)) return effects;
    return effects.map((e: unknown) => {
      if (!e || typeof e !== "object" || (e as ModuleUse).module !== SHADER) return e;
      const entry = e as ModuleUse;
      const p = (entry.params ?? {}) as Record<string, unknown>;
      // Native editor cuts/duplicates carry immutable shader versions, including multiple
      // versions of the same template. Only a shorthand application should pick the latest.
      // Complete entries still pass the shared timeline/WGSL validators before saving.
      if (preserveEmbedded && typeof p.wgsl === "string" && Array.isArray(p.fields)) return e;
      const id = typeof p.shader === "string" ? p.shader : "";
      const def = defs.get(id);
      if (!def) {
        if (typeof p.wgsl === "string" && Array.isArray(p.fields)) return e; // a whole entry (e.g. copied from another clip): validated as is
        problem ??= `There's no shader effect "${id}". Known: ${[...defs.keys()].join(", ") || "none (create one with shader_write)"}.`;
        return e;
      }
      const values = (p.values && typeof p.values === "object" ? p.values : {}) as Record<string, unknown>;
      const bad = shaderValueProblems(def.params, values);
      if (bad.length) problem ??= `${def.name}: ${bad.join(" ")}`;
      return { ...shaderEntry(def, values as Record<string, ShaderValue>), ...(entry.bypass ? { bypass: true } : {}) };
    });
  };
  const out = ops.map((op) => {
    if (!op || typeof op !== "object") return op;
    const o = op as Record<string, unknown>;
    if (o.op === "update_item" && o.set && typeof o.set === "object" && "effects" in (o.set as object)) return { ...o, set: { ...(o.set as object), effects: fill((o.set as { effects: unknown }).effects) } };
    if (o.op === "insert_item" && o.item && typeof o.item === "object" && "effects" in (o.item as object)) return { ...o, item: { ...(o.item as object), effects: fill((o.item as { effects: unknown }).effects) } };
    return op;
  });
  return problem ? { problem } : { ops: out };
}
