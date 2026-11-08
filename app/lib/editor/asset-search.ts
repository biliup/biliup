import chinese from './locales/zh-CN.json'
import english from './locales/en.json'

type AssetWords={
  readonly names:ReadonlyArray<string>
  readonly keywords?:ReadonlyArray<string>
  readonly groups?:ReadonlyArray<string>
  readonly symbols?:ReadonlyArray<string>
}
const normalize=(value:string)=>value.normalize('NFKC').toLowerCase().replace(/\uFE0F/g,'')
const catalogs:ReadonlyArray<Readonly<Record<string,string>>>=[chinese,english]
const searchable=(values:ReadonlyArray<string>)=>normalize(values.flatMap(value=>[value,...catalogs.map(catalog=>catalog[value]??'')]).join(' '))

/** Search built-in labels in either language, independently of the current UI language. */
export function editorAssetScore(query:string,{names,keywords=[],groups=[],symbols=[]}:AssetWords):number|undefined{
  const terms=normalize(query).trim().split(/[\s\p{Dash_Punctuation}]+/u).filter(Boolean)
  if(!terms.length)return 0
  const name=searchable(names),keyword=searchable(keywords),group=searchable(groups),marks=symbols.map(normalize)
  let score=0
  for(const term of terms){
    if(marks.includes(term))score+=3
    else if(name.includes(term))score+=2
    else if(keyword.includes(term))score+=1
    else if((term.length>=3||/\p{Script=Han}/u.test(term))&&group.includes(term))score+=0.5
    else return undefined
  }
  return score
}

export const matchesEditorAsset=(query:string,...texts:string[])=>editorAssetScore(query,{names:texts})!==undefined
