import type { Item } from '@/app/lib/editor/core/timeline/types';
import type { CaptionWord } from './settings';

type Word = Pick<CaptionWord,'text'|'start'|'end'>;
const key=(word:Word)=>word.text.trim();

function byText(words:ReadonlyArray<Word>):Map<string,number[]> {
  const groups=new Map<string,number[]>();
  words.forEach((word,index)=>{
    const text=key(word),group=groups.get(text);
    if(group)group.push(index);else groups.set(text,[index]);
  });
  return groups;
}

/** Only an unambiguous, reciprocal match may carry a user's correction to a new index. */
function candidates(word:Word,words:ReadonlyArray<Word>,group:ReadonlyArray<number>):number[] {
  const out:number[]=[];
  let lo=0,hi=group.length;
  while(lo<hi){const mid=(lo+hi)>>>1;if(words[group[mid]!]!.start<word.start-.5)lo=mid+1;else hi=mid;}
  for(let i=lo;i<group.length;i++){
    const index=group[i]!,candidate=words[index]!;
    if(candidate.start>word.start+.5)break;
    const overlaps=Math.min(word.end,candidate.end)>Math.max(word.start,candidate.start);
    const sameEdges=Math.abs(word.start-candidate.start)<=.15&&Math.abs(word.end-candidate.end)<=.15;
    if(overlaps||sameEdges)out.push(index);
  }
  return out;
}

export function remapCaptionFixes(item:Pick<Item,'words'|'edits'|'breaks'>,words:ReadonlyArray<CaptionWord>) {
  const before=item.words??[],edits=item.edits??[],breaks=item.breaks??[];
  const same=before.length===words.length&&before.every((word,index)=>key(word)===key(words[index]!));
  const mapping=new Map<number,number>();
  const needed=new Set([...edits.map(edit=>edit.word),...breaks]);
  if(same){for(const index of needed)mapping.set(index,index);}
  else {
    const oldGroups=byText(before),newGroups=byText(words);
    for(const index of needed){
      const word=before[index];
      if(!word)continue;
      const matches=candidates(word,words,newGroups.get(key(word))??[]);
      if(matches.length!==1)continue;
      const target=matches[0]!,reverse=candidates(words[target]!,before,oldGroups.get(key(word))??[]);
      if(reverse.length===1&&reverse[0]===index)mapping.set(index,target);
    }
  }
  const nextEdits=edits.flatMap(edit=>{
    const word=mapping.get(edit.word);
    return word===undefined?[]:[{...edit,word}];
  }).sort((a,b)=>a.word-b.word);
  const nextBreaks=breaks.flatMap(index=>{const target=mapping.get(index);return target===undefined?[]:[target]}).sort((a,b)=>a-b);
  return {edits:nextEdits,breaks:nextBreaks,dropped:edits.length+breaks.length-nextEdits.length-nextBreaks.length};
}
