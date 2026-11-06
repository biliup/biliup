// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Pure helpers for the Agent tab's thread (D295): which turns belong to the current chat, and the
// hand-edit note's lines with media imports folded into one.

/** The turns before the current chat's start (New chat) and the ones of the current chat, in order. */
export function splitThread<T extends { readonly startedAt: string }>(turns: ReadonlyArray<T>, threadStart: string | undefined): { readonly earlier: ReadonlyArray<T>; readonly current: ReadonlyArray<T> } {
  const from = threadStart ? Date.parse(threadStart) : NaN;
  if (!Number.isFinite(from)) return { earlier: [], current: turns };
  const earlier: Array<T> = [];
  const current: Array<T> = [];
  for (const t of turns) (Date.parse(t.startedAt) < from ? earlier : current).push(t);
  return { earlier, current };
}

/** One line of the hand-edit note as shown: a plain line, or the media imports folded into one. */
export type NoteLine = { readonly kind: "line"; readonly text: string } | { readonly kind: "imports"; readonly text: string; readonly names: ReadonlyArray<string> };

const IMPORT = /^Added media (.+)$/;

/**
 * The note's "- " lines; two or more "Added media …" lines become one "Added N files" line (where the
 * first of them was) that opens to the names. A single import stays as it was.
 */
export function noteLines(note: string): ReadonlyArray<NoteLine> {
  const lines = note.split("\n").filter((l) => l.startsWith("- ")).map((l) => l.slice(2));
  const names = lines.flatMap((l) => {
    const m = IMPORT.exec(l);
    return m ? [m[1]!] : [];
  });
  // A summary line ("Added media a, b, c, d, e and 18 more") counts its five and the rest.
  const count = names.reduce((n, x) => {
    const more = /^(?:[^,]+, ){4}[^,]+ and (\d+) more$/.exec(x);
    return n + (more ? 5 + Number(more[1]) : 1);
  }, 0);
  if (count < 2) return lines.map((text) => ({ kind: "line", text }));
  const out: Array<NoteLine> = [];
  let folded = false;
  for (const text of lines) {
    if (!IMPORT.test(text)) out.push({ kind: "line", text });
    else if (!folded) {
      folded = true;
      out.push({ kind: "imports", text: `Added ${count} files`, names });
    }
  }
  return out;
}
