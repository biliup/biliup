import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The Inspector's clip box editor (D75): fit, position, size, keys.
import type { Item } from "@/app/lib/editor/core/timeline/types";
import { ChevronLeft, ChevronRight, Diamond, RotateCcw } from "lucide-react";
import type { Fit, KeyProp } from "@/app/lib/editor/core/visual/box";

import { playback, usePlayback } from "@/app/ui/editor/preview/playback";
import { clipState, EASE_NAMES, keyEase, keyFrames, keyState, nearbyKey, planClipBox, planHold, planKeyEase, planLoop, planResetBox, planToggleKey, type BoxPatch } from "@/app/ui/editor/timeline/clip";
import { type ProjectModel } from "@/app/lib/editor/state/project";
import { NumberField, Slider, Group } from "@/app/ui/editor/inspector/fields";
import {useProjectEdit} from '../workspace/projectEdit';
import {editorGestureOrigin, type GestureOrigin} from '../workspace/pointerGesture';
import { Tooltip } from "@/app/ui/editor/controls/Tooltip";

const FITS: ReadonlyArray<readonly [Fit, string, string]> = [
  ["contain", "Fit: contain", "All of the picture inside the box"],
  ["cover", "Fit: cover", "Fill the box; what doesn't fit is cut off"],
  ["fill", "Fit: fill", "Stretch the picture to the box"],
];

/** A keyframe diamond (D75): filled with a key at the playhead, outlined when the property has keys elsewhere. */
function KeyButton({ item, prop, local, inside, disabled, onToggle }: { item: Item; prop: KeyProp; local: number; inside: boolean; disabled: boolean; onToggle: () => void }) {
  useEditorLanguage();
  const state = keyState(item, prop, local);
  const property = editorText(prop === "scale" ? "Scale" : prop === "opacity" ? "Opacity" : prop.toUpperCase());
  const title = !inside ? editorText("Move the playhead over the clip to add a key") : editorText(state === "on" ? "Remove the {0} key at the playhead" : "Add a {0} key at the playhead", {0: property});
  return (
    <Tooltip label={title}>
      <button
        className={`icon-btn key-btn ${state}`}
        data-testid="insp-key"
        data-prop={prop}
        data-state={state}
        aria-label={title}
        aria-pressed={state === "on"}
        disabled={disabled || !inside}
        onClick={onToggle}
      >
        <Diamond />
      </button>
    </Tooltip>
  );
}

/**
 * A clip's box (D75): position and size in canvas pixels, fit, scale (zoom inside the box),
 * opacity, corner radius, keys for x / y / scale / opacity at the playhead, loop, and Reset. Each
 * change is one edit; sliders show a local draft while they move and commit once on release.
 */
