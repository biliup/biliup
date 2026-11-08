// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { textLook } from "@/app/lib/editor/core/text/looks";
import { wordCount, textPlacement, safeMargin } from "@/app/lib/editor/core/visual/render";
import { visualsAt } from "@/app/lib/editor/core/timeline/timeline";
import type { Op } from "@/app/lib/editor/core/timeline/types";
import { type CheckRule, readFrames, holdOf, textLabel, lengthen, secs, fixPeriods, locked, fixDots, familyOf, FILLER_PHRASES, FILLER_RE, startOf, isFootnote, type Placed } from "@/app/lib/editor/core/checks/context";

// ---- the rules ----

/** Text rules: read time, holds, punctuation, fillers, typefaces, sizes and edges. */
export const TEXT_RULES: ReadonlyArray<CheckRule> = [
  {
    id: "text-read-time",
    title: "Time to read",
    description: "Every text stays fully on screen for at least words ÷ 3 seconds (2 s minimum), counted after its in move.",
    severity: "warn",
    check: (ctx) =>
      ctx.texts.flatMap((p) => {
        const need = readFrames(p.item.text, ctx.fps);
        const hold = holdOf(p.item);
        if (!textLabel(p.item.text) || hold >= need) return [];
        const words = wordCount(p.item.text ?? "");
        const fix = lengthen(ctx, p, need - hold);
        return [
          {
            key: `text-read-time:${p.item.id}`,
            message: `"${textLabel(p.item.text)}" is fully on screen for ${secs(hold, ctx.fps)}, and ${words} word${words === 1 ? "" : "s"} need about ${secs(need, ctx.fps)} to read twice.`,
            itemIds: [p.item.id],
            frame: p.start,
            range: { start: p.start, end: p.start + p.item.duration },
            ...(fix ? { fix } : { hint: "Make it longer (or shorten its moves), or cut words." }),
          },
        ];
      }),
  },
  {
    id: "last-text-hold",
    title: "Last text holds",
    description: "The last text of the video stays fully on screen for at least 3 seconds.",
    severity: "warn",
    check: (ctx) => {
      const last = ctx.texts.filter((p) => textLabel(p.item.text)).sort((a, b) => a.start + a.item.duration - (b.start + b.item.duration)).at(-1);
      if (!last) return [];
      const need = 3 * ctx.fps;
      const hold = holdOf(last.item);
      // The read-time rule already asks for more when it needs more.
      if (hold >= need || readFrames(last.item.text, ctx.fps) > need) return [];
      const fix = lengthen(ctx, last, need - hold);
      return [
        {
          key: `last-text-hold:${last.item.id}`,
          message: `The last text, "${textLabel(last.item.text)}", holds for ${secs(hold, ctx.fps)}; give the closing line at least 3 s.`,
          itemIds: [last.item.id],
          frame: last.start,
          range: { start: last.start, end: last.start + last.item.duration },
          ...(fix ? { fix } : { hint: "Start it earlier or make the video end later." }),
        },
      ];
    },
  },
  {
    id: "late-text",
    title: "No new text at the very end",
    description: "No text starts in the last 2 seconds of the video.",
    severity: "warn",
    check: (ctx) =>
      ctx.end <= 2 * ctx.fps
        ? []
        : ctx.texts
            .filter((p) => textLabel(p.item.text) && p.start > ctx.end - 2 * ctx.fps)
            .map((p) => ({
              key: `late-text:${p.item.id}`,
              message: `"${textLabel(p.item.text)}" starts ${secs(ctx.end - p.start, ctx.fps)} before the end, too late to be read.`,
              itemIds: [p.item.id],
              frame: p.start,
              hint: "Bring it in earlier, or drop it.",
            })),
  },
  {
    id: "trailing-period",
    title: "No trailing period",
    description: "Titles, captions and labels don't end with a period; a line break separates two thoughts.",
    severity: "warn",
    check: (ctx) =>
      ctx.texts.flatMap((p) => {
        const text = p.item.text ?? "";
        const fixed = fixPeriods(text);
        if (fixed === undefined) return [];
        return [
          {
            key: `trailing-period:${p.item.id}`,
            message: `"${textLabel(text)}" ends with a period; on screen it reads better without.`,
            itemIds: [p.item.id],
            frame: p.start,
            ...(locked(p) ? { hint: "Unlock it to drop the period." } : { fix: { summary: `Remove the period from "${textLabel(fixed)}"`, ops: [{ op: "update_item" as const, item: p.item.id, set: { text: fixed } }] } }),
          },
        ];
      }),
  },
  {
    id: "dot-separators",
    title: "No dots between words",
    description: "No ·, • or * between words: list with commas or line breaks.",
    severity: "warn",
    check: (ctx) =>
      ctx.texts.flatMap((p) => {
        const text = p.item.text ?? "";
        const fixed = fixDots(text, familyOf(ctx.project, p.item).mono);
        if (fixed === undefined) return [];
        return [
          {
            key: `dot-separators:${p.item.id}`,
            message: `"${textLabel(text)}" separates words with dots, which reads as generated; use commas or line breaks.`,
            itemIds: [p.item.id],
            frame: p.start,
            ...(locked(p) ? { hint: "Unlock it to swap the dots for commas." } : { fix: { summary: `Use commas in "${textLabel(fixed)}"`, ops: [{ op: "update_item" as const, item: p.item.id, set: { text: fixed } }] } }),
          },
        ];
      }),
  },
  {
    id: "filler-phrases",
    title: "No filler phrases",
    description: `No lines that could sit under any product (${FILLER_PHRASES.slice(0, 5).join(", ")}…).`,
    severity: "warn",
    check: (ctx) =>
      ctx.texts.flatMap((p) => {
        const found = [...new Set([...(p.item.text ?? "").matchAll(FILLER_RE)].map((m) => m[1]!.toLowerCase().replace(/\s+/g, " ")))];
        if (!found.length) return [];
        return [
          {
            key: `filler-phrases:${p.item.id}`,
            message: `"${textLabel(p.item.text)}" uses ${found.map((f) => `"${f}"`).join(" and ")}, which could describe any product.`,
            itemIds: [p.item.id],
            frame: p.start,
            hint: "Say the specific thing it does, the way you'd tell another dev.",
          },
        ];
      }),
  },
  {
    id: "frame-one",
    title: "Hook on frame 1",
    description: "Frame 1 is the thumbnail and what autoplay shows muted: it should show the video, not black or a fade from black.",
    severity: "warn",
    check: (ctx) => {
      if (ctx.end <= 0) return [];
      const shown = visualsAt(ctx.project, ctx.starts, 0).filter((v) => !v.item.id.startsWith("dip:"));
      const best = Math.max(0, ...shown.map((v) => v.opacity));
      if (best >= 0.5) return [];
      // Items at frame 0 that fade in: dropping their in move starts on a full frame.
      const fading = ctx.project.tracks.flatMap((track) => (track.hidden || track.kind === "audio" ? [] : track.items.filter((i) => startOf(ctx, i) === 0 && i.animation?.in && !locked({ item: i, track, start: 0 })).map((i) => ({ i }))));
      const ops: Array<Op> = fading.map(({ i }) => {
        const { in: _, ...rest } = i.animation!;
        return { op: "update_item", item: i.id, set: { animation: Object.keys(rest).length ? rest : null } };
      });
      return [
        {
          key: "frame-one",
          message: shown.length ? "Frame 1 fades in from black, so the thumbnail and the muted autoplay start on an empty frame." : "Frame 1 is empty, so the thumbnail and the muted autoplay start on black.",
          itemIds: fading.map(({ i }) => i.id),
          frame: 0,
          ...(ops.length ? { fix: { summary: "Start on a full frame (no fade in)", ops } } : { hint: "Start on the claim: put footage or the first line on frame 1." }),
        },
      ];
    },
  },
  {
    id: "crowded-text",
    title: "Few texts at once",
    description: "At most two texts on screen at the same time (a footnote in the Footnote look at its size or smaller, the disclosure line, doesn't count).",
    severity: "warn",
    check: (ctx) => {
      // Sweep the text spans: the first stretch where three or more overlap, once. A footnote
      // (text.footnote) is the small print best practice asks for ("sped up 8×", how it was
      // measured), not a competing idea, so it doesn't count (D96).
      const events = ctx.texts.filter((p) => !isFootnote(ctx, p.item)).flatMap((p) => [
        { at: p.start, d: 1, p },
        { at: p.start + p.item.duration, d: -1, p },
      ]);
      events.sort((a, b) => a.at - b.at || a.d - b.d);
      const on = new Set<Placed>();
      for (const e of events) {
        if (e.d < 0) on.delete(e.p);
        else on.add(e.p);
        if (on.size > 2) {
          const list = [...on];
          const end = Math.min(...list.map((p) => p.start + p.item.duration));
          return [
            {
              key: "crowded-text",
              message: `${list.length} texts are on screen together at ${secs(e.at, ctx.fps)}; one idea per frame reads better.`,
              itemIds: list.map((p) => p.item.id),
              frame: e.at,
              range: { start: e.at, end },
              hint: "Keep a title and at most one supporting line; move the rest to another moment or into the post.",
            },
          ];
        }
      }
      return [];
    },
  },
  {
    id: "one-typeface",
    title: "One typeface",
    description: "Text uses one typeface family (a mono face for code or labels is fine).",
    severity: "warn",
    check: (ctx) => {
      const by = new Map<string, { name: string; placed: Array<Placed>; tokens: Map<string, number> }>();
      for (const p of ctx.texts) {
        const f = familyOf(ctx.project, p.item);
        if (f.mono) continue;
        const g = by.get(f.key) ?? { name: f.name, placed: [] as Array<Placed>, tokens: new Map<string, number>() };
        g.placed.push(p);
        g.tokens.set(f.token, (g.tokens.get(f.token) ?? 0) + 1);
        by.set(f.key, g);
      }
      if (by.size < 2) return [];
      const groups = [...by.entries()].sort((a, b) => b[1].placed.length - a[1].placed.length);
      const names = groups.map(([, g]) => g.name);
      // The fix: every other text in the family most texts use (the house pair as DM Sans).
      const [majorKey, major] = groups[0]!;
      const token = majorKey === "sans" ? "DM Sans" : [...major.tokens.entries()].sort((a, b) => b[1] - a[1])[0]![0];
      const others = groups.slice(1).flatMap(([, g]) => g.placed);
      const ops: Array<Op> = others.filter((p) => !locked(p)).map((p) => ({ op: "update_item", item: p.item.id, set: { style: { ...(p.item.style ?? {}), font: token } } }));
      return [
        {
          key: "one-typeface",
          message: `Text uses ${names.length} typefaces (${names.slice(0, -1).join(", ")} and ${names.at(-1)}); one family with a real size scale looks designed.`,
          itemIds: others.map((p) => p.item.id),
          frame: Math.min(...others.map((p) => p.start)),
          ...(ops.length ? { fix: { summary: `Set ${ops.length === 1 ? "1 text" : `${ops.length} texts`} in ${major.name}`, ops } } : { hint: "Pick one in the Style tab's font list and use it for every text." }),
        },
      ];
    },
  },
  {
    id: "small-text",
    title: "Readable on a phone",
    description: "Text is at least 28 px tall on a 1080 px canvas (scaled to the canvas), so it reads at phone size.",
    severity: "warn",
    check: (ctx) =>
      ctx.texts.flatMap((p) => {
        const min = Math.round(28 * ctx.scale);
        const { size } = textPlacement(p.item, ctx.project);
        if (!textLabel(p.item.text) || size >= min - 0.5) return [];
        return [
          {
            key: `small-text:${p.item.id}`,
            message: `"${textLabel(p.item.text)}" is ${Math.round(size)} px, below the ${min} px that still reads on a phone.`,
            itemIds: [p.item.id],
            frame: p.start,
            ...(locked(p) ? { hint: "Unlock it to make it bigger." } : { fix: { summary: `Make "${textLabel(p.item.text)}" ${min} px`, ops: [{ op: "update_item" as const, item: p.item.id, set: { style: { ...(p.item.style ?? {}), size: min } } }] } }),
          },
        ];
      }),
  },
  {
    id: "text-edge",
    title: "Inside the safe area",
    description: "Text keeps clear of the frame's edges (about half the safe margin at least).",
    severity: "warn",
    check: (ctx) =>
      ctx.texts.flatMap((p) => {
        if (!textLabel(p.item.text)) return [];
        const { width, height } = ctx.project.canvas;
        const m = safeMargin(ctx.project) / 2;
        const { x, y, w, size } = textPlacement(p.item, ctx.project);
        const look = textLook(p.item.style?.preset);
        const align = p.item.style?.align ?? look.align ?? "left";
        const lines = (p.item.text ?? "").split("\n").length;
        const bottom = y + lines * size * (p.item.style?.lineHeight ?? look.lineHeight);
        const sides = [x < m && (align === "left" || w === undefined) ? "left" : "", y < m ? "top" : "", w !== undefined && x + w > width - m && align === "right" ? "right" : "", bottom > height - m ? "bottom" : ""].filter(Boolean);
        if (!sides.length) return [];
        return [
          {
            key: `text-edge:${p.item.id}`,
            message: `"${textLabel(p.item.text)}" is about at the ${sides.join(" and ")} edge of the frame; feeds crop and overlay there.`,
            itemIds: [p.item.id],
            frame: p.start,
            hint: `Keep it at least ${Math.round(safeMargin(ctx.project))} px from the edges (the safe margin).`,
          },
        ];
      }),
  },
];
