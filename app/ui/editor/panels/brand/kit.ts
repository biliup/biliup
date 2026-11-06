import { editorText } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import type { BrandScope, BrandKit } from "@/app/lib/editor/contracts";

export const COLOR_ROLES = ["background", "surface", "text", "primary", "accent", "muted"] as const;
export const FONT_ROLES = ["headline", "body", "mono"] as const;
export const LOGO_SLOTS = [
  { id: "light", get label() { return editorText("Light"); } },
  { id: "dark", get label() { return editorText("Dark"); } },
  { id: "mark", get label() { return editorText("Mark"); } },
] as const;
export type LogoSlot = (typeof LOGO_SLOTS)[number]["id"];
export const SCOPE_LABEL: Record<BrandScope, string> = { global: "Library", project: "Pinned here" };
export const SCOPE_CHOICE: Record<BrandScope, string> = { global: "Library", project: "Pin to this project" };
export const LOGO_BACKDROP: Record<LogoSlot, string> = { light: "#ffffff", dark: "#111111", mark: "#808080" };
export const HEX6 = /^#[0-9a-fA-F]{6}$/;

/** An id like the server's slug, not taken by `taken`. */
export function freeId(name: string, taken: ReadonlyArray<string>, fallback: string): string {
  const base =
    name
      .toLowerCase()
      .normalize("NFKD")
      .replace(/[̀-ͯ]/g, "")
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "")
      .slice(0, 48) || fallback;
  let id = base;
  for (let n = 2; taken.includes(id); n++) id = `${base}-${n}`;
  return id;
}

export const lines = (text: string) =>
  text
    .split("\n")
    .map((l) => l.trim())
    .filter(Boolean);

export interface KitDraft {
  readonly name: string;
  readonly scope: BrandScope;
  readonly logos: Partial<Record<LogoSlot, string>>;
  readonly colors: ReadonlyArray<{ readonly hex: string; readonly role: string; readonly name?: string | undefined }>;
  readonly fonts: ReadonlyArray<{ readonly family: string; readonly role: string; readonly weight?: number | undefined; readonly file?: string | undefined }>;
  readonly about: string;
  readonly tone: string;
  readonly taglines: string;
  readonly urls: string;
  readonly dos: string;
  readonly donts: string;
  readonly examples: ReadonlyArray<string>;
}
export const draftOfKit = (k: BrandKit): KitDraft => ({
  name: k.name,
  scope: k.scope,
  logos: { ...k.logos },
  colors: k.colors.map((c) => ({ ...c })),
  fonts: k.fonts.map((f) => ({ ...f })),
  about: k.about ?? "",
  tone: k.tone,
  taglines: k.taglines.join("\n"),
  urls: k.urls.join("\n"),
  dos: k.dos.join("\n"),
  donts: k.donts.join("\n"),
  examples: [...k.examples],
});
export const emptyKit = (id: string, name: string, scope: BrandScope): BrandKit => ({ id, name, scope, logos: {}, colors: [], fonts: [], tone: "", taglines: [], urls: [], dos: [], donts: [], examples: [] });

export type View =
  | { readonly kind: "list" }
  | { readonly kind: "kit"; readonly id: string; readonly scope: BrandScope; readonly draft: KitDraft }
  | { readonly kind: "builtin"; readonly id: string }
  | { readonly kind: "skill"; readonly id: string | undefined; readonly scope: BrandScope; readonly draft: { readonly name: string; readonly scope: BrandScope; readonly description: string; readonly body: string } };
