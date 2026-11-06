import type {Project} from '../core/timeline/types'
import {layout} from '../core/timeline/timeline'
import {projectChecks,disabledChecks,type Check} from '../core/checks/checks'
import {reviewRubric,type RubricBrief} from '../core/skills/rubric'
import {sheetFrames,sheetLayout,clock,sheetHtml,type SheetLayout} from './sheets'
import {shotCells} from './video'
import type {VideoResult} from '../analysis/videoResult'
export function agentUi(project:Project,args:{frame?:number;select?:string[];panel?:string}){
  const selected=args.select?.[0]
  if(selected&&!project.tracks.some(track=>track.items.some(item=>item.id===selected)))throw new Error('指定的时间轴片段不存在，请重新读取工程')
  return {...(args.frame!==undefined?{frame:Math.min(args.frame,layout(project).duration)}:{}),...(selected?{select:selected}:{}),...(args.panel?{panel:args.panel}:{})}
}

export function agentView(project:Project,tool:string,args:{frames?:number[];scale?:number;phone?:boolean;media?:string;range?:{start:number;end:number};every?:number;max?:number;columns?:number;at?:string},video?:VideoResult){
  let target=project
  if(tool==='contact_sheet'&&args.media){
    const media=project.media.find(m=>m.id===args.media)
    if(!media||media.kind==='audio')throw new Error('请选择视频或图片素材')
    const duration=media.kind==='image'?1:media.probe?.frames
    if(!duration||!Number.isSafeInteger(duration)||duration<1)throw new Error('素材时长不可用')
    const width=media.probe?.width??project.canvas.width,height=media.probe?.height??project.canvas.height
    target={...project,canvas:{...project.canvas,width,height},tracks:[{id:'view-media',kind:'video',name:media.name,items:[{id:'view-item',type:'clip',media:media.id,start:0,duration}]}],markers:[]}
  }
  const duration=layout(target).duration
  if(duration<=0)throw new Error('时间轴为空')
  const {width:width,height}=target.canvas
  if(tool==='render_frames'){
    const frames=args.frames!;if(frames.some(f=>f>=duration))throw new Error('取帧位置超出时间轴')
    const scale=args.phone?360/width:args.scale??Math.min(1,768/Math.max(width,height))
    return {project:target,frames,width:Math.max(2,Math.round(width*scale/2)*2),fps:project.canvas.fps}
  }
  const start=Math.max(0,Math.floor(args.range?.start??0)),end=Math.min(duration,Math.ceil(args.range?.end??duration))
  if(start>=end)throw new Error('取帧范围为空或超出素材时长')
  if(args.at==='shots'&&(!args.media||args.every!==undefined||!video?.hasVideo))throw new Error('按镜头取帧需要已分析的视频素材，不能同时设置间隔')
  const shots=args.at==='shots'?shotCells(video!,project.canvas.fps,duration,{start,end},args.max??24):undefined
  const frames=shots?shots.shots.map(s=>s.frame):sheetFrames(start,end,args.every,args.max??24)
  if('refused' in frames)throw new Error(frames.refused)
  if(!frames.length)throw new Error('所选范围没有可用镜头')
  const grid=sheetLayout(frames.length,width/height,args.columns)
  return {project:target,frames,width:Math.max(2,Math.round(grid.cellWidth/2)*2),fps:project.canvas.fps,grid,...(shots?{shots:shots.shots,totalShots:shots.total}:{})}
}
export function agentSheet(cells:readonly {jpeg:string;frame:number;sourceSeconds?:number}[],fps:number,grid:SheetLayout){
  return sheetHtml(cells.map(cell=>({jpeg:cell.jpeg,caption:cell.sourceSeconds!==undefined?clock(cell.sourceSeconds):`${cell.frame} · ${clock(cell.frame/fps)}`})),grid)
}
export function agentChecks(project:Project,args:{rules?:string[];severity?:'warn'|'info'},plan:{brief?:RubricBrief}){
  const rubric=reviewRubric(plan.brief)
  const disabled=disabledChecks(project)
  const findings=projectChecks(project,{sound:rubric.sound}).filter(c=>(!args.rules||args.rules.includes(c.id))&&(!args.severity||c.severity===args.severity))
  return {warnings:findings.filter(c=>c.severity==='warn').length,notes:findings.filter(c=>c.severity==='info').length,disabled,rubric,checks:findings.map(c=>({rule:c.id,severity:c.severity,message:c.message,items:c.itemIds??[],...(c.frame!==undefined?{frame:c.frame,seconds:c.frame/project.canvas.fps}:{}),...(c.range?{range:c.range}:{}),...(c.fix?{fix:c.fix}:{}),...(c.hint?{hint:c.hint}:{})}))}
}
