import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// F30 (D174): Settings › App › New items: the transition length (the Transitions panel's Length
// chips, one setting), a photo's length and a text's length, remembered on this Mac.
import { NumberField } from "@/app/ui/editor/inspector/fields";
import { setTransitionSeconds, TRANSITION_LENGTHS, useTransitionSeconds } from "@/app/ui/editor/timeline/transitionLength";
import { DEFAULT_RANGE, setDefaultSeconds, useDefaultSeconds } from "@/app/ui/editor/settings/defaults";

export function DefaultLengths() {
  useEditorLanguage();
  const transition = useTransitionSeconds();
  const still = useDefaultSeconds("still");
  const text = useDefaultSeconds("text");
  return (
    <div className="group" data-testid="settings-defaults">
      <div className="group-label">{editorText("New items")}</div>
      <div className="kv-row">
        <span className="muted">{editorText("Transition length")}</span>
        <select data-testid="settings-transition-length" value={transition ?? ""} onChange={(e) => setTransitionSeconds(e.target.value === "" ? undefined : Number(e.target.value))}>
          {TRANSITION_LENGTHS.map((s) => (
            <option key={String(s)} value={s ?? ""}>
              {s === undefined ? editorText("Each transition's own") : editorText("{0} s", { 0: s })}
            </option>
          ))}
        </select>
      </div>
      <div className="kv-row">
        <span className="muted">{editorText("Photo length")}</span>
        <NumberField label={editorText("s")} testId="settings-still-seconds" value={still} step={0.5} min={DEFAULT_RANGE.min} max={DEFAULT_RANGE.max} onCommit={(v) => setDefaultSeconds("still", v)} />
      </div>
      <div className="kv-row">
        <span className="muted">{editorText("Text length")}</span>
        <NumberField label={editorText("s")} testId="settings-text-seconds" value={text} step={0.5} min={DEFAULT_RANGE.min} max={DEFAULT_RANGE.max} onCommit={(v) => setDefaultSeconds("text", v)} />
      </div>
    </div>
  );
}
