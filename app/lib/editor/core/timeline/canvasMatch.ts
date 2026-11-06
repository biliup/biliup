// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// D209 (issue #15): "Match first clip": a new project takes its first video's size and frame rate.
import { CANVAS_FPS, type Canvas, type CanvasFps, type Media, type Op, type Project } from "@/app/lib/editor/core/timeline/types";

/** The canvas rate nearest to a clip's (29.97002997 → 29.97, 23.98 → 23.976); undefined when none is within 0.05. */
export function nearestCanvasFps(fps: number | undefined): CanvasFps | undefined {
  if (fps === undefined || !Number.isFinite(fps) || fps <= 0) return undefined;
  let best: CanvasFps | undefined;
  for (const r of CANVAS_FPS) if (Math.abs(r - fps) <= 0.05 && (best === undefined || Math.abs(r - fps) < Math.abs(best - fps))) best = r;
  return best;
}

/** Even, 16…8192 (the schema's range; H.264 wants even sizes). */
const side = (v: number) => Math.min(8192, Math.max(16, Math.round(v / 2) * 2));

/**
 * The canvas a clip asks for: its picture as shown (a phone clip turned 90° is portrait), even
 * sides, and its rate when it's one of the canvas rates (else the canvas keeps its own). The
 * background is kept. Undefined when the clip has no size.
 */
export function canvasForClip(canvas: Canvas, clip: { readonly width?: number | undefined; readonly height?: number | undefined; readonly fps?: number | undefined; readonly rotation?: number | undefined }): Canvas | undefined {
  if (!clip.width || !clip.height || !Number.isFinite(clip.width) || !Number.isFinite(clip.height) || clip.width < 0 || clip.height < 0) return undefined;
  const turned = clip.rotation === 90 || clip.rotation === 270;
  const width = side(turned ? clip.height : clip.width);
  const height = side(turned ? clip.width : clip.height);
  return { ...canvas, width, height, fps: nearestCanvasFps(clip.fps) ?? canvas.fps };
}

export function canMatchFirstClip(project: Project): boolean {
  return !project.media.some(media => media.kind === 'video')
    && !project.tracks.some(track => track.items.length)
    && !project.scenes?.length && !project.sections?.length;
}

/** Plan before placing imports. The canvas and media lengths belong to the same undo step. */
export function planImportCanvas(project: Project, incoming: ReadonlyArray<Media>, durations: Readonly<Record<string, number | null>> = {}): {project: Project; ops: Op[]} {
  if (!project.canvas.matchFirstClip) return {project, ops: []};
  const eligible = canMatchFirstClip(project);
  const first = incoming.find(media => media.kind === 'video' && media.probe?.width && media.probe?.height);
  if (eligible && !first) return {project, ops: []};
  // Probes already describe the displayed orientation; do not rotate phone footage twice.
  const matched = eligible && first?.probe ? canvasForClip(project.canvas, first.probe) : undefined;
  if (eligible && !matched) return {project, ops: []};
  const canvas: Canvas = {...(matched ?? project.canvas), matchFirstClip: false};
  const ops: Op[] = [{op: 'update_project', set: {canvas}}];
  const media = project.media.map(media => {
    if (canvas.fps === project.canvas.fps || media.kind === 'image' || media.probe?.frames === undefined) return media;
    const duration = durations[media.id];
    const frames = Math.max(1, Math.floor(typeof duration === 'number'
      ? duration * canvas.fps / 1000 : media.probe.frames * canvas.fps / project.canvas.fps));
    const next = {...media, probe: {...media.probe, frames}};
    ops.push({op: 'update_media', media: media.id, set: {probe: next.probe}});
    return next;
  });
  return {project: {...project, canvas, media}, ops};
}
