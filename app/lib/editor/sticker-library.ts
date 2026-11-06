import {STICKERS,type StickerEntry} from './core/visual/stickers'
import translations from './locales/stickers.zh-CN.json'
import {getEditorLanguage} from './i18n'
import {editorAssetScore} from './asset-search'

// Scoped UI labels: translating a sticker never rewrites its artwork or the user's text.
const labels:Readonly<Record<string,ReadonlyArray<string>>>=translations
export const stickerLabel=(sticker:Pick<StickerEntry,'name'>)=>getEditorLanguage()==='zh-CN'?(labels[sticker.name]?.[0]??sticker.name):sticker.name

export function searchStickers(query:string):ReadonlyArray<StickerEntry>{
  if(!query.trim())return STICKERS
  return STICKERS.flatMap((sticker,index)=>{
    const translated=labels[sticker.name]
    const score=editorAssetScore(query,{
      names:[sticker.name,translated?.[0]??''],
      keywords:[...sticker.keywords,translated?.[1]??''],
      groups:[sticker.group,'Stickers',...(sticker.emoji?['Emoji']:[])],symbols:sticker.emoji?[sticker.emoji]:[],
    })
    return score===undefined?[]:[{sticker,score,index}]
  }).sort((a,b)=>b.score-a.score||a.index-b.index).map(result=>result.sticker)
}
