'use client'
import { useState } from 'react'
import { Button, Progress, Toast } from '@douyinfe/semi-ui'
import { apiResourceUrl, renderJobAction, type RenderJob } from '@/app/lib/renders'
import RevealFileButton from '@/app/ui/RevealFileButton'
import styles from './renders.module.scss'

const stateLabels: Record<RenderJob['state'], string> = {
  queued: '排队中', running: '压制中', ready: '已完成', failed: '失败', cancelled: '已取消',
}
const formatTime = (ms: number | null | undefined) => {
  if (ms == null || !Number.isFinite(ms) || ms < 0) return '—'
  const seconds = Math.floor(ms / 1000), hours = Math.floor(seconds / 3600)
  const minutes = Math.floor((seconds % 3600) / 60), rest = seconds % 60
  return hours ? `${hours}:${String(minutes).padStart(2, '0')}:${String(rest).padStart(2, '0')}` : `${minutes}:${String(rest).padStart(2, '0')}`
}
const metric = (value: number | null | undefined, suffix: string) => value == null || !Number.isFinite(value) ? '—' : `${value.toFixed(1)}${suffix}`

export function RenderJobProgress({ job }: { job: RenderJob }) {
  return <>
    {job.ratio !== null && <Progress percent={Math.round((job.ratio ?? 0) * 100)} showInfo />}
    <div className={styles.jobMetrics}>
      <span>编码器：{job.encoder ?? (job.requested_encoder === 'auto' ? '自动选择中' : job.requested_encoder ?? '—')}</span>
      {(job.encoder_device || job.requested_device) && <span>设备：{job.encoder_device ?? job.requested_device}</span>}
      <span>速度：{metric(job.fps, ' fps')}{job.speed == null ? '' : ` · ${metric(job.speed, '×')}`}</span>
      <span>耗时：{formatTime(job.elapsed_ms)}</span>
      <span>剩余：{job.state === 'running' ? formatTime(job.eta_ms) : '—'}</span>
    </div>
    {job.fallback_reason && <p className={styles.jobNotice}>渲染路径已调整：{job.fallback_reason}</p>}
    {job.error && <p className={styles.jobError}>{job.error}</p>}
  </>
}

export default function RenderJobList({ jobs, canEdit, canDownload }: {
  jobs: RenderJob[]; canEdit: boolean; canDownload: boolean
}) {
  const [busy, setBusy] = useState<number | null>(null)
  const [view, setView] = useState<{ id: number; url: string } | null>(null)
  const action = async (job: RenderJob, name: 'cancel' | 'retry') => {
    setBusy(job.id)
    try { await renderJobAction(job.session_id, job.id, name) }
    catch (e) { Toast.error({ content: e instanceof Error ? e.message : String(e), duration: 6 }) }
    finally { setBusy(null) }
  }
  return <>
    {jobs.length > 0 ? <ul className={styles.jobList}>{jobs.map(job => {
      const url = apiResourceUrl(job.download_url ?? `/v1/renders/${job.id}/download`)
      return <li key={job.id} className={styles.jobCard}>
        <div className={styles.jobHeading}>
          <strong>#{job.id} · {job.session_title || job.session_name || `场次 #${job.session_id}`}</strong>
          <span className={`${styles.jobState} ${styles[`state_${job.state}`]}`}>{stateLabels[job.state]}</span>
        </div>
        <div className={styles.jobSubheading}>
          <a href={`/replay?session=${job.session_id}`}>打开场次</a>
          <span>{job.clip_id ? `切片 #${job.clip_id}` : '整场'}</span>
          <span>{job.phase}</span>
        </div>
        {job.file_name && <div className={styles.outputFile} title={job.file_name}>产物：{job.file_name}</div>}
        <RenderJobProgress job={job} />
        <div className={styles.jobActions}>
          {job.file_name && <RevealFileButton request={{ kind: 'render', id: job.id }} allowed={canDownload} />}
          {['queued', 'running'].includes(job.state) && <Button size="small" disabled={!canEdit || busy !== null} loading={busy === job.id} onClick={() => void action(job, 'cancel')}>取消</Button>}
          {['failed', 'cancelled'].includes(job.state) && <Button size="small" disabled={!canEdit || busy !== null} loading={busy === job.id} onClick={() => void action(job, 'retry')}>重试</Button>}
          {job.state === 'ready' && canDownload && <>
            <Button size="small" onClick={() => setView({ id: job.id, url })}>播放</Button>
            <a href={`${url}?attachment=true`} download>下载 MP4</a>
          </>}
        </div>
        {view?.id === job.id && <div className={styles.jobPreview}>
          <Button size="small" onClick={() => setView(null)}>关闭预览</Button>
          <video src={view.url} controls preload="metadata" />
        </div>}
      </li>
    })}</ul> : <p className={styles.empty}>还没有压制任务。</p>}
  </>
}
