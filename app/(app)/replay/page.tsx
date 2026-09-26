'use client'
import { Suspense } from 'react'
import { useRouter, useSearchParams } from 'next/navigation'
import { Button, Empty, Spin } from '@douyinfe/semi-ui'
import { IconHistory } from '@douyinfe/semi-icons'
import PageHeader from '../components/PageHeader'
import ReplayView from '@/app/ui/replay/ReplayView'
import styles from '@/app/ui/replay/replay.module.scss'

/** 静态导出不支持动态路由，场次、起始位置和要选中的切片走查询串：`/replay?session=<id>&t=<场次毫秒>&clip=<切片 id>` */
export default function ReplayPage() {
  return (
    <Suspense
      fallback={
        <div className={styles.center}>
          <Spin size="large" />
        </div>
      }
    >
      <ReplayRoute />
    </Suspense>
  )
}

function ReplayRoute() {
  const params = useSearchParams()
  const router = useRouter()
  const raw = params.get('session')
  const id = raw && /^\d+$/.test(raw) ? Number(raw) : null
  const t = params.get('t')
  const initialT = t && /^\d+$/.test(t) ? Number(t) : null
  const c = params.get('clip')
  const initialClip = c && /^\d+$/.test(c) ? Number(c) : null
  if (id === null) {
    return (
      <>
        <PageHeader icon={<IconHistory size="large" />} title="录像回看" />
        <div className={styles.center}>
          <Empty
            title="没有指定场次"
            description="在「直播历史」里点某一场的「回看」，或在「历史记录」里点文件的「按场次回看」进入"
          >
            <Button theme="solid" onClick={() => router.push('/job')}>
              去直播历史
            </Button>
          </Empty>
        </div>
      </>
    )
  }
  return (
    <ReplayView
      key={`${id}-${initialT ?? ''}-${initialClip ?? ''}`}
      sessionId={id}
      initialT={initialT}
      initialClip={initialClip}
    />
  )
}
