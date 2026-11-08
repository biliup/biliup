// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// D290 review: which postMessage a graphic's media request is. Kept apart from the store so it's
// testable on its own. A graphic's iframe is sandboxed without allow-same-origin, so its messages
// come from the opaque origin "null"; one that navigated elsewhere (refused by the desktop anyway)
// would carry that page's origin and is ignored. And it must come from one of the compositor's own
// graphic iframes.

export interface MediaAsk {
  readonly biliup: "media";
  readonly seq?: unknown;
  readonly id: string;
  readonly t?: unknown;
  readonly w?: unknown;
  readonly h?: unknown;
  /** Asked while the preview plays: a proxy's picture will do (D212). */
  readonly p?: unknown;
}

/** The media request in a message event, when it's one from a graphic's iframe (`frames`: their windows), else undefined. */
export function graphicMediaAsk(e: { readonly origin: string; readonly data: unknown; readonly source: unknown }, frames: Iterable<unknown>): MediaAsk | undefined {
  const d = e.data as Partial<MediaAsk> | null;
  if (!d || typeof d !== "object" || d.biliup !== "media" || typeof d.id !== "string") return undefined;
  if (e.origin !== "null" || !e.source) return undefined;
  for (const f of frames) if (f === e.source) return d as MediaAsk;
  return undefined;
}
