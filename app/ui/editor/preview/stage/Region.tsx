import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
import { editorEvents } from "@/app/lib/editor/environment";
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Region references (issue #118, like a comment pinned to part of the picture): with the transport's
// Area tool on, a drag over the frame draws a rectangle; on release it goes to the chat as a chip at
// the playhead's frame, or over In to Out when they're set. The chips still in the composer are drawn
// on the preview (a dashed box with their label) while the frame shown is in their time.
import { useContext, useEffect, useRef, useState } from "react";
import { momentLabel } from "@/app/lib/editor/core/text/mentions";

import { playback, usePlayback } from "@/app/ui/editor/preview/playback";
import { actions, useAppSelect } from "@/app/lib/editor/state/store";
import {currentEditorStore,EditorStoreContext,projectOf} from '@/app/lib/editor/state/store/core';
import {isTextComposition} from '@/app/lib/editor/keyboard';
import {beginEditorViewGesture} from '../../workspace/pointerGesture';
import type { ProjectModel } from "@/app/lib/editor/state/project";

interface Rect {
  readonly x: number;
  readonly y: number;
  readonly w: number;
  readonly h: number;
}

// A drag shorter than this (screen px) on either side is a click: nothing is picked.
const MIN_PX = 6;

/** The drag catcher over the frame while the Area tool is on, and the choice once a rectangle is drawn. */
export function RegionSelect({ project, onDone }: { project: ProjectModel; onDone: () => void }) {
  useEditorLanguage();
  const { width, height, fps } = project.canvas;
  const owner=useContext(EditorStoreContext);
  const stop=useRef<(()=>void)|undefined>(undefined);
  // Frame-local screen px while dragging; canvas px once drawn.
  const [drag, setDrag] = useState<{ x0: number; y0: number; x1: number; y1: number } | undefined>();
  const [picked, setPicked] = useState<{ area: Rect; frame:number; start?:number; end?:number } | undefined>();
  const markIn = usePlayback((s) => s.markIn);
  const markOut = usePlayback((s) => s.markOut);
  const frame = Math.floor(usePlayback((s) => !s.playing&&s.hover!==undefined?s.hover:s.frame));
  const local = (e: {clientX:number;clientY:number},target:HTMLElement) => {
    const r = target.getBoundingClientRect();
    return { x: Math.min(r.width, Math.max(0, e.clientX - r.left)), y: Math.min(r.height, Math.max(0, e.clientY - r.top)), r };
  };
  const box = (d: { x0: number; y0: number; x1: number; y1: number }): Rect => ({ x: Math.min(d.x0, d.x1), y: Math.min(d.y0, d.y1), w: Math.abs(d.x1 - d.x0), h: Math.abs(d.y1 - d.y0) });
  const add = (at: { frame: number } | { start: number; end: number }) => {
    const snapshot=owner?.getSnapshot().snapshot;
    if (picked&&owner===currentEditorStore()&&snapshot&&projectOf(snapshot)===project) actions.addRegionToChat(picked.area, at);
    setPicked(undefined);
    onDone();
  };
  useEffect(()=>{stop.current?.();setPicked(undefined);return()=>stop.current?.()},[project,owner]);
  // Esc: the rectangle goes, then the tool.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (isTextComposition(e)||e.key !== "Escape") return;
      e.preventDefault();
      e.stopImmediatePropagation();
      if (picked) setPicked(undefined);
      else onDone();
    };
    editorEvents.addEventListener("keydown", onKey, true);
    return () => editorEvents.removeEventListener("keydown", onKey, true);
  }, [picked, onDone]);
  const hasRange = picked?.start !== undefined && picked.end !== undefined && picked.end > picked.start;
  const shown = drag ? box(drag) : undefined;
  return (
    <div
      className="pv-region-catch"
      data-testid="region-catch"
      onPointerDown={(e) => {
        if (e.button !== 0) return;
        e.preventDefault();e.stopPropagation();stop.current?.();
        const snapshot=owner?.getSnapshot().snapshot;
        if(owner!==currentEditorStore()||!snapshot||projectOf(snapshot)!==project)return;
        const target=e.currentTarget,{x,y,r:bounds}=local(e,target);
        if(bounds.width<=0||bounds.height<=0)return;
        const start={x0:x,y0:y,x1:x,y1:y};
        const clear=()=>{
          stop.current=undefined;setDrag(undefined);
          if(target.hasPointerCapture(e.pointerId))target.releasePointerCapture(e.pointerId);
        };
        const position=(next:PointerEvent)=>{
          const at=local(next,target),r=at.r;
          if(r.left!==bounds.left||r.top!==bounds.top||r.width!==bounds.width||r.height!==bounds.height){stop.current?.();return}
          return {...start,x1:at.x,y1:at.y};
        };
        const pointer=beginEditorViewGesture(e,{
          move:next=>{const nextBox=position(next);if(nextBox)setDrag(nextBox)},
          cancel:clear,
          commit:next=>{
            const nextBox=position(next);clear();if(!nextBox)return;
            const screen=box(nextBox);
            if(screen.w<MIN_PX||screen.h<MIN_PX)return;
            const sx=width/bounds.width,sy=height/bounds.height;
            setPicked({area:{x:screen.x*sx,y:screen.y*sy,w:screen.w*sx,h:screen.h*sy},frame,start:markIn,end:markOut});
          },
        });
        if(!pointer)return;
        stop.current=pointer;
        target.setPointerCapture(e.pointerId);
        playback.pause();playback.seek(frame);
        setPicked(undefined);
        setDrag(start);
      }}
      onLostPointerCapture={()=>stop.current?.()}
    >
      {shown && <div className="pv-region drawing" data-testid="region-rect" style={{ left: shown.x, top: shown.y, width: shown.w, height: shown.h }} />}
      {picked&&<div className="pv-region drawing" data-testid="region-rect" style={{left:`${picked.area.x/width*100}%`,top:`${picked.area.y/height*100}%`,width:`${picked.area.w/width*100}%`,height:`${picked.area.h/height*100}%`}}/>}
      {picked && (
        <div className="pv-region-pick" data-testid="region-pick" style={{ left: `${picked.area.x/width*100}%`, top: `calc(${(picked.area.y+picked.area.h)/height*100}% + 6px)` }} onPointerDown={(e) => e.stopPropagation()}>
          <button className="btn primary" data-testid="region-add-frame" onClick={() => add({ frame:picked.frame })}>
            {editorText("Add to chat at ")}{momentLabel(picked.frame, fps)}
          </button>
          {hasRange && (
            <button className="btn" data-testid="region-add-range" onClick={() => add({ start: picked.start!, end: picked.end! })}>
              {editorText("In to Out (")}{momentLabel(picked.start!, fps)}–{momentLabel(picked.end!, fps)})
            </button>
          )}
          <button className="btn ghost" data-testid="region-cancel" onClick={() => setPicked(undefined)}>
            {editorText("Cancel")}</button>
        </div>
      )}
    </div>
  );
}

/** The region chips still in the composer, on the preview while the frame shown is in their time. */
export function RegionPins({ project }: { project: ProjectModel }) {
  useEditorLanguage();
  const mentions = useAppSelect((s) => s.agentMentions);
  const draft = useAppSelect((s) => s.agentDraft);
  const shownFrame = usePlayback((s) => (!s.playing && s.hover !== undefined ? s.hover : Math.floor(s.frame)));
  const { width, height } = project.canvas;
  const pins = mentions.filter((m) => m.kind === "region" && m.area && draft.includes(`@${m.label}`));
  if (!pins.length) return null;
  return (
    <>
      {pins.map((m) => {
        const a = m.area!;
        const on = m.range ? shownFrame >= m.range.start && shownFrame < m.range.end : shownFrame === m.frame;
        if (!on) return null;
        return (
          <div
            key={m.id + m.label}
            className="pv-region pin"
            data-testid="region-pin"
            data-label={m.label}
            style={{ left: `${(a.x / width) * 100}%`, top: `${(a.y / height) * 100}%`, width: `${(a.w / width) * 100}%`, height: `${(a.h / height) * 100}%` }}
          >
            <span>{editorText(m.label)}</span>
          </div>
        );
      })}
    </>
  );
}
