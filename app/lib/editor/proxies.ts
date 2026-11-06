import {apiFetch,handleResponse} from '../api-streamer'
import type {ProxyState} from './contracts'
import type {EditorStore} from './project-store'

/** One project's background preview cache. Closing a tab does not cancel another user's work. */
export class ProjectProxies {
  private timer?:ReturnType<typeof setTimeout>
  private pending?:Promise<void>
  private dirty=true
  private closed=false
  private lastSync=0
  private abort=new AbortController()
  constructor(private id:number,private store:EditorStore,private canEdit:()=>boolean){}
  private url(){return `/v1/editor/projects/${this.id}/proxies`}
  private signal(){return AbortSignal.any([this.abort.signal,AbortSignal.timeout(30000)])}
  async list():Promise<ProxyState[]>{
    const response=await apiFetch(this.url(),{signal:this.signal()})
    await handleResponse(response)
    return response.json()
  }
  refresh(changed=false):Promise<void>{
    if(this.closed)return Promise.resolve()
    if(changed)this.dirty=true
    if(this.pending)return this.pending
    clearTimeout(this.timer)
    this.pending=this.poll().finally(()=>{
      this.pending=undefined
      if(!this.closed)this.timer=setTimeout(()=>void this.refresh(),this.dirty?1000:2000)
    })
    return this.pending
  }
  private async poll(){
    try{
      const sync=this.canEdit()&&(this.dirty||Date.now()-this.lastSync>=15000)
      this.dirty=false
      const response=await apiFetch(this.url(),{method:sync?'POST':'GET',signal:this.signal()})
      await handleResponse(response)
      const proxies=await response.json() as ProxyState[]
      if(sync)this.lastSync=Date.now()
      if(!this.closed&&JSON.stringify(proxies)!==JSON.stringify(this.store.getSnapshot().proxies))this.store.set({proxies})
    }catch{/* Original media remains usable; explicit cancel/retry actions report errors. */}
  }
  async action(mediaId:string,kind:'cancel'|'retry'){
    if(this.closed)throw new Error('工程已关闭')
    if(!this.canEdit())throw new Error('当前账号没有剪辑权限')
    const response=await apiFetch(`${this.url()}/${encodeURIComponent(mediaId)}/${kind}`,{method:'POST',signal:this.signal()})
    await handleResponse(response)
    // Wait for any older poll before publishing the response to this action.
    await this.pending
    const proxies=await this.list()
    if(!this.closed)this.store.set({proxies})
    return proxies
  }
  dispose(){this.closed=true;this.abort.abort();clearTimeout(this.timer)}
}
