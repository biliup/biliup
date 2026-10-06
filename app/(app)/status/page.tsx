'use client'
import { Spin, Typography } from '@douyinfe/semi-ui'
import useSWR from 'swr'
import { fetcher } from '@/app/lib/api-streamer'
import { IconSetting } from '@douyinfe/semi-icons'
import { JSONTree } from 'react-json-tree'
import PageHeader from '../components/PageHeader'
import dc from '@/app/ui/data-card.module.scss'

interface StatusPayload {
  version?: string
  rooms?: unknown[]
  download_semaphore?: number
  update_semaphore?: number
}

export default function Status() {
  const { Text } = Typography
  const { data, error, isLoading } = useSWR<StatusPayload>('/v1/status', fetcher)

  if (isLoading) {
    return (
      <>
        <PageHeader
          icon={<IconSetting size="large" />}
          title="任务平台"
          description="系统运行状态(调试视图)"
        />
        <div style={{ padding: '80px 0', textAlign: 'center' }}>
          <Spin size="large" />
        </div>
      </>
    )
  }

  const rooms = data?.rooms?.length ?? 0

  return (
    <>
      <PageHeader
        icon={<IconSetting size="large" />}
        title="任务平台"
        description="系统运行状态(调试视图)"
      />
      <div className={dc.content}>
        {/* 概览 */}
        <div className={dc.kpiStrip}>
          <span className={dc.kpiItem}>
            版本 <b>{data?.version ?? '—'}</b>
          </span>
          <span className={dc.kpiItem}>
            监控房间 <b>{rooms}</b>
          </span>
          <span className={dc.kpiItem}>
            下载信号量 <b>{data?.download_semaphore ?? '—'}</b>
          </span>
          <span className={dc.kpiItem}>
            上传任务 <b>{data?.update_semaphore ?? '—'}</b>
          </span>
          {error ? (
            <Text type="danger">状态获取失败,请检查后端连接</Text>
          ) : null}
        </div>

        {/* 原始状态(调试) */}
        <div className={dc.card} style={{ padding: 16 }}>
          <Text type="tertiary" size="small" style={{ display: 'block', marginBottom: 8 }}>
            原始 JSON(调试用)
          </Text>
          <div style={{ overflow: 'auto', fontSize: 12.5 }}>
            <JSONTree data={data} />
          </div>
        </div>
      </div>
    </>
  )
}
