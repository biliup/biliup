// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The default caption style for new captions (D166), remembered in this browser (localStorage, like
// the panel sizes). Settings › App sets it; Make captions passes it to core planMakeCaptions.
import { CAPTION_STYLES } from "@/app/lib/editor/core/text/captions";
import { browserPreference, useBrowserPreference } from '@/app/lib/editor/browser-preference';

const preference = browserPreference<string>('captions.style', '', (value): value is string => typeof value === 'string' && (value === '' || CAPTION_STYLES.some(style => style.id === value)));
export const setDefaultCaptionStyle = preference.set;
export const useDefaultCaptionStyle = () => useBrowserPreference(preference);

