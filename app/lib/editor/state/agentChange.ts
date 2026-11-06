// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// D202, D235 (issue #64): the agent never moves the playhead or the selection. Its ui_show (D65,
// D185) shows a pill over the preview, "Agent changed 0:12–0:18", the user can click to jump there.

interface TimedItem {
  readonly id: string;
  readonly start: number;
  readonly duration: number;
}

/**
 * The stretch an agent ui_show points at, in frames: the selected item's span; else the items that
 * start at its frame (a scene, D185), to the latest end among them; else just the frame.
 */
export function changeRange(tracks: ReadonlyArray<{ readonly items: ReadonlyArray<TimedItem> }>, ui: { readonly frame?: number | undefined; readonly select?: string | undefined }): { from: number; to: number } | undefined {
  const items = tracks.flatMap((t) => t.items);
  const picked = ui.select !== undefined ? items.find((i) => i.id === ui.select) : undefined;
  if (picked) return { from: ui.frame ?? picked.start, to: Math.max(picked.start + picked.duration, ui.frame ?? 0) };
  if (ui.frame === undefined) return undefined;
  const starting = items.filter((i) => i.start === ui.frame);
  return { from: ui.frame, to: starting.length ? Math.max(...starting.map((i) => i.start + i.duration)) : ui.frame };
}

/** m:ss of a frame (whole seconds). */
const clockOf = (frame: number, fps: number) => {
  const s = Math.floor(Math.max(0, frame) / fps);
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
};
/** The pill's words: "Agent changed 0:12–0:18", or "Agent changed 0:12" for one moment. */
export function changeLabel(range: { from: number; to: number }, fps: number): string {
  const a = clockOf(range.from, fps);
  const b = clockOf(range.to, fps);
  return a === b ? `Agent changed ${a}` : `Agent changed ${a}–${b}`;
}
