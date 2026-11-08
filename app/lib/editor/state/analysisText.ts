// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// How an analysis job reads in the editor (the Library tile, the Transcript panel, the inspector's
// Beats group): its own module, so those don't import each other for one line.
import type { AnalysisJob } from "@/app/lib/editor/contracts";
import {editorText} from '@/app/lib/editor/i18n';

export function analysisText(job: AnalysisJob) {
  if(job.analyzer==='restore')return editorText(job.phase==='running'?'Restoring analyses…':'Waiting to restore analyses');
  const pct = `${Math.round(job.progress * 100)}%`;
  if (job.phase === "running" && job.analyzer === "transcript") return `${editorText(job.stage === "model" ? "Downloading the speech model…" : "Transcribing…")} ${pct}`;
  if (job.phase === "running" && job.analyzer === "beats") return `${editorText("Finding the beats…")} ${pct}`;
  if (job.phase === "running") return `${editorText(job.analyzer === "video" ? "Analysing the picture…" : "Analysing the sound…")} ${pct}`;
  return editorText(job.waiting === "export" ? "Waiting for the export to finish" : job.analyzer === "transcript" ? "Waiting to transcribe" : "Waiting to analyse");
}
