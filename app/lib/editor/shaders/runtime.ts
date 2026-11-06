// Shared authoring validation and the same WebGPU runner used by preview/export.
import {buildShader} from './build'
import {samplePicture} from './sample'
import {shaderModuleCode,shaderFrameBytes,shaderParamBytes,shaderValues,type ShaderDef,type ShaderParam} from '../core/visual/shaders'
import {ShaderRunner} from '@/app/ui/editor/preview/shaderRunner'
export function prepareShader(input:{name:string;title?:string;description?:string;params?:ShaderParam[];body:string}){
  if(!/^[a-z0-9][a-z0-9-]{0,40}$/.test(input.name))throw new Error('特效标识需为 1–41 个小写字母、数字或连字符')
  const params=input.params??[],built=buildShader(params,input.body)
  if('problems'in built)throw new Error(built.problems.join('\n'))
  const def:ShaderDef={id:`project.${input.name}`,name:input.title?.trim()||input.name.replace(/-/g,' ').replace(/^./,c=>c.toUpperCase()),...(input.description?.trim()?{description:input.description.trim()}:{}),params,wgsl:built.wgsl}
  return def
}
export async function tryShader(def:ShaderDef){
  const runner=await ShaderRunner.create()
  if('problem'in runner)return {tested:false,note:`GPU preview unavailable: ${runner.problem}`}
  try{
    const width=480,height=270
    const rgba=await runner.drawToPixels({kind:'rgba',data:samplePicture(width,height),width,height},{width,height,code:shaderModuleCode(def),frame:shaderFrameBytes(width,height,.5,false),params:shaderParamBytes(def.params,shaderValues(def.params,{}))})
    const canvas=document.createElement('canvas');canvas.width=width;canvas.height=height
    const ctx=canvas.getContext('2d');if(!ctx)throw new Error('无法绘制特效预览')
    ctx.putImageData(new ImageData(new Uint8ClampedArray(rgba),width,height),0,0)
    return {tested:true,width,height,jpeg:canvas.toDataURL('image/jpeg',.9).split(',')[1]}
  }finally{runner.dispose()}
}
