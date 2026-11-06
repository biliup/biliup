'use client'
import {Suspense,useEffect} from 'react'
import {useRouter,useSearchParams} from 'next/navigation'
import {parseWorkbenchNumber,recordingEditorHref} from '@/app/lib/workbench'

/** The session entry opens the same project editor. */
function ReplayRoute(){
 const params=useSearchParams(),router=useRouter()
 useEffect(()=>{const id=parseWorkbenchNumber(params.get('session'),1);router.replace(id?recordingEditorHref(id,parseWorkbenchNumber(params.get('t'))):'/sessions')},[params,router])
 return <div role="status">正在打开剪辑…</div>
}
export default function ReplayPage(){return <Suspense><ReplayRoute/></Suspense>}
