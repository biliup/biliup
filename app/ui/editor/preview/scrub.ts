// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// D221: the scrub frame cache (issue #12 item 3). While the preview is paused or shuttling (J/L at
// another speed), a clip's exact frames come from here instead of a <video> seek: WebCodecs decodes
// the frames around the playhead ahead of time (from the file the <video> plays, proxy or original,
// fetched with byte ranges from the same token-gated route), and the preview draws the cached one
// into a canvas over the hidden <video>. Playback at 1× stays on <video> (its clock and the mix).
//
// - The sample table comes from core's mp4index (MP4/MOV, H.264/HEVC, one edit). Anything else
//   (WebM, fragmented, a codec WebCodecs refuses here, HDR transfer) keeps the <video> path.
// - The frame shown at t is the one <video> shows: the last sample presented at or before t.
// - Frames are kept as ImageBitmaps (the decoder's VideoFrame closed at once, so the decoder never
//   runs out of buffers), at most SCRUB_MAX_SIDE on the long side, in one LRU of ≤ the budget
//   (300 MB by default) over every source, evicted oldest first and closed.
// - Keys are a source (the URL the <video> plays, which changes with a relinked file or a new proxy)
//   and a sample: edits (trim, speed, effects) never invalidate a frame, they only ask for another.
//   A file changed in place (another size or modification time on any range read) drops its frames.
export { SCRUB_MAX_SIDE, SCRUB_BUDGET, FrameLru, cachedSize, windowRanks } from "@/app/ui/editor/preview/scrub/frames";
export { decoderColour, convertsPrimaries } from "@/app/ui/editor/preview/scrub/colour";
export { sameFile } from "@/app/ui/editor/preview/scrub/source";
export { ScrubCache, mediaIdOf } from "@/app/ui/editor/preview/scrub/cache";
