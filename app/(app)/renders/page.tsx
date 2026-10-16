'use client'
import { useState } from 'react'
import { Button, Spin, Toast } from '@douyinfe/semi-ui'
import { IconVideo } from '@douyinfe/semi-icons'
import PageHeader from '../components/PageHeader'
import { useMe } from '@/app/lib/use-me'
import { useAllRenderJobs } from '@/app/lib/renders'
import GpuMonitorCard from '@/app/ui/renders/GpuMonitorCard'
import RenderJobList from '@/app/ui/renders/RenderJobList'
import styles from '@/app/ui/renders/renders.module.scss'
import dc from '@/app/ui/data-card.module.scss'

const PAGE_SIZE = 25

export default function RenderTasksPage() {
  const { can } = useMe()
  const canView = can('file.view')
  const [state, setState] = useState('')
  const [sessionDraft, setSessionDraft] = useState('')
  const [sessionId, setSessionId] = useState('')
  const [sessionError, setSessionError] = useState('')
  const [offset, setOffset] = useState(0)
  const result = useAllRenderJobs({ state: state || undefined, session_id: sessionId || undefined, limit: PAGE_SIZE, offset }, canView)
  const applySession = () => {
    const value = sessionDraft.trim()
    if (value && (!/^\d+$/.test(value) || !Number.isSafeInteger(Number(value)) || Number(value) <= 0)) {
      const message = '场次 ID 必须是 1 到 Number.MAX_SAFE_INTEGER 之间的正整数。'
      setSessionError(message); Toast.error({ content: message }); return
    }
    setSessionError('')
    setOffset(0); setSessionId(value)
  }
  const total = result.data?.total ?? 0
  if (result.data && offset > 0 && offset >= total) {
    setOffset(Math.max(0, Math.ceil(total / PAGE_SIZE) - 1) * PAGE_SIZE)
  }
  return <>
    <PageHeader icon={<IconVideo size="large" />} title="压制任务" description="查看所有场次的压制进度、结果和所用编码设备" />
    <div className={dc.content}>
      <GpuMonitorCard allowed={canView} />
      <section>
        <div className={styles.filterRow}>
          <label>任务状态<select value={state} onChange={e => { setOffset(0); setState(e.target.value) }}>
            <option value="">全部</option><option value="queued">排队中</option><option value="running">压制中</option>
            <option value="ready">已完成</option><option value="failed">失败</option><option value="cancelled">已取消</option>
          </select></label>
          <label>场次 ID<input aria-invalid={!!sessionError} inputMode="numeric" value={sessionDraft} onChange={e => { setSessionDraft(e.target.value); if (sessionError) setSessionError('') }} onKeyDown={e => { if (e.key === 'Enter') applySession() }} placeholder="全部场次" />{sessionError && <span role="alert">{sessionError}</span>}</label>
          <Button size="small" onClick={applySession}>筛选场次</Button>
          {sessionId && <Button size="small" onClick={() => { setSessionDraft(''); setSessionId(''); setSessionError(''); setOffset(0) }}>清除场次</Button>}
        </div>
        {result.error && <p className={styles.jobError}>任务加载失败：{result.error instanceof Error ? result.error.message : String(result.error)}</p>}
        {!result.data && !result.error ? <div style={{ padding: 32, textAlign: 'center' }}><Spin /></div>
          : result.data && <>
            <RenderJobList jobs={result.data.jobs} canEdit={can('clip.edit')} canDownload={canView} />
            <div className={styles.pager}>
              <span>共 {total} 项 · {result.data.jobs.length ? offset + 1 : 0}–{Math.min(offset + result.data.jobs.length, total)}</span>
              <div className={styles.pagerActions}>
                <Button size="small" disabled={offset === 0} onClick={() => setOffset(Math.max(0, offset - PAGE_SIZE))}>上一页</Button>
                <Button size="small" disabled={offset + PAGE_SIZE >= total} onClick={() => setOffset(offset + PAGE_SIZE)}>下一页</Button>
              </div>
            </div>
          </>}
      </section>
    </div>
  </>
}
