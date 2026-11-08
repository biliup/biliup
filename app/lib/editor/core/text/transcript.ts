// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Transcripts in the editing model (D68, D69), browser-safe: the fillers and pauses derived from a
// transcript's words (the server's transcript_read and the Transcript panel derive the same ones),
// and planTextCut, which turns "remove these words / pauses / fillers" into one timeline edit. The
// transcript is in seconds of the media file; the timeline in project frames (D38).
export { PAUSE_MIN_S, FILLERS, fillersOf, pausesOf, HEAD_S, TAIL_S, handleFrames, tailFrames, DEFAULT_PAUSE_MIN_S } from "@/app/lib/editor/core/text/transcript/fillers";
export type { Filler, Pause } from "@/app/lib/editor/core/text/transcript/fillers";
export { ENVELOPE_RATE, encodeEnvelope, decodeEnvelope, speechThreshold, envelopeInformative, speechEdges, speechWords } from "@/app/lib/editor/core/text/transcript/speech";
export type { SpeechSegment, StoredEnvelope, SpeechEnvelope, HeardWords } from "@/app/lib/editor/core/text/transcript/speech";
export type { CutWord } from "@/app/lib/editor/core/text/transcript/cutSpans";
export { planTextCut } from "@/app/lib/editor/core/text/transcript/textCut";
export type { CutTranscript, TextCutRequest, TextCut, TextCutPlan } from "@/app/lib/editor/core/text/transcript/textCut";
export { soundParts, speechItem, transcriptScope, editWords } from "@/app/lib/editor/core/text/transcript/editWords";
export type { EditWord, TranscriptScope } from "@/app/lib/editor/core/text/transcript/editWords";
