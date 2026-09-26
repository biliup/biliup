'use client'
import { Suspense, useEffect, useState } from 'react'
import { useRouter, useSearchParams } from 'next/navigation'
import { Button, Empty, Spin } from '@douyinfe/semi-ui'
import { IconLive } from '@douyinfe/semi-icons'
import PageHeader from '../components/PageHeader'
import LiveView from '@/app/ui/LiveView'
import styles from '@/app/ui/live-view.module.scss'

/** 静态导出不支持动态路由，直播间走查询串：`/live?streamer=<id>` */
export default function LivePage() {
  return (
    <Suspense
      fallback={
        <div className={styles.center}>
          <Spin size="large" />
        </div>
      }
    >
      <LiveRoute />
    </Suspense>
  )
}

function LiveRoute() {
  const params = useSearchParams()
  const router = useRouter()
  const raw = params.get('streamer')
  const id = raw && /^\d+$/.test(raw) ? Number(raw) : null
  // 从往返缓存（bfcache）恢复时，pagehide 已经把这一页的中转连接全部释放了：重新挂载，拿新的连接
  const [restored, setRestored] = useState(0)
  useEffect(() => {
    const onPageShow = (e: PageTransitionEvent) => {
      if (e.persisted) setRestored((n) => n + 1)
    }
    window.addEventListener('pageshow', onPageShow)
    return () => window.removeEventListener('pageshow', onPageShow)
  }, [])
  if (id === null) {
    return (
      <>
        <PageHeader icon={<IconLive size="large" />} title="直播预览" />
        <div className={styles.center}>
          <Empty title="没有指定直播间" description="在控制台或直播管理的主播卡片上点「预览」进入">
            <Button theme="solid" onClick={() => router.push('/')}>
              去控制台
            </Button>
          </Empty>
        </div>
      </>
    )
  }
  return <LiveView key={`${id}-${restored}`} streamerId={id} />
}
