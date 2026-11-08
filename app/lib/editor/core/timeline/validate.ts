// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Validator for the Biliup project format and edit operations.
// Schema checks catch shape errors; the rules below catch what a schema can't:
// references, overlaps, ranges and module kinds.
import Ajv2020 from "ajv/dist/2020.js";
import type { ErrorObject, ValidateFunction } from "ajv";
// Import the shared schemas so the browser and server use the same data contracts.
import projectSchemaJson from "@/schemas/editor/project.schema.json" with { type: "json" };
import editOpsSchemaJson from "@/schemas/editor/edit-ops.schema.json" with { type: "json" };
import { projectDuration, resolveStarts, timelineProblems } from "@/app/lib/editor/core/timeline/rules";
import { SHADER, shaderDefProblems, shaderValueProblems, type ShaderParam } from "@/app/lib/editor/core/visual/shaders";
import type { Media, ModuleKind, ModuleManifest, Modules, Problem, Project } from "@/app/lib/editor/core/timeline/types";
import {RECORDING_MASK,type EditorMaskRegion} from './masks';

export { projectDuration, resolveStarts, timelineProblems };

const ajv = new Ajv2020({ allErrors: true, strict: false });
const projectSchema = ajv.compile(projectSchemaJson);
ajv.addSchema(editOpsSchemaJson);
// One validator per op kind, so a bad op gets that kind's errors, not every oneOf branch's.
const opSchemas = new Map<unknown, ValidateFunction>(
  editOpsSchemaJson.$defs.op.oneOf.map((branch, i) => [branch.properties.op.const, ajv.compile({ $ref: `${editOpsSchemaJson.$id}#/$defs/op/oneOf/${i}` })]),
);
const UNSAFE_KEYS = new Set(["__proto__", "constructor", "prototype"]);
const paramsCache = new Map<string, ValidateFunction>();

/** ajv's extra detail for "additional/unevaluated property" errors. */
const extra = (e: ErrorObject, key: "additionalProperty" | "unevaluatedProperty") => {
  const value = (e.params as Record<string, unknown> | undefined)?.[key];
  return value ? `: ${value}` : "";
};

/** D206: an enum error names the values it takes ("must be one of: rect, circle, line"), so the fix is in the refusal. */
const allowedOf = (e: ErrorObject) => {
  const values = (e.params as { allowedValues?: unknown } | undefined)?.allowedValues;
  return e.keyword === "enum" && Array.isArray(values) ? ` (${values.map((v) => JSON.stringify(v)).join(", ")})` : "";
};

/**
 * Check a list of edit ops against spec/schema/edit-ops.schema.json, plus what the schema
 * can't say: no prototype keys anywhere and no renaming an item through update_item.
 * Returns problems ({ code, path, message }); empty when the ops are well-formed.
 */
export function validateOps(ops: unknown): Array<Problem> {
  if (!Array.isArray(ops) || ops.length === 0) return [err("ops", "/", "an edit needs at least one op")];
  const errors: Array<Problem> = [];
  const findUnsafe = (value: unknown, path: string) => {
    if (!value || typeof value !== "object") return;
    for (const key of Object.keys(value)) {
      if (UNSAFE_KEYS.has(key)) errors.push(err("unsafe-key", `${path}/${key}`, `"${key}" is not allowed`));
      else findUnsafe((value as Record<string, unknown>)[key], `${path}/${key}`);
    }
  };
  ops.forEach((op: { op?: unknown; set?: unknown } | null | undefined, i) => {
    const path = `/${i}`;
    findUnsafe(op, path);
    const check = opSchemas.get(op?.op);
    if (!check) return errors.push(err("unknown-op", `${path}/op`, `unknown op ${JSON.stringify(op?.op)}`));
    if (!(check(op) as boolean)) {
      for (const e of check.errors!) errors.push(err("schema", path + (e.instancePath || ""), `${op!.op}: ${e.message}${allowedOf(e)}${extra(e, "additionalProperty")}`));
    }
    if (op!.op === "update_item" && op!.set && typeof op!.set === "object" && "id" in op!.set) {
      errors.push(err("rename", `${path}/set/id`, "an item's id can't be changed"));
    }
  });
  return errors;
}

function err(code: string, path: string, message: string): Problem {
  return { code, path, message };
}

