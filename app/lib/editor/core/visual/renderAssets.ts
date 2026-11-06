import {resolveGraphicBrand,MAX_GRAPHIC_DOC_BYTES} from './graphics'
import {resolveGraphicFonts,MAX_GRAPHIC_FONT_BYTES,type FontSource} from './fonts'
export interface RenderAssets {readonly fonts:ReadonlyArray<FontSource>;readonly logos:Readonly<Partial<Record<'light'|'dark'|'mark',string>>>;readonly warnings:ReadonlyArray<string>}
/** The preview and export inline exactly the same resources and respect the same graphic budget. */
export function graphicWithAssets(html:string,assets:RenderAssets):string{
  const branded=resolveGraphicBrand(html,assets.logos)
  const budget=Math.max(0,Math.min(MAX_GRAPHIC_FONT_BYTES,MAX_GRAPHIC_DOC_BYTES-new TextEncoder().encode(branded).length-16000))
  const result=resolveGraphicFonts(branded,assets.fonts,budget)
  if(new TextEncoder().encode(result).length+16000>MAX_GRAPHIC_DOC_BYTES)throw new Error('图形包含的品牌资源超过文档大小限制')
  return result
}
