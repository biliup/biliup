// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Manual timeline editing (phase 4 chunk 3, D52): pure planners that turn a gesture into edit ops.
// The timeline previews a drag by applying the planned ops locally with the editor core's engine (no
// server round trip per mouse move) and sends one project.edit when the mouse is released, so
// every action is validated by the server, undoes in one step and reaches the agent's note (D6).
import { nearestFreeSpot } from "@/app/lib/editor/core/timeline/arrange";

export { SNAP_PX, findItem, homeKind, fitsTrack, lockOf, secondsLabel, draftOf, withGroups } from "@/app/ui/editor/timeline/edit/base";
export type { Placed, Plan } from "@/app/ui/editor/timeline/edit/base";
export { MIN_BEAT_PX, beatsInItem, visibleBeats, beatTargets, snapPlayhead, snapTargets, trimSnapTargets, bladeFrame } from "@/app/ui/editor/timeline/edit/snap";
export type { BeatMap, BeatSnap, Snap } from "@/app/ui/editor/timeline/edit/snap";
export { planMove, planMoveToNewTrack, newTrackIndexOver, planGroupMove, planLayer } from "@/app/ui/editor/timeline/edit/move";
export type { MovePlan, NewTrackSide, GroupMovePlan } from "@/app/ui/editor/timeline/edit/move";
export { planRippleDelete, planTrim, planSplit, planBladeSplit, planGroupTrim } from "@/app/ui/editor/timeline/edit/trim";
export type { TrimPlan, SplitPlan } from "@/app/ui/editor/timeline/edit/trim";
export { mainCuts, nearestCut, planSetTransition, trackFlag, planTrackFlag, cutFit } from "@/app/ui/editor/timeline/edit/transitions";
export type { TrackFlag, CutFit } from "@/app/ui/editor/timeline/edit/transitions";
export { withOverlayRipple, planGrouping } from "@/app/ui/editor/timeline/edit/ripple";
export { planSpeed } from "@/app/ui/editor/timeline/edit/speed";
export { planRename, readName } from "@/app/ui/editor/timeline/edit/names";
export type { NameTarget } from "@/app/ui/editor/timeline/edit/names";

export { nearestFreeSpot };
