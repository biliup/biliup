// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Turns the user's manual edits since the agent's last turn into the short note
// the agent gets before its next turn (D6). The same entries feed the
// "You made N edits by hand" line in the Agent tab.
import { componentInfo } from "@/app/lib/editor/core/visual/components";
import { fxWords } from "@/app/lib/editor/core/visual/fx";
import { DUCK, effectName, effectSummary, LIMITER, targetWords } from "@/app/lib/editor/core/audio/effects";
import type { Id, Op, Project, Session } from "@/app/lib/editor/core/timeline/types";

const secs = (frames: number, fps: number) => `${(Math.abs(frames) / fps).toFixed(1)}s`;

function label(project: Project, id: Id) {
  for (const t of project.tracks) {
    const item = t.items.find((i) => i.id === id);
    if (item) {
      const media = item.media && project.media.find((m) => m.id === item.media);
      const name = item.name ?? (item.text ? `text "${item.text.replace(/\n/g, " ")}"` : (media || undefined)?.name ?? (item.component === "component.solid" ? "background" : (componentInfo(item.component)?.name.toLowerCase() ?? item.component)) ?? (item.type === "graphic" ? `graphic ${item.src ?? ""}`.trim() : undefined) ?? id);
      return `${name} (${id})`;
    }
  }
  return id;
}

/** A volume in words: "−6 dB", "+3.5 dB", "0 dB", "−∞ dB (silent)" at or below −60. */
export const dbWords = (db: unknown) => {
  if (typeof db !== "number") return "0 dB";
  if (db <= -60) return "−∞ dB (silent)";
  const n = Math.round(db * 10) / 10;
  return `${n < 0 ? "−" : n > 0 ? "+" : ""}${Math.abs(n)} dB`;
};

/** A pan in words (D83): "centre", "30 % left", "100 % right". */
export const panWords = (pan: unknown) => (typeof pan !== "number" || pan === 0 ? "centre" : `${Math.round(Math.abs(pan) * 100)} % ${pan < 0 ? "left" : "right"}`);

/** A planner's summary for an effect change (D84–D87), in the past tense: "Added EQ to Music", "Ducked Music under Voice by −12 dB". */
const EFFECT_HINT = /^(Add|Remove|Move|Bypass|Turn on|Turn off|Set (EQ|Compressor|Voice|Noise removal|the master limiter|the loudness target)|Duck|Stop ducking)\b/;
const PAST: ReadonlyArray<[RegExp, string]> = [
  [/^Add\b/, "Added"],
  [/^Remove\b/, "Removed"],
  [/^Move\b/, "Moved"],
  [/^Bypass\b/, "Bypassed"],
  [/^Turn on\b/, "Turned on"],
  [/^Turn off\b/, "Turned off"],
  [/^Duck\b/, "Ducked"],
  [/^Stop ducking\b/, "Stopped ducking"],
];
const past = (hint: string) => PAST.reduce((s, [re, word]) => s.replace(re, word), hint);
/** A track's effects in words (an agent's update_track, which has no summary): "EQ (low cut 80 Hz), ducking under voice by −12 dB". */
export function effectsWords(effects: ReadonlyArray<{ readonly module: string; readonly params?: { readonly [key: string]: unknown }; readonly bypass?: boolean }>): string {
  if (!effects.length) return "none";
  return effects.map((fx) => (fx.module === DUCK ? `ducking ${effectSummary(fx)}` : `${effectName(fx.module)} (${effectSummary(fx)})`) + (fx.bypass ? " bypassed" : "")).join(", ");
}

/**
 * Hide, lock, mute and volume in words (the timeline's track buttons); undefined for anything
 * else. update_track replaces the mixer whole, so a mute toggle on a track with a volume and a
 * volume change on a muted track both send gainDb and mute: the edit's summary ("Mute track …",
 * "Set volume of track …") says which it was.
 */
