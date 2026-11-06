// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// A re-shot recording swapped in (D98): the media entry gets the new file in place (update_media),
// so every clip that uses it plays the new take. A take shorter than the old one would leave clips
// running past its end, which the timeline rules refuse (media-range): those are trimmed to the new
// end, or held on its last frame (D94), in the same edit, each with a warning the caller passes on.
// Transcript captions belong to that source recording; a different file removes them in the
// same undoable edit rather than rendering the old spoken words over the replacement.
import type { Id, Item, Media, MediaProbe, Op, Project } from "@/app/lib/editor/core/timeline/types";
import { APPEARANCE_KEYS, pieceAppearance } from './arrange/retime';
import { finishReplacement } from './arrange/replace';

export type Overrun = "trim" | "hold";

export interface ReshootPlan {
  readonly ops: ReadonlyArray<Op>;
  readonly warnings: ReadonlyArray<string>;
}

const secs = (frames: number, fps: number) => `${(frames / fps).toFixed(1)} s`;

/**
 * The ops that swap `next` in for media `mediaId`: update_media, then for every item that would run
 * past the new end, a trim (overrun "trim") or a hold (clips; overrun "hold", or a clip that starts
 * past the new end, which can't be trimmed to nothing). An audio item left with no sound, or with
 * nothing to play, is deleted.
 */
export function planReshoot(
  project: Project,
  mediaId: Id,
  next: { readonly path: string; readonly probe: MediaProbe; readonly name?: string | undefined; readonly source?: Media["source"] | undefined },
  overrun: Overrun = "trim",
): ReshootPlan {
  const media = project.media.find((m) => m.id === mediaId);
  if (!media) throw new Error(`There's no media ${mediaId} in the project.`);
  const fps = project.canvas.fps;
  const set: Record<string, unknown> = { path: next.path, probe: next.probe };
  if (next.name) set.name = next.name;
  if (next.source) set.source = next.source;
  const ops: Array<Op> = [{ op: "update_media", media: mediaId, set }];
  const warnings: Array<string> = [];
  const frames = next.probe.frames;
  for (const track of project.tracks) {
    for (const item of track.items as ReadonlyArray<Item>) {
      if(item.type==='captions'&&item.source===mediaId&&media.path!==next.path){
        ops.push({op:'delete_item',item:item.id});
        warnings.push(`Captions ${item.id} were removed because the replacement has different source audio. Transcribe the new file to make captions.`);
        continue;
      }
      if (item.type === 'danmaku' && item.media === mediaId && media.path !== next.path) {
        ops.push({ op: 'delete_item', item: item.id });
        warnings.push(`Danmaku ${item.id} was removed because it belongs to the original recording.`);
        continue;
      }
      if (item.media !== mediaId) continue;
      const label = `${item.type} ${item.id}`;
      if (item.type === "audio" && next.probe.hasAudio === false) {
        ops.push({ op: "delete_item", item: item.id });
        warnings.push(`The new take has no sound, so ${label} (its sound) was removed.`);
        continue;
      }
      if (!frames || media.kind === "image" || item.loop || (item.type === "clip" && item.hold)) continue;
      const from = item.in ?? 0;
      const speed = item.speed ?? 1;
      const used = from + item.duration * speed;
      if (used <= frames) continue;
      const fits = Math.floor((frames - from) / speed);
      if (item.type === "clip" && (overrun === "hold" || fits < 1)) {
        ops.push({ op: "update_item", item: item.id, set: { hold: true } });
        warnings.push(`The new take is ${secs(used - frames, fps)} shorter than ${label} needs, so it holds the take's last frame for the rest.`);
      } else if (fits >= 1) {
        ops.push({ op: "trim_item", item: item.id, edge: "end", frames: item.duration - fits });
        const appearance = pieceAppearance(item, 0, fits, fits, fps, true, true);
        const set = Object.fromEntries(APPEARANCE_KEYS.filter(key => item[key] !== undefined).map(key => [key, appearance[key] ?? null]));
        if (Object.keys(set).length) ops.push({ op: 'update_item', item: item.id, set });
        warnings.push(`The new take is shorter, so ${label} was trimmed by ${secs(item.duration - fits, fps)} to end with it.`);
      } else {
        ops.push({ op: "delete_item", item: item.id });
        warnings.push(`The new take ends before ${label} starts in it, so ${label} was removed.`);
      }
    }
  }
  const finished = finishReplacement(project, ops);
  if ('problem' in finished) throw new Error(finished.problem);
  return { ops: finished.ops, warnings };
}
