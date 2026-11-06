// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { EASES } from "@/app/lib/editor/core/visual/box";
import { DEFAULT_THEME, DM_SANS, markup, DM_MONO, esc, num } from "@/app/lib/editor/core/visual/render";
import { type Draw, safeSrc, mediaSrc, str, color, px, fg, accent, mute, numberLabel } from "@/app/lib/editor/core/visual/components/shared";

// ---- end card ----

export function drawEndCard({ props, w, h, target }: Draw) {
  const k = Math.min(w / 1920, h / 1080);
  const logo = safeSrc(mediaSrc(target, props.logo));
  const title = str(props.title);
  const code = str(props.code);
  const url = str(props.url);
  const bg = typeof props.background === "string" ? color(target, props.background, DEFAULT_THEME.colors.bg) : undefined;
  const parts: Array<string> = [];
  if (logo) parts.push(`<img src="${logo}" alt="" style="display:block;height:${px(128 * k)};max-width:${px(w * 0.6)};object-fit:contain">`);
  if (title) parts.push(`<div style="font-family:${DM_SANS};font-weight:700;font-size:${px(92 * k)};line-height:1.08;letter-spacing:-0.02em;color:${fg(target)};--accent:${accent(target)};text-align:center;max-width:${px(w * 0.86)}">${markup(title)}</div>`);
  if (code) parts.push(`<div style="font-family:${DM_MONO};font-weight:500;font-size:${px(40 * k)};line-height:1.2;color:${fg(target)};padding:${px(18 * k)} ${px(34 * k)};border-radius:${px(16 * k)};background:rgb(128 128 128 / 16%);border:${px(1.5 * k)} solid rgb(128 128 128 / 30%);white-space:pre">${esc(code)}</div>`);
  if (url) parts.push(`<div style="font-family:${DM_MONO};font-weight:400;font-size:${px(32 * k)};line-height:1.2;color:${mute(target)}">${esc(url)}</div>`);
  return `<div style="position:absolute;inset:0;display:flex;flex-direction:column;align-items:center;justify-content:center;gap:${px(40 * k)};${bg ? `background:${bg};` : ""}">${parts.join("")}</div>`;
}

// ---- code block ----

export function drawCodeBlock({ props, w, h, target }: Draw) {
  const file = str(props.file);
  const prompt = props.prompt === true;
  const lines = str(props.code).replace(/\r/g, "").split("\n");
  const longest = Math.max(1, ...lines.map((l) => [...l].length + (prompt ? 2 : 0)), file ? [...file].length * 0.8 : 0);
  // Sized to fit: rows of 1.5 em, a file line of 0.8 em + a gap, 0.9 em of padding all round
  // (1.8 em both ways), DM Mono's advance 0.6 em.
  const rows = lines.length * 1.5 + (file ? 1.4 : 0);
  const size = Math.max(4, Math.min(h / (rows + 1.8), w / (longest * 0.6 + 1.8), 64 * Math.min(w / 760, h / 120, 1.6)));
  const pad = size * 0.9;
  const body = lines
    .map((line) => {
      const add = line.startsWith("+ ");
      const del = line.startsWith("- ");
      const text = esc(add || del ? line.slice(2) : line) || " ";
      const style = add ? `color:${accent(target)}` : del ? `color:${mute(target)};text-decoration:line-through` : "";
      return `<div style="white-space:pre;${style}">${prompt ? `<span style="color:${mute(target)}">$ </span>` : ""}${text}</div>`;
    })
    .join("");
  const header = file ? `<div style="color:${mute(target)};font-size:0.8em;margin-bottom:0.6em;white-space:pre">${esc(file)}</div>` : "";
  return `<div style="position:absolute;inset:0;display:flex;flex-direction:column;justify-content:center;padding:0 ${px(pad)};background:#111113;border:${px(Math.max(1, size * 0.05))} solid #2A2A2E;border-radius:${px(Math.min(h / 2, size * 0.6))};font-family:${DM_MONO};font-weight:400;font-size:${px(size)};line-height:1.5;color:${fg(target)};overflow:hidden">${header}${body}</div>`;
}

// ---- chart ----

interface Bar {
  readonly label: string;
  readonly value: number;
  readonly accent?: boolean;
}
const barsOf = (v: unknown): Array<Bar> =>
  Array.isArray(v) ? v.filter((b): b is Bar => !!b && typeof b === "object" && typeof (b as Bar).value === "number" && Number.isFinite((b as Bar).value)).map((b) => ({ label: str(b.label), value: Math.max(0, b.value), accent: b.accent === true })) : [];

