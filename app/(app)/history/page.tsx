'use client'
import { Suspense } from 'react'
import { usePathname, useSearchParams } from 'next/navigation'
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
  const pathname = usePathname()
  const params = useSearchParams()
  const tab = parseHistoryTab(params.get('tab'))

  const switchTab = (key: string) => {
    const next = new URLSearchParams(params.toString())
    next.set('tab', key)
    // 不走 router.replace：那是一次异步的软导航，切走「实时监视」时播放器要等导航提交才卸载，
    // 期间 mpegts.js 的销毁会和 MSE 回调撞上。原生 replaceState 由 Next 同步到 useSearchParams
    window.history.replaceState(null, '', `${pathname}?${next.toString()}`)
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
