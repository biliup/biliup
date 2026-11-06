// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import {currentEditorStore} from '@/app/lib/editor/state/store/core';
import type {EditorStore} from '@/app/lib/editor/project-store';
// A double click on an element on the canvas (F23, D162) asks the Inspector to focus the element's
// main text field (component/schema.ts mainTextParam) with its text selected. The request waits for
// the element's Inspector to mount (the selection and the Inspector tab change first).
const listeners = new Set<() => void>();
let pending: {item:string;owner:EditorStore} | undefined;

export function requestTextFocus(itemId: string) {
  const owner=currentEditorStore();
  if(!owner)return;
  pending = {item:itemId,owner};
  for (const l of listeners) l();
}

/** Calls `focus` now and on every later request for `itemId`, while subscribed; it says whether it focused. */
export function onTextFocus(itemId: string, focus: () => boolean, reveal:()=>void): () => void {
  const owner=currentEditorStore();
  let frame:number|undefined;
  let active=true;
  let handled:typeof pending;
  const check = () => {
    if(frame!==undefined)cancelAnimationFrame(frame);
    const request=pending;
    if(!active||!request||request.item!==itemId||request.owner!==owner||currentEditorStore()!==owner)return;
    handled=request;
    reveal();
    // A few frames: the field may not be laid out yet (the tab switch, the section opening).
    let tries = 0;
    const attempt = () => {
      frame=undefined;
      if(!active||pending!==request||currentEditorStore()!==owner)return;
      if (focus()) pending = undefined;
      else if (++tries < 20) frame=requestAnimationFrame(attempt);
      else pending = undefined;
    };
    frame=requestAnimationFrame(attempt);
  };
  listeners.add(check);
  check();
  return () => {active=false;listeners.delete(check);if(frame!==undefined)cancelAnimationFrame(frame);if(pending===handled)pending=undefined};
}
