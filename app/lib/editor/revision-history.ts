import {fetcher} from '../api-streamer'
import type {HistoryEntry,HistoryState} from './contracts'

/** A project's loaded history survives saves and background agent edits. Cursors use revisions. */
export class RevisionHistory {
  private entries:HistoryEntry[]=[]
  private total=0
  private before:number|null=null
  private problem?:string
  private pending:Promise<void>=Promise.resolve()
  constructor(private id:number,private signal:AbortSignal){}
  private enqueue(operation:()=>Promise<void>){
    const task=this.pending.then(operation)
    this.pending=task.catch(()=>{})
    return task
  }
  private page(before?:number){return fetcher(`/v1/editor/projects/${this.id}/history${before===undefined?'':`?before=${before}`}`,{signal:this.signal}) as Promise<HistoryState>}
  private merge(entries:ReadonlyArray<HistoryEntry>){
    const byRevision=new Map(this.entries.map(entry=>[entry.revision,entry]))
    for(const entry of entries)byRevision.set(entry.revision,entry)
    this.entries=[...byRevision.values()].sort((a,b)=>b.revision-a.revision)
  }
  head(reset=false,changedBranch=false){return this.enqueue(()=>this.readHead(reset,changedBranch))}
  private async readHead(reset:boolean,changedBranch:boolean){
    const known=reset?undefined:(changedBranch?this.entries.at(-1)?.revision:this.entries[0]?.revision)
    const first=await this.page()
    const entries=[...first.entries]
    let before=first.before
    // Catch up every intervening revision, even after more than one page of remote edits.
    while(known!==undefined&&before!==null&&(entries.at(-1)?.revision??Infinity)>known){
      const page=await this.page(before);entries.push(...page.entries);before=page.before
    }
    // A save can finish while this page is in flight. Keep its newer revision.
    if(reset||known===undefined){this.entries=this.entries.filter(entry=>entry.revision>first.revision);this.before=before}
    this.merge(entries)
    this.total=first.total+this.entries.filter(entry=>entry.revision>first.revision).length
    this.problem=undefined
  }
  failed(error:unknown){this.problem=error instanceof Error?error.message:String(error)}
  more(){return this.enqueue(()=>this.readMore())}
  private async readMore(){
    if(this.problem){await this.readHead(false,true);return}
    if(this.before===null)return
    const page=await this.page(this.before)
    this.merge(page.entries);this.before=page.before
    this.total=page.total+this.entries.filter(entry=>entry.revision>page.revision).length
  }
  record(entry:HistoryEntry){
    if(!this.entries.some(item=>item.revision===entry.revision))this.total++
    this.merge([entry])
  }
  state(revision:number):HistoryState{
    return {dir:`biliup:${this.id}`,revision,problem:this.problem,total:this.total,before:this.problem?(this.before??revision+1):this.before,entries:this.entries.filter(entry=>entry.revision<=revision).map(entry=>this.problem?{...entry,current:false}:entry)}
  }
}
