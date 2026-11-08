import type { Media } from '@/app/lib/editor/core/timeline/types';

export type CaptureKind = 'screen' | 'camera' | 'voiceover' | 'system';

/** Capture identity belongs to the media, independent of editable names and UI language. */
export function captureKind(media: Pick<Media, 'source'> | undefined): CaptureKind | undefined {
  if (media?.source?.module !== 'capture.browser') return undefined;
  const kind = media.source.kind;
  return kind === 'screen' || kind === 'camera' || kind === 'voiceover' || kind === 'system' ? kind : undefined;
}
