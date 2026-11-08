// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import {browserPreference,useBrowserPreference} from '@/app/lib/editor/browser-preference'

export type DefaultLength='still'|'text'
export const DEFAULT_SECONDS:Readonly<Record<DefaultLength,number>>={still:5,text:3}
export const DEFAULT_RANGE={min:0.5,max:60} as const
const valid=(value:unknown):value is number=>typeof value==='number'&&Number.isFinite(value)&&value>=DEFAULT_RANGE.min&&value<=DEFAULT_RANGE.max
const preferences={
  still:browserPreference('defaults.stillSeconds',DEFAULT_SECONDS.still,valid),
  text:browserPreference('defaults.textSeconds',DEFAULT_SECONDS.text,valid),
}
export const defaultSeconds=(kind:DefaultLength)=>preferences[kind].get()
export const defaultFrames=(kind:DefaultLength,fps:number)=>Math.max(1,Math.round(defaultSeconds(kind)*fps))
export const useDefaultSeconds=(kind:DefaultLength)=>useBrowserPreference(preferences[kind])
export function setDefaultSeconds(kind:DefaultLength,seconds:number|undefined){
  if(seconds!==undefined&&!Number.isFinite(seconds))return
  preferences[kind].set(seconds===undefined?DEFAULT_SECONDS[kind]:Math.min(DEFAULT_RANGE.max,Math.max(DEFAULT_RANGE.min,seconds)))
}