function trackFlag(track: Id, set: { readonly [key: string]: unknown }, hint?: string): string | undefined {
  const keys = Object.keys(set);
  if (keys.length !== 1) return undefined;
  const on = (v: unknown) => v === true;
  if (keys[0] === "hidden") return `${on(set.hidden) ? "Hid" : "Showed"} track ${track}`;
  if (keys[0] === "locked") return `${on(set.locked) ? "Locked" : "Unlocked"} track ${track}`;
  if (keys[0] === "mixer") {
    const mixer = (set.mixer ?? {}) as { readonly mute?: unknown; readonly gainDb?: unknown; readonly pan?: unknown; readonly solo?: unknown; readonly effects?: ReadonlyArray<{ readonly module: string; readonly params?: { readonly [key: string]: unknown }; readonly bypass?: boolean }> };
    // Effects and ducking (D84, D85): the Mixer's summary says what changed; an agent's edit lists the chain.
    if (hint && EFFECT_HINT.test(hint)) return past(hint);
    if (!hint && Array.isArray(mixer.effects) && mixer.effects.length) {
      const rest = [typeof mixer.gainDb === "number" ? `volume ${dbWords(mixer.gainDb)}` : "", typeof mixer.pan === "number" && mixer.pan !== 0 ? `panned ${panWords(mixer.pan)}` : "", mixer.mute === true ? "muted" : "", mixer.solo === true ? "soloed" : ""].filter(Boolean);
      return `Set effects of track ${track}: ${effectsWords(mixer.effects)}${rest.length ? `; ${rest.join(", ")}` : ""}`;
    }
    // The Mixer's pan and solo (D83): the summary says which field the gesture changed.
    if (hint && /^pan\b/i.test(hint)) return `Panned track ${track} ${panWords(mixer.pan)}`;
    if (hint && /^(un)?solo\b/i.test(hint)) return `${mixer.solo === true ? "Soloed" : "Unsoloed"} track ${track}`;
    const muteNews = hint && /^(un)?mute\b/i.test(hint) ? true : hint && /\bvolume\b/i.test(hint) ? false : !("gainDb" in mixer);
    // Without a summary that says (the agent's edits): pan and solo are listed with the rest.
    const extra = hint && (muteNews || /\bvolume\b/i.test(hint)) ? "" : [typeof mixer.pan === "number" && mixer.pan !== 0 ? `, panned ${panWords(mixer.pan)}` : "", mixer.solo === true ? ", soloed" : ""].join("");
    if (muteNews && mixer.mute === true) return `Muted track ${track}${extra}`;
    if (muteNews && mixer.mute === false) return `Unmuted track ${track}${extra}`;
    if (!muteNews) return `Set volume of track ${track} to ${dbWords(mixer.gainDb)}${mixer.mute === true ? " (muted)" : ""}${extra}`;
    if (extra) return `Set mixer of track ${track}: ${extra.slice(2)}`;
    if (set.mixer === null) return `Reset the mixer of track ${track}`;
  }
  return undefined;
}

const quote = (text: string, max = 60) => {
  const flat = text.replace(/\n/g, " / ");
  return `"${flat.length > max ? `${flat.slice(0, max - 1)}…` : flat}"`;
};
const round = (n: unknown) => (typeof n === "number" ? String(Math.round(n)) : String(n));
const MOVE_NAMES: Readonly<Record<string, string>> = { "move.fade": "fade", "move.rise": "rise", "move.slide-left": "slide left", "move.wipe": "wipe", "move.word-by-word": "word by word", "move.zoom": "zoom", "move.typewriter": "typewriter", "move.bounce": "bounce", "move.blur": "blur", "move.scale": "scale", "move.pop": "pop", "move.pulse": "pulse", "move.wiggle": "wiggle" };
const STYLE_WORDS: Readonly<Record<string, (v: unknown) => string>> = {
  preset: (v) => `style ${v}`,
  size: (v) => `size ${round(v)}`,
  weight: (v) => `weight ${v}`,
  color: (v) => `colour ${v}`,
  align: (v) => `aligned ${v}`,
  font: (v) => `font ${v}`,
  tracking: (v) => `tracking ${v}`,
  lineHeight: (v) => `line height ${v}`,
  uppercase: (v) => (v ? "capitals" : "normal case"),
  case: (v) => (v === "lower" ? "lower case" : v === "title" ? "title case" : "normal case"),
  italic: (v) => (v ? "italic" : "upright"),
  // D80: outline, shadow and box in words; false = removed.
  stroke: (v) => (v === false ? "no outline" : v && typeof v === "object" ? `outline ${round((v as { width?: unknown }).width)} px${(v as { color?: unknown }).color ? ` ${(v as { color?: unknown }).color}` : ""}` : "outline"),
  shadow: (v) => (v === false ? "no shadow" : v && typeof v === "object" ? `shadow blur ${round((v as { blur?: unknown }).blur ?? 0)}` : "shadow"),
  background: (v) => (v === false ? "no box" : v && typeof v === "object" ? `${(v as { wrap?: unknown }).wrap === "lines" ? "a box per line" : "a box"} ${(v as { color?: unknown }).color ?? ""}`.trim() : "box"),
};

