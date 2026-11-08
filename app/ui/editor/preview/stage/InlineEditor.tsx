// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { useRef, useState, useLayoutEffect } from "react";
import {isTextComposition} from '@/app/lib/editor/keyboard';
import type {GestureOrigin} from '../../workspace/pointerGesture';

export interface InlineEdit {
  readonly id: string;
  /** The text element's inline style, so the editor sits exactly over it with the same type. */
  readonly css: string;
  readonly text: string;
  readonly origin: GestureOrigin;
}

/**
 * Text editing in place: a textarea over the hidden text with the same inline style (position,
 * font, size, colour, alignment), growing with its content. Enter saves, ⇧Enter adds a line,
 * Esc cancels, blur saves. Editor shortcuts don't fire while it has focus (Editor.tsx skips
 * editable targets).
 */
export function InlineEditor({ edit, onDone }: { edit: InlineEdit; onDone: (text: string | undefined) => void }) {
  const ref = useRef<HTMLTextAreaElement>(null);
  const [value, setValue] = useState(edit.text);
  const done = useRef(false);
  const composing=useRef(false),blurred=useRef(false);
  useLayoutEffect(() => {
    const el = ref.current!;
    // Same box and type as the text; a fit-to-content box grows with what's typed instead.
    el.style.cssText = `${edit.css.replace(/width:max-content;?/, "")};opacity:1;visibility:visible;transform:none;clip-path:none`;
    el.focus();
    el.select();
  }, [edit.css]);
  const finish = (text: string | undefined) => {
    if (done.current) return;
    done.current = true;
    onDone(text);
  };
  return (
    <textarea
      ref={ref}
      className="pv-inline"
      data-testid="inline-editor"
      data-item={edit.id}
      value={value}
      spellCheck={false}
      rows={1}
      onChange={(e) => setValue(e.target.value)}
      onFocus={()=>{blurred.current=false}}
      onBlur={(e) => {blurred.current=true;if(!composing.current)finish(e.currentTarget.value)}}
      onCompositionStart={()=>{composing.current=true}}
      onCompositionEnd={e=>{composing.current=false;setValue(e.currentTarget.value);if(blurred.current)finish(e.currentTarget.value)}}
      onPointerDown={(e) => e.stopPropagation()}
      onKeyDown={(e) => {
        // Mid-composition (IME) Esc and Enter belong to the input method.
        if (composing.current||isTextComposition(e.nativeEvent)) return;
        if (e.key === "Escape") {
          e.preventDefault();
          e.stopPropagation();
          finish(undefined);
        } else if (e.key === "Enter" && !e.shiftKey) {
          e.preventDefault();
          e.stopPropagation();
          finish(e.currentTarget.value);
        }
      }}
    />
  );
}
