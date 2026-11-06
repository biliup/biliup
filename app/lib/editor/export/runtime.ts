// Pure export bridge shared with the Rust renderer, not a client-supplied FFmpeg command.
import { outputSize, fontFaceCss, cubeText, validateProject, decodeEnvelope, type Project } from '@/app/lib/editor/core'
import {envelopeMediaIds,pictureItemIds} from './inputs'
import {renderWindow} from './range'
import { planExport, exportCommand, planLuts, planLoudness, RASTER_CSS, type RasterOutput, type Quality } from './exportPlan'
import type { VideoChoice } from './plan/codec'
import { builtinModules } from '@/app/lib/editor/core/timeline/builtin-modules'
import { shaderSourceArgs } from './plan/video'
import { shaderFrameBytes, shaderModuleCode, shaderParamBytes, shaderTime } from '@/app/lib/editor/core/visual/shaders'
import { ShaderRunner } from '@/app/ui/editor/preview/shaderRunner'
import {writeSubtitles,flattenCues,subtitleLanguageTag} from '../core/text/subtitles'
import {subtitlesForExport} from './subtitles'
import {fontFaceRules,fontRule,familyKey} from '../core/visual/fonts'
import {checkContext} from '../core/checks/context'
import {graphicWithAssets,type RenderAssets} from '../core/visual/renderAssets'
import {loudnessGain} from './plan/loudness'
import {audioCommand} from './plan/audio'
export {loadGenerated,drawGenerated,loadGraphicDocument,drawGraphic} from './generation'
export {graphicMediaReply} from './graphicMedia'
export {selectRenderInputs,pictureItemIds} from './inputs'
export {danmakuRenderWindows} from './danmaku'
export {agentEdit,agentModules} from '../agent/runtime'
export {prepareShader,tryShader} from '../shaders/runtime'
export {agentBrandPlan,agentBrandImports,agentBrandPicture,kitChanges} from '../agent/brand'
export {agentWebRead} from '../agent/web'
export {agentDocs} from '../agent/docs'
export {agentView,agentChecks,agentSheet,agentUi} from '../agent/view'
export {agentAnalysis,agentRank} from '../agent/analysis'
export {agentSpeechTarget,agentSpeechResult} from '../agent/speech'
export {agentSpeechPlan,agentSubtitles} from '../agent/speechEdit'
export {agentReviewPlan,agentReviewInput,agentReviewFinish} from '../agent/review'
export {agentMotionBegin,agentMotionPixels,agentMotionFinish} from '../agent/review'
export {agentMotionRefineStart,agentMotionRefineNext} from '../agent/review'
export {rehearsalPlan,rehearsalChunk} from './rehearse'
export {frameStart,frameNext} from './frame-runtime'
export {exportRequest,agentExportRequest} from './request'
export {agentLibrarySearch} from '../agent/library'
export {prepareLibrarySearch,finishLibrarySearch} from '../library-search'
export {resetGraphicBlur,beginGraphicSamples,showGraphicSource,addGraphicSample,finishGraphicSamples,nextGraphicSamples,showGraphicMean} from './graphicBlur'

export interface Options {
  quality: Quality
  video?: VideoChoice & { fps?: number; shortSide?: number }
  range?: { from: number; to: number }
  burnCaptions?: boolean
  motionBlur?: boolean
  subtitles?: {file?:'srt'|'vtt';track?:boolean;language?:string}
}

