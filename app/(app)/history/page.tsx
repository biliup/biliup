'use client'
import { Suspense } from 'react'
import { usePathname, useRouter, useSearchParams } from 'next/navigation'
import { Spin, Tabs, TabPane } from '@douyinfe/semi-ui'
import { IconHistory } from '@douyinfe/semi-icons'
import { type HistoryTab, parseHistoryTab } from '@/app/lib/history'
import PageHeader from '../components/PageHeader'
import LiveMonitor from '@/app/ui/LiveMonitor'
import SessionsTab from './SessionsTab'
import FilesTab from './FilesTab'
import dc from '@/app/ui/data-card.module.scss'
import styles from './page.module.scss'

/**
 * 历史记录：「直播场次」（每场一行，从这里进回看页）、「录制文件」（按文件回放）、
 * 「实时监视」（正在录制的直播间多路同屏）三个 Tab，当前 Tab 在查询串 `?tab=` 里。
 * 旧的「直播历史」`/job` 重定向到 `?tab=sessions`。
 */
export default function History() {
  return (
    <Suspense
      fallback={
        <div style={{ padding: '80px 0', textAlign: 'center' }}>
          <Spin size="large" />
        </div>
      }
    >
      <HistoryTabs />
    </Suspense>
  )
}

function HistoryTabs() {
  const router = useRouter()
  const pathname = usePathname()
  const params = useSearchParams()
  const tab = parseHistoryTab(params.get('tab'))

  const switchTab = (key: string) => {
    const next = new URLSearchParams(params.toString())
    next.set('tab', key)
    router.replace(`${pathname}?${next.toString()}`, { scroll: false })
  }

  return (
    <>
      <PageHeader
        icon={<IconHistory size="large" />}
        title="历史记录"
        description="按场次回看、打标记和剪切片，或按文件在线回放；「实时监视」同屏查看正在录制的直播间"
      />
      <div className={dc.content}>
        {/* keepDOM + lazyRender：场次和文件两个列表切走再切回，筛选、排序、分页、展开行都还在 */}
        <Tabs
          type="line"
          activeKey={tab}
          onChange={switchTab}
          className={styles.tabs}
          keepDOM
          lazyRender
        >
          <TabPane tab="直播场次" itemKey={'sessions' satisfies HistoryTab}>
            <SessionsTab />
          </TabPane>
          <TabPane tab="录制文件" itemKey={'files' satisfies HistoryTab}>
            <FilesTab />
          </TabPane>
          <TabPane tab="实时监视" itemKey={'monitor' satisfies HistoryTab}>
            {/* 切走即卸载播放器、断开全部预览连接 */}
            {tab === 'monitor' ? <LiveMonitor /> : null}
          </TabPane>
        </Tabs>
      </div>
    </>
  )
}
