import mpegts from 'mpegts.js'

export interface LoaderHooks { onResponse?: (response:Response)=>void }

/** Shared by live viewing and editor media elements; abort always closes the fetch stream. */
export class AbortableFetchLoader extends mpegts.BaseLoader {
  private controller:AbortController|null=null
  private onResponse:LoaderHooks['onResponse']
  _needStash=true
  constructor(_seekHandler:unknown,config:unknown){
    super('abortable-fetch-loader')
    this.onResponse=(config as LoaderHooks|null)?.onResponse
  }
  static isSupported(){return typeof self.fetch==='function'&&typeof self.ReadableStream==='function'}
  destroy(){this.abort();super.destroy()}
  open(dataSource:{url:string;withCredentials?:boolean},range:{from:number;to:number}){
    this.abort()
    const controller=new AbortController();this.controller=controller
    const active=()=>this.controller===controller&&!controller.signal.aborted
    const ranged=range.from>0||range.to>=0
    this._status=mpegts.LoaderStatus.kConnecting
    void fetch(dataSource.url,{
      method:'GET',mode:'cors',cache:'no-store',credentials:dataSource.withCredentials?'include':'same-origin',signal:controller.signal,
      ...(ranged?{headers:{Range:`bytes=${range.from}-${range.to>=0?range.to:''}`}}:{}),
    }).then(async response=>{
      if(!active())return
      this.onResponse?.(response)
      if(!active())return
      if(!response.ok||!response.body||(ranged&&response.status!==206)){
        const body=response.ok?'':await response.text().catch(()=>'')
        if(!active())return
        this._status=mpegts.LoaderStatus.kError
        controller.abort()
        this.onError?.(mpegts.LoaderErrors.HTTP_STATUS_CODE_INVALID as unknown as mpegts.LoaderErrors,{code:response.status,msg:body.trim()||(ranged&&response.status===200?'The server did not honor the media byte range.':response.statusText)})
        return
      }
      const contentRange=response.headers.get('Content-Range')
      const total=contentRange?.match(/^bytes\s+\d+-\d+\/(\d+)$/i)?.[1]??(response.status===200?response.headers.get('Content-Length'):null)
      if(total&&Number.isSafeInteger(Number(total)))this.onContentLengthKnown?.(Number(total))
      if(!active())return
      const reader=response.body.getReader()
      let received=0
      for(;;){
        const {done,value}=await reader.read()
        if(!active())return
        if(done){this._status=mpegts.LoaderStatus.kComplete;this.onComplete?.(range.from,range.from+received-1);return}
        this._status=mpegts.LoaderStatus.kBuffering
        const chunk=value.buffer.slice(value.byteOffset,value.byteOffset+value.byteLength) as ArrayBuffer
        const at=range.from+received;received+=chunk.byteLength
        this.onDataArrival?.(chunk,at,received)
      }
    }).catch((error:Error&{code?:number})=>{
      if(!active())return
      this._status=mpegts.LoaderStatus.kError
      controller.abort()
      const kind=error.name==='TypeError'||error.code===19?mpegts.LoaderErrors.EARLY_EOF:mpegts.LoaderErrors.EXCEPTION
      this.onError?.(kind as unknown as mpegts.LoaderErrors,{code:error.code??-1,msg:error.message})
    })
  }
  abort(){this._status=mpegts.LoaderStatus.kComplete;this.controller?.abort();this.controller=null}
}

export function destroyMpegts(player:mpegts.Player){
  // Unload first: SourceBuffer teardown can fail after a decoding error.
  try{player.unload()}catch{}
  try{player.detachMediaElement()}catch{}
  try{player.destroy()}catch{}
}

export {mpegts}
