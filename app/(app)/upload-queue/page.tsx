'use client'
import { useCallback, useEffect, useRef, useState } from 'react'
import { Button, Spin } from '@douyinfe/semi-ui'
import { IconCloudStroked } from '@douyinfe/semi-icons'
import PageHeader from '../components/PageHeader'
import { useMe } from '@/app/lib/use-me'
import {
  fetchUploadQueue, formatUploadSpeed, type UploadQueuePage,
} from '@/app/lib/upload-queue'
import { UploadTaskList } from '@/app/ui/upload-queue/UploadTaskList'
import styles from '@/app/ui/upload-queue/upload-queue.module.scss'
import dc from '@/app/ui/data-card.module.scss'

const PAGE_SIZE = 25

export default function UploadQueuePage() {
  const { can } = useMe()
  const canView = can('file.view')
  const [source, setSource] = useState('')
  const [state, setState] = useState('')
  const [offset, setOffset] = useState(0)
  const [data, setData] = useState<UploadQueuePage | null>(null)
  const [loading, setLoading] = useState(false)
  const [connectionError, setConnectionError] = useState('')
  const [visible, setVisible] = useState(true)
  const hasLoaded = useRef(false)

  const load = useCallback(async (initial = false) => {
    if (!canView) return
    if (initial && !hasLoaded.current) setLoading(true)
    try {
      const next = await fetchUploadQueue({ source, state, limit: PAGE_SIZE, offset })
      setData(next)
      hasLoaded.current = true
      setConnectionError('')
    } catch (error) {
      setConnectionError(error instanceof Error ? error.message : String(error))
    } finally { setLoading(false) }
  }, [canView, source, state, offset])

  useEffect(() => {
    const onVisibility = () => setVisible(document.visibilityState === 'visible')
    onVisibility()
    document.addEventListener('visibilitychange', onVisibility)
    return () => document.removeEventListener('visibilitychange', onVisibility)
  }, [])

  useEffect(() => {
    let stopped = false
    let timer: ReturnType<typeof setTimeout> | undefined
    const poll = async (initial: boolean) => {
      await load(initial)
      if (!stopped && visible) timer = setTimeout(() => poll(false), 1000)
    }
    if (visible) void poll(true)
    return () => { stopped = true; if (timer) clearTimeout(timer) }
  }, [load, visible])

  const changeSource = (value: string) => { setSource(value); setOffset(0) }
  const changeState = (value: string) => { setState(value); setOffset(0) }
  const summary = data?.summary ?? { uploading: 0, queued: 0, attention: 0, speed_bytes_per_sec: 0 }
  const tasks = data?.tasks ?? []
  const total = data?.total ?? 0

  return <>
    <PageHeader icon={<IconCloudStroked size="large" />} title="上传队列" description="查看各投稿来源的任务顺序、上传速度和处理状态" />
    <div className={dc.content}>
      {!canView ? <div className={styles.notice}>需要“文件查看”权限才能查看上传队列。</div> : <>
        <section className={styles.summary} aria-label="上传队列概况">
          <SummaryCard label="上传中" value={summary.uploading} detail="正在传输的任务" tone="blue" />
          <SummaryCard label="等待中" value={summary.queued} detail="等待调度或新分段" tone="grey" />
          <SummaryCard label="需要关注" value={summary.attention} detail="失败、中断或结果待确认" tone="orange" />
          <SummaryCard label="总上传速度" value={formatUploadSpeed(summary.speed_bytes_per_sec)} detail="所有上传任务合计" tone="green" />
        </section>

        {connectionError && <div className={`${styles.notice} ${styles.connectionNotice}`} role="status">
          <strong>队列连接中断</strong><span>{connectionError}。保留上次成功读取的数据，页面会继续重试。</span>
          <Button size="small" onClick={() => void load(true)}>立即重试</Button>
        </div>}
        {data?.error && <div className={`${styles.notice} ${styles.persistenceNotice}`} role="status">
          <strong>监控记录异常</strong><span>{data.error}</span>
        </div>}

        <section className={styles.panel}>
          <div className={styles.toolbar}>
            <label>来源<select value={source} onChange={event => changeSource(event.target.value)}>
              <option value="">全部来源</option><option value="automatic">自动投稿</option><option value="manual">手动投稿</option>
              <option value="clip">切片发布</option><option value="sync">边录边传</option><option value="ha">HA 投稿</option>
              <option value="ha_append">HA 追加投稿</option><option value="ha_resume">HA 补投</option>
            </select></label>
            <label>状态<select value={state} onChange={event => changeState(event.target.value)}>
              <option value="">全部状态</option><option value="queued">等待中</option><option value="running">上传中</option>
              <option value="retrying">重试中</option><option value="submitting">提交稿件</option><option value="succeeded">成功</option>
              <option value="partial">部分成功</option><option value="failed">失败</option><option value="interrupted">已中断</option>
              <option value="unknown">结果未知</option><option value="cancelled">已取消</option><option value="paused">已暂停</option>
              <option value="skipped">未投稿</option>
            </select></label>
            <span className={styles.polling}>{visible ? '每秒更新' : '切回页面后继续更新'}</span>
          </div>

          {!data && loading ? <div className={styles.loading}><Spin /><span>正在读取上传队列…</span></div>
            : tasks.length > 0 ? <UploadTaskList tasks={tasks} />
              : data ? <div className={styles.empty}><strong>当前没有上传任务</strong><span>调整来源或状态筛选，或等待新投稿任务进入队列。</span></div>
                : connectionError ? <div className={styles.empty}>暂时无法读取上传队列。</div>
                  : <div className={styles.loading}><Spin /></div>}

          <footer className={styles.pager}>
            <span>共 {total} 项 · {tasks.length ? `${offset + 1}–${Math.min(offset + tasks.length, total)}` : '0'}</span>
            <div><Button size="small" disabled={offset === 0} onClick={() => setOffset(Math.max(0, offset - PAGE_SIZE))}>上一页</Button>
              <Button size="small" disabled={offset + PAGE_SIZE >= total} onClick={() => setOffset(offset + PAGE_SIZE)}>下一页</Button></div>
          </footer>
        </section>
      </>}
    </div>
  </>
}

function SummaryCard({ label, value, detail, tone }: { label: string; value: string | number; detail: string; tone: string }) {
  return <div className={`${styles.summaryCard} ${styles[`tone_${tone}`]}`}>
    <span>{label}</span><strong>{value}</strong><small>{detail}</small>
  </div>
}
