// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Replace a fragment from the editor (issue #118): the In to Out range on the timeline plays another
// media from the library, nothing after it moves (core planReplaceRange). The clip it replaces in:
// the selected clip or sound when the range is inside it, else the main-track clip it's inside.
import { planReplaceRange } from "@/app/lib/editor/core/timeline/arrange";

import { layoutOf, type ProjectModel } from "@/app/lib/editor/state/project";

export function planReplaceMarks(project: ProjectModel, media: string, at: { readonly markIn: number | undefined; readonly markOut: number | undefined; readonly selected: string | undefined }, from = 0) {
  if (at.markIn === undefined || at.markOut === undefined || at.markOut <= at.markIn) return { problem: "Set In and Out on the timeline first (I and O): the piece to replace." };
  const { starts } = layoutOf(project);
  const inside = (id: string, duration: number) => {
    const start = starts.get(id) ?? 0;
    return start <= at.markIn! && at.markOut! <= start + duration;
  };
  const items = project.tracks.flatMap((t) => t.items.map((item) => ({ track: t, item })));
  const selected = items.find(({ item }) => item.id === at.selected && (item.type === "clip" || item.type === "audio") && inside(item.id, item.duration));
  const main = items.find(({ track, item }) => track.role === "main" && item.type === "clip" && inside(item.id, item.duration));
  const target = selected ?? main;
  if (!target) return { problem: "In to Out isn't inside one clip: select the clip or sound to replace a piece of, with In and Out inside it." };
  return planReplaceRange(project, { item: target.item.id, from: at.markIn, to: at.markOut, media, in: from });
}