let planned: ReturnType<typeof planExport>
let options: Options
let gain: Awaited<ReturnType<typeof planLoudness>> = {}
let measuring: { pass?: ReturnType<typeof audioCommand>; done?: boolean; error?: string; result?:{gainDb?:number;lufs?:number} } = {}
let acceptMeasure: ((stderr: string) => void) | undefined
let shaderRunner: ShaderRunner | undefined
let renderAssets:RenderAssets={fonts:[],logos:{},warnings:[]}
export function frameAssets(assets:RenderAssets){renderAssets=assets}
export async function frameShade(width:number,height:number,code:string,frame:number[],params:number[]|undefined,rgba:string){
  if(!Number.isInteger(width)||!Number.isInteger(height)||width<1||height<1||width*height>16_777_216)throw new Error('着色器尺寸无效')
  await checkShaders()
  const data=Uint8Array.from(atob(rgba),c=>c.charCodeAt(0));if(data.length!==width*height*4)throw new Error('着色器输入大小无效')
  const pixels=await shaderRunner!.drawToPixels({kind:'rgba',data,width,height},{width,height,code,frame:new Float32Array(frame).buffer,...(params?{params:new Float32Array(params).buffer}:{})})
  const chunks:string[]=[];for(let at=0;at<pixels.length;at+=32768)chunks.push(String.fromCharCode(...pixels.subarray(at,at+32768)))
  return btoa(chunks.join(''))
}
/** Input comes from the validated saved revision, with paths resolved by the Rust media store. */
export function prepare(project: Project, paths: Record<string, string>, input: Options, analysis:Record<string,{hasAudio:boolean;envelope?:unknown}>={},graphics:Record<string,string>={},assets:RenderAssets={fonts:[],logos:{},warnings:[]}) {
  renderAssets=assets
  graphics=Object.fromEntries(Object.entries(graphics).map(([src,html])=>[src,graphicWithAssets(html,assets)]))
  const problems = validateProject(project, builtinModules)
  if (problems.length) throw new Error(problems.map(problem => `${problem.path}: ${problem.message}`).join('；'))
  options = input
  gain = {}
  renderWindow(project,input.range)
  const visible = new Set(pictureItemIds(project,input.range))
  // A missing graphic provider must never silently turn an animated layer into blank frames.
  for (const track of project.tracks) for(const item of track.items) if(visible.has(item.id)&&item.type==='graphic'&&!graphics[item.src!]) {
    throw new Error(`动态图形不可读取：${item.name??item.src}`)
  }
  const size = outputSize(project.canvas, input.quality, input.video?.shortSide)
  planned = planExport(project, { ...size, mediaPath: media => {
    const file = paths[media.id]
    if (!file) throw new Error(`素材不可读取：${media.id}`)
    return file
  }, graphic:src=>graphics[src],envelopes:new Map(Object.entries(analysis).map(([id,result])=>[id,result.hasAudio?(decodeEnvelope(result.envelope)??null):null])), burnCaptions: input.burnCaptions, motionBlur: input.motionBlur, range:input.range??undefined })
  if ('problem' in planned) throw new Error(planned.problem)
  const plan = planned
  const families=new Set(assets.fonts.map(font=>familyKey(font.family)))
  const fontWarnings=fontRule(family=>families.has(familyKey(family)),new Map(Object.entries(graphics))).check(checkContext(project)).map(check=>check.message)
  const subtitlePlan=subtitlesForExport(project,input.range??undefined)
  if('problem' in subtitlePlan)throw new Error(subtitlePlan.problem)
  const {cues}=subtitlePlan,subs=input.subtitles
  if((subs?.file||subs?.track)&&!cues.length)throw new Error('导出范围内没有字幕')
  const subtitles={file:subs?.file?{format:subs.file,text:writeSubtitles(cues,project.canvas.fps,subs.file)}:null,
    soft:subs?.track?writeSubtitles(flattenCues(cues),project.canvas.fps,'srt'):null,
    language:subs?.track?subtitleLanguageTag(subs.language??'zh'):null}
  return { width: size.width, height: size.height, canvas: planned.canvas, fps: planned.fps,
    subtitles,
    graphics:plan.visuals.flatMap((layer,index)=>layer.kind==='graphic'?[{index,doc:layer.doc,frames:layer.frames,samples:layer.motion?.ts,info:{fps:plan.fps,duration:(layer.item.in??0)+layer.item.duration,...plan.canvas,props:layer.item.props??{}}}]:[]),
    frames: planned.frames, background: planned.backgroundHtml, warnings: [...new Set([...assets.warnings,...fontWarnings,...planned.warnings])],
    shaders: plan.visuals.flatMap((layer, index) => (layer.kind === 'video' || layer.kind === 'image') && layer.fx?.shader ? [{
      index, width: layer.fx.w, height: layer.fx.h, frames: layer.f1 - layer.f0,
      name: `${layer.item.name ?? layer.item.id} / ${layer.fx.shader.def.name}`,
      decode: shaderSourceArgs(layer, { caps: { encoders: new Set() } }, plan.width, plan.height, plan.fps),
    }] : []),
    rasters: planned.visuals.flatMap((layer, index) => layer.kind === 'raster' ? [{ index, states: layer.states, frames: layer.frames }] : []),
    luts: planLuts(planned).map(lut => ({ key: lut.key, text: cubeText(lut) })) }
}

