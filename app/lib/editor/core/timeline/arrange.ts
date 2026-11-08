// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Arranging media on the timeline (feedback round 1: D122 tracks, D123 detach audio, D124 audio
// tracks, D125 speed). Pure planners shared by the renderer (drops, "Add to timeline", the context
// menu), the server (a Finder drop imports and places in one edit) and the agent's convenience ops.
// Browser-safe.
//
// D122, what "main" means: the main track is the spine (in order, ripple, transitions); D224: its
// items may have gaps before them (a delete leaves one, a drag makes one; ripple delete closes up).
// Every other video track is an overlay with free starts, drawn over the tracks below it (track
// order is the stacking order); clips there never ripple and carry no transitions. D215: Main takes
// any picture (text too), a take goes on it with its parts linked (planPutOnMain), and transcripts
// and captions work on any item (transcript.ts, captions.ts).
export { IMAGE_SECONDS, takenIds, busySpans, isFree, nearestFreeSpot, mainSlot, itemTypeFor, placedLength, freeAudioTrack, planPlaceMedia, tryOps } from "@/app/lib/editor/core/timeline/arrange/place";
export type { PlaceMedia, PlacedItem, PlaceMediaPlan } from "@/app/lib/editor/core/timeline/arrange/place";
export { planDetachAudio, SPEED_PRESETS, SPEED_RANGE, speedLabel, planSetSpeed, withLinkedAudio } from "@/app/lib/editor/core/timeline/arrange/retime";
export type { DetachAudioPlan } from "@/app/lib/editor/core/timeline/arrange/retime";
export { planPutOnMain } from "@/app/lib/editor/core/timeline/arrange/putOnMain";
export { planReplaceRange, planReplaceMedia, canReplaceMedia } from "@/app/lib/editor/core/timeline/arrange/replace";
export type { ReplaceRange } from "@/app/lib/editor/core/timeline/arrange/replace";
export { gapOpsFor, keepMainTimes, mainStartsById, rippleMainGaps, shiftMainItems } from "@/app/lib/editor/core/timeline/arrange/gaps";
export type { PutOnMainPlan } from "@/app/lib/editor/core/timeline/arrange/putOnMain";