/**
 * update_item in words, one clause per changed field: text, style, position, animation and a
 * background's colours read as sentences; anything else as "changed <field>".
 */
function describeUpdate(id: Id, set: { readonly [key: string]: unknown }, project: Project, it: (id: Id) => string, fps: number): string {
  const keys = Object.keys(set);
  if (keys.length === 1 && keys[0] === "text" && typeof set.text === "string") return `Changed the text of ${id} to ${quote(set.text)}`;
  // D224: the empty space before a main item.
  if (keys.length === 1 && keys[0] === "gap") return typeof set.gap === "number" && set.gap > 0 ? `Left a ${secs(set.gap, fps)} gap before ${it(id)} on the main track` : `Closed the gap before ${it(id)} on the main track`;
  const sound = describeSound(id, set, it, fps);
  if (sound) return sound;
  const captions = describeCaptions(id, set, project, it, fps);
  if (captions) return captions;
  const parts: Array<string> = [];
  const rest: Array<string> = [];
  for (const key of keys) {
    const value = set[key];
    if (key === "text" && typeof value === "string") parts.push(`text ${quote(value)}`);
    else if (key === "style" && value && typeof value === "object") {
      const words = Object.entries(value as Record<string, unknown>).filter(([k]) => STYLE_WORDS[k]).map(([k, v]) => STYLE_WORDS[k]!(v));
      parts.push(words.length ? words.join(", ") : "style reset");
    } else if (key === "transform" && value && typeof value === "object") {
      const t = value as Record<string, unknown>;
      const pos = [t.x !== undefined ? `x ${round(t.x)}` : "", t.y !== undefined ? `y ${round(t.y)}` : ""].filter(Boolean).join(", ");
      const size = t.w !== undefined && t.h !== undefined ? `, box ${round(t.w)}×${round(t.h)}` : t.w !== undefined ? `, box ${round(t.w)} wide` : t.h !== undefined ? `, box ${round(t.h)} high` : "";
      // A clip's box (D75): fit, zoom, opacity and corners.
      const extra = [
        typeof t.fit === "string" ? `fit ${t.fit}` : "",
        typeof t.scale === "number" ? `scale ${Math.round(t.scale * 100)}%` : "",
        typeof t.opacity === "number" ? `opacity ${Math.round(t.opacity * 100)}%` : "",
        typeof t.radius === "number" && t.radius > 0 ? `corners ${round(t.radius)}` : "",
      ].filter(Boolean);
      parts.push(`position ${pos || "default"}${size}${extra.length ? `, ${extra.join(", ")}` : ""}`);
    } else if (key === "transform" && value === null) parts.push("position and size reset");
    else if (key === "keys") {
      // Keyframes (D75): which properties are keyed, at which times.
      const keys = Array.isArray(value) ? (value as ReadonlyArray<Record<string, unknown>>) : [];
      const props = ["x", "y", "scale", "opacity"].filter((p) => keys.some((k) => typeof k[p] === "number"));
      parts.push(
        props.length
          ? `keys ${props.map((p) => `${p} at ${keys.filter((k) => typeof k[p] === "number").map((k) => secs(Number(k.frame), fps)).join(", ")}`).join("; ")}`
          : "no keys",
      );
    } else if (key === "effects") parts.push(fxWords(value));
    else if (key === "loop") parts.push(value === true ? "looped" : "not looped");
    else if (key === "hold") parts.push(value === true ? "holding its last frame" : "not holding its last frame");
    else if (key === "animation") {
      const a = (value ?? {}) as Record<string, { module?: string; duration?: number } | undefined>;
      const one = (k: "in" | "out") => (a[k]?.module ? `${MOVE_NAMES[a[k]!.module!] ?? a[k]!.module} ${k} ${secs(a[k]!.duration ?? 0, fps)}` : `no ${k} animation`);
      parts.push(`${one("in")}, ${one("out")}${a.loop?.module ? `, ${MOVE_NAMES[a.loop.module] ?? a.loop.module} loop every ${secs(a.loop.duration ?? 0, fps)}` : ""}`);
    } else if (key === "props" && value && typeof value === "object") {
      const p = value as Record<string, unknown>;
      const item = project.tracks.flatMap((tr) => tr.items).find((i) => i.id === id);
      if (item && componentInfo(item.component)) {
        // A component's props (D76), in words: text values quoted, the rest by name.
        parts.push(Object.entries(p).map(([k, v]) => (typeof v === "string" ? `${k} ${quote(v, 40)}` : k)).join(", ") || "no props");
      } else parts.push(p.to !== undefined ? `colour ${p.color} to ${p.to} (gradient)` : p.color !== undefined ? `colour ${p.color}` : `props ${Object.keys(p).join(", ")}`);
    } else rest.push(key);
  }
  if (!parts.length) return `Changed ${keys.join(", ")} on ${it(id)}`;
  const tail = `${parts.join("; ")}${rest.length ? `; changed ${rest.join(", ")}` : ""}`;
  if (keys.length === 1 && keys[0] === "transform" && set.transform) return `Moved ${it(id)} on the canvas to ${tail.replace(/^position /, "")}`;
  if (keys.every((k) => k === "keys")) return `Keyframed ${it(id)}: ${tail}`;
  return `${keys.length === 1 && keys[0] === "animation" ? "Animated" : "Restyled"} ${it(id)}: ${tail}`;
}

