// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import {browserPreference,useBrowserPreference} from '@/app/lib/editor/browser-preference'

/** Undefined keeps each transition's own default length. */
export const TRANSITION_LENGTHS:ReadonlyArray<number|undefined>=[undefined,0.25,0.5,1,1.5]
const preference=browserPreference<number|undefined>('transitionSeconds',undefined,(value):value is number|undefined=>value===undefined||typeof value==='number'&&TRANSITION_LENGTHS.includes(value))
export const transitionSeconds=preference.get
export const setTransitionSeconds=preference.set
export const useTransitionSeconds=()=>useBrowserPreference(preference)
