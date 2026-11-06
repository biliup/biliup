// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import type { ProjectSnapshot, ImportJob, AnalysisJob } from "@/app/lib/editor/contracts";
import { playback } from "@/app/ui/editor/preview/playback";
import { layoutOf } from "@/app/lib/editor/state/project";
import { state, set, projectOf } from "@/app/lib/editor/state/store/core";
import { actions } from "@/app/lib/editor/state/store/actions";
import { retryActivePreviews } from '@/app/ui/editor/preview/resources';
import type { AppStateShape } from "@/app/lib/editor/state/store/types";

// The newest revision this window has shown, per project. A project's revisions only go up on the
// server (they're its own since D71, kept in history.db), so a call result at or below it is old
// news (e.g. it arrived after a reconnect's refetch). The subscription is always applied; the mark only rises.
const newestRevision = new Map<string, number>();

/** From the subscription: authoritative, always applied. */
export function applyFromServer(snapshot: ProjectSnapshot | null) {
  if (snapshot) newestRevision.set(snapshot.dir, Math.max(newestRevision.get(snapshot.dir) ?? -1, snapshot.revision));
  const switched = snapshot?.dir !== state.snapshot?.dir;
  set({
    snapshot,
    ...(switched ? { notice: undefined, importReport: undefined, exportOpen: false, panelRequest: undefined, agentChange: undefined, agentDraft: "", agentMentions: [], agentImages: [], agentQueued: undefined } : {}),
  });
  // Then the playhead follows the project: back to the start for another project, clamped when it
  // got shorter. After the snapshot, so nothing draws the old project at the reset playhead (the
  // preview of the old project ignores changes once the open folder isn't its own).
  if (switched) playback.reset();
  // D186: the brand state follows the project (its kits, skills and choice).
  if (switched) void actions.brandGet();
  if (snapshot) {
    const { frames, end } = layoutOf(projectOf(snapshot));
    playback.setProject(frames, projectOf(snapshot).canvas.fps, end);
  }
}

/** From a call result: applied only if it is newer than anything shown. */
export function applyFromCall(snapshot: ProjectSnapshot) {
  if (snapshot.revision <= (newestRevision.get(snapshot.dir) ?? -1)) return;
  applyFromServer(snapshot);
}

// Finished jobs whose result has been shown (or that finished before this window saw them).
const reported = new Set<string>();
let jobsSeen = false;

/** Jobs as the server reports them: progress, finished results and newly made posters. */
export function onJobs(jobs: ReadonlyArray<ImportJob>) {
  const before = new Map(state.jobs.map((j) => [j.id, j]));
  let report = state.importReport;
  let posters = false;
  for (const job of jobs) {
    const prev = before.get(job.id);
    if (prev?.postersDone !== job.postersDone && job.postersDone > 0) posters = true;
    const finished = job.phase === "done" || job.phase === "failed";
    // On (re)connect, jobs that finished earlier are history, not news.
    if (finished && !jobsSeen) reported.add(job.id);
    if (finished && !reported.has(job.id)) {
      reported.add(job.id);
      posters = true;
      // The agent's imports (D65) are reported in its turn, not as the Library's import report.
      if (job.projectDir === state.snapshot?.dir && !job.agent) {
        const failed = job.phase === "failed" ? job.result?.failed ?? [{ path: `${job.total} file${job.total === 1 ? "" : "s"}`, message: job.error ?? "The import stopped." }] : job.result?.failed ?? [];
        const skipped = job.result?.skipped ?? [];
        report = failed.length || skipped.length ? { ...report, failed: [...(report?.failed ?? []), ...failed], skipped: [...(report?.skipped ?? []), ...skipped] } : report;
      }
    }
  }
  jobsSeen = true;
  set({ jobs, importReport: report });
  if (posters) retryActivePreviews();
}

/** The open project's analysis jobs for a media item (also joined ones, same content) that are waiting or running. */
export const activeAnalyses = (s: AppStateShape, mediaId: string) =>
  s.analysisJobs.filter((j) => j.projectDir === s.snapshot?.dir && (j.mediaId === mediaId || j.mediaIds.includes(mediaId)) && j.version === (s.snapshot ? projectOf(s.snapshot).media.find(m => m.id === mediaId)?.path : undefined) && (j.phase === "queued" || j.phase === "running"));

// Analysis jobs this window saw waiting or running: if one of them fails, the Library says why
// (a cancel, by the user or a project closing, says nothing).
const analysesWatched = new Set<string>();
export function onAnalysisJobs(analysisJobs: ReadonlyArray<AnalysisJob>) {
  let analysisError = state.analysisError;
  for (const job of analysisJobs) {
    if (job.phase === "queued" || job.phase === "running") analysesWatched.add(job.id);
    else if (analysesWatched.delete(job.id) && job.phase === "failed" && job.projectDir === state.snapshot?.dir) {
      const name = state.snapshot ? (projectOf(state.snapshot).media.find((m) => m.id === job.mediaId)?.name ?? job.mediaId) : job.mediaId;
      analysisError = `Couldn't analyse ${name}: ${job.error ?? "the analysis stopped."}`;
    }
  }
  set({ analysisJobs, analysisError });
}

/** The open project's import jobs that are still running or waiting. */
export const activeJobs = (s: AppStateShape) => s.jobs.filter((j) => j.projectDir === s.snapshot?.dir && j.phase !== "done" && j.phase !== "failed");