const CAPTION_KEYS = new Set(["edits", "breaks", "mode", "lines", "hideFillers", "words", "start", "duration", "coversMain"]);

/** A captions item's own fields (D82) in words, when they are all an update_item changes: the corrected words by id, so the agent can find them. */
function describeCaptions(id: Id, set: { readonly [key: string]: unknown }, project: Project, it: (id: Id) => string, fps: number): string | undefined {
  const item = project.tracks.flatMap((t) => t.items).find((i) => i.id === id);
  const keys = Object.keys(set);
  if (item?.type !== "captions" || !keys.length || !keys.every((k) => CAPTION_KEYS.has(k))) return undefined;
  const parts: Array<string> = [];
  for (const k of keys) {
    const v = set[k];
    if (k === "edits") {
      const list = Array.isArray(v) ? (v as ReadonlyArray<{ word: number; text: string }>) : [];
      parts.push(list.length ? `corrections ${list.map((e) => `w${e.word} ${e.text === "" ? "hidden" : quote(e.text, 30)}`).join(", ")}` : "no corrections");
    } else if (k === "breaks") parts.push(Array.isArray(v) && v.length ? `new cards at ${(v as ReadonlyArray<number>).map((w) => `w${w}`).join(", ")}` : "no forced cards");
    else if (k === "mode") parts.push(`mode ${String(v ?? "line")}`);
    else if (k === "lines") parts.push(v && typeof v === "object" ? `lines ${Object.entries(v).map(([a, b]) => `${a} ${b}`).join(", ")}` : "default lines");
    else if (k === "hideFillers") parts.push(v === true ? "fillers hidden" : "fillers shown");
    else if (k === "words") parts.push(`${Array.isArray(v) ? v.length : 0} words from the transcript`);
    else if (k === "duration" && typeof v === "number") parts.push(`covering ${secs(v, fps)}`);
  }
  return `Changed the captions ${it(id)}: ${parts.join("; ")}`;
}

const SOUND_KEYS = new Set(["gainDb", "fadeIn", "fadeOut", "gainKeys"]);
const fadeSecs = (v: unknown, fps: number) => (typeof v === "number" && v > 0 ? secs(v, fps).replace("s", " s") : undefined);

