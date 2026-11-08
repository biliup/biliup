// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import type { Item, ModuleUse } from "@/app/lib/editor/core/timeline/types";
import { SHADER } from "@/app/lib/editor/core/visual/shaders";
import { fxSkips } from "@/app/lib/editor/core/visual/fx/clipFx";
import { type LookDef, LOOK, lookInfo, num, type AdjustKey, ADJUST, adjustOf, NO_ADJUST, ADJUST_KEYS, ADJUST_INFO, effectInfo, effectParams } from "@/app/lib/editor/core/visual/fx/defs";

// ---- edits ----

type Plan = { readonly ops: ReadonlyArray<{ op: "update_item"; item: string; set: Record<string, unknown> }>; readonly summary: string } | { readonly problem: string };

interface Target {
  readonly tracks: ReadonlyArray<{ readonly kind: string; readonly locked?: boolean; readonly items: ReadonlyArray<Pick<Item, "id" | "type" | "effects" | "media" | "locked" | "name">> }>;
  readonly media?: ReadonlyArray<{ readonly id: string; readonly kind: string; readonly name?: string; readonly path?: string }>;
}

function clipOf(project: Target, id: string): { item: Pick<Item, "id" | "type" | "effects" | "name">; name: string } | { problem: string } {
  for (const t of project.tracks)
    for (const it of t.items) {
      if (it.id !== id) continue;
      const media = it.media ? project.media?.find((m) => m.id === it.media) : undefined;
      if (it.type !== "clip" || !media || media.kind === "audio") return { problem: "Effects go on video clips and stills." };
      // SVG stills are drawn by Chromium in the export, not decoded: looks and effects don't apply to them (D138).
      if (fxSkips(media)) return { problem: "Looks and effects don't apply to SVG images." };
      if (t.locked || it.locked) return { problem: "That clip is locked: unlock it to change its effects." };
      return { item: it, name: it.name ?? media.name ?? it.id };
    }
  return { problem: "Pick a clip first." };
}

const setEffects = (id: string, list: ReadonlyArray<ModuleUse>, summary: string): Plan => ({ ops: [{ op: "update_item", item: id, set: { effects: list.length ? list : null } }], summary });
const pct = (v: number) => `${Math.round(v * 100)}%`;

/** Give a clip a look (one look per clip: it replaces the one it had), or with undefined take it off. */
export function planSetLook(project: Target, itemId: string, look: string | undefined, opts: { intensity?: number; custom?: LookDef; label?: string } = {}): Plan {
  const c = clipOf(project, itemId);
  if ("problem" in c) return c;
  const list = [...(c.item.effects ?? [])];
  const at = list.findIndex((e) => e.module === LOOK);
  if (look === undefined) {
    if (at < 0) return { ops: [], summary: "" };
    list.splice(at, 1);
    return setEffects(itemId, list, `Remove the filter from ${c.name}`);
  }
  const info = lookInfo(look);
  if (!info && !opts.custom) return { problem: `There's no filter called ${look}.` };
  const before = at >= 0 ? ((list[at]!.params ?? {}) as Record<string, unknown>) : undefined;
  const intensity = num(opts.intensity ?? (before?.look === look ? before?.intensity : undefined), 0, 1, 1);
  const params: Record<string, unknown> = { look, intensity: Math.round(intensity * 100) / 100, ...(opts.custom ? { custom: opts.custom, label: opts.label ?? look } : {}) };
  // An off filter (bypass, F18) stays off while its intensity changes; a new filter is on.
  const entry = { module: LOOK, params, ...(at >= 0 && list[at]!.bypass && before?.look === look ? { bypass: true } : {}) };
  if (at >= 0) list[at] = entry;
  else list.unshift(entry);
  const name = info?.name ?? opts.label ?? look;
  return setEffects(itemId, list, opts.intensity !== undefined && before?.look === look ? `Set ${name} on ${c.name} to ${pct(intensity)}` : `Filter ${name} on ${c.name}`);
}

/** Change a clip's Adjust (a patch of its values; 0 is neutral); an all-neutral Adjust is removed. */
export function planAdjust(project: Target, itemId: string, patch: Partial<Record<AdjustKey, number>> | "reset"): Plan {
  const c = clipOf(project, itemId);
  if ("problem" in c) return c;
  const list = [...(c.item.effects ?? [])];
  const at = list.findIndex((e) => e.module === ADJUST);
  const before = at >= 0 ? adjustOf(list[at]!.params) : NO_ADJUST;
  const next = patch === "reset" ? NO_ADJUST : adjustOf({ ...before, ...patch });
  const params: Record<string, number> = {};
  for (const k of ADJUST_KEYS) if (next[k] !== 0) params[k] = Math.round(next[k] * 1000) / 1000;
  if (Object.keys(params).length) {
    const entry = { module: ADJUST, params, ...(at >= 0 && list[at]!.bypass ? { bypass: true } : {}) };
    if (at >= 0) list[at] = entry;
    else list.push(entry);
  } else if (at >= 0) list.splice(at, 1);
  else return { ops: [], summary: "" };
  const keys = patch === "reset" ? [] : (Object.keys(patch) as Array<AdjustKey>);
  const info = keys.length === 1 ? ADJUST_INFO.find((a) => a.key === keys[0]) : undefined;
  const summary = patch === "reset" ? `Reset the adjustments of ${c.name}` : info ? `Set ${info.name.toLowerCase()} of ${c.name} to ${adjustWord(info.key, next[info.key])}` : `Adjust ${c.name}`;
  return setEffects(itemId, list, summary);
}

