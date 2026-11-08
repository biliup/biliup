import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Renaming in place (T7, G4): a text field over a name; Enter or leaving it saves, Esc cancels.
import { useEffect, useRef } from "react";
import { isTextComposition } from '@/app/lib/editor/keyboard';
import { editorGestureOrigin,ownsEditorGesture,type GestureOrigin } from '../workspace/pointerGesture';

export function RenameField({ value, onDone, testId }: { value: string; onDone: (name: string | undefined,origin?:GestureOrigin) => void; testId: string }) {
  useEditorLanguage();
  const ref = useRef<HTMLInputElement>(null);
  const done = useRef(false);
  const origin=useRef<GestureOrigin|undefined>(undefined);
  const composing=useRef(false),blurred=useRef(false);
  const finish = (name: string | undefined) => {
    if (done.current) return;
    done.current = true;
    const from=origin.current;
    onDone(from&&ownsEditorGesture(from)?name:undefined,from);
  };
  useEffect(() => {
    done.current=false;
    const from=editorGestureOrigin();origin.current=from;
    if(!from){finish(undefined);return}
    const release=from.owner.subscribe(()=>{if(!ownsEditorGesture(from))finish(undefined)});
    ref.current?.focus();
    ref.current?.select();
    return()=>{done.current=true;release()};
  }, []);
  return (
    <input
      ref={ref}
      className="renaming"
      data-testid={testId}
      defaultValue={value}
      aria-label={editorText("Name")}
      onPointerDown={(e) => e.stopPropagation()}
      onDoubleClick={(e) => e.stopPropagation()}
      onFocus={()=>{blurred.current=false}}
      onCompositionStart={()=>{composing.current=true}}
      onCompositionEnd={e=>{
        composing.current=false;
        if(blurred.current&&document.hasFocus())finish(e.currentTarget.value);
      }}
      onKeyDown={(e) => {
        e.stopPropagation();
        if(composing.current||isTextComposition(e.nativeEvent))return;
        if(e.key==='Enter'||e.key==='Escape')e.preventDefault();
        if (e.key === "Enter") finish(e.currentTarget.value);
        else if (e.key === "Escape") finish(undefined);
      }}
      // Switching applications keeps the name open; clicking elsewhere in the editor saves it.
      onBlur={(e)=>{blurred.current=true;if(!composing.current&&document.hasFocus())finish(e.currentTarget.value)}}
    />
  );
}
