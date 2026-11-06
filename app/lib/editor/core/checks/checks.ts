// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Taste checks (D90): what a project does against the best practices for short product videos
// as far as the model alone can tell. Each rule is objective (read
// time, punctuation, sizes, speeds, sound) and says one plain sentence per finding; some carry a
// fix (edit ops planned against the project, applied as one undoable edit). Checks never block an
// export: the editor lists them, the export dialog mentions them, the agent's project_check returns
// them.
//
// Browser-safe and pure, O(items) (O(items log items) for the overlaps), so the editor runs it on
// every snapshot. Rules are a registry (CHECK_RULES): a module can pass more rules later (Phase 8).
// A project turns rules off with checks.disabled (rule ids).
import type { Id, Op, Project } from "@/app/lib/editor/core/timeline/types";
import { type CheckRule, type Check, checkContext, type CheckFix, type CheckSeverity, textLabel } from "@/app/lib/editor/core/checks/context";
import { TEXT_RULES } from "@/app/lib/editor/core/checks/rules/text";
import { MOTION_RULES, CANVAS_RULES } from "@/app/lib/editor/core/checks/rules/motion";
import { SOUND_RULES } from "@/app/lib/editor/core/checks/rules/sound";
import type { SoundProfile } from "@/app/lib/editor/core/checks/soundSheet";
export { textLabel, readFrames, FILLER_PHRASES, fixDots, fixPeriods, FOOTNOTE_LOOK } from "@/app/lib/editor/core/checks/context";
export type { CheckSeverity, CheckFix, Check, CheckContext, Placed, CheckRule } from "@/app/lib/editor/core/checks/context";
export { SFX_PER_10S, SFX_REPEAT_MAX, SFX_HITS_PER_15S, SFX_UNDER_MUSIC_DB, SFX_FIX_UNDER_DB, SFX_SYNC_BEFORE_S, SFX_SYNC_AFTER_S, SLOW_BPM, FAST_CUT_S, SOUND_PROFILES, SOUND_LIMITS, soundProfileText, sfxItems, visualEvents, soundSheet, SOUND_HARD, soundHard } from "@/app/lib/editor/core/checks/soundSheet";
export type { SoundProfile, SoundLimits, SfxKind, PlacedSfx, VisualEvent } from "@/app/lib/editor/core/checks/soundSheet";

export const CHECK_RULES: ReadonlyArray<CheckRule> = [...TEXT_RULES, ...MOTION_RULES, ...SOUND_RULES, ...CANVAS_RULES];

/** The rule ids, in the registry's order. */
export const CHECK_IDS = CHECK_RULES.map((r) => r.id);

/** The rule ids a project turned off (checks.disabled), ignoring anything that isn't a string. */
export const disabledChecks = (project: Pick<Project, "checks">): ReadonlyArray<string> => (Array.isArray(project.checks?.disabled) ? project.checks.disabled.filter((d): d is string => typeof d === "string") : []);

/**
 * The project's checks: every rule (CHECK_RULES, plus the `rules` a module adds; one with a built-in
 * id replaces it) that isn't in the project's checks.disabled or `disabled`, warnings first, then
 * by frame. `speechMedia`: media known to hold speech (a transcript), for the ducking rule; media
 * that captions were made from count too. Never throws: a rule that fails is skipped.
 */
export function projectChecks(project: Project, options: { readonly rules?: ReadonlyArray<CheckRule>; readonly disabled?: ReadonlyArray<string>; readonly speechMedia?: Iterable<Id>; readonly sound?: SoundProfile } = {}): Array<Check> {
  const off = new Set([...disabledChecks(project), ...(options.disabled ?? [])]);
  const ctx = checkContext(project, options);
  const out: Array<Check> = [];
  for (const rule of allRules(options.rules)) {
    if (off.has(rule.id)) continue;
    try {
      for (const found of rule.check(ctx)) out.push({ ...found, id: rule.id, severity: found.severity ?? rule.severity });
    } catch {
      // A broken rule mustn't take the others (or the editor) down.
    }
  }
  const rank = (c: Check) => (c.severity === "warn" ? 0 : 1);
  return out.sort((a, b) => rank(a) - rank(b) || (a.frame ?? Infinity) - (b.frame ?? Infinity));
}

/** The built-in rules with a module's added (same id: the module's replaces the built-in one). */
function allRules(extra: ReadonlyArray<CheckRule> = []): ReadonlyArray<CheckRule> {
  if (!extra.length) return CHECK_RULES;
  const ids = new Set(extra.map((r) => r.id));
  return [...CHECK_RULES.filter((r) => !ids.has(r.id)), ...extra];
}

/** A rule by id (the built-in ones and `extra`), for titles in the UI. */
export const checkRule = (id: string, extra: ReadonlyArray<CheckRule> = []) => allRules(extra).find((r) => r.id === id);

