'use client'
import {Suspense} from 'react'
import {useSearchParams} from 'next/navigation'
import dynamic from 'next/dynamic'
import {parseWorkbenchNumber} from '@/app/lib/workbench'
const WorkbenchEditor=dynamic(()=>import('@/app/ui/editor/WorkbenchEditor'),{ssr:false})
export default function Workbench(){return <Suspense><Workspace/></Suspense>}
function Workspace(){
 const params=useSearchParams()
 return <WorkbenchEditor sourceId={parseWorkbenchNumber(params.get('session'),1)} selectionId={parseWorkbenchNumber(params.get('selection'),1)} projectId={parseWorkbenchNumber(params.get('project'),1)} active initialT={parseWorkbenchNumber(params.get('t'))} initialEnd={parseWorkbenchNumber(params.get('end'))}/>
}