/** The same source selection as preview ducking/noise removal; Rust decodes registered media. */
export function envelopeSources(project:Project,range?:{from:number;to:number}) {
  renderWindow(project,range)
  return envelopeMediaIds(project).map(id=>{
    const media=project.media.find(media=>media.id===id)
    if(!media)throw new Error(`素材不存在：${id}`)
    return {id,seconds:(media.probe?.frames??0)/project.canvas.fps,hasAudio:media.probe?.hasAudio!==false}
  })
}

export function prepareRangeAudio() {
  if (!planned || 'problem' in planned) throw new Error('尚未生成导出计划')
  if (planned.audioWindow.from === 0 || !planned.audio.length) return null
  return audioCommand(planned, gain, ['-map', '[aout]', '-c:a', 'pcm_f32le', '-ar', '48000', '-ac', '2', '-rf64', 'auto', 'range-audio.wav'])
}

export function command(background: string, rasters: Record<number, RasterOutput>, luts: Record<string, string>, output: string,
  encoders: string[] = ['libx264', 'libx265'], shaded: Record<number, string> = {}, preparedAudio = false) {
  if (!planned || 'problem' in planned) throw new Error('尚未生成导出计划')
  const plan = planned
  return exportCommand(plan, { background, rasters: new Map(Object.entries(rasters).map(([index, raster]) => [plan.visuals[Number(index)]!, raster])),
    luts: new Map(Object.entries(luts)), output, caps: { encoders: new Set(encoders) },
    shaded: new Map(Object.entries(shaded).map(([index, file]) => [plan.visuals[Number(index)]!, file])),
    subtitles:options.subtitles?.track?{file:'subtitles.srt',language:subtitleLanguageTag(options.subtitles.language??'zh'),show:options.burnCaptions===false}:undefined,
    quality: options.quality, video: options.video ?? undefined, audio: gain,
    preparedAudio: preparedAudio ? 'range-audio.wav' : undefined })
}

export async function checkShaders() {
  if (shaderRunner && !shaderRunner.lost) return
  const runner = await ShaderRunner.create()
  if ('problem' in runner) throw new Error(`无法导出着色器特效：${runner.problem}`)
  shaderRunner = runner
}

/** Same runner, input colour/alpha, frame clock and parameters as the live preview. */
export async function shade(index: number, frame: number, rgba: string) {
  if (!planned || 'problem' in planned) throw new Error('尚未生成导出计划')
  const layer = planned.visuals[index]
  if (!layer || (layer.kind !== 'video' && layer.kind !== 'image') || !layer.fx?.shader) throw new Error('着色器图层无效')
  if (!Number.isInteger(frame) || frame < 0 || frame >= layer.f1 - layer.f0) throw new Error('着色器帧无效')
  await checkShaders()
  const { w: width, h: height, shader: use, start } = layer.fx
  const data = Uint8Array.from(atob(rgba), char => char.charCodeAt(0))
  if (data.length !== width * height * 4) throw new Error('着色器输入帧大小无效')
  const pixels = await shaderRunner!.drawToPixels({ kind: 'rgba', data, width, height }, {
    width, height, code: shaderModuleCode(use.def), params: shaderParamBytes(use.def.params, use.values),
    frame: shaderFrameBytes(width, height, shaderTime(layer.f0 + frame, start, planned.fps), false),
  })
  // Avoid exceeding the argument stack on full-resolution pictures.
  const chunks: string[] = []
  for (let at = 0; at < pixels.length; at += 32768) chunks.push(String.fromCharCode(...pixels.subarray(at, at + 32768)))
  return btoa(chunks.join(''))
}