export function drawChart({ props, w, h, target }: Draw) {
  const bars = barsOf(props.bars);
  const k = Math.min(w / 1000, h / 600);
  const unit = str(props.unit);
  const max = typeof props.max === "number" && props.max > 0 ? props.max : Math.max(1e-9, ...bars.map((b) => b.value));
  const valueSize = 34 * k;
  const labelSize = 28 * k;
  // Room for the value over the tallest bar and the label under every bar.
  const top = valueSize * 1.7;
  const bottom = labelSize * 2.1;
  const area = Math.max(1, h - top - bottom);
  const slot = w / Math.max(1, bars.length);
  const barW = Math.min(slot * 0.56, 150 * k);
  const neutral = mute(target);
  const columns = bars
    .map((b, i) => {
      const bh = Math.max(b.value > 0 ? 2 * k : 0, (Math.min(b.value, max) / max) * area);
      const x = i * slot + slot / 2;
      const fill = b.accent ? accent(target) : neutral;
      return [
        `<div style="position:absolute;left:${px(x - barW / 2)};top:${px(top + area - bh)};width:${px(barW)};height:${px(bh)};background:${fill};border-radius:${px(6 * k)} ${px(6 * k)} 0 0"></div>`,
        `<div style="position:absolute;left:${px(x - slot / 2)};width:${px(slot)};top:${px(top + area - bh - valueSize * 1.45)};text-align:center;font-family:${DM_SANS};font-weight:600;font-size:${px(valueSize)};line-height:1.2;color:${fg(target)};font-variant-numeric:tabular-nums;white-space:nowrap">${esc(numberLabel(b.value) + unit)}</div>`,
        `<div style="position:absolute;left:${px(x - slot / 2)};width:${px(slot)};top:${px(top + area + labelSize * 0.6)};text-align:center;font-family:${DM_SANS};font-weight:500;font-size:${px(labelSize)};line-height:1.2;color:${neutral};white-space:nowrap;overflow:hidden;text-overflow:ellipsis">${esc(b.label)}</div>`,
      ].join("");
    })
    .join("");
  const axis = `<div style="position:absolute;left:0;right:0;top:${px(top + area)};height:${px(Math.max(1, 2 * k))};background:${neutral};opacity:0.5"></div>`;
  return `<div style="position:absolute;inset:0">${columns}${axis}</div>`;
}

// ---- terminal ----

export interface TerminalLine {
  readonly text: string;
  readonly kind: "input" | "output";
  /** The shown part (input lines type in). */
  readonly shown: string;
  /** The cursor is at the end of this line. */
  readonly cursor: boolean;
}

/**
 * What a terminal shows at `local` frames: input lines type in at `speed` characters a second
 * from `typeFrom` (one after another, a quarter second apart), output lines appear at once a tenth
 * of a second after the line before; a line's `at` holds it until that frame. The cursor sits at the
 * end of the line being typed, else on a fresh prompt line once everything is shown (`idle`).
 */
export function terminalState(props: { readonly [key: string]: unknown }, local: number, fps: number): { lines: Array<TerminalLine>; idle: boolean } {
  const speed = typeof props.speed === "number" && props.speed > 0 ? props.speed : 30;
  const list = Array.isArray(props.lines) ? props.lines.filter((l): l is { text: string; kind?: string; at?: number } => !!l && typeof l === "object" && typeof (l as { text?: unknown }).text === "string") : [];
  let clock = typeof props.typeFrom === "number" ? Math.max(0, props.typeFrom) : 0;
  const lines: Array<TerminalLine> = [];
  let typing = false;
  let waitingFor: string | undefined;
  for (const l of list) {
    if (typeof l.at === "number") clock = Math.max(clock, l.at);
    if (local < clock) {
      waitingFor = l.kind === "input" ? "input" : "output";
      break;
    }
    if (l.kind === "input") {
      const chars = [...l.text];
      const frames = Math.ceil((chars.length * fps) / speed);
      const n = Math.min(chars.length, Math.floor(((local - clock) * speed) / fps));
      const done = n >= chars.length;
      lines.push({ text: l.text, kind: "input", shown: chars.slice(0, n).join(""), cursor: !done });
      clock += frames + Math.round(fps * 0.25);
      if (!done) {
        typing = true;
        break;
      }
    } else {
      lines.push({ text: l.text, kind: "output", shown: l.text, cursor: false });
      clock += Math.max(1, Math.round(fps * 0.1));
    }
  }
  // Idle (a fresh prompt with the cursor): everything shown, or waiting to type the next input line.
  const all = lines.length === list.length && !typing;
  return { lines, idle: all || waitingFor === "input" };
}