/** An Adjust value as the Inspector shows it: stops for exposure, −100 to 100 for the rest. */
export const adjustWord = (key: AdjustKey, v: number) => (key === "exposure" ? `${v > 0 ? "+" : ""}${Math.round(v * 100) / 100} EV` : `${v > 0 ? "+" : ""}${Math.round(v * 100)}`);

/** Add an effect to a clip (with its defaults), or with `params` change the one it has. */
export function planEffect(project: Target, itemId: string, module: string, params?: Record<string, number>): Plan {
  const c = clipOf(project, itemId);
  const info = effectInfo(module);
  if (!info) return { problem: `There's no effect called ${module}.` };
  if ("problem" in c) return c;
  const list = [...(c.item.effects ?? [])];
  const at = list.findIndex((e) => e.module === module);
  if (at >= 0 && !params) {
    if (!list[at]!.bypass) return { ops: [], summary: "" };
    list[at] = { ...list[at]!, bypass: false };
    return setEffects(itemId, list, `Enable ${info.name.toLowerCase()} on ${c.name}`);
  }
  const base = at >= 0 ? effectParams(module, list[at]!.params) : effectParams(module, {});
  const next = effectParams(module, { ...base, ...(params ?? {}) });
  const entry = { module, ...(info.params.length ? { params: next } : {}), ...(at >= 0 && list[at]!.bypass ? { bypass: true } : {}) };
  if (at >= 0) {
    list[at] = entry;
    const keys = Object.keys(params ?? {});
    const p = keys.length === 1 ? info.params.find((d) => d.key === keys[0]) : undefined;
    return setEffects(itemId, list, p ? `Set ${info.name.toLowerCase()} ${p.name.toLowerCase()} on ${c.name} to ${Math.round(next[p.key]! * 100) / 100}${p.unit ? ` ${p.unit}` : ""}` : `Change ${info.name.toLowerCase()} on ${c.name}`);
  }
  list.push(entry);
  return setEffects(itemId, list, `Add ${info.name.toLowerCase()} to ${c.name}`);
}

/** Take an effect (or the look, or Adjust) off a clip. */
export function planRemoveEffect(project: Target, itemId: string, module: string): Plan {
  const c = clipOf(project, itemId);
  if ("problem" in c) return c;
  const list = (c.item.effects ?? []).filter((e) => e.module !== module);
  if (list.length === (c.item.effects ?? []).length) return { problem: "That clip doesn't have it." };
  const name = module === LOOK ? "the filter" : module === ADJUST ? "the adjustments" : module === SHADER ? "the shader effect" : (effectInfo(module)?.name.toLowerCase() ?? module);
  return setEffects(itemId, list, `Remove ${name} from ${c.name}`);
}

/**
 * Turn one of a clip's effects (or its filter, or Adjust) off or on (F18): an off entry keeps its
 * place and settings (`bypass`, as the mixer's audio effects) and isn't drawn by the preview or the
 * export (clipFx skips it).
 */
export function planEffectBypass(project: Target, itemId: string, module: string, on: boolean): Plan {
  const c = clipOf(project, itemId);
  if ("problem" in c) return c;
  const list = [...(c.item.effects ?? [])];
  const at = list.findIndex((e) => e.module === module);
  if (at < 0) return { problem: "That clip doesn't have it." };
  if (!list[at]!.bypass === on) return { problem: on ? "It's already on." : "It's already off." };
  const { bypass: _, ...rest } = list[at]!;
  list[at] = on ? rest : { ...rest, bypass: true };
  const name = module === LOOK ? "the filter" : module === ADJUST ? "the adjustments" : module === SHADER ? "the shader effect" : (effectInfo(module)?.name.toLowerCase() ?? module);
  return setEffects(itemId, list, `Turn ${name} on ${c.name} ${on ? "on" : "off"}`);
}

/** A clip's effects in words (History, D136): "filter Warm 80%, exposure +0.3 EV, blur 8 px". */
export function fxWords(effects: unknown): string {
  const list = Array.isArray(effects) ? (effects as ReadonlyArray<ModuleUse>) : [];
  if (!list.length) return "no effects";
  return list
    .map((e) => {
      if (e.module === LOOK) {
        const p = (e.params ?? {}) as Record<string, unknown>;
        const name = typeof p.label === "string" ? p.label : (lookInfo(String(p.look))?.name ?? String(p.look));
        return `filter ${name} ${pct(num(p.intensity, 0, 1, 1))}${e.bypass ? " (off)" : ""}`;
      }
      if (e.module === ADJUST) {
        const a = adjustOf(e.params);
        const parts = ADJUST_INFO.filter((i) => a[i.key] !== 0).map((i) => `${i.name.toLowerCase()} ${adjustWord(i.key, a[i.key])}`);
        return (parts.length ? parts.join(", ") : "no adjustments") + (e.bypass ? " (off)" : "");
      }
      if (e.module === SHADER) {
        const p = (e.params ?? {}) as Record<string, unknown>;
        return `shader ${typeof p.name === "string" ? p.name : String(p.shader)}${e.bypass ? " (off)" : ""}`;
      }
      const info = effectInfo(e.module);
      if (!info) return e.module;
      const p = effectParams(e.module, e.params);
      const first = info.params[0];
      return `${info.name.toLowerCase()}${first ? ` ${Math.round(p[first.key]! * 100) / 100}${first.unit ? ` ${first.unit}` : ""}` : ""}${e.bypass ? " (off)" : ""}`;
    })
    .join(", ");
}
