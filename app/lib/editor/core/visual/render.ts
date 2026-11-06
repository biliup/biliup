// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Shared by the preview and export: the one place text and backgrounds are turned into HTML, so
// an exported frame shows exactly what the editor showed (D39). Everything that decides how an
// item looks is inline on its element; RENDER_CSS holds only the few rules both need, and both
// inject it as is.
import { textLook, type TextLook, type WordStyle } from "@/app/lib/editor/core/text/looks";
import type { Canvas, Item, Project, TextBackground, TextShadow, TextStroke } from "@/app/lib/editor/core/timeline/types";

const ENTITIES: Readonly<Record<string, string>> = { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" };
export const esc = (s: unknown): string => String(s).replace(/[&<>"]/g, (c) => ENTITIES[c]!);

type Theme = NonNullable<Project["theme"]>;
/**
 * What the renderer needs of a project: its canvas and its theme (optional in the format), and how
 * a component's `{ media }` reference becomes an image URL there (D76): the preview's
 * /media/file URL, the export's data: URL. Without it, media references draw nothing.
 */
export type RenderTarget = {
  readonly canvas: Pick<Canvas, "width" | "height" | "background">;
  readonly theme?: Theme | undefined;
  readonly mediaUrl?: ((mediaId: string) => string | undefined) | undefined;
};

/**
 * The theme a project without one (or without some tokens) falls back to. New projects have no
 * theme, so these tokens are what "fg", "bg" and "accent" mean there; a project's own theme wins.
 */
export const DEFAULT_THEME = {
  colors: { bg: "#0A0A0B", fg: "#F2F1EE", mute: "#8C8C91", accent: "#99DAFF" },
  fonts: { sans: "", mono: "" },
} as const;

/** The system font stacks the preview and export both use when a theme names no font. */
const SANS = `-apple-system, BlinkMacSystemFont, "SF Pro Display", "Inter", "Helvetica Neue", sans-serif`;
const MONO = `"SF Mono", ui-monospace, Menlo, monospace`;

/**
 * The fonts Biliup bundles (D77, SIL OFL 1.1, assets/fonts/OFL.txt): DM Sans (variable, weights
 * 100–1000) and DM Mono 400 / 500. The preview and the export rasteriser both load them from data:
 * URLs with the same @font-face rules (fontFaceCss), so text measures the same in both. Text items
 * use them by name (style.font "DM Sans"); components use them by default.
 */
export const FONT_FACES = [
  { family: "DM Sans", file: "DMSans-Variable.woff2", weight: "100 1000" },
  { family: "DM Mono", file: "DMMono-Regular.woff2", weight: "400" },
  { family: "DM Mono", file: "DMMono-Medium.woff2", weight: "500" },
] as const;
export type FontFile = (typeof FONT_FACES)[number]["file"];

/** @font-face rules for the bundled fonts, given each file's URL (no font-display: they block until loaded). */
export const fontFaceCss = (url: (file: FontFile) => string) =>
  FONT_FACES.map((f) => `@font-face{font-family:"${f.family}";font-style:normal;font-weight:${f.weight};src:url(${url(f.file)}) format("woff2")}`).join("\n");

/** The bundled fonts' stacks (components draw with these; D77). */
export const DM_SANS = `'DM Sans', ${SANS.replace(/"/g, "'")}`;
export const DM_MONO = `'DM Mono', ${MONO.replace(/"/g, "'")}`;

/** Theme colours by token: the project's own over the defaults. */
export function themeColors(target: Pick<RenderTarget, "theme">): Record<string, string> {
  return { ...DEFAULT_THEME.colors, ...(target.theme?.colors ?? {}) };
}

// A colour or font name goes inside a style attribute (and a solid's background): only plain
// colour syntax is accepted, so nothing can end the declaration or smuggle in an image (url(),
// image-set(), var(), several layers). Anything else falls back.
const SAFE_COLOR = /^(#[0-9a-f]{3,8}|[a-z]{3,30}|(rgba?|hsla?)\(\s*-?[\d.]+(deg|%)?(\s*[,\s/]\s*-?[\d.]+%?){2,3}\s*\))$/i;
const SAFE_FONT = /^[\p{L}\p{N} _.-]{1,80}$/u;

/** A theme token ("fg") or a CSS colour as a CSS colour; `fallback` for anything unusable. */
export function resolveColor(target: Pick<RenderTarget, "theme">, value: unknown, fallback: string): string {
  if (typeof value !== "string" || !value.trim()) return fallback;
  const token = themeColors(target)[value];
  const color = token ?? value;
  return SAFE_COLOR.test(color) ? color : fallback;
}

/** A font token ("sans", "mono") or a family name as a CSS font-family list. */
export function resolveFont(target: Pick<RenderTarget, "theme">, value: unknown): string {
  const fonts: Record<string, string> = { ...(target.theme?.fonts ?? {}) };
  const name = typeof value === "string" && value ? value : "sans";
  const family = fonts[name] ?? (name === "sans" || name === "mono" ? "" : name);
  const stack = name === "mono" ? MONO : SANS;
  return family && SAFE_FONT.test(family) ? `"${family}", ${stack}` : stack;
}

// The looks (size, position, weight, font, stroke, shadow, box…) live in the text-style manifests
// (D80, looks.ts); textHtml uses them as fallbacks for the fields an item doesn't set.
export { PLAIN_LOOK, TEXT_STYLES, textLook } from "@/app/lib/editor/core/text/looks";
export type { TextLook } from "@/app/lib/editor/core/text/looks";

/** The safe margin in canvas pixels: the theme's, else 1/15 of the shorter side (72 px at 1080p). */
export const safeMargin = (target: RenderTarget) => target.theme?.margin ?? Math.round(Math.min(target.canvas.width, target.canvas.height) / 15);

/** The project's canvas colour, shown where nothing is drawn (black by default). */
export const canvasBackground = (target: RenderTarget) => resolveColor(target, target.canvas.background, "#000");

/** Rules the preview and export inject; everything per item is inline. */
export const RENDER_CSS = [
  ".pv-text{position:absolute;white-space:pre-wrap;overflow-wrap:break-word;cursor:default;user-select:none;box-sizing:border-box}",
  ".pv-text .pv-w{display:inline-block}",
  ".pv-text .pv-line{-webkit-box-decoration-break:clone;box-decoration-break:clone}",
  ".pv-text .accent{color:var(--accent)}",
  ".pv-text.acc-box .accent{background:var(--accent-bg);border-radius:0.18em;padding:0 0.14em;margin:0 -0.14em;-webkit-box-decoration-break:clone;box-decoration-break:clone}",
  ".pv-text b{font-weight:800}",
  ".pv-text code{font-family:var(--mono);font-size:0.9em}",
  ".pv-solid{position:absolute;inset:0}",
  ".pv-comp{position:absolute;box-sizing:border-box;user-select:none;cursor:default}",
  ".pv-comp *{box-sizing:border-box}",
  ".pv-comp .accent{color:var(--accent)}",
  ".pv-comp b{font-weight:800}",
].join("\n");

/** Markup → HTML, after escaping: [[accent]], **bold**, `mono`. */
export const markup = (text: string) =>
  esc(text)
    .replace(/\[\[(.*?)\]\]/gs, '<span class="accent">$1</span>')
    .replace(/\*\*(.+?)\*\*/gs, "<b>$1</b>")
    .replace(/`([^`]+)`/g, "<code>$1</code>");

/** An item's motion at one frame (core visualsAt computes it from its animation). */
export interface Motion {
  /** Offset in canvas pixels. */
  readonly dx?: number;
  readonly dy?: number;
  /** How much of a wipe is revealed, 0–1 (1 = all of it). */
  readonly reveal?: number;
  /** A zoom move (D81): the text's scale about its box's centre. */
  readonly scale?: number;
  /** A word-by-word move (D81): each word's progress, 0 (not there yet) → 1 (arrived), and how it arrives. */
  readonly words?: ReadonlyArray<number>;
  readonly wordStyle?: WordStyle;
  /** A typewriter move (D163): how many characters of the visible text show (the rest are hidden, or left out with a box). */
  readonly chars?: number;
  /** A blur move (D163): the blur radius in % of the canvas height (cqh, so the preview and export blur alike). */
  readonly blur?: number;
  /** A wiggle loop (D163): a tilt in degrees about the box's centre, added to the item's rotation. */
  readonly rotate?: number;
  /** A pop move (D163): opacity drawn in the HTML (not a linear ramp, so it isn't in the item's opacity). */
  readonly fade?: number;
}

export const num = (n: number) => String(Math.round(n * 10000) / 10000);


/** A colour at an opacity: hex as rgb(r g b / a%), anything else mixed with transparent. */
export function withAlpha(color: string, opacity: number): string {
  const a = Math.min(1, Math.max(0, opacity));
  if (a >= 1) return color;
  const hex = /^#([0-9a-f]{3}|[0-9a-f]{6})$/i.exec(color)?.[1];
  if (hex) {
    const full = hex.length === 3 ? [...hex].map((c) => c + c).join("") : hex;
    const [r, g, b] = [0, 2, 4].map((i) => parseInt(full.slice(i, i + 2), 16));
    return `rgb(${r} ${g} ${b} / ${num(a * 100)}%)`;
  }
  return `color-mix(in srgb, ${color} ${num(a * 100)}%, transparent)`;
}

/**
 * A text item's stroke, shadow and background box as it draws (D80): the item's own (canvas px),
 * else its look's (em of the font size, so in px at this size); `false` on the item = none, even
 * when the look has one. Shared by the preview and inspector.
 */
export function textDecor(item: Pick<Item, "style">, look: TextLook, size: number): { stroke?: Required<Pick<TextStroke, "width">> & TextStroke; shadow?: TextShadow; background?: TextBackground } {
  const s = item.style ?? {};
  const em = (v: number | undefined) => (v === undefined ? undefined : v * size);
  const stroke = s.stroke === false ? undefined : s.stroke ?? (look.stroke ? { width: em(look.stroke.width)!, ...(look.stroke.color ? { color: look.stroke.color } : {}) } : undefined);
  const lookShadow = look.shadow ? { ...look.shadow, x: em(look.shadow.x), y: em(look.shadow.y), blur: em(look.shadow.blur) } : undefined;
  const shadow = s.shadow === false ? undefined : s.shadow ?? lookShadow;
  const lb = look.background;
  const background = s.background === false ? undefined : s.background ?? (lb ? { ...lb, ...(lb.padding !== undefined ? { padding: em(lb.padding) } : {}), ...(lb.radius !== undefined ? { radius: em(lb.radius) } : {}) } : undefined);
  const clean = <T extends object>(o: T | undefined) => (o ? (Object.fromEntries(Object.entries(o).filter(([, v]) => v !== undefined)) as T) : undefined);
  return {
    ...(stroke && stroke.width > 0 ? { stroke: clean(stroke)! } : {}),
    ...(shadow ? { shadow: clean(shadow)! } : {}),
    ...(background ? { background: clean(background)! } : {}),
  };
}

/** A background box's padding: the number is the sides', top and bottom get half (labels and caption lines read better wider). */
const padCss = (p: number) => `${num(p / 2)}px ${num(p)}px`;

/**
 * Wraps every word of markup HTML (markup()'s output) in a span with style(i), i counting the
 * words of the visible text (runs of non-space between whitespace; a word cut by a tag keeps one
 * index). Whitespace and line breaks stay text, so the lines wrap and break as without the spans.
 * style(i) null leaves word i out, with the whitespace that would separate it (the words left out
 * are always a run at one end, D81), so what's left reads as if it were the whole text.
 */
function wrapWords(html: string, style: (i: number) => string | null, leavesOut = false): { html: string; count: number } {
  let i = -1;
  let inWord = false;
  let out = "";
  let pending = "";
  let shown = false;
  for (const [, tag, text] of html.matchAll(/(<[^>]+>)|([^<]+)/g)) {
    if (tag) {
      out += tag;
      continue;
    }
    for (const piece of text!.split(/(\s+)/)) {
      if (!piece) continue;
      if (/^\s+$/.test(piece)) {
        inWord = false;
        // Only held back when words may be left out (otherwise exactly where markup put it).
        if (leavesOut) pending += piece;
        else out += piece;
        continue;
      }
      if (!inWord) i++;
      const css = style(i);
      if (css !== null) {
        if (shown) out += pending;
        shown = true;
        out += `<span class="pv-w"${css ? ` style="${css}"` : ""}>${piece}</span>`;
      }
      pending = "";
      inWord = true;
    }
  }
  return { html: out, count: i + 1 };
}

/** One visible character of markup HTML: an entity (esc's &amp; and the like) or a code point. */
const CHAR = /&(?:#\d+|#x[0-9a-f]+|[a-z]+);|[\s\S]/giu;

/** How many characters a text's markup shows (what a typewriter move counts, D163): tags don't count, an entity is one. */
export const charCount = (text: string) => {
  let n = 0;
  for (const [, tag, chunk] of markup(text).matchAll(/(<[^>]+>)|([^<]+)/g)) if (!tag) n += chunk!.match(CHAR)?.length ?? 0;
  return n;
};

/**
 * Markup HTML with only its first `shown` visible characters (D163): the rest of each text run is
 * wrapped in an invisible span (tags pass through, so the nesting holds and the lines keep their
 * layout), or left out with `leaveOut` (a box then grows with the characters).
 */
export function cutChars(html: string, shown: number, leaveOut = false): string {
  let left = Math.max(0, Math.floor(shown));
  let out = "";
  for (const [, tag, chunk] of html.matchAll(/(<[^>]+>)|([^<]+)/g)) {
    if (tag) {
      out += tag;
      continue;
    }
    const chars = chunk!.match(CHAR) ?? [];
    const keep = chars.slice(0, left).join("");
    const rest = chars.slice(left).join("");
    left = Math.max(0, left - chars.length);
    out += keep;
    if (rest && !leaveOut) out += `<span style="opacity:0">${rest}</span>`;
  }
  return out;
}

/** How many words a text's markup shows (what a word-by-word move counts, D81). */
export const wordCount = (text: string) => wrapWords(markup(text), () => "").count;

const backOut = (p: number) => {
  const c1 = 1.70158;
  return 1 + (c1 + 1) * (p - 1) ** 3 + c1 * (p - 1) ** 2;
};

/** One word's look at progress p (0 → 1) of a word-by-word move. */
export function wordCss(p: number, style: WordStyle = "pop"): string {
  if (p >= 1) return "";
  const q = Math.max(0, p);
  if (style === "fade") return `opacity:${num(q)}`;
  if (style === "rise") return `opacity:${num(q)};transform:translateY(${num((1 - (1 - (1 - q) ** 3)) * 0.4)}em)`;
  return `opacity:${num(Math.min(1, q * 2))};transform:scale(${num(0.6 + 0.4 * backOut(q))})`;
}

/**
 * Where a text item is drawn (canvas px), the one rule textHtml and the Inspector/drag share
 * (D80): transform x / y, else the look's (a spanning look: the safe margin); transform.w, else —
 * only when the item has no x of its own — the look's width (span: the safe area between the
 * margins); no w = as wide as its text. size = style.size, else the look's.
 */
export function textPlacement(item: Pick<Item, "transform" | "style">, target: Pick<RenderTarget, "canvas" | "theme">): { x: number; y: number; w: number | undefined; size: number } {
  const { width, height } = target.canvas;
  const look = textLook(item.style?.preset);
  const margin = safeMargin(target as RenderTarget);
  const x = item.transform?.x ?? (look.span ? margin : width * look.x);
  const y = item.transform?.y ?? height * look.y;
  const w = item.transform?.w ?? (item.transform?.x === undefined ? (look.span ? width - 2 * margin : look.w !== undefined ? width * look.w : undefined) : undefined);
  return { x, y, w, size: item.style?.size ?? height * look.size };
}

/** A text item's own opacity (transform.opacity, D129), 0–1; its fades multiply it. */
export const textOpacity = (item: Pick<Item, "transform">) => {
  const o = item.transform?.opacity;
  return typeof o === "number" && Number.isFinite(o) ? Math.min(1, Math.max(0, o)) : 1;
};
/** A text item's rotation in degrees about its box's centre (transform.rotate, D129). */
export const textRotation = (item: Pick<Item, "transform">) => {
  const r = item.transform?.rotate;
  return typeof r === "number" && Number.isFinite(r) ? r % 360 : 0;
};

/**
 * A text item as HTML. transform (x, y) is the box's top-left corner and transform.w its width in
 * canvas pixels (no w: as wide as its longest line, wrapping at the canvas edge); style.align
 * aligns the lines inside the box. A text item without transform or size (the format allows it,
 * and the agent can write one) falls back to its preset's look instead of breaking the preview.
 * Stroke, shadow and background box (D80) come from the item, else its look; the word-by-word and
 * zoom moves (D81) come in `motion`.
 */
export function textHtml(item: Item, target: RenderTarget, { opacity = 1, selected = false, motion }: { opacity?: number; selected?: boolean; motion?: Motion } = {}): string {
  const { width, height } = target.canvas;
  const look = textLook(item.style?.preset);
  const s = item.style ?? {};
  const { x, y, w, size } = textPlacement(item, target);
  const decor = textDecor(item, look, size);
  const extra = s as { readonly accent?: unknown; readonly accentBox?: unknown };
  const accentBox = typeof extra.accentBox === "string" && extra.accentBox ? resolveColor(target, extra.accentBox, DEFAULT_THEME.colors.accent) : undefined;
  const css = [
    `left:${num((x / width) * 100)}%`,
    item.anchor === "bottom" ? `bottom:${num((1 - y / height) * 100)}%` : `top:${num((y / height) * 100)}%`,
    w && w > 0 ? `width:${num((w / width) * 100)}%` : `width:max-content;max-width:${num(Math.max(5, 100 - (x / width) * 100))}%`,
    `font-size:${num((size / height) * 100)}cqh`,
    `font-weight:${s.weight ?? look.weight}`,
    `font-family:${resolveFont(target, s.font ?? look.font).replace(/"/g, "'")}`,
    `letter-spacing:${num(s.tracking ?? look.tracking)}em`,
    `line-height:${num(s.lineHeight ?? look.lineHeight)}`,
    `color:${resolveColor(target, s.color ?? look.color, DEFAULT_THEME.colors.fg)}`,
    `text-align:${s.align ?? look.align ?? "left"}`,
    // Captions (D166) may colour the speaking word (style.accent: the highlight, the karaoke fill) and box it (style.accentBox).
    `--accent:${resolveColor(target, typeof extra.accent === "string" ? extra.accent : "accent", DEFAULT_THEME.colors.accent)}`,
    `--mono:${resolveFont(target, "mono").replace(/"/g, "'")}`,
    `opacity:${num(opacity * textOpacity(item) * (motion?.fade ?? 1))}`,
  ];
  if (accentBox) css.push(`--accent-bg:${accentBox}`);
  if (s.italic) css.push("font-style:italic");
  if (decor.shadow) {
    const sh = decor.shadow;
    css.push(`text-shadow:${num(sh.x ?? 0)}px ${num(sh.y ?? 0)}px ${num(Math.max(0, sh.blur ?? 0))}px ${withAlpha(resolveColor(target, sh.color, "#000000"), sh.opacity ?? 1)}`);
  }
  if (decor.stroke) css.push(`-webkit-text-stroke:${num(decor.stroke.width)}px ${resolveColor(target, decor.stroke.color, "#000000")};paint-order:stroke fill`);
  const textCase = s.case === "lower" ? "lowercase" : s.case === "title" ? "capitalize" : (s.uppercase ?? look.uppercase) ? "uppercase" : undefined;
  if (textCase) css.push(`text-transform:${textCase}`);
  const bg = decor.background;
  const bgFill = bg ? withAlpha(resolveColor(target, bg.color, "#000000"), bg.opacity ?? 1) : undefined;
  if (bg && bg.wrap !== "lines") css.push(`background:${bgFill};padding:${padCss(Math.max(0, bg.padding ?? 0))};border-radius:${num(Math.max(0, bg.radius ?? 0))}px`);
  const moved = !!(motion?.dx || motion?.dy);
  const scaled = motion?.scale !== undefined && motion.scale !== 1;
  const turn = textRotation(item) + (motion?.rotate ?? 0);
  const parts = [moved ? `translate(${num(((motion!.dx ?? 0) / width) * 100)}cqw,${num(((motion!.dy ?? 0) / height) * 100)}cqh)` : "", turn ? `rotate(${num(turn)}deg)` : "", scaled ? `scale(${num(motion!.scale!)})` : ""].filter(Boolean);
  if (parts.length) css.push(`transform:${parts.join(" ")}`);
  if (motion?.reveal !== undefined && motion.reveal < 1) css.push(`clip-path:inset(-20% ${num((1 - motion.reveal) * 100)}% -20% -5%)`);
  if (motion?.blur) css.push(`filter:blur(${num(motion.blur)}cqh)`);
  let body = markup(item.text ?? "");
  if (motion?.words) {
    const words = motion.words;
    // With a box (one box or one per line), words that haven't arrived (or have left) are left
    // out, so the box grows with the words instead of showing empty first (D81).
    const hide = !!bg && words.some((w) => w <= 0);
    body = wrapWords(body, (i) => (hide && (words[i] ?? 1) <= 0 ? null : wordCss(words[i] ?? 1, motion.wordStyle)), hide).html;
  }
  // Typewriter (D163): as with word by word, a box grows with the characters (the rest left out); without one they keep their place.
  if (motion?.chars !== undefined) body = cutChars(body, motion.chars, !!bg);
  if (bg && bg.wrap === "lines") body = `<span class="pv-line" style="background:${bgFill};padding:${padCss(Math.max(0, bg.padding ?? 0))};border-radius:${num(Math.max(0, bg.radius ?? 0))}px">${body}</span>`;
  return `<div class="pv-text${accentBox ? " acc-box" : ""}${selected ? " sel-box" : ""}" data-item="${esc(item.id)}" style="${css.join(";")}">${body}</div>`;
}

/** The built-in full-frame background (component.solid): a colour, or a gradient to `to`. */
export const SOLID = "component.solid";

/** Film grain at `amount` (D131): a 240 px tile of SVG fractal noise (deterministic, no network) as a CSS image. */
const grain = (amount: number) =>
  `url('data:image/svg+xml,${encodeURIComponent(
    `<svg xmlns='http://www.w3.org/2000/svg' width='240' height='240'><filter id='n'><feTurbulence type='fractalNoise' baseFrequency='0.85' numOctaves='3' stitchTiles='stitch'/><feColorMatrix type='saturate' values='0'/></filter><rect width='240' height='240' filter='url(#n)' opacity='${num(amount)}'/></svg>`,
  ).replace(/'/g, "%27")}') 0 0 / 240px 240px`;

/**
 * A component.solid's CSS background (D131): props.color; with props.to a gradient from color
 * (through props.via) to `to`, linear at props.angle (180 = top to bottom) or radial
 * (props.gradient "radial", from the centre out); props.noise 0–1 lays film grain over it.
 * Backgrounds saved before D131 ({color}, {color, to, angle}) draw as they did.
 */
export function solidBackground(item: Item, target: Pick<RenderTarget, "theme">): string {
  const props = item.props ?? {};
  const from = resolveColor(target, props.color, DEFAULT_THEME.colors.bg);
  let fill = from;
  if (props.to !== undefined) {
    const to = resolveColor(target, props.to, from);
    const stops = props.via !== undefined ? [from, resolveColor(target, props.via, from), to] : [from, to];
    if (props.gradient === "radial") fill = `radial-gradient(circle at 50% 50%, ${stops.join(", ")})`;
    else {
      const angle = typeof props.angle === "number" && Number.isFinite(props.angle) ? props.angle : 180;
      fill = `linear-gradient(${num(angle)}deg, ${stops.join(", ")})`;
    }
  }
  const noise = typeof props.noise === "number" && Number.isFinite(props.noise) ? Math.min(1, Math.max(0, props.noise)) : 0;
  if (noise <= 0) return fill;
  return `${grain(noise)}, ${fill}`;
}

/** A component.solid as HTML (export draws it like this; the preview sets the same background). */
export function solidHtml(item: Item, target: RenderTarget, { opacity = 1 }: { opacity?: number } = {}): string {
  return `<div class="pv-solid" data-item="${esc(item.id)}" style="background:${solidBackground(item, target)};opacity:${num(opacity)}"></div>`;
}