export function drawTerminal({ props, w, h, local, fps, target }: Draw) {
  const k = Math.min(w / 1000, h / 620);
  const bar = 52 * k;
  const prompt = typeof props.prompt === "string" ? props.prompt : "$";
  const state = terminalState(props, local, fps);
  const total = (Array.isArray(props.lines) ? props.lines.length : 0) + 1;
  const size = Math.max(4, Math.min(26 * k, (h - bar - 56 * k) / (total * 1.5)));
  const cursor = `<span style="display:inline-block;width:0.6em;height:1.15em;vertical-align:-0.2em;background:#E6E6E6"></span>`;
  const promptHtml = `<span style="color:${accent(target)}">${esc(prompt)}</span> `;
  const rows = state.lines.map((l) =>
    l.kind === "input"
      ? `<div style="white-space:pre-wrap;color:#F2F1EE">${promptHtml}${esc(l.shown)}${l.cursor ? cursor : ""}</div>`
      : `<div style="white-space:pre-wrap;color:#B8B8BE">${esc(l.shown) || " "}</div>`,
  );
  if (state.idle) rows.push(`<div style="white-space:pre;color:#F2F1EE">${promptHtml}${cursor}</div>`);
  const title = str(props.title);
  const dots = [0, 1, 2].map((i) => `<i style="display:block;width:${px(14 * k)};height:${px(14 * k)};border-radius:50%;background:#3A3A3F;margin-right:${px(9 * k)}"></i>`).join("");
  return (
    `<div style="position:absolute;inset:0;border-radius:${px(18 * k)};background:#141416;border:${px(Math.max(1, 1.5 * k))} solid #2C2C30;overflow:hidden">` +
    `<div style="position:relative;height:${px(bar)};display:flex;align-items:center;padding:0 ${px(20 * k)};background:#1D1D20;border-bottom:${px(Math.max(1, k))} solid #0E0E10">${dots}` +
    `<span style="position:absolute;left:0;right:0;text-align:center;font-family:${DM_SANS};font-weight:500;font-size:${px(21 * k)};color:#8C8C91;white-space:nowrap">${esc(title)}</span></div>` +
    `<div style="padding:${px(26 * k)} ${px(30 * k)};font-family:${DM_MONO};font-weight:400;font-size:${px(size)};line-height:1.5">${rows.join("")}</div></div>`
  );
}

// ---- agent log (D95) ----

const numOr = (v: unknown, fallback: number) => (typeof v === "number" && Number.isFinite(v) ? v : fallback);
const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));
/** "2:41": minutes and whole seconds. */
const clockLabel = (s: number) => {
  const r = Math.round(s);
  return `${Math.floor(r / 60)}:${String(r % 60).padStart(2, "0")}`;
};
/** DM Sans has no tabular figures: each character in a cell of fixed width, so a changing number never moves its neighbours. */
const cells = (text: string) => [...text].map((c) => `<span style="display:inline-block;text-align:center;width:${/[0-9]/.test(c) ? "0.57em" : "0.27em"}">${esc(c)}</span>`).join("");

/**
 * What an agent log shows at `local` frames: the source second (srcAt + seconds × speed), the run's
 * clock (from runStart, stopping at durationMs), the token count reached, the lines in, and whether
 * the run is done.
 */
export function agentLogState(props: { readonly [key: string]: unknown }, local: number, fps: number): { src: number; clock: number; tokens: number; lines: number; done: boolean } {
  const speed = numOr(props.speed, 1) > 0 ? numOr(props.speed, 1) : 1;
  const src = Math.max(0, numOr(props.srcAt, 0)) + (Math.max(0, local) / fps) * speed;
  const runStart = numOr(props.runStart, 0);
  const length = Math.max(0, numOr(props.durationMs, Infinity)) / 1000;
  const clock = clamp(src - runStart, 0, length);
  let tokens = 0;
  for (const k of Array.isArray(props.tokens) ? props.tokens : []) {
    const t = k && typeof k === "object" ? (k as { t?: unknown; cum?: unknown }) : {};
    if (typeof t.t === "number" && typeof t.cum === "number" && t.t <= src) tokens = t.cum;
  }
  const lines = (Array.isArray(props.lines) ? props.lines : []).filter((l) => !!l && typeof l === "object" && typeof (l as { t?: unknown }).t === "number" && (l as { t: number }).t <= src).length;
  return { src, clock, tokens, lines, done: Number.isFinite(length) && src >= runStart + length };
}

