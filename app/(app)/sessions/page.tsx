'use client'
import {Suspense} from 'react'
import {useSearchParams} from 'next/navigation'
import Link from 'next/link'
import {CalendarDays} from 'lucide-react'
import PageHeader from '../components/PageHeader'
import SessionsTab from '../workbench/SessionsTab'
import {parseWorkbenchNumber} from '@/app/lib/workbench'
import styles from '@/app/ui/data-card.module.scss'

export default function SessionsPage(){return <Suspense><Sessions/></Suspense>}
function Sessions(){
 const params=useSearchParams(),streamerId=parseWorkbenchNumber(params.get('streamer'),1)
 return <><PageHeader title="直播场次" icon={<CalendarDays size={24}/>} actions={streamerId?<Link href="/sessions">全部场次</Link>:undefined}/><div className={styles.content}><SessionsTab key={streamerId??'all'} streamerId={streamerId}/></div></>
}
