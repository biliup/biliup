// Adapted from Uncut, MIT; licenses/uncut/LICENSE.
// Shader effects (D213) made with TypeGPU: a TypeGPU function (tgpu.fn with its types from
// typegpu/data) resolved to the WGSL core's shaders.ts checks and both drawers compile. Used where
// a shader is authored, never where one is drawn: plugin:check resolves a plugin's exports (and
// --write puts the result in its manifest), shader_write builds the agent's from a body and a param
// list. TypeGPU is bundled for the isolated browser runtime: resolving needs no GPU.
//
// Only TypeGPU's WGSL-bodied functions resolve here (tgpu.fn(...)`(uv, t, size, p) { ... }`):
// functions with JavaScript bodies ('use gpu') need the unplugin-typegpu build step, which Biliup
// doesn't run.
import tgpu from "typegpu";
import * as d from "typegpu/data";

// The shaders module alone (D219: plugin:check's locked-down child may read just that file of core).
import { MAX_SHADER_WGSL, shaderDefProblems, shaderParamProblems, stripWgslComments, type ShaderParam } from "../core/visual/shaders";

type Schema = { readonly type?: string; readonly propTypes?: Record<string, { readonly type?: string }>; $name?: (n: string) => unknown };
type AnyFn = { readonly resourceType?: unknown; readonly shell?: { readonly argTypes?: ReadonlyArray<Schema>; readonly returnType?: Schema }; $name: (n: string) => unknown };

const typeName = (s: Schema | undefined) => s?.type ?? "?";

/** Whether a value is a TypeGPU function (tgpu.fn(...)…). */
export const isTgpuFn = (v: unknown): v is AnyFn => !!v && (typeof v === "object" || typeof v === "function") && "shell" in (v as object) && typeof (v as AnyFn).$name === "function";

/**
 * A TypeGPU function → the effect's WGSL, or the problems: its signature must be
 * (vec2f, f32, vec2f[, a struct]) → vec4f and the struct's fields the params' (numbers and switches
 * f32, colours vec3f, in order). Names the function "effect" and the struct "Params".
 */
export function resolveShaderFn(fn: unknown, params: ReadonlyArray<ShaderParam>): { readonly wgsl: string } | { readonly problems: ReadonlyArray<string> } {
  if (!isTgpuFn(fn)) return { problems: ["It isn't a TypeGPU function (tgpu.fn([...], d.vec4f)`...`)."] };
  const args = fn.shell?.argTypes ?? [];
  const want = params.length ? ["vec2f", "f32", "vec2f", "struct"] : ["vec2f", "f32", "vec2f"];
  const got = args.map(typeName);
  if (got.join(",") !== want.join(",") || typeName(fn.shell?.returnType) !== "vec4f") {
    return { problems: [`Its signature must be tgpu.fn([d.vec2f, d.f32, d.vec2f${params.length ? ", Params" : ""}], d.vec4f) (uv, t, size${params.length ? ", params" : ""}); it is (${got.join(", ")}) → ${typeName(fn.shell?.returnType)}.`] };
  }
  const problems: Array<string> = [...shaderParamProblems(params)];
  if (params.length) {
    const struct = args[3]!;
    const fields = Object.entries(struct.propTypes ?? {}).map(([k, v]) => `${k}: ${typeName(v)}`);
    const expected = params.map((p) => `${p.key}: ${p.type === "color" ? "vec3f" : "f32"}`);
    if (fields.join(", ") !== expected.join(", ")) problems.push(`Its params struct must be d.struct({ ${expected.map((f) => f.replace(": ", ": d.")).join(", ")} }) (numbers and switches d.f32, colours d.vec3f, in the manifest's order); it is { ${fields.map((f) => f.replace(": ", ": d.")).join(", ")} }.`);
    struct.$name?.("Params");
  }
  if (problems.length) return { problems };
  let wgsl: string;
  try {
    fn.$name("effect");
    wgsl = tgpu.resolve([fn as never]);
  } catch (error) {
    return { problems: [`TypeGPU couldn't resolve it: ${(error as Error).message.split("\n")[0]}`] };
  }
  wgsl = stripWgslComments(wgsl)
    .split("\n")
    .map((l) => l.trimEnd())
    .filter((l, i, all) => l || (i > 0 && all[i - 1]))
    .join("\n")
    .trim();
  const defProblems = shaderDefProblems({ params, wgsl });
  return defProblems.length ? { problems: defProblems } : { wgsl };
}

/**
 * The agent's shader (shader_write): its params and the WGSL body of `effect`, typed by TypeGPU
 * (the params struct made with d.struct, the function with tgpu.fn), resolved and checked. The
 * body is WGSL, `(uv, t, size, p) { ... }` or with its types written out; nothing of the agent's
 * runs here as JavaScript.
 */
export function buildShader(params: ReadonlyArray<ShaderParam>, body: string): { readonly wgsl: string } | { readonly problems: ReadonlyArray<string> } {
  if (typeof body !== "string" || !body.trim()) return { problems: ["The shader's body is empty."] };
  if (body.length > MAX_SHADER_WGSL) return { problems: [`The shader's body is over ${MAX_SHADER_WGSL / 1024} KB.`] };
  const paramProblems = shaderParamProblems(params);
  if (paramProblems.length) return { problems: paramProblems };
  const src = stripWgslComments(body).trim();
  if (!/^\([^)]*\)\s*(->\s*vec4<?f(?:32>)?\s*)?\{[\s\S]*\}$/.test(src)) return { problems: ["The body is the function's arguments and its block: (uv, t, size, p) { ... return vec4f(...); }."] };
  const fields = Object.fromEntries(params.map((p) => [p.key, p.type === "color" ? d.vec3f : d.f32]));
  const args = params.length ? [d.vec2f, d.f32, d.vec2f, d.struct(fields)] : [d.vec2f, d.f32, d.vec2f];
  let fn: unknown;
  try {
    fn = tgpu.fn(args as never, d.vec4f)(src);
  } catch (error) {
    return { problems: [`TypeGPU refused it: ${(error as Error).message.split("\n")[0]}`] };
  }
  return resolveShaderFn(fn, params);
}
