'use client'
import { Banner, Button, Popconfirm, Spin, Table, Tag, Toast, Tooltip, Typography } from '@douyinfe/semi-ui'
import { IconDeleteStroked } from '@douyinfe/semi-icons'
import Link from 'next/link'
import useSWR from 'swr'
import { fetcher, type StreamerInfo } from '@/app/lib/api-streamer'
import {
  isReadable, replayHref, type SessionDetail, type SessionSummary, type SessionPage, type SessionMergeCandidate,
  sessionUrl, deleteSession, mergeCandidatesUrl, dismissSessionMerge, mergeProblem, formatSpan, formatPrecise,
} from '@/app/lib/sessions'
import { formatSize } from '@/app/lib/use-dashboard'
import { humDate } from '@/app/lib/utils'
import { type AutoClipAvailability, useAutoClip, useSessionJob } from '@/app/lib/auto-clip'
import { JobCell } from '@/app/ui/auto-clip/AutoClipJob'
import { useMe } from '@/app/lib/use-me'
import { ReportedError } from '@/app/lib/markers'
import { useState } from 'react'
import Filter from './Filter'
import { useIsMobile } from '../../lib/useIsMobile'
import dc from '@/app/ui/data-card.module.scss'
import styles from './sessions.module.scss'
import SessionMergeDialog from './SessionMergeDialog'

