// Uncut tools/library.ts; searches use the same catalog and matching as the editor panels.
import {filterSfx,filterFolderSounds,onlyWhenAsked,aliasOf,clipNote,type SoundFolder} from '../core/audio/library'
export function agentLibrarySearch(args:{kind:'music'|'sfx';query?:string;category?:string;page?:number},folders:ReadonlyArray<SoundFolder>,fps:number,project:number){
  const query=args.query??'',page=args.page??1,offset=(page-1)*16
  const sounds=args.kind==='sfx'?filterSfx({query,category:args.category}):[]
  const yours=args.category?[]:filterFolderSounds(folders,query)
  const alias=query?aliasOf(query):undefined
  return {source:'included',page,pages:Math.max(1,Math.ceil(Math.max(sounds.length,yours.length)/16)),total:sounds.length+yours.length,
    sounds:sounds.slice(offset,offset+16).map(s=>({id:`sfx:${s.id}`,title:s.title,category:s.category,seconds:s.seconds,frames:Math.round(s.seconds*fps),...(s.peaks.length?{peakFrame:Math.round((s.peaks.indexOf(Math.max(...s.peaks))+0.5)*s.seconds*fps/s.peaks.length)}:{}),author:s.author,license:'CC0',preview:`/editor-sfx/${encodeURIComponent(s.file)}`,...(onlyWhenAsked(s)?{onlyWhenAsked:true}:{})})),
    yours:yours.slice(offset,offset+16).map(s=>({id:`folder:${s.id}`,title:s.name,folder:s.folderName,file:s.rel,
      ...(s.seconds!==undefined?{seconds:s.seconds,frames:Math.round(s.seconds*fps)}:{}),
      preview:`/v1/editor/projects/${project}/library/sounds/${encodeURIComponent(s.id)}/file`})),
    ...(alias?.clip?{clip:clipNote(alias)}:{}),
    note:args.kind==='music'?'用户已导入的音频文件夹；软件不附带音乐。用 library_add 加入当前工程，再用 beat_grid 查看节拍。':'内置 CC0 音效与用户已导入音频；用 library_add 加入当前工程，再放到音轨。peakFrame 表示素材最响的位置，可用于对齐真实动作。onlyWhenAsked 音效只在用户要求时使用。'}
}
