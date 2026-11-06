/** Shared spacing for transcript tokens, captions and subtitle files. */
const unspaced = /[\p{Script=Han}\p{Script=Hiragana}\p{Script=Katakana}々〆〇]/u;
const closing = /^[,.;:!?%\)\]\}，。！？、；：％）】》〉」』〕〗〙〛”’…]/u;
const opening = /[\(\[\{（【《〈「『〔〖〘〚“‘]$/u;

export function wordSeparator(left:string,right:string):string {
  const a=left.trim(),b=right.trim();
  if(!a||!b||closing.test(b)||opening.test(a))return '';
  const end=Array.from(a).at(-1)!,start=Array.from(b)[0]!;
  // CJK punctuation also closes an unspaced token when the next token is Latin.
  if(unspaced.test(end)||unspaced.test(start)||/[，。！？、；：）】》〉」』〕〗〙〛]/u.test(end))return '';
  return ' ';
}

/** Render decorations after choosing separators, so markup cannot change spacing. */
export function joinWords(words:ReadonlyArray<string>,render:(word:string,index:number)=>string=(word)=>word):string {
  return words.map((word,index)=>(index?wordSeparator(words[index-1]!,word):'')+render(word,index)).join('');
}

/** Count Unicode code points, not UTF-16 halves (including supplementary Han characters). */
export const textCharacters=(text:string):number=>Array.from(text).length;

/** Estimate average glyph width for automatic caption wrapping; explicit limits stay literal. */
export function wideCharacterRatio(words:ReadonlyArray<string>):number {
  const text=joinWords(words),chars=Array.from(text);
  return chars.length?chars.filter(char=>unspaced.test(char)||/[\u3000-\u303f\uff01-\uff60\uffe0-\uffe6]/u.test(char)).length/chars.length:0;
}
