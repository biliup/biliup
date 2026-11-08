// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Legibility from the text census (packages/core census.ts): render_frames' note and review_submit's
// finding and facts. The keynote films' step lines stood 2 to 2.5 % tall (a piece typed under its
// default, a thread's step rows smaller than its message), the critic called them readable by eye,
// and nothing measured them; whole-editor moments were blocked one per round. The census lists every
// word under the floor in each frame, so the agent sees them all before the critic.
import { censusList, LETTER_SHARE, SMALL_TEXT, softList, type CensusRun, type SoftRun } from "../core/visual/census";

import type { CriticFinding } from "./critic";

/** A frame's census as render_frames and review_submit get it. */
export interface CensusFrame {
  readonly frame: number;
  readonly small?: ReadonlyArray<CensusRun>;
  readonly measured?: number;
  /** Canvases, images or videos (or a zoomed or fading graphic) on the frame whose words the census can't read. */
  readonly unseen?: number;
  /** Words big enough to read but blurred, faded or dimmed, and what does it (the stage's props, a sequence swap, the graphic's own style). */
  readonly soft?: ReadonlyArray<SoftRun>;
  /** An biliupUI stage's preview empty (black) on the frame: the share of the frame it covers (%). */
  readonly emptyPreview?: number;
}

/** Frames whose biliupUI stage shows an empty (black) preview, with the fix ("" when none). */
export function emptyPreviewNote(list: ReadonlyArray<CensusFrame>, fps?: number): string {
  const empty = list.filter((f) => f.emptyPreview);
  if (!empty.length) return "";
  const at = (f: CensusFrame) => `frame ${f.frame}${fps ? ` (${(f.frame / fps).toFixed(2)} s)` : ""}, ${f.emptyPreview} % of the frame`;
  return `The editor stage's preview is empty (black) on ${empty.slice(0, 6).map(at).join("; ")}${empty.length > 6 ? `; ${empty.length - 6} more frames` : ""}: nothing is under the playhead, or its clip's media couldn't be read (a wrong media id, an audio file). Put the cut's clips in timeline.clips with their media ids ({ media: id, in: s }: the stage shows the clip under the playhead itself) or pass preview: { frame: yourCanvas } from frame 0; or keep the empty preview off screen.`;
}

/** What the census can't see, said wherever it would otherwise sound complete. */
const UNSEEN = "text drawn on a canvas, in an image or video, or in a zoomed or fading graphic isn't measured: judge it by eye";

/** The smallest biliupUI type (px) whose letters reach SMALL_TEXT on a canvas this tall. */
export const minType = (height: number) => Math.ceil((SMALL_TEXT / LETTER_SHARE) * height);

/** The one-call fixes for small words in an biliupUI film, with this canvas's numbers. */
export const legibleFix = (height: number) =>
  `Set the line the moment is about as an biliupUI piece at its default size (an biliupUI.sequence key without type: words 4.8 % of the height, letters about 3.5 %; type under ${minType(height)} px on this ${height} px canvas leaves letters under ${SMALL_TEXT * 100} %, and a piece's type sizes its first line, so a thread's 12 px step rows come out at 12/13 of its message: give the steps their own "turn.steps" key), or push the camera in until its words read: key = biliupUI.frameText(el, world, { width, height }) (fits false: too wide, set it as a piece). Labels and chrome around the moment may stay small.`;

const frames = (list: ReadonlyArray<CensusFrame>) => list.filter((f) => f.small?.length);

/** Frames whose big words are blurred, faded or dimmed, each with what does it ("" when none). */
export function softNote(list: ReadonlyArray<CensusFrame>, max = 3): string {
  const soft = list.filter((f) => f.soft?.length);
  if (!soft.length) return "";
  return `Words big enough to read but softened (the census skips them as unreadable): ${soft.slice(0, max).map((f) => `frame ${f.frame}: ${softList(f.soft!, 2)}`).join("; ")}${soft.length > max ? `; ${soft.length - max} more frames` : ""}. Fine for a stage or a passing swap; not on a frame where that word is the moment's subject.`;
}

