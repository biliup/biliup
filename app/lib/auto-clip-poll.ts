export const ACTIVE_AUTO_CLIP_POLL_MS = 3_000
export const DISCOVER_AUTO_CLIP_POLL_MS = 60_000

/** Keep looking for tasks that the backend may enqueue after this row first loaded. */
export function sessionJobRefreshInterval(job: { state?: string } | null | undefined): number {
  return job?.state === 'queued' || job?.state === 'running'
    ? ACTIVE_AUTO_CLIP_POLL_MS
    : DISCOVER_AUTO_CLIP_POLL_MS
}
