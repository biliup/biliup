// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The composer's / menu (D232, issue #111): a "Templates" group first (a picked one is filled from
// the project and lands in the composer as a brief), then the available editing skills.
// One list, so the arrows and Enter walk across both groups.

export interface SlashSkill {
  readonly kind: "skill";
  readonly id: string;
  readonly name: string;
  readonly description: string;
  readonly builtin: boolean;
}
export interface SlashTemplate {
  readonly kind: "template";
  readonly id: string;
  readonly name: string;
  readonly description: string;
  readonly builtin: boolean;
}
export type SlashItem = SlashSkill | SlashTemplate;

/** Templates at most, then skills at most, in the menu. */
export const SLASH_SKILLS_MAX = 12;
export const SLASH_TEMPLATES_MAX = 8;

const words = (s: string) => s.normalize('NFKC').toLowerCase().replace(/[^\p{L}\p{N}]+/gu, "-");

/**
 * The menu for what's typed after /: templates whose id or name starts with it first, then the
 * ones whose id or name has it; then the skills the same way (a dashed query matches spaced words:
 * "beat-cut" finds "Beat-cut reel", "one-shape" finds "One shape, never cut").
 */
export function slashItems(q: string, skills: ReadonlyArray<{ readonly id: string; readonly name: string; readonly description: string; readonly builtin?: boolean | undefined }>, templates: ReadonlyArray<{ readonly id: string; readonly name: string; readonly description: string; readonly builtin?: boolean | undefined }>, displayName: (entry: { readonly name: string; readonly builtin?: boolean | undefined }) => string = entry => entry.name): Array<SlashItem> {
  const query = words(q);
  const rank = <T extends { readonly id: string; readonly name: string; readonly builtin?: boolean | undefined }>(all: ReadonlyArray<T>) => {
    const labels = (entry: T) => [entry.id, words(entry.name), words(displayName(entry))];
    const starts = (entry: T) => labels(entry).some(label => label.startsWith(query));
    return [
      ...all.filter(starts),
      ...all.filter(entry => !starts(entry) && labels(entry).some(label => label.includes(query))),
    ];
  };
  return [
    ...rank(templates).slice(0, SLASH_TEMPLATES_MAX).map((x): SlashTemplate => ({ kind: "template", id: x.id, name: x.name, description: x.description, builtin: !!x.builtin })),
    ...rank(skills).slice(0, SLASH_SKILLS_MAX).map((x): SlashSkill => ({ kind: "skill", id: x.id, name: x.name, description: x.description, builtin: !!x.builtin })),
  ];
}
