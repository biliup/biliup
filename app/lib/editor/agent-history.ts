import {fetcher} from '../api-streamer'
import type {AgentHistoryCursor,AgentState,AgentTurn} from './contracts'

const compare=(a:AgentTurn,b:AgentTurn)=>a.startedAt<b.startedAt?-1:a.startedAt>b.startedAt?1:a.id<b.id?-1:a.id>b.id?1:0
export function mergeAgentTurns(older:ReadonlyArray<AgentTurn>,newer:ReadonlyArray<AgentTurn>,revision:number|undefined,running:string|null):AgentTurn[]{
  const byId=new Map(older.map(turn=>[turn.id,turn]))
  for(const turn of newer)byId.set(turn.id,turn)
  return [...byId.values()].sort(compare).map(turn=>{
    if(turn.undoRevision==null||turn.undoneAt)return turn
    return {...turn,undo:running?{state:'blocked',reason:'请先停止或等待本轮完成'}:turn.undoRevision===revision?{state:'available'}:{state:'blocked',reason:'工程有后续或交错编辑，请在历史中查看版本'}}
  })
}
export function readAgentPage(id:number,signal:AbortSignal,cursor?:AgentHistoryCursor){
  const query=cursor?`?before=${cursor.createdAt}&beforeId=${encodeURIComponent(cursor.id)}`:''
  return fetcher(`/v1/editor/projects/${id}/agent${query}`,{signal}) as Promise<AgentState>
}
/** Fill gaps after reconnecting; polling only the newest page must never erase loaded history. */
export async function readAgentHead(id:number,signal:AbortSignal,current:AgentState|null|undefined){
  const head=await readAgentPage(id,signal)
  // Restoring a branch can change an older turn from undone to present (or back).
  const changedBranch=current&&current.historyBranch!==head.historyBranch
  const known=changedBranch?current?.turns[0]:current?.turns.at(-1)
  let before=head.historyCursor,turns=head.turns
  while(known&&before&&turns[0]&&compare(turns[0],known)>0){
    const page=await readAgentPage(id,signal,before)
    turns=[...page.turns,...turns];before=page.historyCursor
  }
  return {...head,turns,historyCursor:current?.turns.length?current.historyCursor:before}
}