// Resume the upstream loudness algorithm after each Rust-owned FFmpeg measurement.
export function prepareLoudness(project:Project,paths:Record<string,string>,analysis:Record<string,{hasAudio:boolean;envelope?:unknown}>={}){
  const problems=validateProject(project,builtinModules)
  if(problems.length)throw new Error(problems.map(p=>p.message).join('; '))
  planned=planExport(project,{...outputSize(project.canvas,'draft'),mediaPath:media=>{const path=paths[media.id];if(!path)throw new Error(`素材不可读取：${media.id}`);return path},graphic:()=>'<div></div>',envelopes:new Map(Object.entries(analysis).map(([id,result])=>[id,result.hasAudio?(decodeEnvelope(result.envelope)??null):null]))})
  if('problem' in planned)throw new Error(planned.problem)
  return {ready:true}
}
export function measure(target: number, stderr?: string, preview=false) {
  if (!planned || 'problem' in planned) throw new Error('尚未生成导出计划')
  if (stderr === undefined) {
    measuring = {}
    const next=(pass:Parameters<Parameters<typeof planLoudness>[2]>[0])=>new Promise<string>(resolve => {
      measuring = { pass }; acceptMeasure = resolve
    })
    if(preview)void loudnessGain(planned,target,next).then(value=>{measuring={done:true,result:value}},error=>{measuring={error:String(error)}})
    else void planLoudness(planned,target,next).then(value => { gain = value; measuring = { done: true } }, error => { measuring = { error: String(error) } })
  } else {
    const accept = acceptMeasure
    acceptMeasure = undefined
    measuring = {}
    accept?.(stderr)
  }
  return Promise.resolve().then(() => Promise.resolve()).then(() => measuring)
}

/** The same bundled fonts and layout CSS as the live preview. No remote assets. */
export async function draw(html: string, fonts: Record<string, string>) {
  document.head.innerHTML = ''
  document.body.replaceChildren()
  const outer=document.createElement('style');outer.textContent='html,body{margin:0;padding:0;background:transparent;overflow:hidden}iframe{display:block;border:0;position:absolute;inset:0;width:100vw;height:100vh}';document.head.append(outer)
  // Raster policy belongs to its script-free frame. A CSP on the parent would persist after
  // head replacement and block the animated graphic frames rendered later in the same job.
  const frame=document.createElement('iframe');frame.sandbox.add('allow-same-origin');frame.referrerPolicy='no-referrer'
  const css='html,body{margin:0;width:100%;height:100%;overflow:hidden;background:transparent}body{position:relative;container-type:size}'+RASTER_CSS+fontFaceCss(file=>fonts[file]??'')+fontFaceRules(renderAssets.fonts)
  await new Promise<void>((resolve,reject)=>{
    const timer=setTimeout(()=>reject(new Error('静态图层加载超时')),15000)
    frame.onload=()=>{clearTimeout(timer);resolve()};frame.onerror=()=>{clearTimeout(timer);reject(new Error('静态图层加载失败'))}
    frame.srcdoc=`<!doctype html><html><head><meta charset="utf-8"><meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src data:; font-src data:; style-src 'unsafe-inline'; script-src 'none'; base-uri 'none'; form-action 'none'"><style>${css}</style></head><body>${html}</body></html>`
    document.body.append(frame)
  })
  const content=frame.contentDocument;if(!content)throw new Error('静态图层不可读取')
  await content.fonts.ready
  const invalid=Array.from(content.fonts).filter(font=>font.status==='error');if(invalid.length)throw new Error(`导出字体无法加载：${invalid.map(font=>font.family).join('、')}`)
  await Promise.all(Array.from(content.images).map(image => image.decode()))
  await new Promise<void>(resolve => requestAnimationFrame(() => requestAnimationFrame(() => resolve())))
}