/** render_frames' note: each frame's words under the floor, then the fix; undefined when none (or no census). */
export function legibilityNote(list: ReadonlyArray<CensusFrame>, height: number, fps?: number): string | undefined {
  const small = frames(list);
  const blind = list.filter((f) => f.unseen).map((f) => f.frame);
  const caveat = blind.length ? ` On frame${blind.length === 1 ? "" : "s"} ${blind.slice(0, 6).join(", ")}${blind.length > 6 ? " …" : ""}, ${UNSEEN}.` : "";
  const soft = softNote(list);
  const empty = emptyPreviewNote(list, fps);
  const tail = `${caveat}${soft ? ` ${soft}` : ""}${empty ? ` ${empty}` : ""}`;
  if (!small.length) return tail ? tail.trim() : undefined;
  const lines = small.slice(0, 6).map((f) => `frame ${f.frame}: ${censusList(f.small!, 4)}`);
  return `Small text, measured on the graphics' words (letters' height as % of the frame's height; words a viewer must read need ${SMALL_TEXT * 100} %): ${lines.join("; ")}${small.length > 6 ? `; ${small.length - 6} more frames` : ""}. ${legibleFix(height)}${tail}`;
}

/** The census near a moment (seconds): the closest measured frame within half a second, as "measured there: …". */
export function censusNear(list: ReadonlyArray<CensusFrame>, at: number, fps: number): string | undefined {
  const f = at * fps;
  const near = list.filter((x) => (x.measured ?? 0) > 0 && Math.abs(x.frame - f) <= fps / 2).sort((a, b) => Math.abs(a.frame - f) - Math.abs(b.frame - f))[0];
  // A softened word on the moment's own frame (or the closest one with any) says what softens it:
  // T4-keynote's Export card was judged 0.08 s into a swap three rounds running, read as dim styling.
  const soft = list.filter((x) => x.soft?.length && Math.abs(x.frame - f) <= fps / 2).sort((a, b) => Math.abs(a.frame - f) - Math.abs(b.frame - f))[0];
  const softLine = soft ? `; softened at f ${soft.frame}: ${softList(soft.soft!, 2)}` : "";
  if (!near) return soft ? softLine.slice(2) : undefined;
  return `${near.small?.length ? `measured at f ${near.frame}: ${censusList(near.small, 3)}` : `measured at f ${near.frame}: no graphic word under ${SMALL_TEXT * 100} %${near.unseen ? ` (${UNSEEN})` : ""}`}${softLine}`;
}

/** The critic's fact line (in <motion>): the sheet's frames with words under the floor, or that none has. */
export function legibilityFacts(list: ReadonlyArray<CensusFrame>, fps: number): Array<string> {
  const measured = list.filter((f) => (f.measured ?? 0) > 0);
  const blind = list.filter((f) => f.unseen).length;
  if (!measured.length) return blind ? [`Words on the graphics weren't measured in the contact sheet (${UNSEEN}).`] : [];
  const small = frames(measured);
  const head = `Words measured on the graphics (DOM text on screen, not faded or blurred; letters' height as % of the frame's height; ${SMALL_TEXT * 100} % is the floor for words a viewer must read; ${UNSEEN})`;
  if (!small.length) return [`${head}: none under ${SMALL_TEXT * 100} % among the DOM words of the ${measured.length} contact-sheet frames measured${blind ? ` (${blind} frames also show canvases, images or unmeasured graphics: their words are unknown)` : ""}.`];
  return [`${head}: ${small.map((f) => `f ${f.frame} (${(f.frame / fps).toFixed(1)} s): ${censusList(f.small!, 3)}`).join("; ")}. Small labels and chrome around the moment's subject are texture; the line the moment is about isn't.`];
}

/** review_submit's finding for the agent (never a blocker): every sheet frame with small words at once, and the fix. */
export function legibilityFindings(list: ReadonlyArray<CensusFrame>, fps: number, height: number): Array<CriticFinding> {
  const small = frames(list);
  if (!small.length) return [];
  const first = small[0]!.frame;
  return [{ rule: "legibility", key: `legibility:${first}`, frame: first, hard: false, message: `Small text in ${small.length} of the contact sheet's frames (letters under ${SMALL_TEXT * 100} % of the height, measured on the graphics' words): ${small.slice(0, 8).map((f) => `f ${f.frame} (${(f.frame / fps).toFixed(1)} s): ${censusList(f.small!, 3)}`).join("; ")}. Where one of them is the line the moment is about, fix them all in this round, not one per round: ${legibleFix(height)}` }];
}
