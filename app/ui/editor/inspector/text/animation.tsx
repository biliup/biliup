import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The text Inspector's Animation tab (D81, laid out per D129; Loop per D163): In, Out and Loop, each a move and its
// length, plus the move's own options (word by word's style, zoom's scale and ease). One edit per
// change, to every selected text.
import type { Item } from "@/app/lib/editor/core/timeline/types";
import { LOOP_DEFAULTS, WIGGLE_MOVE, WORD_STYLES, ZOOM_DEFAULTS } from "@/app/lib/editor/core/text/looks";

import { type ProjectModel } from "@/app/lib/editor/state/project";
import { LOOPS, MOVES, moveLabel, planAnimationChange, type AnimationChange, type AnimationSlot } from "@/app/ui/editor/preview/text";
import { NumInput, Prop, Section, Segmented } from "@/app/ui/editor/inspector/controls";
import { useTextEdit } from "@/app/ui/editor/inspector/text/edit";
import { EASE_NAMES } from "@/app/ui/editor/timeline/clip";
import { editorGestureOrigin, type GestureOrigin } from "@/app/ui/editor/workspace/pointerGesture";

// These moves consume params.ease in the shared preview/export renderer.
const EASE_MOVES = new Set(["move.rise", "move.slide-left", "move.wipe", "move.zoom", "move.bounce", "move.blur", "move.scale", "move.pop"]);

function Slot({ project, item, slot, disabled }: { project: ProjectModel; item: Item; slot: AnimationSlot; disabled: boolean }) {
  useEditorLanguage();
  const edit = useTextEdit(project, item);
  const fps = project.canvas.fps;
  const current = item.animation?.[slot];
  const animationModule = current?.module ?? "";
  const frames = current?.duration ?? Math.min(item.duration, slot === "loop" ? fps : Math.round(fps / 3));
  const list = slot === "loop" ? LOOPS : MOVES;
  const params = current?.params ?? {};
  const set = (change: AnimationChange, origin = editorGestureOrigin()) => edit.commit((p, id) => planAnimationChange(p, id, slot, change), origin);
  const preview = (change: AnimationChange) => edit.preview((p, id) => planAnimationChange(p, id, slot, change));
  const param = (key: string, value: unknown): AnimationChange => ({ type: "param", module: animationModule, key, value });
  const setParam = (key: string, value: unknown, origin?: GestureOrigin) => set(param(key, value), origin);
  const duration = (seconds: number): AnimationChange => ({ type: "duration", frames: Math.round(seconds * fps) });
  return (
    <Section title={slot === "in" ? editorText("In") : slot === "out" ? editorText("Out") : editorText("Loop")} testId={`insp-sec-anim-${slot}`}>
      <Prop label={editorText("Type")}>
        <select className="pselect" data-testid={`insp-anim-${slot}`} value={animationModule} disabled={disabled} onChange={(e) => set({ type: "module", module: e.target.value })}>
          {list.map((m) => (
            <option key={m.id} value={m.id}>
              {m.id ? editorText(moveLabel(m.id, slot)) : editorText(m.label)}
            </option>
          ))}
          {animationModule && !list.some((m) => m.id === animationModule) && <option value={animationModule}>{animationModule}</option>}
        </select>
      </Prop>
      <Prop label={slot === "loop" ? editorText("Every") : editorText("Duration")}>
        <NumInput testId={`insp-anim-${slot}-dur`} unit="s" value={Number((frames / fps).toFixed(3))} step={0.1} min={1 / fps} max={item.duration / fps} disabled={disabled || !animationModule} onPreview={v => preview(duration(v))} onCancel={edit.cancel} onCommit={(v, origin) => v !== undefined && set(duration(v), origin)} />
      </Prop>
      {slot === "loop" && animationModule && (
        <Prop label={animationModule === WIGGLE_MOVE ? editorText("Tilt") : editorText("Grow")}>
          <NumInput
            testId="insp-anim-loop-amount"
            unit={animationModule === WIGGLE_MOVE ? "°" : "%"}
            value={animationModule === WIGGLE_MOVE ? (typeof params.amount === "number" ? params.amount : LOOP_DEFAULTS[animationModule]!) : Math.round((typeof params.amount === "number" ? params.amount : LOOP_DEFAULTS[animationModule] ?? 0.08) * 100)}
            min={animationModule === WIGGLE_MOVE ? 0.5 : 1}
            max={animationModule === WIGGLE_MOVE ? 30 : 50}
            step={animationModule === WIGGLE_MOVE ? 0.5 : 1}
            disabled={disabled}
            onPreview={v => preview(param("amount", animationModule === WIGGLE_MOVE ? v : v / 100))}
            onCancel={edit.cancel}
            onCommit={(v, origin) => v !== undefined && setParam("amount", animationModule === WIGGLE_MOVE ? v : v / 100, origin)}
          />
        </Prop>
      )}
      {animationModule === "move.word-by-word" && (
        <Prop label={editorText("Words")}>
          <Segmented testId={`insp-anim-${slot}-word-style`} attr="style" options={WORD_STYLES.map((w) => [w, editorText(w[0]!.toUpperCase() + w.slice(1))] as const)} value={(params.style as string | undefined) ?? "pop"} disabled={disabled} onChange={(w) => setParam("style", w)} />
        </Prop>
      )}
      {animationModule === "move.zoom" && (
        <>
          <Prop label={slot === "in" ? editorText("From") : editorText("To")}>
            <NumInput testId={`insp-anim-${slot}-zoom-scale`} unit="%" value={Math.round((typeof params.scale === "number" ? params.scale : ZOOM_DEFAULTS.scale) * 100)} min={5} max={400} step={5} disabled={disabled} onPreview={v => preview(param("scale", v / 100))} onCancel={edit.cancel} onCommit={(v, origin) => v !== undefined && setParam("scale", v / 100, origin)} />
          </Prop>
          <label className="check-row">
            <input type="checkbox" checked={params.fade !== false} disabled={disabled} onChange={event => setParam("fade", event.target.checked)} />
            {editorText("Fade")}
          </label>
        </>
      )}
      {slot !== "loop" && EASE_MOVES.has(animationModule) && (
        <Prop label={editorText("Ease")}>
          <select className="pselect" aria-label={editorText("Ease")} data-testid={`insp-anim-${slot}-ease`} value={typeof params.ease === "string" ? params.ease : ""} disabled={disabled} onChange={event => setParam("ease", event.target.value || undefined)}>
            <option value="">{editorText("Default curve")}</option>
            {EASE_NAMES.map(([value, label]) => <option key={value} value={value}>{editorText(label)}</option>)}
          </select>
        </Prop>
      )}
    </Section>
  );
}

export function TextAnimationTab({ project, item, disabled }: { project: ProjectModel; item: Item; disabled: boolean }) {
  useEditorLanguage();
  return (
    <div className="pstack" data-testid="text-animation">
      <Slot project={project} item={item} slot="in" disabled={disabled} />
      <Slot project={project} item={item} slot="out" disabled={disabled} />
      <Slot project={project} item={item} slot="loop" disabled={disabled} />
    </div>
  );
}
