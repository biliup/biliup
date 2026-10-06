'use client'
import { Spin, Table, Tooltip, Typography } from '@douyinfe/semi-ui'
import { SortOrder } from '@douyinfe/semi-ui/lib/es/table'
import Link from 'next/link'
import useSWR from 'swr'
import { fetcher, type StreamerInfo } from '@/app/lib/api-streamer'
import { isReadable, replayHref, type SessionDetail, sessionUrl } from '@/app/lib/sessions'
import { humDate } from '@/app/lib/utils'
import { type AutoClipAvailability, useAutoClip, useSessionJob } from '@/app/lib/auto-clip'
import { JobCell } from '@/app/ui/auto-clip/AutoClipJob'
import Filter from './Filter'
import { useIsMobile } from '../../lib/useIsMobile'
import dc from '@/app/ui/data-card.module.scss'
import styles from './sessions.module.scss'

/** 「直播场次」：每场一行，能按场次回看的点「回看」进回看页 */
export default function SessionsTab() {
  const isMobile = useIsMobile()
  const { Text } = Typography
  const { data: data, error, isLoading } = useSWR<any[]>('/v1/streamer-info', fetcher)
  const autoClip = useAutoClip()

  if (isLoading) {
    return (
      <div style={{ padding: '80px 0', textAlign: 'center' }}>
        <Spin size="large" />
      </div>
    )
  }

  const columns = [
    {
      title: '名称',
      dataIndex: 'name',
      onFilter: (value: any, record: any) => record.name.includes(value),
      renderFilterDropdown: Filter,
    },
    {
      title: '标题',
      dataIndex: 'title',
      render: (text: any) => (
        <Text strong style={{ whiteSpace: 'nowrap' }}>
          {text}
        </Text>
      ),
      onFilter: (value: any, record: any) => record.title.includes(value),
      renderFilterDropdown: Filter,
    },
    ...(autoClip.visible
      ? [
          {
            title: '自动切片',
            dataIndex: 'auto_clip',
            render: (_: unknown, record: StreamerInfo) => (
              <AutoClipCell record={record} availability={autoClip} />
            ),
          },
        ]
      : []),
    ...(isMobile
      ? []
      : [
          {
            title: '链接',
            dataIndex: 'url',
          },
          {
            title: '封面',
            dataIndex: 'live_cover_path',
          },
        ]),
    {
      title: '更新日期',
      dataIndex: 'date',
      defaultSortOrder: 'descend' as SortOrder,
      sorter: (a: any, b: any) => (a.date - b.date > 0 ? 1 : -1),
      render: (time: number) => humDate(time),
    },
    {
      title: '',
      dataIndex: 'replay',
      fixed: 'right' as const,
      width: 72,
      render: (_: unknown, record: StreamerInfo) =>
        record.has_timeline ? (
          <Link href={replayHref(record.id)} className={styles.replayLink}>
            回看
          </Link>
        ) : (
          <Tooltip content="这一场在升级前录制，没有时间轴，不能按场次回看；请在「录制文件」里按文件播放">
            <Text type="tertiary" className={styles.replayOff}>
              回看
            </Text>
          </Tooltip>
        ),
    },
  ]

  return (
    <div className={dc.card}>
      <Table
        size="small"
        rowKey="id"
        scroll={{ x: 'max-content' }}
        columns={columns}
        dataSource={data}
        expandedRowRender={expandRowRender}
        empty={error ? '加载失败，请检查后端连接' : '暂无数据'}
      />
    </div>
  )
}

/** 只在当前页渲染，当前页各行的任务合成一个请求；没有时间轴的场次不能生成 */
function AutoClipCell({ record, availability }: { record: StreamerInfo; availability: AutoClipAvailability }) {
  const { data, isLoading } = useSessionJob(record.has_timeline ? record.id : null)
  if (!record.has_timeline) {
    return (
      <Tooltip content="这一场在升级前录制，没有时间轴，不能生成候选">
        <Typography.Text type="tertiary" className={styles.replayOff}>
          —
        </Typography.Text>
      </Tooltip>
    )
  }
  return <JobCell sessionId={record.id} job={data ?? null} availability={availability} loading={isLoading} />
}

const baseName = (path: string) => path.split(/[\\/]/).pop() ?? path

// 展开子行:该次直播的录制文件列表；有时间轴的场次，能回看的文件可以直接定位到这一段
const FileLists = ({ record }: { record: StreamerInfo }) => {
  const { data: files, isLoading } = useSWR(`/v1/streamer-info/files/${record.id}`, fetcher)
  const { data: detail } = useSWR<SessionDetail>(
    record.has_timeline ? sessionUrl(record.id) : null,
    fetcher,
    {
      revalidateOnFocus: false,
      shouldRetryOnError: false,
    }
  )
  const startOf = new Map(
    detail?.segments.filter(isReadable).map(seg => [seg.file_name, seg.start_ms]) ?? []
  )

  if (isLoading) return <div>加载中...</div>
  if (!files || files.length === 0) return <div>暂无文件</div>

  return (
    <div style={{ padding: '4px 8px', fontSize: 13, color: 'var(--semi-color-text-1)' }}>
      文件列表:
      {files.map((it: any) => {
        const start = startOf.get(baseName(it.file))
        return (
          <div
            key={it.id}
            style={{ padding: '2px 0 2px 24px', fontVariantNumeric: 'tabular-nums' }}
          >
            {it.file}
            {start !== undefined ? (
              <Link href={replayHref(record.id, start)} className={styles.fileReplay}>
                回看这一段
              </Link>
            ) : null}
          </div>
        )
      })}
    </div>
  )
}

const expandRowRender = (record: any) => {
  return <FileLists record={record} />
}