export function ClipBoxEditor({ project, item, locked }: { project: ProjectModel; item: Item; locked: boolean }) {
  useEditorLanguage();
  const frame = usePlayback((s) => Math.round(s.frame));
  // A component (D76) has a box and opacity; fit, zoom, corners and loop are a clip's.
  const component = item.type === "component";
  const state = clipState(project, item.id, frame);
  const editing=useProjectEdit();
  if (!state) return null;
  const { box, local, inside } = state;
  const media = item.media ? project.media.find((m) => m.id === item.media) : undefined;
  const edit=(patch:BoxPatch,origin?:GestureOrigin)=>editing.commit(p=>planClipBox(p,item.id,patch,frame),origin);
  const key = (prop: KeyProp) => <KeyButton item={item} prop={prop} local={local} inside={inside} disabled={locked} onToggle={() => { void editing.commit(p => planToggleKey(p, item.id, prop, frame), editorGestureOrigin()); }} />;
  const radius = typeof item.transform?.radius === "number" ? item.transform.radius : 0;
  return (
    <div data-testid="clip-box-editor">
      <Group label={editorText("Position & size")}>
        <div className="field-row key-row">
          <NumberField label={editorText("X")} testId="insp-box-x" value={Math.round(box.x)} disabled={locked} onCommit={(v,origin) => v !== undefined && edit({ x: v },origin)} />
          {key("x")}
          <NumberField label={editorText("Y")} testId="insp-box-y" value={Math.round(box.y)} disabled={locked} onCommit={(v,origin) => v !== undefined && edit({ y: v },origin)} />
          {key("y")}
        </div>
        <div className="field-row">
          <NumberField label={editorText("W")} testId="insp-box-w" value={Math.round(box.w)} min={1} disabled={locked} onCommit={(v,origin) => v !== undefined && edit({ w: v },origin)} />
          <NumberField label={editorText("H")} testId="insp-box-h" value={Math.round(box.h)} min={1} disabled={locked} onCommit={(v,origin) => v !== undefined && edit({ h: v },origin)} />
        </div>
        {!component && (
          <div className="seg fit" role="group">
            {FITS.map(([fit, label, title]) => (
              <button key={fit} className={box.fit === fit ? "active" : ""} data-testid="insp-fit" data-fit={fit} title={editorText(title)} disabled={locked} onClick={() => edit({ fit }, editorGestureOrigin())}>
                {editorText(label)}
              </button>
            ))}
          </div>
        )}
      </Group>
      <Group label={component ? editorText("Opacity") : editorText("Scale and opacity")}>
        {!component && (
        <div className="slider-row key-row">
          <Slider
            testId="insp-scale"
            value={Math.round(box.scale * 100)}
            min={10}
            max={400}
            title={editorText("Zoom inside the box")}
            disabled={locked}
            onPreview={v=>editing.preview(project,planClipBox(project,item.id,{scale:v/100},frame))}
            onCancel={editing.cancel}
            onCommit={(v,origin)=>edit({scale:v/100},origin)}
          />
          <NumberField label={editorText("Scale %")} testId="insp-scale-num" value={Math.round(box.scale * 100)} min={5} max={2000} disabled={locked} onCommit={(v,origin) => v !== undefined && edit({ scale: v / 100 },origin)} />
          {key("scale")}
        </div>
        )}
        <div className="slider-row key-row">
          <Slider
            testId="insp-opacity"
            value={Math.round(box.opacity * 100)}
            min={0}
            max={100}
            disabled={locked}
            onPreview={v=>editing.preview(project,planClipBox(project,item.id,{opacity:v/100},frame))}
            onCancel={editing.cancel}
            onCommit={(v,origin)=>edit({opacity:v/100},origin)}
          />
          <NumberField label={editorText("Opacity %")} testId="insp-opacity-num" value={Math.round(box.opacity * 100)} min={0} max={100} disabled={locked} onCommit={(v,origin) => v !== undefined && edit({ opacity: v / 100 },origin)} />
          {key("opacity")}
        </div>
        {!component && (
          <div className="field-row">
            <NumberField label={editorText("Radius")} testId="insp-radius" value={radius} min={0} disabled={locked} onCommit={(v,origin) => edit({ radius: v ?? 0 },origin)} />
          </div>
        )}
      </Group>
      {keyFrames(item).length > 0 && <KeysGroup project={project} item={item} frame={frame} local={local} inside={inside} locked={locked} />}
      <div className="group">
        {media?.kind === "video" && (
          <label className="check-row">
            <input type="checkbox" data-testid="insp-loop" checked={item.loop === true} disabled={locked} onChange={(e) => {
                const on = e.target.checked;
                void editing.commit((p) => planLoop(p, item.id, on), editorGestureOrigin());
              }} />
            {editorText("Loop the footage when the clip is longer")}</label>
        )}
        {media?.kind === "video" && (
          <label className="check-row">
            <input type="checkbox" data-testid="insp-hold" checked={item.hold === true} disabled={locked || item.loop === true} onChange={(e) => {
                const on = e.target.checked;
                void editing.commit((p) => planHold(p, item.id, on), editorGestureOrigin());
              }} />
            {editorText("Hold the last frame when the clip is longer")}</label>
        )}
        <button className="btn" data-testid="insp-box-reset" disabled={locked || (!item.transform && !item.keys)} onClick={() => { void editing.commit((p) => planResetBox(p, item.id), editorGestureOrigin()); }}>
          <RotateCcw />
          {editorText("Reset position and size")}</button>
      </div>
    </div>
  );
}

/**
 * The clip's keys (F5, D148): previous and next key (also [ and ]), and with the playhead on a key
 * its ease, how it moves on to the next key (the same curve in the preview and the export).
 */
function KeysGroup({ project, item, frame, local, inside, locked }: { project: ProjectModel; item: Item; frame: number; local: number; inside: boolean; locked: boolean }) {
  useEditorLanguage();
  const editing = useProjectEdit();
  const frames = keyFrames(item);
  const ease = inside ? keyEase(item, local) : undefined;
  const previous = nearbyKey(project, frame, -1, [item.id]);
  const next = nearbyKey(project, frame, 1, [item.id]);
  const go = (to: number | undefined) => {
    if (to !== undefined) (playback.pause(), playback.seek(to));
  };
  return (
    <Group label={editorText("Keys ({0})", { 0: frames.length })}>
      <div className="field-row key-nav" data-testid="insp-keys" data-count={frames.length}>
        <Tooltip label={editorText("Previous key")} shortcut="prevKey">
          <button className="icon-btn" aria-label={editorText("Previous key")} data-testid="insp-key-prev" disabled={previous === undefined} onClick={() => go(previous)}>
            <ChevronLeft />
          </button>
        </Tooltip>
        <span className="mono small" data-testid="insp-key-at">{ease ? editorText("Key at {0} s", { 0: (local / project.canvas.fps).toFixed(2) }) : editorText(inside ? "Between keys" : "Outside the clip")}</span>
        <Tooltip label={editorText("Next key")} shortcut="nextKey">
          <button className="icon-btn" aria-label={editorText("Next key")} data-testid="insp-key-next" disabled={next === undefined} onClick={() => go(next)}>
            <ChevronRight />
          </button>
        </Tooltip>
      </div>
      {ease && (
        <select className="pselect" aria-label={editorText("Ease to the next key")} data-testid="insp-key-ease" data-ease={ease} value={ease} disabled={locked} onChange={event => {
          const chosen = EASE_NAMES.find(([value]) => value === event.target.value);
          if (chosen) void editing.commit(p => planKeyEase(p, item.id, local, chosen[0]), editorGestureOrigin());
        }}>
          {EASE_NAMES.map(([value, label]) => <option key={value} value={value}>{editorText(label)}</option>)}
        </select>
      )}
    </Group>
  );
}
