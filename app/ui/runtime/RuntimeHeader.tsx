'use client'
import type {ReactNode} from 'react'
import Link from 'next/link'
import {Activity} from 'lucide-react'
import {IconCustomerSupport} from '@douyinfe/semi-icons'
import {useMe} from '@/app/lib/use-me'
import PageHeader from '@/app/(app)/components/PageHeader'
import styles from './runtime.module.scss'

export default function RuntimeHeader({active,actions}:{active:'status'|'logs';actions?:ReactNode}){
 const {can}=useMe()
 return <><PageHeader title="运行状态" icon={<Activity size={24}/>} actions={actions}/><nav className={styles.tabs} aria-label="运行状态">
  {can('streamer.view')&&<Link href="/runtime" prefetch={false} aria-current={active==='status'?'page':undefined}><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d="M4 5h16M4 12h16M4 19h10M18 15l3 3-3 3"/></svg>状态信息</Link>}
  {can('log.view')&&<Link href="/runtime/logs" prefetch={false} aria-current={active==='logs'?'page':undefined}><IconCustomerSupport/>实时日志</Link>}
 </nav></>
}