export function drawAgentLog({ props, w, h, local, fps, target }: Draw) {
  const k = Math.min(w / 470, h / 739);
  const size = numOr(props.size, 19) * k;
  const state = agentLogState(props, local, fps);
  const title = str(props.title);
  const list = (Array.isArray(props.lines) ? props.lines : []).filter((l): l is { t: number; tool?: string; text: string } => !!l && typeof l === "object" && typeof (l as { t?: unknown }).t === "number" && typeof (l as { text?: unknown }).text === "string");
  // Only the lines that can be on screen: the log is bottom-anchored and clipped above, and a line
  // takes at least one row (size × 1.45 + its 12 px gap), so the last ceil(area ÷ row) + 1 are all
  // that can show (a long log stays a few KB of HTML a frame).
  const area = Math.max(0, h - 256 * k);
  const fit = Math.ceil(area / (size * 1.45 + 12 * k)) + 1;
  const reached = list.filter((l) => l.t <= state.src);
  const shown = reached.slice(Math.max(0, reached.length - fit));
  const rows = shown
    .map((l) =>
      typeof l.tool === "string" && l.tool
        ? `<div style="margin-top:${px(12 * k)}"><span style="font-family:${DM_MONO};font-size:${px(size - 2 * k)};color:#7D7D85">${esc(l.tool)}</span> <span style="color:#B9B9C0">${esc(l.text)}</span></div>`
        : `<div style="margin-top:${px(12 * k)};color:#ECECEE">${esc(l.text)}</div>`,
    )
    .join("");
  const dots = ["#FF5F57", "#FEBC2E", "#28C840"].map((c) => `<i style="display:block;width:${px(12 * k)};height:${px(12 * k)};border-radius:50%;background:${c}"></i>`).join("");
  const label = str(props.tokenLabel)
    .split("\n")
    .map((part) => esc(part))
    .join("<br>");
  return (
    `<div style="position:absolute;inset:0;display:flex;flex-direction:column;border-radius:${px(18 * k)};background:rgb(16 17 20 / 94%);box-shadow:0 0 0 ${px(Math.max(1, k))} rgb(255 255 255 / 8%);overflow:hidden;color:#ECECEE;font-family:${DM_SANS};font-weight:400;font-size:${px(size)};line-height:1.45">` +
    `<div style="height:${px(44 * k)};flex:none;display:flex;align-items:center;gap:${px(8 * k)};padding:0 ${px(16 * k)};background:#1D1E22;border-bottom:${px(Math.max(1, k))} solid #000">${dots}` +
    `<span style="flex:1;text-align:center;margin-right:${px(52 * k)};white-space:nowrap;overflow:hidden;font-family:${DM_MONO};font-weight:500;font-size:${px(15 * k)};color:#8D8D94">${esc(title)}</span></div>` +
    `<div style="padding:${px(26 * k)} ${px(30 * k)} ${px(20 * k)};flex:none;border-bottom:${px(Math.max(1, k))} solid #26262B">` +
    `<div style="font-weight:600;font-size:${px(40 * k)};line-height:1;letter-spacing:-0.01em">${esc(str(props.model))}</div>` +
    `<div style="display:flex;align-items:baseline;gap:${px(28 * k)};margin-top:${px(20 * k)}">` +
    `<span style="font-weight:500;font-size:${px(64 * k)};line-height:1;white-space:nowrap">${cells(clockLabel(state.clock))}</span>` +
    `<span style="color:#8D8D94;font-size:${px(22 * k)};line-height:1.2"><span style="color:#ECECEE">${cells(Math.round(state.tokens).toLocaleString("en-US"))}</span><br>${label}</span></div>` +
    `<div style="margin-top:${px(14 * k)};font-weight:500;font-size:${px(22 * k)};line-height:1.25;color:${accent(target)};height:${px(28 * k)};white-space:nowrap">${state.done ? esc(str(props.doneLabel)) : ""}</div></div>` +
    `<div style="position:relative;flex:1;overflow:hidden;padding:0 ${px(30 * k)};-webkit-mask-image:linear-gradient(to bottom,transparent 0,#000 ${px(70 * k)});mask-image:linear-gradient(to bottom,transparent 0,#000 ${px(70 * k)})">` +
    `<div style="position:absolute;left:${px(30 * k)};right:${px(30 * k)};bottom:${px(22 * k)}">${rows}</div></div></div>`
  );
}