/** 「直播场次」：每场一行，能按场次回看的点「回看」进回看页 */
export default function SessionsTab() {
  const isMobile = useIsMobile()
  const { Text } = Typography
  const autoClip = useAutoClip()
  const { can } = useMe()
  const canEdit = can('clip.edit')
  const [page, setPage] = useState(1)
  const [pageSize, setPageSize] = useState(20)
  const { data, error, isLoading, mutate } = useSWR<SessionPage>(`/v1/sessions?page=${page}&page_size=${pageSize}`, fetcher, { keepPreviousData: true, refreshInterval: 10_000 })
  const { data: candidateData, error: candidatesError, mutate: reloadCandidates } = useSWR<{ items: SessionMergeCandidate[] }>(canEdit ? mergeCandidatesUrl : null, fetcher, { refreshInterval: 10_000 })
  const [selected, setSelected] = useState<SessionSummary[]>([])
  const [merge, setMerge] = useState<{ ids: number[]; candidate?: SessionMergeCandidate } | null>(null)
  const [dismissing, setDismissing] = useState<number | null>(null)
  const [deleting, setDeleting] = useState<number | null>(null)
  const [legacyOpen, setLegacyOpen] = useState(false)

  const removeSession = async (record: Pick<SessionSummary, 'id'>) => {
    setDeleting(record.id)
    try {
      await deleteSession(record.id)
      await mutate()
      setSelected(previous => previous.filter(source => source.id !== record.id))
      Toast.success({ content: '已删除场次历史', duration: 2 })
    } catch (error) {
      if (!(error instanceof ReportedError)) {
        Toast.error({ content: `删除失败：${error instanceof Error ? error.message : String(error)}`, duration: 4 })
      }
    } finally {
      setDeleting(null)
    }
  }

  const dismiss = async (candidate: SessionMergeCandidate) => {
    setDismissing(candidate.id)
    try {
      await dismissSessionMerge(candidate.id)
      await reloadCandidates()
      Toast.info('已忽略这次续接建议')
    } catch (error) {
      if (!(error instanceof ReportedError)) Toast.error(`忽略失败：${error instanceof Error ? error.message : String(error)}`)
    } finally { setDismissing(null) }
  }

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
      dataIndex: 'streamer_name',
      onFilter: (value: any, record?: SessionSummary) => record?.streamer_name.includes(value) ?? false,
      renderFilterDropdown: Filter,
    },
    {
      title: '标题',
      dataIndex: 'title',
      render: (text: string, record: SessionSummary) => (
        <div className={styles.sessionTitle}>
          <Text strong ellipsis={{ showTooltip: true }} style={{ maxWidth: 340 }}>{text || '无标题'}</Text>
          {record.kind === 'group' ? <Tag size="small" color="blue">合并 · {record.source_session_ids?.length ?? 0} 场</Tag> : null}
        </div>
      ),
      onFilter: (value: any, record: any) => record.title.includes(value),
      renderFilterDropdown: Filter,
    },
    ...(autoClip.visible
      ? [
          {
            title: '自动切片',
            dataIndex: 'auto_clip',
            render: (_: unknown, record: SessionSummary) => (
              <AutoClipCell record={record} availability={autoClip} />
            ),
          },
        ]
      : []),
    ...(isMobile
      ? []
      : [
          {
            title: '时长 / 分段',
            dataIndex: 'duration_ms',
            render: (_: unknown, record: SessionSummary) => `${formatSpan(record.duration_ms)} · ${record.segment_count} 段`,
          },
          {
            title: '录像大小',
            dataIndex: 'bytes',
            render: (bytes: number) => formatSize(bytes),
          },
        ]),
    {
      title: '开播时间',
      dataIndex: 'started_at',
      render: (time: number) => new Date(time).toLocaleString(),
    },
    {
      title: '状态',
      dataIndex: 'recording',
      render: (recording: boolean) => <Tag size="small" color={recording ? 'red' : 'grey'}>{recording ? '录制中' : '已结束'}</Tag>,
    },
    {
      title: '',
      dataIndex: 'replay',
      fixed: 'right' as const,
      width: can('clip.edit') ? 128 : 72,
      render: (_: unknown, record: SessionSummary) =>
        <div className={styles.actions}>
          {record.segment_count > 0 || record.kind === 'group' ? (
            <Link href={replayHref(record.id)} className={styles.replayLink}>
              回看
            </Link>
          ) : (
            <Tooltip content="这一场在升级前录制，没有时间轴，不能按场次回看；请在「录制文件」里按文件播放">
              <Text type="tertiary" className={styles.replayOff}>
                回看
              </Text>
            </Tooltip>
          )}
          {can('clip.edit') ? (
            <Popconfirm
              title="删除这场直播历史？"
              content={record.kind === 'group'
                ? '删除合并场次及其新建工作台记录，原始场次、录像和旧工作台记录保留。正在录制或有未完成任务时无法删除。'
                : '会删除回看时间轴、标记和切片记录；原始录像、切片与合成文件保留在磁盘。正在录制或有未完成任务时无法删除。'}
              okText="删除历史"
              okType="danger"
              onConfirm={() => removeSession(record)}
            >
              <Button
                size="small"
                theme="borderless"
                type="danger"
                icon={<IconDeleteStroked />}
                aria-label={`删除 ${record.streamer_name} 的场次历史`}
                loading={deleting === record.id}
                disabled={deleting !== null}
              />
            </Popconfirm>
          ) : null}
        </div>,
    },
  ]

  return (
    <div className={styles.sessionPage}>
      {candidateData?.items.length ? <div className={dc.card}>
        <div className={styles.mergeToolbar}><strong>场次续接建议</strong><Text type="tertiary" size="small">同一主播停录后重新开播，可确认合并到完整工作台</Text></div>
        <div className={styles.candidates}>
          {candidateData.items.map(candidate => <div key={candidate.id} className={styles.candidate}>
            <div className={styles.candidateBody}>
              <strong>{candidate.streamer_name} · 停录 {formatSpan(candidate.gap_ms)}</strong>
              <span>{candidate.previous_title || '无标题'} → {candidate.source_title || '无标题'}</span>
              <Text type="tertiary" size="small">{new Date(candidate.previous_started_at).toLocaleString()} → {new Date(candidate.source_started_at).toLocaleString()}{candidate.recipe_diff ? ' · 压制配方不同' : ''}</Text>
            </div>
            <Button size="small" theme="solid" onClick={() => setMerge({ ids: [candidate.group_session_id ?? candidate.previous_session_id, candidate.source_session_id], candidate })}>查看并合并</Button>
            <Button size="small" theme="borderless" type="tertiary" disabled={dismissing !== null} loading={dismissing === candidate.id} onClick={() => dismiss(candidate)}>忽略</Button>
          </div>)}
        </div>
      </div> : null}
      {candidatesError ? <Banner type="warning" description="续接建议加载失败，稍后会自动重试" /> : null}
      <div className={dc.card}>
      {canEdit ? <div className={styles.mergeToolbar}>
        <Text type="tertiary">已选择 {selected.length} 场</Text>
        <Tooltip content={mergeProblem(selected) ?? '预览顺序、停录缺口和压制配方后合并'}>
          <span><Button size="small" disabled={!!mergeProblem(selected)} onClick={() => setMerge({ ids: selected.map(source => source.id) })}>合并所选场次</Button></span>
        </Tooltip>
        {selected.length ? <Button size="small" theme="borderless" type="tertiary" onClick={() => setSelected([])}>清空选择</Button> : null}
      </div> : null}
      <Table
        size="small"
        rowKey="id"
        scroll={{ x: 'max-content' }}
        columns={columns}
        dataSource={data?.items ?? []}
        rowSelection={canEdit ? {
          selectedRowKeys: selected.map(source => source.id),
          getCheckboxProps: (record: SessionSummary) => ({ disabled: record.kind === 'group' || record.group_session_id != null }),
          onChange: (keys, rows) => {
            const byId = new Map([...selected, ...rows as SessionSummary[]].map(source => [source.id, source]))
            setSelected((keys ?? []).map(id => byId.get(Number(id))).filter((source): source is SessionSummary => !!source))
          },
        } : undefined}
        pagination={{ currentPage: page, pageSize, total: data?.total ?? 0, showSizeChanger: true, pageSizeOpts: [10, 20, 50, 100],
          onPageChange: setPage, onPageSizeChange: size => { setPageSize(size); setPage(1) } }}
        expandedRowRender={expandRowRender}
        empty={error ? '加载失败，请检查后端连接' : '暂无数据'}
      />
      </div>
      <details className={styles.legacyHistory} onToggle={event => setLegacyOpen(event.currentTarget.open)}>
        <summary>无时间轴的场次历史</summary>
        {legacyOpen ? <LegacySessions deleting={deleting} canEdit={canEdit} onDelete={removeSession} /> : null}
      </details>
      {merge ? <SessionMergeDialog key={`${merge.ids.join(',')}:${merge.candidate?.id ?? 'manual'}`} sourceIds={merge.ids} candidate={merge.candidate} canReadRecipe={can('file.view')}
        onClose={() => setMerge(null)} onMerged={id => {
          setMerge(null); setSelected([]); void mutate(); void reloadCandidates()
          Toast.success({ content: <span>已合并场次 · <Link href={replayHref(id)} className={styles.replayLink}>进入完整工作台</Link></span>, duration: 5 })
        }} /> : null}
    </div>
  )
}

