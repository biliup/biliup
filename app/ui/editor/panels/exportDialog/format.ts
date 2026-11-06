import { editorText } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import type { ExportQuality, ExportLoudness } from "@/app/lib/editor/contracts";
import type { ExportEncoder } from "@/app/lib/editor/core/export";
import { LOUDNESS_PRESETS, choiceOfTarget, targetWords } from "@/app/lib/editor/core/audio/effects";
import type { RangeKind } from "@/app/ui/editor/panels/exportRange";

export const ENCODER_NAMES: Readonly<Record<ExportEncoder, string>> = { hardware: "hardware", software: "software" };

export const QUALITIES: ReadonlyArray<{ readonly id: ExportQuality; readonly label: string; readonly hint: string }> = [
  { id: "draft", get label() { return editorText("Draft"); }, get hint() { return editorText("Fast, for a quick look"); } },
  { id: "standard", get label() { return editorText("Standard"); }, get hint() { return editorText("For sharing anywhere"); } },
  { id: "high", get label() { return editorText("High"); }, get hint() { return editorText("Full size, the best copy"); } },
];

type LoudnessCard = { readonly id: ExportLoudness; readonly label: string; readonly sub: string; readonly hint: string };
const lufsText = (lufs: number) => `${lufs < 0 ? "−" : ""}${Math.abs(lufs)} LUFS`;
/** The loudness cards (D87): the presets, the project's own target when it isn't one, and As mixed. */
export function loudnessCards(target: number): ReadonlyArray<LoudnessCard> {
  const hints: Readonly<Record<string, string>> = {
    normal: "Evens out the volume so it plays as loud as other videos on YouTube, Instagram and TikTok.",
    quiet: "A little quieter, as podcasts and Apple Music expect.",
    broadcast: "The TV level (EBU R128): quieter, with more room for peaks.",
  };
  return [
    ...LOUDNESS_PRESETS.map((p) => ({ id: p.id, label: p.name, sub: lufsText(p.lufs), hint: hints[p.id]! })),
    ...(choiceOfTarget(target) === "target" ? [{ id: "target" as const, get label() { return editorText("Project"); }, sub: lufsText(target), hint: `The project's own loudness target (${targetWords(target)}), set in the Mixer.` }] : []),
    { id: "off", get label() { return editorText("As mixed"); }, sub: "No change", get hint() { return editorText("Keeps your levels exactly as they sound in the editor."); } },
  ];
}

export const RANGE_KINDS: ReadonlyArray<readonly [RangeKind, string]> = [
  ["all", "Whole timeline"],
  ["marks", "In to Out"],
  ["selection", "Selected clips"],
  ["custom", "Custom"],
];

/** A download filename, ending in .mp4. Storage belongs to the export task. */
export const fileName = (name: string) => {
  const clean=name.replace(/[\\/:*?"<>|\u0000-\u001f\u007f-\u009f]/g,'-').trim().replace(/^\.+|\.+$/g,'');
  const stem=clean.replace(/\.mp4$/i,'').trim(),encoder=new TextEncoder();
  let result='',bytes=0;
  for(const char of stem){const size=encoder.encode(char).length;if(bytes+size>180)break;result+=char;bytes+=size;}
  return `${result||editorText('Untitled')}.mp4`;
};
export const baseName = (path: string) => path.slice(path.lastIndexOf("/") + 1);
export const fpsText = (fps: number) => `${Number(fps.toFixed(3))} fps`;

export const eta = (s: number | undefined) => (s === undefined ? "" : editorText('Remaining')+` ${Math.max(1,Math.round(s))}s`);