/** Volume, fades and volume keys in words, when they are all an update_item changes. */
function describeSound(id: Id, set: { readonly [key: string]: unknown }, it: (id: Id) => string, fps: number): string | undefined {
  const keys = Object.keys(set);
  if (!keys.length || !keys.every((k) => SOUND_KEYS.has(k))) return undefined;
  if (keys.length === 1) {
    const [key] = keys;
    if (key === "gainDb") return `Set volume of ${it(id)} to ${dbWords(set.gainDb)}`;
    if (key === "fadeIn") return fadeSecs(set.fadeIn, fps) ? `Faded in ${it(id)} over ${fadeSecs(set.fadeIn, fps)}` : `Removed the fade in on ${it(id)}`;
    if (key === "fadeOut") return fadeSecs(set.fadeOut, fps) ? `Faded out ${it(id)} over ${fadeSecs(set.fadeOut, fps)}` : `Removed the fade out on ${it(id)}`;
  }
  const parts = keys.map((k) =>
    k === "gainDb"
      ? `volume ${dbWords(set.gainDb)}`
      : k === "fadeIn" || k === "fadeOut"
        ? `${fadeSecs(set[k], fps) ? `fade ${k === "fadeIn" ? "in" : "out"} ${fadeSecs(set[k], fps)}` : `no fade ${k === "fadeIn" ? "in" : "out"}`}`
        : `${Array.isArray(set.gainKeys) ? set.gainKeys.length : 0} volume keys`,
  );
  return `Changed the sound of ${it(id)}: ${parts.join(", ")}`;
}

