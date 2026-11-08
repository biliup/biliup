// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { overlaps } from "@/app/lib/editor/core/visual/transitions";
import type { Id } from "@/app/lib/editor/core/timeline/types";
import { type CheckRule, mediaOf, textLabel, secs, startOf, type Check } from "@/app/lib/editor/core/checks/context";

/** Motion and timing rules: speeds, entrances, transitions and gaps. */
export const MOTION_RULES: ReadonlyArray<CheckRule> = [
  {
    id: "one-speed",
    title: "One footage speed",
    description: "Footage plays at one constant speed; cutting idle time is fine, mixing speeds isn't.",
    severity: "warn",
    check: (ctx) => {
      const by = new Map<number, Array<Id>>();
      for (const track of ctx.project.tracks) {
        if (track.hidden || track.kind !== "video") continue;
        for (const item of track.items) {
          const m = item.type === "clip" ? mediaOf(ctx, item) : undefined;
          if (m?.kind === "video") by.set(item.speed ?? 1, [...(by.get(item.speed ?? 1) ?? []), item.id]);
        }
      }
      if (by.size < 2) return [];
      const speeds = [...by.keys()].sort((a, b) => a - b).map((s) => `${Math.round(s * 100) / 100}×`);
      // The odd ones out: every clip not at the most used speed (1× wins a tie).
      const base = [...by.entries()].sort((a, b) => b[1].length - a[1].length || Number(b[0] === 1) - Number(a[0] === 1))[0]![0];
      const odd = [...by.entries()].filter(([s]) => s !== base).flatMap(([, list]) => list);
      return [
        {
          key: "one-speed",
          message: `Clips play at ${speeds.slice(0, -1).join(", ")} and ${speeds.at(-1)}; mixed speeds in one demo read as dishonest.`,
          itemIds: odd,
          frame: Math.min(...odd.map((id) => ctx.starts.get(id) ?? 0)),
          hint: "Use one speed for all footage and say it once in the post (for example \"sped up 3×\").",
        },
      ];
    },
  },
  {
    id: "same-entrance",
    title: "Varied entrances",
    description: "Three or more texts that all enter with the same move read as a template.",
    severity: "info",
    check: (ctx) => {
      const texts = ctx.texts.filter((p) => textLabel(p.item.text));
      if (texts.length < 3) return [];
      const first = texts[0]!.item.animation?.in?.module;
      if (!first || !texts.every((p) => p.item.animation?.in?.module === first)) return [];
      return [
        {
          key: "same-entrance",
          message: `All ${texts.length} texts enter with the same ${first.replace(/^move\./, "").replace(/-/g, " ")} move, which looks templated.`,
          itemIds: texts.map((p) => p.item.id),
          frame: texts[0]!.start,
          hint: "Let most text simply cut in, and keep a move for the one line that needs it.",
        },
      ];
    },
  },
  {
    id: "long-transition",
    title: "Short transitions",
    description: "Cuts are hard or very short crossfades (1 s at most).",
    severity: "info",
    check: (ctx) =>
      ctx.project.tracks.flatMap((track) =>
        track.role !== "main"
          ? []
          : track.items
              .filter((i) => i.transitionIn && overlaps(i.transitionIn.module) && i.transitionIn.duration > ctx.fps)
              .map((i) => ({
                key: `long-transition:${i.id}`,
                message: `The ${secs(i.transitionIn!.duration, ctx.fps)} transition into ${i.name ?? i.id} is long; hard cuts or very short crossfades keep the pace.`,
                itemIds: [i.id],
                frame: startOf(ctx, i),
                ...(track.locked || i.locked ? {} : { fix: { summary: "Shorten the transition to 0.3 s", ops: [{ op: "set_transition" as const, item: i.id, transition: { ...i.transitionIn!, duration: Math.max(2, Math.round(0.3 * ctx.fps)) } }] } }),
              })),
      ),
  },
  {
    id: "blank-gap",
    title: "No blank stretches",
    description: "Nothing shows black in the middle of the video.",
    severity: "warn",
    check: (ctx) => {
      if (ctx.end <= 0) return [];
      const spans: Array<[number, number]> = [];
      for (const track of ctx.project.tracks) {
        if (track.hidden || track.kind === "audio") continue;
        for (const item of track.items) if (item.type !== "captions") spans.push([startOf(ctx, item), startOf(ctx, item) + item.duration]);
      }
      spans.sort((a, b) => a[0] - b[0]);
      const out: Array<Omit<Check, "id" | "severity">> = [];
      let covered = 0;
      for (const [s, e] of spans) {
        // A gap from frame 0 is the frame-one rule's.
        if (s > covered && covered > 0 && s - covered >= Math.round(ctx.fps / 5)) out.push({ key: `blank-gap:${covered}`, message: `Nothing shows from ${secs(covered, ctx.fps)} to ${secs(s, ctx.fps)}, so the video goes black there.`, frame: covered, range: { start: covered, end: s }, hint: "Close the gap or fill it with footage." });
        covered = Math.max(covered, e);
      }
      return out;
    },
  },
];

/** Canvas rules: the format. */
export const CANVAS_RULES: ReadonlyArray<CheckRule> = [
  {
    id: "canvas-format",
    title: "Canvas for the feed",
    description: "Phone feeds show 4:5 (1080×1350) biggest; 16:9 suits YouTube, decks and sites.",
    severity: "info",
    check: (ctx) => {
      const { width, height } = ctx.project.canvas;
      if (Math.abs(width / height - 4 / 5) < 0.01) return [];
      return [{ key: "canvas-format", message: `The canvas is ${width}×${height}; for X and other phone feeds, 4:5 (1080×1350) shows bigger, while 16:9 is right for YouTube and sites.`, hint: "Pick the format when you make the project; keep this one if it's for a landscape screen." }];
    },
  },
];
