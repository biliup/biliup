// Adapted from Uncut brand/importer.ts (MIT); see licenses/uncut/LICENSE.
import type { BrandImported } from './contracts';

export const MAX_BRAND_IMPORT_FILES = 24;
export const isBrandImport = (name:string) => /\.(svg|png|jpe?g|webp|gif|ttf|otf|woff2?)$/i.test(name);

export function describeBrandFile(name:string):BrandImported {
  const stem=name.replace(/\.[^.]+$/, '');
  if (/\.(ttf|otf|woff2?)$/i.test(name)) {
    const family=stem
      .replace(/[-_ ](thin|hairline|extra-?light|ultra-?light|light|regular|book|normal|medium|semi-?bold|demi-?bold|bold|extra-?bold|ultra-?bold|black|heavy|italic|oblique|variable|vf|var|wght|\d{3})([-_ ].*)?$/i, '')
      .replace(/\[.*\]$/, '').replace(/[-_]+/g, ' ')
      .replace(/([a-z])([A-Z])/g, '$1 $2').replace(/([A-Z]+)([A-Z][a-z])/g, '$1 $2').trim()
      .split(/\s+/).filter(Boolean).map(w=>w.length<=2&&w===w.toLowerCase()?w.toUpperCase():w[0]!.toUpperCase()+w.slice(1)).join(' ').slice(0,80);
    return {name,kind:'font',family:family||stem.slice(0,80)};
  }
  const slot=/(^|[^a-z])(mark|logomark|icon|symbol|glyph|favicon|avatar|monogram)([^a-z]|$)/i.test(stem)?'mark'
    :/(^|[^a-z])(dark|white|inverse|inverted|negative|reversed|knockout|on-?dark|for-?dark)([^a-z]|$)/i.test(stem)?'dark':'light';
  return {name,kind:'logo',slot};
}
