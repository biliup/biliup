import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The Inspector for several selected items (Fable QA round 2, F2, D147): a header with how many
// and of what, then the controls they share. Every change goes to each selected item it applies to
// as one edit (timeline/multi.ts planForAll): the filter's strength and removal, Adjust, effects,
// speed, volume and opacity. A value shows when all of them have it, else the first one's.
import { SPEED_PRESETS, SPEED_RANGE, speedLabel } from "@/app/lib/editor/core/timeline/arrange";
import { MAX_DB, SILENCE_DB } from "@/app/lib/editor/core/audio/audio";
import { fxSkips } from "@/app/lib/editor/core/visual/fx";
import { ADJUST, ADJUST_INFO, adjustOf, effectInfo, effectParams, LOOK, lookInfo, planAdjust, planEffect, planRemoveEffect, planSetLook, type AdjustKey, type LookDef } from "@/app/lib/editor/core/visual/fx";
import type { Item } from "@/app/lib/editor/core/timeline/types";
import { AtSign, Layers, RotateCcw, X } from "lucide-react";

import { hasSound, planGain } from "@/app/ui/editor/audio/sound";
import { usePlayback } from "@/app/ui/editor/preview/playback";
import { type ProjectModel } from "@/app/lib/editor/state/project";
import { actions } from "@/app/lib/editor/state/store";
import { Tooltip } from "@/app/ui/editor/controls/Tooltip";
import { clipState, isBoxed, planClipBox } from "@/app/ui/editor/timeline/clip";
import { findItem, planSpeed, type Plan } from "@/app/ui/editor/timeline/edit";
import { planForAll } from "@/app/ui/editor/timeline/multi";
import { NumInput, Prop, Section } from "@/app/ui/editor/inspector/controls";
import { Live } from "@/app/ui/editor/inspector/effects";
import { Group, send, type PlanResult } from "@/app/ui/editor/inspector/fields";
import { hasSpeed } from "@/app/ui/editor/inspector/speed";
import { useProjectEdit } from '../workspace/projectEdit';
import { editorGestureOrigin } from '../workspace/pointerGesture';

const NOUNS: Readonly<Record<Item['type'], readonly [string, string]>> = {
  clip: ["{0} clip", "{0} clips"],
  component: ["{0} element", "{0} elements"],
  graphic: ["{0} graphic", "{0} graphics"],
  text: ["{0} text item", "{0} text items"],
  captions: ["{0} caption track", "{0} caption tracks"],
  audio: ["{0} sound", "{0} sounds"],
  danmaku: ["{0} danmaku track", "{0} danmaku tracks"],
};

/** "3 clips, 1 text". */
export function selectionWords(items: ReadonlyArray<Item>): string {
  const counts = new Map<Item['type'], number>();
  for (const i of items) counts.set(i.type, (counts.get(i.type) ?? 0) + 1);
  return [...counts].map(([type, n]) => editorText(NOUNS[type][n === 1 ? 0 : 1], {0:n})).join(", ");
}

const round = (v: number, step: number) => Math.round(v / step) * step;

