// Export requests use the same timeline range, target and size definitions as the dialog.
import {layout,projectTargetLufs,choiceOfTarget,outputSize,type Project} from '../core'
import type {ExportVideo,ExportQuality} from '../contracts'
import {exportRangeProblem} from './range'
import {motionBlurDefault} from '../core/visual/motionBlur'

export function exportRequest(project:Project,input:{quality:ExportQuality;video?:ExportVideo;range?:{from:number;to:number};targetLufs?:number|null}) {
  if(!['draft','standard','high'].includes(input.quality))throw new Error('导出质量无效')
  const video=input.video??{}
  if(video.codec!==undefined&&!['h264','hevc'].includes(video.codec))throw new Error('视频编码无效')
  if(video.encoder!==undefined&&!['hardware','software'].includes(video.encoder))throw new Error('编码器无效')
  if(video.fps!==undefined&&(!Number.isFinite(video.fps)||video.fps<1||video.fps>120))throw new Error('导出帧率必须在 1 到 120 之间')
  if(video.shortSide!==undefined&&(!Number.isInteger(video.shortSide)||video.shortSide<64||video.shortSide>4320))throw new Error('导出短边尺寸必须在 64 到 4320 像素之间')
  if(video.bitrateMbps!==undefined&&(!Number.isFinite(video.bitrateMbps)||video.bitrateMbps<0.1||video.bitrateMbps>500))throw new Error('视频码率必须在 0.1 到 500 Mbps 之间')
  if(input.targetLufs!=null&&(!Number.isFinite(input.targetLufs)||input.targetLufs< -70||input.targetLufs> -5))throw new Error('响度目标必须在 -70 到 -5 LUFS 之间')
  const rangeProblem=exportRangeProblem(project,input.range)
  if(rangeProblem)throw new Error(rangeProblem)
  const size=outputSize(project.canvas,input.quality,video.shortSide)
  const frames=input.range?input.range.to-input.range.from:layout(project).duration
  return {width:size.width,height:size.height,fps:video.fps??project.canvas.fps,seconds:frames/project.canvas.fps}
}

export function agentExportRequest(project:Project,args:{preset?:ExportQuality;name?:string;loudness?:string;targetLufs?:number;range?:'all'|'marks'|'selection';fps?:number;shortSide?:number;codec?:string;encoder?:string;bitrateMbps?:number;burnCaptions?:boolean;motionBlur?:boolean;subtitles?:unknown},ranges:ReadonlyArray<{kind:string;from:number;to:number}>) {
  const kind=args.range??'all'
  const range=kind==='all'?undefined:ranges.find(r=>r.kind===kind)
  if(kind!=='all'&&!range)throw new Error(kind==='marks'?'发送消息时没有设置入出点':'发送消息时没有选中片段')
  const target=projectTargetLufs(project)
  const quality=args.preset??'standard'
  return {output:args.name??project.name,quality,loudness:args.loudness??choiceOfTarget(target),targetLufs:args.targetLufs??target,
    ...(range?{range:{from:range.from,to:range.to}}:{}),
    video:{codec:args.codec,encoder:args.encoder,bitrateMbps:args.bitrateMbps,fps:args.fps,shortSide:args.shortSide},
    burnCaptions:args.burnCaptions??true,motionBlur:args.motionBlur??motionBlurDefault(project,quality),subtitles:args.subtitles}
}
