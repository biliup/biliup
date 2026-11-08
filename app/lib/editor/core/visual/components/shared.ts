// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { type RenderTarget, num, resolveColor, DEFAULT_THEME, esc } from "@/app/lib/editor/core/visual/render";
import type { Item, ModuleManifest } from "@/app/lib/editor/core/timeline/types";

/** A component's placement block. */
export interface ComponentBlock {
  readonly size: { readonly width: number; readonly height: number };
  readonly fullFrame?: boolean;
  readonly animated?: boolean;
  readonly defaults?: { readonly [key: string]: unknown };
  readonly screens?: { readonly [device: string]: { readonly size: { readonly width: number; readonly height: number }; readonly screen: { readonly x: number; readonly y: number; readonly width: number; readonly height: number; readonly radius?: number } } };
}

/** What a renderer gets: the item's props, its box size (canvas px), the local frame and the target. */
export interface Draw {
  readonly item: Item;
  readonly props: { readonly [key: string]: unknown };
  readonly w: number;
  readonly h: number;
  readonly local: number;
  readonly fps: number;
  readonly target: RenderTarget;
}

export interface ComponentInfo {
  readonly id: string;
  readonly name: string;
  readonly description: string;
  readonly manifest: ModuleManifest & { readonly component: ComponentBlock };
  readonly block: ComponentBlock;
  /** JSON Schema of the props (the manifest's params). */
  readonly params: { readonly properties?: { readonly [key: string]: unknown }; readonly required?: ReadonlyArray<string> };
  readonly render: (d: Draw) => string;
}

export const manifestOf = (json: unknown) => json as ModuleManifest & { readonly component: ComponentBlock; readonly description: string; readonly params: ComponentInfo["params"] };

// ---- shared bits ----

export const str = (v: unknown) => (typeof v === "string" ? v : "");
export const px = (n: number) => `${num(n)}px`;
export const color = (target: RenderTarget, v: unknown, fallback: string) => resolveColor(target, v, fallback);
export const fg = (target: RenderTarget) => color(target, "fg", DEFAULT_THEME.colors.fg);
export const mute = (target: RenderTarget) => color(target, "mute", DEFAULT_THEME.colors.mute);
export const accent = (target: RenderTarget) => color(target, "accent", DEFAULT_THEME.colors.accent);
/** A media reference prop ({ media: id }) as an image URL in this target, or undefined. */
export const mediaSrc = (target: RenderTarget, v: unknown) => {
  const id = v && typeof v === "object" ? (v as { media?: unknown }).media : undefined;
  return typeof id === "string" ? target.mediaUrl?.(id) : undefined;
};
/** Image files a component can show ({ media } props): what Chromium decodes (no TIFF, no HEIC). */
export const COMPONENT_IMAGE = /\.(svg|png|jpe?g|gif|webp|bmp)$/i;
/** URLs go in a src="…" attribute: only data:, http(s): and the export's placeholder pass. */
export const safeSrc = (url: string | undefined) => (url && /^(data:image\/|https?:\/\/|biliup-media:)/.test(url) ? esc(url) : undefined);

/** "42", "1.5", "0.25": up to two decimals. */
export const numberLabel = (n: number) => String(Math.round(n * 100) / 100);
