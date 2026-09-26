'use client'
import { Suspense, useEffect, useState } from 'react'
import { useRouter, useSearchParams } from 'next/navigation'
import { Button, Empty, Spin } from '@douyinfe/semi-ui'
import { IconLive } from '@douyinfe/semi-icons'
import useSWR from 'swr'
import PageHeader from '../components/PageHeader'
import { fetcher, LiveStreamerEntity } from '@/app/lib/api-streamer'
import { canPreview, STREAMERS_REFRESH_MS } from '@/app/lib/use-dashboard'
import LiveView from '@/app/ui/LiveView'
import { livePageHref } from '@/app/ui/LivePreview'
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
  if (id === null) return <PickLive />
  return <LiveView key={`${id}-${restored}`} streamerId={id} />
}

/** 没带直播间（例如从侧栏进来）：直接换到第一路能预览的直播间，一路都没有时说明怎么进 */
function PickLive() {
  const router = useRouter()
  const { data: streamers, error } = useSWR<LiveStreamerEntity[]>('/v1/streamers', fetcher, {
    refreshInterval: STREAMERS_REFRESH_MS,
  })
  const first = streamers?.find(canPreview)
  useEffect(() => {
    if (first) router.replace(livePageHref(first.id))
  }, [first, router])
  if (first || (!streamers && !error)) {
    return (
      <div className={styles.center}>
        <Spin size="large" />
      </div>
    )
  }
  return (
    <>
      <PageHeader icon={<IconLive size="large" />} title="直播预览" />
      <div className={styles.center}>
        <Empty
          title={error ? '加载直播间失败' : '现在没有能预览的直播间'}
          description="有直播间开播并开始录制后，从这里进来会直接打开第一路；也可以在控制台的主播卡片上点「预览」"
        >
          <Button theme="solid" onClick={() => router.push('/')}>
            去控制台
          </Button>
        </Empty>
      </div>
    </>
  )
}
