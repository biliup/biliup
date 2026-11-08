'use client'
import {Monitor} from 'lucide-react'
import LiveMonitor from '@/app/ui/LiveMonitor'
import PageHeader from '../components/PageHeader'
import styles from '@/app/ui/data-card.module.scss'

export default function MonitorPage(){
  return <>
    <PageHeader icon={<Monitor size={24}/>} title="监控大屏"/>
    <div className={styles.content}><LiveMonitor/></div>
  </>
}
