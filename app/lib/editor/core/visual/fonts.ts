// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Fonts everywhere text is drawn (D198, issue #34). The bundled DM Sans and DM Mono (D77) and the
// brand kit's font files are declared with @font-face from data: URLs: in the preview's page and
// the export rasteriser's page for text items and components, and inside each graphic's document
// (resolveGraphicFonts, applied where graphics are read, next to the bundled libraries of D193, so
// the preview, the export and render_frames load the very same document). A family a text, the
// theme or a graphic asks for that is none of those and isn't installed on this Mac is a finding
// (fontRule), not a silent fallback.
//
// Core can't read files: the server passes the faces (FontSource, with data: URLs) and says which
// families are installed.
import { textLabel, type CheckRule } from "@/app/lib/editor/core/checks/checks";
import type { Item, Project } from "@/app/lib/editor/core/timeline/types";

/** One @font-face: a family, its weight (one value or a "100 900" range), style and a data: URL. */
export interface FontSource {
  readonly family: string;
  readonly weight?: string | undefined;
  readonly style?: "normal" | "italic" | undefined;
  readonly format: "woff2" | "woff" | "truetype" | "opentype";
  readonly url: string;
}

/** The font file formats a kit can hold, by extension. */
export const FONT_FORMATS: Readonly<Record<string, FontSource["format"]>> = { ".woff2": "woff2", ".woff": "woff", ".ttf": "truetype", ".otf": "opentype" };
export const FONT_MIME: Readonly<Record<FontSource["format"], string>> = { woff2: "font/woff2", woff: "font/woff", truetype: "font/ttf", opentype: "font/otf" };

/** CSS generic families and the system aliases every Mac draws: never a missing font. */
export const GENERIC_FAMILIES: ReadonlySet<string> = new Set([
  "serif",
  "sans-serif",
  "monospace",
  "cursive",
  "fantasy",
  "system-ui",
  "ui-serif",
  "ui-sans-serif",
  "ui-monospace",
  "ui-rounded",
  "emoji",
  "math",
  "fangsong",
  "-apple-system",
  "blinkmacsystemfont",
  "inherit",
  "initial",
  "unset",
  "revert",
  "default",
]);

