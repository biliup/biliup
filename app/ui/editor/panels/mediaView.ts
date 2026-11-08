import { editorText } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// How the Media panel lists the project's media (F26, D168): a grid or a list, sorted by name, date
// added, length or kind. Shared by editor tabs in this browser.
import { browserPreference, useBrowserPreference } from '@/app/lib/editor/browser-preference';

export type MediaViewMode = "grid" | "list";
export type MediaSort = "name" | "date" | "duration" | "type";
export interface MediaView {
  readonly mode: MediaViewMode;
  readonly sort: MediaSort;
}

export const MEDIA_SORTS: ReadonlyArray<{ readonly id: MediaSort; readonly label: string }> = [
  { id: "date", get label() { return editorText("Date added"); } },
  { id: "name", get label() { return editorText("Name"); } },
  { id: "duration", get label() { return editorText("Length"); } },
  { id: "type", get label() { return editorText("Type"); } },
];

const DEFAULT: MediaView = { mode: "grid", sort: "date" };
const view = browserPreference('media.view', DEFAULT, (value): value is MediaView => {
  if (!value || typeof value !== 'object') return false;
  const candidate = value as Partial<MediaView>;
  return (candidate.mode === 'grid' || candidate.mode === 'list') && MEDIA_SORTS.some(sort => sort.id === candidate.sort);
});

export const useMediaView = () => useBrowserPreference(view);
export const setMediaMode = (mode: MediaViewMode) => view.set({ ...view.get(), mode });
export const setMediaSort = (sort: MediaSort) => view.set({ ...view.get(), sort });

const KIND_ORDER = { video: 0, image: 1, audio: 2 } as const;

/**
 * The media in the panel's order: date added is the project's order (imports append), newest
 * first; name A to Z (numbers in order); length longest first (pictures have none, last); type
 * video, image, sound, then by name.
 */
export function sortMedia<M extends { readonly name: string; readonly kind: "video" | "image" | "audio"; readonly probe?: { readonly frames?: number | undefined } | undefined }>(media: ReadonlyArray<M>, sort: MediaSort): Array<M> {
  const byName = (a: M, b: M) => a.name.localeCompare(b.name, undefined, { numeric: true, sensitivity: "base" });
  const list = [...media];
  switch (sort) {
    case "date":
      return list.reverse();
    case "name":
      return list.sort(byName);
    case "duration":
      return list.sort((a, b) => (b.kind === "image" ? -1 : b.probe?.frames ?? -1) - (a.kind === "image" ? -1 : a.probe?.frames ?? -1) || byName(a, b));
    case "type":
      return list.sort((a, b) => KIND_ORDER[a.kind] - KIND_ORDER[b.kind] || byName(a, b));
  }
}
