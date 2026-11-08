// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The Export dialog's range (G7, D127): which part a choice means right now. When the In/Out
// marks or the selection it was set to are gone (an undo, the agent), it falls back to the whole
// timeline and says so, never exporting something else silently.
export type RangeKind = "all" | "marks" | "selection" | "custom";
export interface FrameSpan {
  readonly from: number;
  readonly to: number;
}

export function effectiveRange(kind: RangeKind, available: { readonly marks?: FrameSpan | undefined; readonly selection?: FrameSpan | undefined; readonly custom: FrameSpan }): { readonly kind: RangeKind; readonly range?: FrameSpan; readonly fellBack?: "marks" | "selection" } {
  if (kind === "marks") return available.marks ? { kind, range: available.marks } : { kind: "all", fellBack: "marks" };
  if (kind === "selection") return available.selection ? { kind, range: available.selection } : { kind: "all", fellBack: "selection" };
  if (kind === "custom") return { kind, range: available.custom };
  return { kind: "all" };
}