// ---- compare table (D95) ----

interface CompareRow {
  readonly label: string;
  readonly values: ReadonlyArray<number>;
  readonly text: ReadonlyArray<string>;
}
const compareRows = (v: unknown): Array<CompareRow> =>
  (Array.isArray(v) ? v : [])
    .filter((r): r is { label?: unknown; values?: unknown; text?: unknown } => !!r && typeof r === "object")
    .map((r) => {
      const values = (Array.isArray(r.values) ? r.values : []).map((x) => (typeof x === "number" && Number.isFinite(x) ? Math.max(0, x) : 0));
      const text = Array.isArray(r.text) ? r.text.map((x) => str(x)) : [];
      return { label: str(r.label), values, text };
    });

export function drawCompareTable({ props, w, h, local, fps }: Draw) {
  const k = Math.min(w / 1400, h / 600);
  const size = numOr(props.size, 24) * k;
  const names = (Array.isArray(props.names) ? props.names : []).map((n) => str(n));
  const rows = compareRows(props.rows);
  const t = Math.max(0, local) / fps;
  const start = numOr(props.start, 0.35);
  const step = numOr(props.step, 0.55);
  const grow = Math.max(0, numOr(props.grow, 0.7));
  const lin = (x: number, a: number, b: number) => (b <= a ? (x >= b ? 1 : 0) : clamp((x - a) / (b - a), 0, 1));
  const out = EASES.out;
  const body = rows
    .map((r, i) => {
      const t0 = start + i * step;
      const kr = out(lin(t, t0, t0 + 0.35));
      const kb = out(lin(t, t0 + 0.15, t0 + 0.15 + grow));
      const kv = out(lin(t, t0 + 0.15 + grow * 0.6, t0 + 0.15 + grow));
      const max = Math.max(...r.values, 0) || 1;
      const bars = names
        .map((n, j) => {
          const v = r.values[j] ?? 0;
          const shown = r.text[j] ?? numberLabel(v);
          return (
            `<div style="display:flex;align-items:center;gap:${px(20 * k)}">` +
            `<span style="width:${px(130 * k)};flex:none;color:#A9A9B0;white-space:nowrap">${esc(n)}</span>` +
            `<span style="flex:1;height:${px(14 * k)};border-radius:${px(7 * k)};background:rgb(255 255 255 / 6%);overflow:hidden"><i style="display:block;height:100%;width:${num((v / max) * 100 * kb)}%;border-radius:${px(7 * k)};background:#ECECEE"></i></span>` +
            `<b style="width:${px(170 * k)};flex:none;text-align:right;font-weight:500;font-size:${px(40 * k)};line-height:1;white-space:nowrap;opacity:${num(kv)}">${esc(shown)}</b></div>`
          );
        })
        .join("");
      return (
        `<div style="display:flex;align-items:center;gap:${px(40 * k)};padding:${px(22 * k)} 0;${i ? `border-top:${px(Math.max(1, k))} solid rgb(255 255 255 / 12%);` : ""}opacity:${num(kr)};transform:translateY(${px((1 - kr) * 18 * k)})">` +
        `<div style="width:${px(300 * k)};flex:none;color:#A9A9B0;font-size:${px(28 * k)}">${esc(r.label)}</div>` +
        `<div style="flex:1;display:flex;flex-direction:column;gap:${px(14 * k)}">${bars}</div></div>`
      );
    })
    .join("");
  return `<div style="position:absolute;inset:0;box-sizing:border-box;padding:${px(34 * k)} ${px(56 * k)};border-radius:${px(22 * k)};background:rgb(16 17 20 / 94%);box-shadow:0 0 0 ${px(Math.max(1, k))} rgb(255 255 255 / 8%);color:#ECECEE;font-family:${DM_SANS};font-weight:400;font-size:${px(size)};line-height:1.2">${body}</div>`;
}
