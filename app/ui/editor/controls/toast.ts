// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// How long an error toast stays (#67, D239): long enough to read. About 5 s for a short line, a
// little more per character (roughly a slow reading pace), never more than 15 s.
export const TOAST_MIN_MS = 5000;
export const TOAST_MAX_MS = 15000;
export const toastMs = (text: string): number => Math.round(Math.min(TOAST_MAX_MS, Math.max(TOAST_MIN_MS, 3500 + 55 * text.trim().length)));
