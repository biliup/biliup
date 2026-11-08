// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import {booleanPreference,useBrowserPreference} from '@/app/lib/editor/browser-preference'

const preference=booleanPreference('preview.proxies')
export const proxiesOn=preference.get
export const setProxiesOn=preference.set
export const useProxiesOn=()=>useBrowserPreference(preference)
