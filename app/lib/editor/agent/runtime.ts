// The assistant and manual editing use the same atomic edit planner and linked audio rules.
import {createHistory,editHistory} from '../history'
import type {Op,Project} from '../core/timeline/types'
import {builtinModules} from '../core/timeline/builtin-modules'
import {describeOp} from '../core/timeline/edit-log'
import {expandShaderEffects,shaderUseOf,type ShaderDef} from '../core/visual/shaders'
import {planImportCanvas} from '../core/timeline/canvasMatch'
import {mediaForCanvas} from '../project'

export function agentEdit(project:Project,input:{ops:readonly Op[];summary?:string;turnId:string;shaders?:readonly ShaderDef[];importDurations?:Readonly<Record<string,number|null>>}){
  if(!Array.isArray(input.ops)||input.ops.length===0||input.ops.length>500)throw new Error('每次编辑需要 1–500 个操作')
  if(typeof input.turnId!=='string'||!input.turnId||input.turnId.length>100)throw new Error('助手轮次标识无效')
  const defs=new Map<string,ShaderDef>()
  for(const track of project.tracks)for(const item of track.items)for(const effect of item.effects??[]){if(effect.module!=='effect.shader')continue;const use=shaderUseOf(effect);if(use)defs.set(use.def.id,use.def)}
  for(const def of input.shaders??[])defs.set(def.id,def)
  const expanded=expandShaderEffects(input.ops,defs,true)
  if('problem'in expanded)throw new Error(expanded.problem)
  let ops=expanded.ops as Op[]
  // This flag is supplied by the registered import service, never by timeline_edit.
  if(input.importDurations && ops.every(op=>op.op==='add_media')){
    const matching=planImportCanvas(project,ops.flatMap(op=>op.op==='add_media'?[op.media]:[]),input.importDurations)
    ops=[...matching.ops,...ops.map(op=>op.op==='add_media'
      ?{...op,media:mediaForCanvas(op.media,input.importDurations![op.media.id]??null,matching.project.canvas)}:op)]
  }
  const history=createHistory(project)
  const next=editHistory(history,{actor:'agent',baseRevision:history.current.revision,turnId:input.turnId,summary:input.summary,ops})
  return {project:next.current.project,ops:ops.length,summary:input.summary||'编辑时间轴',lines:ops.map(op=>describeOp(op,project))}
}
export function agentModules(shaders:readonly ShaderDef[]=[]){
  return [...[...builtinModules.values()].map(module=>({id:module.id,kind:module.kind,name:module.name,...(module.params?{params:module.params}:{})})),...shaders.map(def=>({id:def.id,kind:'effect',name:def.name,description:def.description,params:def.params,apply:{module:'effect.shader',params:{shader:def.id,values:{}}}}))]
}