/** The update_project that turns a rule off (or back on) for the project. */
export function planDisableCheck(project: Pick<Project, "checks">, id: string, off = true): CheckFix {
  const now = disabledChecks(project);
  const next = off ? [...new Set([...now, id])] : now.filter((d) => d !== id);
  const title = checkRule(id)?.title ?? id;
  const rest = { ...(project.checks ?? {}) } as Record<string, unknown>;
  delete rest.disabled;
  const checks = next.length ? { ...rest, disabled: next } : Object.keys(rest).length ? rest : null;
  return { summary: off ? `Turn off the check "${title}"` : `Turn on the check "${title}"`, ops: [{ op: "update_project", set: { checks } }] };
}

const hex6 = (c: unknown): string | undefined => {
  if (typeof c !== "string") return undefined;
  const m = /^#([0-9a-f]{3}|[0-9a-f]{6})$/i.exec(c.trim());
  if (!m) return undefined;
  const h = m[1]!.toLowerCase();
  return `#${h.length === 3 ? [...h].map((x) => x + x).join("") : h}`;
};
const rgbOf = (h: string) => [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16));
const nearest = (h: string, palette: ReadonlyArray<string>) => palette.reduce((best, p) => {
  const [a, b] = [rgbOf(h), rgbOf(p)];
  const d = (a[0]! - b[0]!) ** 2 + (a[1]! - b[1]!) ** 2 + (a[2]! - b[2]!) ** 2;
  return d < best.d ? { p, d } : best;
}, { p: palette[0]!, d: Infinity }).p;

/**
 * D186: the brand kit's colours rule (U7): text colours, outlines, text boxes and backgrounds
 * (component.solid) written as a hex colour that isn't one of the kit's. Theme tokens (fg, accent…)
 * are fine: the theme carries the kit. Added to project_check's rules while a kit is active; the
 * fix moves each colour to the kit's nearest.
 */
export function brandColourRule(kitName: string, colors: ReadonlyArray<string>): CheckRule {
  const palette = [...new Set(colors.map(hex6).filter((c): c is string => !!c))];
  return {
    id: "brand-colours",
    title: "Brand colours",
    description: `Colours come from the brand kit "${kitName}".`,
    severity: "warn",
    check: (ctx) => {
      if (!palette.length) return [];
      const off = (c: unknown) => {
        const h = hex6(c);
        return h && !palette.includes(h) ? h : undefined;
      };
      const out: Array<Omit<Check, "id" | "severity"> & { severity?: CheckSeverity }> = [];
      for (const track of ctx.project.tracks) {
        if (track.hidden) continue;
        for (const item of track.items) {
          const at = ctx.starts.get(item.id) ?? 0;
          const lock = track.locked === true || item.locked === true;
          if (item.type === "text" && item.style) {
            const st = item.style as Record<string, unknown>;
            const found = [off(st.color), off((st.stroke as { color?: unknown } | undefined)?.color), off((st.background as { color?: unknown } | undefined)?.color)].filter((x): x is string => !!x);
            if (!found.length) continue;
            const fixStyle: Record<string, unknown> = { ...st };
            if (off(st.color)) fixStyle.color = nearest(off(st.color)!, palette);
            if (st.stroke && typeof st.stroke === "object" && off((st.stroke as { color?: unknown }).color)) fixStyle.stroke = { ...(st.stroke as object), color: nearest(off((st.stroke as { color?: unknown }).color)!, palette) };
            if (st.background && typeof st.background === "object" && off((st.background as { color?: unknown }).color)) fixStyle.background = { ...(st.background as object), color: nearest(off((st.background as { color?: unknown }).color)!, palette) };
            out.push({ key: `brand-colours:${item.id}`, message: `"${textLabel(item.text)}" uses ${found.join(", ")}, not a colour of the kit "${kitName}".`, itemIds: [item.id], frame: at, ...(lock ? {} : { fix: { summary: `Use the kit's colours on "${textLabel(item.text)}"`, ops: [{ op: "update_item", item: item.id, set: { style: fixStyle } } as Op] } }) });
          } else if (item.type === "component" && item.component === "component.solid" && item.props) {
            const pr = item.props as Record<string, unknown>;
            const keys = ["color", "to", "via"].filter((k) => off(pr[k]));
            if (!keys.length) continue;
            const props = { ...pr, ...Object.fromEntries(keys.map((k) => [k, nearest(off(pr[k])!, palette)])) };
            out.push({ key: `brand-colours:${item.id}`, message: `A background uses ${keys.map((k) => off(pr[k])).join(", ")}, not a colour of the kit "${kitName}".`, itemIds: [item.id], frame: at, ...(lock ? {} : { fix: { summary: "Use the kit's colours on the background", ops: [{ op: "update_item", item: item.id, set: { props } } as Op] } }) });
          }
        }
      }
      return out;
    },
  };
}
