/** Capture permissions belong to this browser and the current biliup origin. */
export type MediaAccessKind = 'microphone' | 'camera' | 'screen'
export type MediaAccess = 'granted' | 'denied' | 'restricted' | 'unknown'

async function permission(kind:MediaAccessKind):Promise<MediaAccess> {
 if(!globalThis.isSecureContext||!navigator.mediaDevices||typeof MediaRecorder==='undefined')return 'restricted'
 if(kind==='screen')return typeof navigator.mediaDevices.getDisplayMedia==='function'?'unknown':'restricted'
 if(!navigator.mediaDevices.getUserMedia)return 'restricted'
 try {
  const status=await navigator.permissions.query({name:kind as PermissionName})
  return status.state==='prompt'?'unknown':status.state
 }catch{return 'unknown'}
}

export async function captureAccess():Promise<Readonly<Record<MediaAccessKind,MediaAccess>>> {
 const [microphone,camera,screen]=await Promise.all([permission('microphone'),permission('camera'),permission('screen')])
 return {microphone,camera,screen}
}

/** Observe changes without opening permission prompts or acquiring a device. */
export function watchCaptureAccess(refresh:()=>void) {
 let closed=false
 const cleanups:Array<()=>void>=[]
 window.addEventListener('focus',refresh)
 navigator.mediaDevices?.addEventListener('devicechange',refresh)
 for(const kind of ['microphone','camera'] as const){
  if(!navigator.permissions)break
  void navigator.permissions.query({name:kind as PermissionName}).then(status=>{
   if(closed)return
   status.addEventListener('change',refresh)
   cleanups.push(()=>status.removeEventListener('change',refresh))
  }).catch(()=>{})
 }
 return()=>{
  closed=true
  window.removeEventListener('focus',refresh)
  navigator.mediaDevices?.removeEventListener('devicechange',refresh)
  for(const cleanup of cleanups)cleanup()
 }
}