/** An op in words, for the agent's note and turn cards. `hint` is the edit's summary, if any. */
export function describeOp(op: Op, project: Project, hint?: string): string {
  const fps = project.canvas.fps;
  const it = (id: Id) => label(project, id);
  switch (op.op) {
    case "trim_item":
      return `${op.frames > 0 ? "Trimmed" : "Extended"} ${it(op.item)} by ${secs(op.frames, fps)} at the ${op.edge}`;
    case "update_item":
      return describeUpdate(op.item, op.set, project, it, fps);
    case "split_item":
      return `Split ${it(op.item)} at ${secs(op.at, fps)}`;
    case "move_item":
      return `Moved ${it(op.item)}${op.track ? ` to track ${op.track}` : ""}${op.start !== undefined ? ` to ${secs(op.start, fps)}` : ""}${op.index !== undefined ? ` to position ${op.index + 1}` : ""}`;
    case "delete_item":
      return `Deleted ${op.item}`;
    case "insert_item":
      if (op.item.type === "component" && op.item.component === "component.solid") {
        const p = op.item.props ?? {};
        return `Added background ${op.item.id} (${p.to !== undefined ? `gradient ${p.color} to ${p.to}` : String(p.color ?? "default colour")}) on track ${op.track}`;
      }
      if (op.item.type === "component" && componentInfo(op.item.component)) return `Added ${componentInfo(op.item.component)!.name.toLowerCase()} ${op.item.id} on track ${op.track}`;
      if (op.item.type === "graphic") return `Added graphic ${op.item.id} (${op.item.src ?? "no file"}) on track ${op.track}`;
      if (op.item.type === "captions") return `Added captions ${op.item.id} of ${op.item.source} (${op.item.mode ?? "line"} mode, ${op.item.words?.length ?? 0} words) on track ${op.track}`;
      return `Added ${op.item.type} ${op.item.id} on track ${op.track}`;
    case "set_transition":
      return op.transition ? `Set ${op.transition.module} into ${it(op.item)}` : `Removed the transition into ${it(op.item)}`;
    case "update_track":
      return trackFlag(op.track, op.set, hint) ?? `Changed ${Object.keys(op.set).join(", ")} on track ${op.track}`;
    case "add_track":
      return `Added ${op.track.kind ?? ""} track ${op.track.name ? `"${op.track.name}" (${op.track.id})` : op.track.id}`.replace("  ", " ");
    case "delete_track":
      return `Deleted track ${op.track}`;
    case "add_media":
      return `Added media ${op.media.name} (${op.media.id})`;
    case "update_media": {
      const name = project.media.find((m) => m.id === op.media)?.name ?? op.media;
      if ("path" in op.set) return `Swapped in a new file for ${name} (${op.media})`;
      return `Changed ${Object.keys(op.set).join(", ")} of media ${name} (${op.media})`;
    }
    case "delete_media":
      return `Removed media ${project.media.find((m) => m.id === op.media)?.name ?? op.media} (${op.media}) from the project`;
    case "update_project": {
      // The master volume (D83); update_project replaces master whole.
      const keys = Object.keys(op.set);
      if (keys.length === 1 && keys[0] === "master") {
        const m = (op.set.master ?? {}) as { readonly gainDb?: unknown; readonly loudness?: { readonly targetLufs?: unknown }; readonly effects?: ReadonlyArray<{ readonly module: string; readonly params?: { readonly [key: string]: unknown }; readonly bypass?: boolean }> };
        // The loudness target and the limiter (D86, D87): the Mixer's summary, else what the master now says.
        if (hint && EFFECT_HINT.test(hint)) return past(hint);
        if (hint?.startsWith("Set master volume")) return `Set master volume to ${dbWords(m.gainDb)}`;
        // update_project replaces master whole: say every part it now has (volume, target, limiter).
        const limiter = m.effects?.find((fx) => fx.module === LIMITER);
        const parts = [
          typeof m.gainDb === "number" ? `volume ${dbWords(m.gainDb)}` : "",
          typeof m.loudness?.targetLufs === "number" ? `loudness target ${targetWords(m.loudness.targetLufs)}` : "",
          limiter ? (limiter.bypass ? "limiter off" : `limiter ${effectSummary(limiter)}`) : "",
        ].filter(Boolean);
        if (parts.length === 1 && typeof m.gainDb === "number") return `Set master volume to ${dbWords(m.gainDb)}`;
        if (parts.length) return `Set the master: ${parts.join(", ")}`;
      }
      // Issue #118: the marked sections, replaced whole; the editor's summary says what changed.
      if (keys.length === 1 && keys[0] === "sections") {
        if (hint) return past(hint);
        const list = (op.set.sections ?? []) as ReadonlyArray<{ readonly start: number; readonly end: number; readonly state: string }>;
        return list.length ? `Marked sections: ${list.map((s) => `${s.state === "locked" ? "locked" : "to redo"} ${s.start}–${s.end}`).join(", ")}` : "Cleared the marked sections";
      }
      // Taste checks turned off (D90): checks is replaced whole.
      if (keys.length === 1 && keys[0] === "checks") {
        // The editor's summary names the one rule switched ("Turn off the check …"); otherwise the
        // op only says the whole list now, so say that rather than guess what changed.
        if (hint && /^Turn (off|on) the check /.test(hint)) return past(hint);
        const off = (op.set.checks as { readonly disabled?: unknown } | null)?.disabled;
        return Array.isArray(off) && off.length ? `Set the checks turned off to ${off.join(", ")}` : "Turned every check back on";
      }
      // D291: the export's motion blur (merged field by field).
      if (keys.length === 1 && keys[0] === "motionBlur") {
        const b = op.set.motionBlur as { readonly on?: unknown; readonly samples?: unknown; readonly shutter?: unknown } | null;
        if (!b) return "Set motion blur back to its defaults";
        const parts = [typeof b.on === "boolean" ? (b.on ? "on" : "off") : "", typeof b.samples === "number" ? `${b.samples} samples` : "", typeof b.shutter === "number" ? `${b.shutter}° shutter` : ""].filter(Boolean);
        return parts.length ? `Set motion blur: ${parts.join(", ")}` : "Changed motion blur";
      }
      return `Changed project ${keys.join(", ")}`;
    }
    default:
      // Every op is handled above; this is for ops from a newer format.
      return String((op as { op: unknown }).op).replace(/_/g, " ");
  }
}

/** The note the agent receives. Empty string when the user changed nothing. */
export function manualEditsNote(session: Session, sinceRevision: number): string {
  const entries = session.log.filter((e) => e.revision > sinceRevision && e.actor === "user");
  const lines = entries.flatMap((e) => e.ops.map((op) => `- ${describeOp(op, session.project, e.summary)}`));
  if (!lines.length) return "";
  return [
    `The user made ${lines.length} edit${lines.length === 1 ? "" : "s"} by hand since your last turn (now at revision ${session.revision}).`,
    "They are intentional. Keep them, build on them, and don't undo or question them unless the user asks.",
    ...lines,
  ].join("\n");
}
