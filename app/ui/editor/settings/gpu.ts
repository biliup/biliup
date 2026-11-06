// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import {booleanPreference,useBrowserPreference} from '@/app/lib/editor/browser-preference'

const preference=booleanPreference('preview.gpu')
export const gpuCompositorOn=preference.get
export const setGpuCompositorOn=preference.set
export const useGpuCompositorOn=()=>useBrowserPreference(preference)