/** A family as compared: unquoted, single-spaced, lower case. */
export const familyKey = (family: string) => family.replace(/^["']|["']$/g, "").replace(/\s+/g, " ").trim().toLowerCase();

const SAFE_FAMILY = /^[\p{L}\p{N} _.-]{1,80}$/u;

/** The families of a CSS font-family list, in order, unquoted (a var() or an expression ends it). */
export function familiesOf(stack: string): Array<string> {
  const out: Array<string> = [];
  for (const raw of stack.split(",")) {
    const f = raw.trim().replace(/\s*!important$/i, "");
    if (!f) continue;
    if (/[$(){}`]/.test(f)) break;
    const unq = f.replace(/^(["'])(.*)\1$/, "$2").trim();
    if (!unq || !SAFE_FAMILY.test(unq)) break;
    out.push(unq.replace(/\s+/g, " "));
  }
  return out;
}

const isGeneric = (family: string) => GENERIC_FAMILIES.has(familyKey(family));

// font-family: … (CSS, inline styles), font-family="…" (SVG), fontFamily = "…" / fontFamily: "…" (JS).
const FAMILY_DECL = /font-family\s*:\s*([^;}<>\n]+)|font-family\s*=\s*(["'])(.*?)\2|fontFamily\s*[:=]\s*(["'`])(.*?)\4/gi;
// The font shorthand: CSS `font: 600 64px/1.1 "DM Sans", sans-serif` and canvas `ctx.font = "…"` (also in a template literal).
const FONT_SHORTHAND = /(?:\bfont\s*:\s*([^;}<>\n]+)|\.font\s*=\s*(["'`])(.*?)\2)/gi;
const SIZE = /(?:\d+(?:\.\d+)?|\$\{[^}]*\})(?:px|pt|em|rem|%|vh|vw|vmin|vmax|cqh|cqw|ex|ch)(?:\s*\/\s*\S+)?\s+(.+)$/i;

/** The font stacks a graphic's HTML names (CSS, SVG attributes, canvas ctx.font and style.fontFamily), each as its families. */
export function graphicFontStacks(html: string): Array<Array<string>> {
  const stacks: Array<Array<string>> = [];
  for (const m of html.matchAll(FAMILY_DECL)) {
    const stack = familiesOf((m[1] ?? m[3] ?? m[5] ?? "").replace(/["'`]\s*[;+].*$/, ""));
    if (stack.length) stacks.push(stack);
  }
  for (const m of html.matchAll(FONT_SHORTHAND)) {
    const value = (m[1] ?? m[3] ?? "").trim();
    const after = SIZE.exec(value)?.[1];
    if (!after) continue;
    const stack = familiesOf(after);
    if (stack.length) stacks.push(stack);
  }
  return stacks;
}

/** Every family a graphic's HTML names (each once, in order, generics left out). */
export function graphicFontFamilies(html: string): Array<string> {
  const seen = new Map<string, string>();
  for (const stack of graphicFontStacks(html)) for (const f of stack) if (!isGeneric(f) && !seen.has(familyKey(f))) seen.set(familyKey(f), f);
  return [...seen.values()];
}

/**
 * The families a graphic declares itself: an @font-face whose src is a data: URL (the only fonts
 * its CSP loads). Those are drawn in that font, so they're never missing.
 */
export function graphicDeclaredFamilies(html: string): Array<string> {
  const seen = new Map<string, string>();
  for (const m of html.matchAll(/@font-face\s*\{([^}]*)\}/gi)) {
    const body = m[1]!;
    if (!/src\s*:[^;]*url\(\s*["']?data:/i.test(body)) continue;
    const family = /font-family\s*:\s*(["']?)([^;"']+)\1/i.exec(body)?.[2]?.trim();
    if (family && SAFE_FAMILY.test(family) && !seen.has(familyKey(family))) seen.set(familyKey(family), family);
  }
  return [...seen.values()];
}

/** The family each stack asks for first (what the author meant), generics left out, each once. */
export function graphicAskedFamilies(html: string): Array<string> {
  const seen = new Map<string, string>();
  for (const stack of graphicFontStacks(html)) {
    const first = stack[0];
    if (first && !isGeneric(first) && !seen.has(familyKey(first))) seen.set(familyKey(first), first);
  }
  return [...seen.values()];
}

/** @font-face rules for the faces (no font-display: text waits for them). */
export function fontFaceRules(sources: ReadonlyArray<FontSource>): string {
  return sources
    .filter((s) => SAFE_FAMILY.test(s.family) && /^data:[a-z0-9/+.-]+;base64,[A-Za-z0-9+/=]+$/.test(s.url))
    .map((s) => `@font-face{font-family:"${s.family}";font-style:${s.style ?? "normal"};font-weight:${s.weight && /^\d{3,4}( \d{3,4})?$/.test(s.weight) ? s.weight : "100 1000"};src:url(${s.url}) format("${s.format}")}`)
    .join("\n");
}

/** The most bytes of font data one graphic's document carries (the export loads it as a data: URL, D193). */
export const MAX_GRAPHIC_FONT_BYTES = 600_000;

/**
 * A graphic's HTML with the faces of every available family it names declared in a <style> before
 * it (`sources`: the bundled fonts and the brand kit's). Faces go in until `budget` bytes of font
 * data; one past it becomes a comment saying so (the graphic then draws that family in a fallback,
 * and project_check names it). Families it doesn't name aren't added. Nothing else changes.
 */
export function resolveGraphicFonts(html: string, sources: ReadonlyArray<FontSource>, budget = MAX_GRAPHIC_FONT_BYTES): string {
  const named = new Set(graphicFontFamilies(html).map(familyKey));
  // A family the declarations don't show but the script names (a canvas font built from a
  // constant: const SANS = "'DM Sans', sans-serif"; ctx.font = `600 ${s}px ${SANS}`): declared too,
  // after the ones named outright, so canvas text never falls back to another font.
  for (const s of sources) if (mentionsFamily(html, s.family)) named.add(familyKey(s.family));
  if (!named.size) return html;
  const direct = new Set(graphicFontFamilies(html).map(familyKey));
  const ordered = [...sources.filter((s) => direct.has(familyKey(s.family))), ...sources.filter((s) => !direct.has(familyKey(s.family)))];
  const used: Array<FontSource> = [];
  const left: Array<string> = [];
  let bytes = 0;
  for (const s of ordered) {
    if (!named.has(familyKey(s.family))) continue;
    if (bytes + s.url.length > budget) {
      if (!left.includes(s.family)) left.push(s.family);
      continue;
    }
    bytes += s.url.length;
    used.push(s);
  }
  if (!used.length && !left.length) return html;
  const note = left.length ? `<!-- biliup: the font${left.length === 1 ? "" : "s"} ${left.map((f) => f.replace(/--/g, "- -")).join(", ")} left out: the graphic's fonts are over ${Math.round(budget / 1000)} KB -->\n` : "";
  return `${used.length ? `<style data-biliup-fonts>${fontFaceRules(used)}</style>\n` : ""}${note}${html}`;
}

/** Whether the HTML names `family` anywhere as a whole name (any case, any spacing between its words): in a string, a comment or markup. */
function mentionsFamily(html: string, family: string): boolean {
  if (!SAFE_FAMILY.test(family)) return false;
  const words = family.trim().split(/\s+/).map((w) => w.replace(/[.\-]/g, "\\$&"));
  return new RegExp(`(?<![\\p{L}\\p{N}_-])${words.join("\\s+")}(?![\\p{L}\\p{N}_-])`, "iu").test(html);
}

/** What a text item's font resolves to, as resolveFont does: the theme's family for a name, else the name (sans and mono: the default stacks). */
export function textFamily(project: Pick<Project, "theme">, item: Pick<Item, "style">): string | undefined {
  const value = item.style?.font;
  const name = typeof value === "string" && value ? value : "sans";
  const fonts = (project.theme?.fonts ?? {}) as Record<string, string>;
  const family = fonts[name] ?? (name === "sans" || name === "mono" ? "" : name);
  return family && SAFE_FAMILY.test(family) ? family : undefined;
}

/**
 * D198: the fonts rule. A text item, the theme or a graphic asks for a family that isn't bundled,
 * in the brand kit or installed (`available`), so it would be drawn in a fallback font. `graphics`:
 * src → the graphic file's HTML. An error for project_check and a blocker ("font") for the critic.
 */
export function fontRule(available: (family: string) => boolean, graphics: ReadonlyMap<string, string> = new Map()): CheckRule {
  const ok = (f: string) => isGeneric(f) || available(f);
  const fix = "使用内置字体，或将所需字体文件导入当前品牌。";
  return {
    id: "font-missing",
    title: "Fonts are there",
    description: "Every font a text, the theme or a graphic asks for is bundled, in the brand kit or installed, so nothing falls back to another font.",
    severity: "warn",
    check: (ctx) => {
      const out: Array<{ key: string; message: string; itemIds?: Array<string>; frame?: number; hint: string }> = [];
      const themeFonts = Object.entries((ctx.project.theme?.fonts ?? {}) as Record<string, unknown>).filter((e): e is [string, string] => typeof e[1] === "string");
      for (const [name, family] of themeFonts) {
        if (SAFE_FAMILY.test(family) && !ok(family)) out.push({ key: `font-missing:theme:${name}`, message: `主题 ${name} 使用的字体“${family}”未随工程提供，导出可能使用替代字体。`, hint: fix });
      }
      for (const p of ctx.texts) {
        const family = textFamily(ctx.project, p.item);
        if (!family || ok(family) || themeFonts.some(([, f]) => f === family)) continue;
        out.push({ key: `font-missing:${p.item.id}`, message: `“${textLabel(p.item.text)}”使用的字体“${family}”未随工程提供，导出可能使用替代字体。`, itemIds: [p.item.id], frame: p.start, hint: fix });
      }
      const users = new Map<string, Array<{ id: string; at: number }>>();
      for (const track of ctx.project.tracks) {
        if (track.hidden) continue;
        for (const item of track.items) if (item.type === "graphic" && typeof item.src === "string") users.set(item.src, [...(users.get(item.src) ?? []), { id: item.id, at: ctx.starts.get(item.id) ?? 0 }]);
      }
      for (const [src, items] of users) {
        const html = graphics.get(src);
        if (html === undefined) continue;
        // A family the graphic declares itself (its own @font-face with a data: font) is there.
        const own = new Set(graphicDeclaredFamilies(html).map(familyKey));
        const missing = graphicAskedFamilies(html).filter((f) => !ok(f) && !own.has(familyKey(f)));
        if (!missing.length) continue;
        out.push({ key: `font-missing:${src}`, message: `图形 ${src} 使用的字体 ${missing.join('、')} 未随工程提供，导出可能使用替代字体。`, itemIds: items.map((i) => i.id), frame: Math.min(...items.map((i) => i.at)), hint: fix });
      }
      return out;
    },
  };
}
