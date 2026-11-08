// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import type { Op } from "@/app/lib/editor/core/timeline/types";
import { componentHtml } from "@/app/lib/editor/core/visual/components";
import { textHtml } from "@/app/lib/editor/core/visual/render";
import { getPlayback } from "@/app/ui/editor/preview/playback";
import { type ProjectModel, planAddText, type Item } from "@/app/lib/editor/state/project";
import { TEXT_PRESETS } from "@/app/lib/editor/state/project";
import { planAddComponent } from "@/app/ui/editor/preview/components";

/**
 * What a library tile dragged over the canvas would add at `at` (X2, D132): the element its drop
 * makes (the same planner), drawn by core (textHtml / componentHtml) as the export would, for the
 * faint ghost under the pointer. Undefined when the plan is refused or there's nothing to draw.
 */
export function dropGhostHtml(project: ProjectModel, what: { text?: string | undefined; component?: string | undefined; props?: Record<string,unknown> }, at: { x: number; y: number }): string | undefined {
  const frame = getPlayback().frame;
  if (what.text !== undefined) {
    const preset = TEXT_PRESETS.find((p) => p.id === what.text);
    if (!preset) return undefined;
    const plan = planAddText(project, preset, frame, at);
    if ("problem" in plan) return undefined;
    return (plan.ops as ReadonlyArray<Op>).flatMap((op) => (op.op === "insert_item" ? [textHtml(op.item as Item, project).replace(/ data-item="[^"]*"/, "")] : [])).join("");
  }
  if (!what.component) return undefined;
  const props = what.props;
  const plan = planAddComponent(project, what.component, frame, { at, ...(props ? { props } : {}) });
  if ("problem" in plan) return undefined;
  const item = (plan.ops as ReadonlyArray<Op>).flatMap((op) => (op.op === "insert_item" && (op.item as Item).type === "component" ? [op.item as Item] : []))[0];
  return item ? componentHtml(item, project, 0, { fps: project.canvas.fps })?.replace(/ data-item="[^"]*"/, "") : undefined;
}