export function validateProject(input: unknown, modules: Modules): Array<Problem> {
  if (!projectSchema(input)) {
    return projectSchema.errors!.map((e) => err("schema", e.instancePath || "/", `${e.message}${allowedOf(e)}${extra(e, "additionalProperty")}${extra(e, "unevaluatedProperty")}`));
  }
  const project = input as unknown as Project;
  const errors: Array<Problem> = [];
  const media = new Map<string, Media>(project.media.map((m) => [m.id, m]));
  const trackIds = new Set(project.tracks.map((t) => t.id));
  const duration = projectDuration(project);

  // Unique ids across media, tracks, items and scenes.
  const seen = new Set<string>();
  const ids: Array<[string, string]> = [
    ...project.media.map((m): [string, string] => [m.id, `/media/${m.id}`]),
    ...project.tracks.map((t): [string, string] => [t.id, `/tracks/${t.id}`]),
    ...project.tracks.flatMap((t) => t.items.map((i): [string, string] => [i.id, `/tracks/${t.id}/items/${i.id}`])),
    ...(project.scenes ?? []).map((s): [string, string] => [s.id, `/scenes/${s.id}`]),
  ];
  for (const [id, path] of ids) {
    if (seen.has(id)) errors.push(err("duplicate-id", path, `id "${id}" is used more than once`));
    seen.add(id);
  }

  const mains = project.tracks.filter((t) => t.role === "main");
  if (mains.length !== 1) errors.push(err("main-track-count", "/tracks", `expected exactly one main track, found ${mains.length}`));

  const validateModuleUse = (id: string, kinds: ReadonlyArray<ModuleKind>, path: string) => {
    const m = modules.get(id);
    if (!m) errors.push(err("unknown-module", path, `module "${id}" is not installed`));
    else if (!kinds.includes(m.kind)) errors.push(err("module-kind", path, `module "${id}" is a ${m.kind}, expected ${kinds.join(" or ")}`));
    return m;
  };
  const checkParams = (manifest: ModuleManifest | undefined, value: unknown, path: string) => {
    if (!manifest?.params) return;
    if (!paramsCache.has(manifest.id)) paramsCache.set(manifest.id, ajv.compile(manifest.params));
    const check = paramsCache.get(manifest.id)!;
    if (!check(value ?? {})) {
      for (const e of check.errors!) errors.push(err("params", path + e.instancePath, `${manifest.id}: ${e.message}${allowedOf(e)}${extra(e, "additionalProperty")}`));
    }
  };
  const checkMediaRefs = (value: unknown, path: string) => {
    if (value && typeof value === "object") {
      const ref = value as { media?: unknown };
      if (typeof ref.media === "string" && Object.keys(value).length === 1 && !media.has(ref.media)) {
        errors.push(err("unknown-media", path, `media "${ref.media}" does not exist`));
      }
      for (const [k, v] of Object.entries(value)) checkMediaRefs(v, `${path}/${k}`);
    }
  };

  // A media's source only says where it came from (D98): a recording made with a plugin that
  // isn't installed here (or isn't in this map: plugins are discovered at run time) still plays.
  // A known module must be a capture source or a generator.
  for (const m of project.media) {
    if (m.source && modules.has(m.source.module)) validateModuleUse(m.source.module, ["capture-source", "generator"], `/media/${m.id}/source`);
  }

  for (const track of project.tracks) {
    const tpath = `/tracks/${track.id}`;
    const main = track.role === "main";
    if (main && track.kind !== "video") errors.push(err("main-track-kind", tpath, "the main track must be a video track"));
    for (const fx of track.mixer?.effects ?? []) {
      const m = validateModuleUse(fx.module, ["audio-processor"], `${tpath}/mixer`);
      checkParams(m, fx.params, `${tpath}/mixer/${fx.module}`);
      for (const under of (fx.params?.under as ReadonlyArray<string> | undefined) ?? []) {
        // D85: a deleted key track would leave the duck pointing at nothing; say which track ducks under it.
        if (!trackIds.has(under)) errors.push(err("unknown-track", `${tpath}/mixer`, `track "${track.name}" (${track.id}) ducks under track "${under}", which doesn't exist: remove or retarget its audio.duck entry (in the same edit, to delete "${under}")`));
      }
    }

    track.items.forEach((item, index) => {
      const ipath = `${tpath}/items/${item.id}`;
      if (item.type === "component") {
        const m = validateModuleUse(item.component!, ["component"], ipath);
        checkParams(m, item.props, `${ipath}/props`);
      }
      if (item.props) checkMediaRefs(item.props, `${ipath}/props`);
      if (item.effects) checkMediaRefs(item.effects, `${ipath}/effects`);
      if (item.type === "text" && item.style?.preset) validateModuleUse(item.style.preset, ["text-style"], `${ipath}/style`);
      if (item.type === "captions" && item.style?.preset) validateModuleUse(item.style.preset, ["text-style"], `${ipath}/style`);
      for (const [k, a] of Object.entries(item.animation ?? {})) {
        // A move's params (word-by-word style, zoom scale and ease; D81) are checked against its manifest.
        const m = validateModuleUse(a.module, ["move"], `${ipath}/animation/${k}`);
        if (a.params !== undefined) checkParams(m, a.params, `${ipath}/animation/${k}/params`);
      }
      for (const fx of item.effects ?? []) {
        const kinds: ReadonlyArray<ModuleKind> = track.kind === "audio" ? ["audio-processor"] : ["effect"];
        const m = validateModuleUse(fx.module, kinds, `${ipath}/effects`);
        checkParams(m, fx.params, `${ipath}/effects/${fx.module}`);
        if(fx.module===RECORDING_MASK){
          const path=`${ipath}/effects/${fx.module}`;
          if(item.effects!.filter(effect=>effect.module===RECORDING_MASK).length>1)errors.push(err('mask-effect',path,'A clip can have only one mask effect.'));
          if(item.type!=='clip')errors.push(err('mask-item',path,'Masks need a video or image clip.'));
          const regions=fx.params?.regions;
          if(Array.isArray(regions)){
            const ids=new Set<string>(),end=Math.max(1,Math.round(item.duration*1000/project.canvas.fps));
            for(const region of regions as EditorMaskRegion[]){
              if(!region||typeof region!=='object')continue;
              if(ids.has(region.id))errors.push(err('mask-id',path,'Mask region IDs must be unique.'));
              ids.add(region.id);
              if(region.x+region.width>1.000001||region.y+region.height>1.000001)errors.push(err('mask-bounds',path,'The mask must stay inside the source picture.'));
              if(Array.isArray(region.intervals)&&region.intervals.some(span=>span&&!(span.from_ms>=0&&span.to_ms>span.from_ms&&span.to_ms<=end)))errors.push(err('mask-range',path,'Mask ranges must stay inside the clip.'));
              if(region.effect_type==='image'&&!project.media.some(media=>media.id===region.image?.media&&media.kind==='image'))errors.push(err('mask-image',path,'The mask image is no longer in the project.'));
            }
          }
        }
        // A shader effect (D213): its definition must still pass the checks both drawers rely on.
        if (fx.module === SHADER && m) {
          const p = (fx.params ?? {}) as { fields?: unknown; wgsl?: unknown; values?: unknown };
          if (Array.isArray(p.fields) && typeof p.wgsl === "string") {
            const fields = p.fields as ReadonlyArray<ShaderParam>;
            for (const problem of [...shaderDefProblems({ params: fields, wgsl: p.wgsl }), ...shaderValueProblems(fields, (p.values && typeof p.values === "object" ? p.values : {}) as Record<string, unknown>)]) errors.push(err("params", `${ipath}/effects/${fx.module}`, `${SHADER}: ${problem}`));
          }
        }
      }
      // The transition's own limits (its module manifest); the timeline rules are in rules.ts.
      if (item.transitionIn) {
        const tr = item.transitionIn;
        const limits = validateModuleUse(tr.module, ["transition"], `${ipath}/transitionIn`)?.transition;
        if (main && index > 0 && limits && (tr.duration < (limits.minFrames ?? 1) || tr.duration > (limits.maxFrames ?? Infinity))) {
          errors.push(err("transition-length", ipath, `${tr.module} takes ${limits.minFrames} to ${limits.maxFrames} frames`));
        }
      }
    });
  }
  errors.push(...timelineProblems(project));

  // Scenes and markers sit inside the video, scenes don't overlap.
  const scenes = [...(project.scenes ?? [])].sort((a, b) => a.start - b.start);
  scenes.forEach((s, i) => {
    if (s.end <= s.start) errors.push(err("scene-range", `/scenes/${s.id}`, "a scene must end after it starts"));
    if (s.end > duration) errors.push(err("scene-range", `/scenes/${s.id}`, `ends at ${s.end}, after the video ends (${duration})`));
    if (i > 0 && s.start < scenes[i - 1]!.end) errors.push(err("scene-overlap", `/scenes/${s.id}`, `overlaps "${scenes[i - 1]!.id}"`));
  });
  // Issue #118: marked sections end after they start, have their own ids and don't overlap.
  const sections = [...(project.sections ?? [])].sort((a, b) => a.start - b.start);
  sections.forEach((s, i) => {
    if (s.end <= s.start) errors.push(err("section-range", `/sections/${s.id}`, "a section must end after it starts"));
    if (sections.findIndex((x) => x.id === s.id) !== i) errors.push(err("duplicate-id", `/sections/${s.id}`, `section id "${s.id}" is used twice`));
    if (i > 0 && s.start < sections[i - 1]!.end) errors.push(err("section-overlap", `/sections/${s.id}`, `overlaps "${sections[i - 1]!.id}"`));
  });
  for (const mk of project.markers ?? []) {
    if (mk.frame > duration) errors.push(err("marker-range", "/markers", `marker at ${mk.frame} is after the video ends (${duration})`));
  }
  for (const fx of project.master?.effects ?? []) {
    const m = validateModuleUse(fx.module, ["audio-processor"], "/master/effects");
    checkParams(m, fx.params, `/master/effects/${fx.module}`);
  }
  return errors;
}
