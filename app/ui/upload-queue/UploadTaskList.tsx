'use client'
import { useState } from 'react'
import Link from 'next/link'
import { Button, Spin, Tag } from '@douyinfe/semi-ui'
import {
  fetchUploadQueueTask, formatUploadBytes, formatUploadSpeed, uploadPhaseLabel,
  groupUploadTasks, uploadSourceLabels, uploadStateLabel, uploadStateMeta, uploadTaskProgress, uploadPercent,
  type UploadQueueTask,
} from '@/app/lib/upload-queue'
import styles from './upload-queue.module.scss'

function dateTime(value: number) {
  if (!Number.isFinite(value)) return '—'
  return new Date(value).toLocaleString('zh-CN')
}

function StateTag({ state }: { state: string }) {
  const meta = uploadStateMeta[state] ?? { label: uploadStateLabel(state), tone: 'grey' as const }
  return <Tag size="small" color={meta.tone}>{meta.label}</Tag>
}

function ProgressBar({ confirmed, total }: { confirmed: number; total: number | null }) {
  const percent = uploadPercent(confirmed, total)
  return <div className={styles.progressWrap}>
    {percent == null ? <div className={styles.progressUnknown}><span style={{ width: '38%' }} /></div>
      : <progress className={styles.progress} max={100} value={percent} aria-label={`上传进度 ${Math.round(percent)}%`} />}
    <span>{percent == null ? `${formatUploadBytes(confirmed)} · 总量未知` : `${Math.round(percent)}%`}</span>
  </div>
}

function FileRows({ files }: { files: NonNullable<UploadQueueTask['files']> }) {
  if (files.length === 0) return <p className={styles.noFiles}>此任务暂无文件明细。</p>
  return <div className={styles.filesTable}>
    <div className={`${styles.fileRow} ${styles.fileHeader}`}><span>顺序 / 文件</span><span>进度</span><span>状态</span><span>重试 / 错误</span></div>
    {files.slice().sort((a, b) => a.order - b.order || a.id.localeCompare(b.id)).map(file => {
      const meta = uploadStateMeta[file.state] ?? { label: uploadStateLabel(file.state), tone: 'grey' as const }
      const ratio = uploadTaskProgress({ bytes_confirmed: file.bytes_confirmed, total_bytes: file.total_bytes })
      return <div className={styles.fileRow} key={file.id}>
      <span className={styles.fileName}><b>#{file.order}</b> {file.name}</span>
        <span>{ratio}</span>
        <span><Tag size="small" color={meta.tone}>{meta.label}</Tag></span>
        <span className={styles.fileError}>{file.retries ? `${file.retries} 次重试` : '—'}{file.error ? ` · ${file.error}` : ''}</span>
      </div>
    })}
  </div>
}

function TaskCard({ task }: { task: UploadQueueTask }) {
  const [expanded, setExpanded] = useState(false)
  const [files, setFiles] = useState(task.files)
  const [loading, setLoading] = useState(false)
  const [detailError, setDetailError] = useState('')
  const progress = uploadTaskProgress(task)
  const toggleDetails = async () => {
    if (expanded) { setExpanded(false); return }
    setExpanded(true)
    if (files) return
    setLoading(true); setDetailError('')
    try {
      const detail = await fetchUploadQueueTask(task.id)
      setFiles(detail.files ?? [])
    } catch (error) {
      setDetailError(error instanceof Error ? error.message : String(error))
    } finally { setLoading(false) }
  }
  return <article className={styles.task}>
    <div className={styles.taskMain}>
      <div className={styles.orderCell}>
        {task.queue_position != null ? <span className={styles.queuePosition}>#{task.queue_position}</span> : <span className={styles.orderFallback}>序 #{task.order}</span>}
      </div>
      <div className={styles.taskInfo}>
        <div className={styles.titleLine}>
          <strong title={task.title}>{task.title || '未命名任务'}</strong>
          <StateTag state={task.state} />
          {task.source && <Tag size="small" color="violet" type="light">{uploadSourceLabels[task.source] ?? '其他来源'}</Tag>}
        </div>
        <div className={styles.metaLine}>
          {task.streamer_name && <span>{task.streamer_name}</span>}
          {task.template_name && <span>模板：{task.template_name}</span>}
          {task.session_id != null && <Link href={`/replay?session=${encodeURIComponent(task.session_id)}`}>场次 #{task.session_id}</Link>}
          {task.bvid && <a href={`https://www.bilibili.com/video/${encodeURIComponent(task.bvid)}`} target="_blank" rel="noreferrer">稿件 {task.bvid}</a>}
        </div>
        <div className={styles.progressLine}>
          <ProgressBar confirmed={task.bytes_confirmed} total={task.total_bytes} />
          <span>{progress}</span>
          {task.current_file && <span className={styles.currentFile} title={task.current_file}>当前：{task.current_file}</span>}
        </div>
        <div className={styles.metaLine}>
          <span>速度 {formatUploadSpeed(task.speed_bytes_per_sec)}</span>
          {task.phase && <span>{uploadPhaseLabel(task.phase)}</span>}
          <span>{task.files_succeeded} 成功 · {task.files_failed} 失败</span>
          {task.retries > 0 && <span>{task.retries} 次重试</span>}
          {task.next_retry_at != null && <span>下次重试：{dateTime(task.next_retry_at)}</span>}
          <span>创建于 {dateTime(task.created_at)}</span>
        </div>
        {(task.error || task.state === 'unknown' || task.state === 'interrupted' || task.state === 'partial') && <p className={styles.taskError}>{task.error || (task.state === 'unknown' ? '提交结果暂时无法确认，请检查稿件列表。' : task.state === 'interrupted' ? '服务重启时任务尚未完成。' : '部分文件上传失败。')}</p>}
      </div>
      <Button size="small" type="tertiary" onClick={toggleDetails}>{expanded ? '收起明细' : '文件明细'}</Button>
    </div>
    {expanded && <div className={styles.detail}>
      {loading ? <div className={styles.detailLoading}><Spin size="small" /> 正在读取文件明细…</div>
        : detailError ? <p className={styles.detailError}>明细加载失败：{detailError}</p>
          : <FileRows files={files ?? []} />}
    </div>}
  </article>
}

export function UploadTaskList({ tasks }: { tasks: UploadQueueTask[] }) {
  return <div className={styles.groups}>
    {groupUploadTasks(tasks).map(group => <section className={styles.group} key={group.key}>
      <header className={styles.groupHeader}><h2>{group.label}</h2><span>{group.tasks.length} 项</span></header>
      {group.tasks.map(task => <TaskCard key={task.id} task={task} />)}
    </section>)}
  </div>
}
