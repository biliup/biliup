// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Hiding a library tile never changes elements already placed on the timeline.
import {browserPreference,useBrowserPreference} from '@/app/lib/editor/browser-preference'

export const HIDDEN_BY_DEFAULT:ReadonlyArray<string>=['component.agent-log','component.compare-table','component.device-frame','component.terminal','component.code-block']
const preference=browserPreference('elements.hidden',HIDDEN_BY_DEFAULT,(value):value is ReadonlyArray<string>=>Array.isArray(value)&&value.every(id=>typeof id==='string'))
export const useHiddenElements=()=>useBrowserPreference(preference)
export const hideElement=(id:string)=>{
  const hidden=preference.get()
  if(!hidden.includes(id))preference.set([...hidden,id])
}
export const showElement=(id:string)=>{
  const hidden=preference.get()
  if(hidden.includes(id))preference.set(hidden.filter(value=>value!==id))
}