export function MultiInspector({ project, ids, disabled }: { project: ProjectModel; ids: ReadonlyArray<string>; disabled: boolean }) {
  useEditorLanguage();
  const edit = useProjectEdit();
  const linkOverlays = usePlayback(state => state.linkOverlays);
  const frame = usePlayback(state => Math.round(state.frame));
  const placed = ids.map((id) => findItem(project, id)).filter((p) => p !== undefined);
  const items = placed.map((p) => p.item);
  const lockedIds = new Set(placed.filter((p) => p.track.locked || p.item.locked).map((p) => p.item.id));
  const editable = items.filter((i) => !lockedIds.has(i.id));
  const fxClips = editable.filter((i) => {
    const media = i.type === "clip" && i.media ? project.media.find((m) => m.id === i.media) : undefined;
    return !!media && media.kind !== "audio" && !fxSkips(media);
  });
  const speedItems = editable.filter((i) => hasSpeed(project, i));
  const soundItems = editable.filter((i) => hasSound(project, i));
  const boxItems = editable.filter((i) => isBoxed(project, i));
  /** One edit over `targets`; `plan` per item, `label` for the joined edit. */
  const all = (targets: ReadonlyArray<Item>, plan: (p: ProjectModel, id: string) => Plan | { readonly problem: string }, label: (n: number) => string) => (p: ProjectModel): PlanResult =>
    planForAll(
      p,
      targets.map((i) => i.id),
      plan,
      label,
    );
  const shared = <T,>(list: ReadonlyArray<Item>, of: (i: Item) => T): { value: T; same: boolean } => {
    const values = list.map(of);
    return { value: values[0]!, same: values.every((v) => JSON.stringify(v) === JSON.stringify(values[0])) };
  };

  // Filter: the same look on every clip (its strength editable), or mixed.
  const lookOf = (i: Item) => ((i.effects ?? []).find((e) => e.module === LOOK)?.params ?? undefined) as { look?: string; intensity?: number; label?: string; custom?: LookDef } | undefined;
  const looks = fxClips.length ? shared(fxClips, (i) => {const look=lookOf(i);return look?{look:look.look,custom:look.custom}:null}) : undefined;
  const anyLook = fxClips.some((i) => lookOf(i));
  const firstLook = fxClips.map(lookOf).find((l) => l);
  const lookName = firstLook?.label ?? lookInfo(firstLook?.look)?.name ?? firstLook?.look;
  const adjust = fxClips.length ? adjustOf((fxClips[0]!.effects ?? []).find((e) => e.module === ADJUST)?.params) : undefined;
  const adjusted = fxClips.some((i) => ADJUST_INFO.some((a) => adjustOf((i.effects ?? []).find((e) => e.module === ADJUST)?.params)[a.key] !== 0));
  const shownAdjust = (key: AdjustKey) => (key === "exposure" ? round(adjust![key], 0.05) : Math.round(adjust![key] * 100));
  // Effects any of them has: settings from the first that has it, changes go to those that have it.
  const modules = [...new Set(fxClips.flatMap((i) => (i.effects ?? []).map((e) => e.module)))].filter((m) => m !== LOOK && m !== ADJUST && effectInfo(m));
  const speed = speedItems.length ? shared(speedItems, (i) => i.speed ?? 1) : undefined;
  const gain = soundItems.length ? shared(soundItems, (i) => i.gainDb ?? 0) : undefined;
  const opacityOf = (i: Item) => Math.round((clipState(project, i.id, frame)?.box.opacity ?? 1) * 100);
  const opacity = boxItems.length ? shared(boxItems, opacityOf) : undefined;
  const n = (list: ReadonlyArray<Item>) => editorText(list.length === 1 ? '{0} item' : '{0} items', {0:list.length});
  const speeds = (p: ProjectModel, value: number) => all(speedItems, (current, id) => planSpeed(current, id, value, undefined, linkOverlays), count => editorText('Set {0} items to {1} speed', {0: count, 1: speedLabel(value)}))(p);

  return (
    <div data-testid="inspector-multi" data-count={items.length}>
      <div className="insp-title">
        <Layers />
        <span className="ellipsis grow" data-testid="inspector-multi-title">
          {editorText("{0} items selected", {0:items.length})}</span>
        {/* D264 (#77): every selected item as a chip in the Agent tab's composer. */}
        <Tooltip label={editorText("Add these {0} items to the agent chat", { 0: items.length })} shortcut="addToChat">
          <button className="small-btn insp-chat" data-testid="multi-add-to-chat" onClick={() => items.forEach((i) => actions.addItemToChat(i.id))}>
            <AtSign />
            {editorText("Add to chat")}</button>
        </Tooltip>
      </div>
      <div className="insp-sub" data-testid="inspector-multi-sub">
        {selectionWords(items)}
        {lockedIds.size ? editorText(", {0} locked", { 0: lockedIds.size }) : ""}
      </div>
      {fxClips.length > 0 && (
        <>
          <Section
            title={editorText("Filter")}
            testId="multi-filter"
            remember="filter"
            action={
              anyLook ? (
                <Tooltip label={editorText("Remove the filter from all")}>
                  <button className="icon-btn" data-testid="multi-filter-remove" disabled={disabled} onClick={() => send(all(fxClips.filter((i) => lookOf(i)), (p, id) => planSetLook(p, id, undefined), (k) => editorText('Remove filters from {0} clips',{0:k})))}>
                    <X />
                  </button>
                </Tooltip>
              ) : undefined
            }
          >
            {looks?.same && firstLook?.look ? (
              <Prop label={lookName ?? editorText("Filter")} testId="multi-filter-intensity">
                <Live
                  project={project}
                  value={Math.round((firstLook.intensity ?? 1) * 100)}
                  min={0}
                  max={100}
                  step={1}
                  unit="%"
                  testId="multi-filter-amount"
                  disabled={disabled}
                  plan={(p, v) => all(fxClips, (q, id) => {
                    const item=findItem(q,id)?.item,look=item&&lookOf(item);
                    if(!look?.look)return {problem:'The filter changed. Select the clips again.'};
                    return planSetLook(q,id,look.look,{intensity:v/100,...(look.custom?{custom:look.custom,label:look.label}:{})});
                  },(k)=>editorText('Set filter intensity on {0} clips to {1}%',{0:k,1:v}))(p)}
                />
              </Prop>
            ) : (
              <div className="muted small" data-testid="multi-filter-state">
                {anyLook ? editorText("Different filters") : editorText("No filter")}
              </div>
            )}
          </Section>
          <Section
            title={editorText("Adjust")}
            testId="multi-adjust"
            remember="adjust"
            action={
              adjusted ? (
                <Tooltip label={editorText("Reset all")}>
                  <button className="icon-btn" data-testid="multi-adjust-reset" disabled={disabled} onClick={() => send(all(fxClips, (p, id) => planAdjust(p, id, "reset"), (k) => editorText('Reset adjustments on {0} clips',{0:k})))}>
                    <RotateCcw />
                  </button>
                </Tooltip>
              ) : undefined
            }
          >
            {ADJUST_INFO.map((a) => (
              <Prop key={a.key} label={a.name} testId={`multi-adjust-${a.key}`}>
                <Live
                  project={project}
                  value={shownAdjust(a.key)}
                  min={a.key === "exposure" ? a.min : -100}
                  max={a.key === "exposure" ? a.max : 100}
                  step={a.key === "exposure" ? 0.05 : 1}
                  {...(a.key === "exposure" ? { unit: "EV" } : {})}
                  testId={`multi-adjust-${a.key}-value`}
                  disabled={disabled}
                  plan={(p, v) => all(fxClips, (q, id) => planAdjust(q, id, { [a.key]: a.key === "exposure" ? v : v / 100 }), (k) => editorText('Set {0} on {1} clips',{0:editorText(a.name),1:k}))(p)}
                />
              </Prop>
            ))}
          </Section>
          {modules.length > 0 && (
            <Section title={editorText("Effects")} testId="multi-effects" remember="effects">
              {modules.map((module) => {
                const info = effectInfo(module)!;
                const having = fxClips.filter((i) => (i.effects ?? []).some((e) => e.module === module));
                const params = effectParams(module, (having[0]!.effects ?? []).find((e) => e.module === module)?.params);
                return (
                  <div key={module} className="fx-entry" data-testid="multi-effect" data-module={module} data-count={having.length}>
                    <div className="fx-entry-head">
                      <span>
                        {editorText(info.name)} <span className="muted small">{having.length === fxClips.length ? editorText("all") : editorText("{0} of {1}", { 0: having.length, 1: fxClips.length })}</span>
                      </span>
                      <Tooltip label={editorText("Remove {0} from all", { 0: editorText(info.name) })}>
                        <button className="icon-btn" data-testid="multi-effect-remove" disabled={disabled} onClick={() => send(all(having, (p, id) => planRemoveEffect(p, id, module), (k) => editorText('Remove {0} from {1} clips',{0:editorText(info.name),1:k})))}>
                          <X />
                        </button>
                      </Tooltip>
                    </div>
                    {info.params.map((d) => (
                      <Prop key={d.key} label={d.name} testId={`multi-effect-${d.key}`}>
                        <Live project={project} value={round(params[d.key]!, d.step)} min={d.min} max={d.max} step={d.step} {...(d.unit ? { unit: d.unit } : {})} testId={`multi-effect-${d.key}-value`} disabled={disabled} plan={(p, v) => all(having, (q, id) => planEffect(q, id, module, { [d.key]: v }), (k) => editorText('Set {0} / {1} on {2} clips',{0:editorText(info.name),1:editorText(d.name),2:k}))(p)} />
                      </Prop>
                    ))}
                  </div>
                );
              })}
            </Section>
          )}
        </>
      )}
      {speed && (
        <Group label={editorText("Speed ({0})", { 0: n(speedItems) })}>
          <div className="speed-row" role="group" aria-label={editorText("Speed")} data-testid="multi-speed">
            {SPEED_PRESETS.map((s) => (
              <button key={s} className={`small-btn ${speed.same && Math.abs(s - speed.value) < 1e-6 ? "active" : ""}`} data-testid={`multi-speed-${s}`} disabled={disabled} onClick={() => { void edit.commit(p => speeds(p, s), editorGestureOrigin()); }}>
                {speedLabel(s)}
              </button>
            ))}
          </div>
          <div className="field-row">
            <NumInput label="×" ariaLabel={editorText("Speed")} testId="multi-speed-custom" value={speed.same ? Math.round(speed.value * 1000) / 1000 : undefined} placeholder={editorText("Mixed")} step={0.05} min={SPEED_RANGE.min} max={SPEED_RANGE.max} disabled={disabled} onPreview={v => edit.preview(project, speeds(project, v))} onCancel={edit.cancel} onCommit={(v, origin) => v !== undefined && void edit.commit(p => speeds(p, v), origin)} />
          </div>
        </Group>
      )}
      {gain && (
        <Group label={editorText("Volume ({0})", { 0: n(soundItems) })}>
          <Prop label={editorText("dB")} testId="multi-volume">
            <Live project={project} value={gain.value} min={SILENCE_DB} max={MAX_DB} step={0.5} unit="dB" testId="multi-volume-value" disabled={disabled} plan={(p, v) => all(soundItems, (q, id) => planGain(q, id, v), (k) => editorText('Set volume of {0} items to {1} dB',{0:k,1:v}))(p)} />
          </Prop>
        </Group>
      )}
      {opacity && (
        <Group label={editorText("Opacity ({0})", { 0: n(boxItems) })}>
          <Prop label="%" testId="multi-opacity">
            <Live project={project} value={opacity.value} min={0} max={100} step={1} unit="%" testId="multi-opacity-value" disabled={disabled} plan={(p, v) => all(boxItems, (q, id) => planClipBox(q, id, { opacity: v / 100 }, frame), (k) => editorText('Set opacity of {0} items to {1}%',{0:k,1:v}))(p)} />
          </Prop>
        </Group>
      )}
    </div>
  );
}