/** 保留升级前的场次和文件入口；这类记录不能进入虚拟时间轴。 */
function LegacySessions({ canEdit, deleting, onDelete }: {
  canEdit: boolean; deleting: number | null; onDelete: (record: Pick<SessionSummary, 'id'>) => Promise<void>
}) {
  const { data, isLoading, error, mutate } = useSWR<StreamerInfo[]>('/v1/streamer-info', fetcher)
  return <div className={styles.legacyTable}>
    <Typography.Text type="tertiary" size="small">这些场次没有回看时间轴，可展开查找原始录制文件；写出时间轴后会显示在上方列表。</Typography.Text>
    <Table size="small" rowKey="id" loading={isLoading} dataSource={data?.filter(record => !record.has_timeline) ?? []}
      columns={[
        { title: '名称', dataIndex: 'name' },
        { title: '标题', dataIndex: 'title' },
        { title: '日期', dataIndex: 'date', render: (date: number) => humDate(date) },
        { title: '', dataIndex: 'actions', render: (_: unknown, record: StreamerInfo) => canEdit ? <Popconfirm
          title="删除这场直播历史？" content="原始录像文件保留在磁盘。" okText="删除历史" okType="danger"
          onConfirm={async () => { await onDelete(record); await mutate() }}>
          <Button size="small" theme="borderless" type="danger" icon={<IconDeleteStroked />} disabled={deleting !== null} loading={deleting === record.id} aria-label={`删除 ${record.name} 的旧场次历史`} />
        </Popconfirm> : null },
      ]}
      expandedRowRender={record => record ? <LegacyFiles sessionId={record.id} /> : null}
      empty={error ? '读取场次历史失败' : '没有无时间轴的场次'} />
  </div>
}

function LegacyFiles({ sessionId }: { sessionId: number }) {
  const { data, isLoading, error } = useSWR<{ id: number; file: string }[]>(`/v1/streamer-info/files/${sessionId}`, fetcher)
  return <div className={styles.legacyFiles}>{isLoading ? '正在读取文件…' : error ? '文件列表读取失败' : data?.length ? data.map(file => <div key={file.id}>{file.file}</div>) : '暂无文件'}</div>
}

/** 只在当前页渲染，当前页各行的任务合成一个请求；没有时间轴的场次不能生成 */
function AutoClipCell({ record, availability }: { record: SessionSummary; availability: AutoClipAvailability }) {
  const { data, isLoading } = useSessionJob(record.kind !== 'group' && record.segment_count > 0 ? record.id : null)
  if (record.kind === 'group') {
    return <TextForGroup />
  }
  if (record.segment_count === 0) {
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

function TextForGroup() {
  return <Tooltip content="在完整工作台查看各来源候选；生成任务按原场次运行"><Typography.Text type="tertiary">来源候选</Typography.Text></Tooltip>
}

// 展开子行:该次直播的录制文件列表；有时间轴的场次，能回看的文件可以直接定位到这一段
const FileLists = ({ record }: { record: SessionSummary }) => {
  const { data: detail, isLoading } = useSWR<SessionDetail>(
    sessionUrl(record.id),
    fetcher,
    {
      revalidateOnFocus: false,
      shouldRetryOnError: false,
    }
  )

  if (isLoading) return <div>加载中...</div>
  if (!detail) return <div>读取场次详情失败</div>

  return (
    <div style={{ padding: '4px 8px', fontSize: 13, color: 'var(--semi-color-text-1)' }}>
      {detail.kind === 'group' ? <div className={styles.members}>
        <strong>来源场次</strong>
        {detail.members?.map(member => <div key={member.source_session_id}>
          <Link href={replayHref(member.source_session_id)} className={styles.replayLink}>原场次 #{member.source_session_id}</Link>
          <span>{member.title || '无标题'} · 合并位置 {formatPrecise(member.offset_ms)}{member.gap_before_ms > 0 ? ` · 前有 ${formatSpan(member.gap_before_ms)} 缺口` : ''}</span>
        </div>)}
      </div> : null}
      文件列表:
      {detail.segments.map((segment) => {
        const start = isReadable(segment) ? segment.start_ms : undefined
        return (
          <div
            key={segment.id}
            style={{ padding: '2px 0 2px 24px', fontVariantNumeric: 'tabular-nums' }}
          >
            {baseName(segment.file_name)}
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
