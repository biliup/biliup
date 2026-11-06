import {editorText,useEditorLanguage} from '@/app/lib/editor/i18n'
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import type {Item} from '@/app/lib/editor/core/timeline/types'
import {Highlighter} from 'lucide-react'
import {useRef} from 'react'
import type {ProjectModel} from '@/app/lib/editor/state/project'
import {findItem} from '@/app/ui/editor/timeline/edit'
import {planText,toggleAccent} from '@/app/ui/editor/preview/text'
import {useTextDraft} from './draft'

export function TextContent({project,item,disabled}:{project:ProjectModel;item:Item;disabled:boolean}){
 useEditorLanguage()
 const ref=useRef<HTMLTextAreaElement>(null)
 const field=useTextDraft({project,value:item.text??'',disabled,multiline:true,
  read:p=>{const current=findItem(p,item.id)?.item;return current?.type==='text'?current.text??'':undefined},
  plan:(p,text)=>planText(p,item.id,text),
 })
 const accent=()=>{
  const el=ref.current
  if(!el||field.isComposing())return
  const next=toggleAccent(field.input.value,el.selectionStart,el.selectionEnd)
  if(next.text===field.input.value)return
  field.change(next.text);field.commit()
  requestAnimationFrame(()=>{if(el===ref.current&&el.isConnected)el.setSelectionRange(next.from,next.to)})
 }
 return <div className="group">
  <div className="group-label row">
   <span>{editorText('Text')}</span>
   <button className="btn ghost small-btn" data-testid="insp-accent" title={editorText('Accent the selected words ([[ ]] in the text)')} disabled={disabled} onMouseDown={e=>e.preventDefault()} onClick={accent}>
    <Highlighter/>{editorText('Accent')}
   </button>
  </div>
  <textarea ref={ref} className="field input text-area" data-testid="insp-text" rows={3} disabled={disabled} spellCheck={false} {...field.input}/>
 </div>
}
