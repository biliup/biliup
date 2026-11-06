// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import {booleanPreference,useBrowserPreference} from '@/app/lib/editor/browser-preference'

const preference=booleanPreference('preview.scrubCache')
export const scrubCacheOn=preference.get
export const setScrubCacheOn=preference.set
export const useScrubCacheOn=()=>useBrowserPreference(preference)
