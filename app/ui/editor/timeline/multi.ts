// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Several selected items edited at once (Fable QA round 2, F2, D147): one planner run per item,
// each against the project as the earlier ones left it, joined into one edit (one revision, one
// Undo). Callers choose applicable items first. A failure on any target refuses the entire edit.
import type { Op } from "@/app/lib/editor/core/timeline/types";

import type { ProjectModel } from "@/app/lib/editor/state/project";
import { draftOf, type Plan } from "@/app/ui/editor/timeline/edit";

type Problem = { readonly problem: string };

export interface ManyPlan extends Plan {
  /** The items it changes. */
  readonly ids: ReadonlyArray<string>;
}

/**
 * `plan` for each of `ids`, chained: item n is planned against the project with items 0…n−1's ops
 * applied, so planners that read neighbours (a main clip's speed moves the clips after it) see the
 * real timeline. `summary(n, one)` labels the edit (`one`: the single item's own label).
 */
export function planForAll(project: ProjectModel, ids: ReadonlyArray<string>, plan: (p: ProjectModel, id: string) => Plan | Problem, summary: (n: number, one: string) => string): ManyPlan | Problem {
  const unique = [...new Set(ids)];
  let current = project;
  const ops: Array<Op> = [];
  const done: Array<string> = [];
  let one = "";
  for (const id of unique) {
    const p = plan(current, id);
    if ("problem" in p) return p;
    if (!p.ops.length) continue;
    const draft = draftOf(current, p.ops);
    if ("problem" in draft) return draft;
    current = draft.project;
    ops.push(...p.ops);
    done.push(id);
    one = p.summary;
  }
  if (!done.length) return { ops: [], summary: "", ids: [] };
  return { ops, summary: done.length === 1 ? one : summary(done.length, one), ids: done };
}
