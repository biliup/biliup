'use client'
import React, { useEffect, useRef, useState } from 'react'
import { Banner, Button, Progress, Toast, Tooltip, Typography } from '@douyinfe/semi-ui'
import { IconExternalOpen } from '@douyinfe/semi-icons'
import { ReportedError } from '@/app/lib/markers'
import { useMe } from '@/app/lib/use-me'
import {
  archiveUrl,
  JOB_STEPS,
  type JobState,
  type JobStep,
  type PublishJob,
  removeJob,
  resumeQueue,
  retryJob,
  stepText,
  usePublishQueue,
} from '@/app/lib/publish'
import styles from './publish.module.scss'

export const NO_SUBMIT = '发布需要 upload.submit 权限，请让管理员把你的角色改成操作员'
export const RATE_LIMITED_TITLE = 'B 站提示上传太频繁，已暂停'

function errorText(e: unknown): string {
  return e instanceof Error ? e.message : String(e)
}

function stepState(job: PublishJob, step: JobStep): 'done' | 'current' | 'todo' {
  if (job.state === 'done') return 'done'
  const at = job.step ? JOB_STEPS.indexOf(job.step) : -1
  const index = JOB_STEPS.indexOf(step)
  return index < at ? 'done' : index === at ? 'current' : 'todo'
}

/** 稿件号链接（新标签页打开 B 站视频页） */
export function BvLink({ bvid }: { bvid: string }) {
  return (
    <a className={styles.bvLink} href={archiveUrl(bvid)} target="_blank" rel="noopener noreferrer">
      <IconExternalOpen size="small" aria-hidden="true" />
      {bvid}
    </a>
  )
}

/**
 * 一个发布任务的分步进度：导出 → 上传 → 投稿，现在在做什么、上传比例；失败时给原因和「重试发布」，
 * 排队中 / 导出上传中可以移出队列（正在投稿时不能：请求可能已经到了 B 站）。成功后显示稿件号。
 */
export function JobStatus({ job, canSubmit }: { job: PublishJob; canSubmit: boolean }) {
  const [busy, setBusy] = useState(false)
  const act = async (run: () => Promise<void>, failure: string) => {
    setBusy(true)
    try {
      await run()
    } catch (e) {
      if (!(e instanceof ReportedError)) Toast.error({ content: `${failure}：${errorText(e)}`, duration: 5 })
    } finally {
      setBusy(false)
    }
  }
  const ratio = job.state === 'running' && job.ratio !== null ? Math.round(job.ratio * 100) : null
  const parts = job.clip_ids.length > 1 ? `（${job.clip_ids.length} 个切片合成一个稿件）` : ''
  const text =
    job.state === 'queued'
      ? `${job.detail}${parts}`
      : job.state === 'running'
        ? `${job.detail}${ratio !== null ? ` ${ratio}%` : ''}${parts}`
        : job.state === 'paused'
          ? `已暂停${parts}：${RATE_LIMITED_TITLE}，点横幅上的「继续」接着传`
          : job.state === 'failed'
            ? `发布失败${parts}：${job.error ?? '原因未知'}`
            : `已投稿${parts}，等 B 站审核`
  return (
    <div
      className={styles.job}
      data-state={job.state}
      data-job={job.id}
      role={job.state === 'failed' ? 'alert' : 'status'}
    >
      <span className={styles.steps} aria-label="发布进度">
        {JOB_STEPS.map((step, i) => (
          <React.Fragment key={step}>
            {i > 0 ? (
              <span className={styles.stepArrow} aria-hidden="true">
                →
              </span>
            ) : null}
            <span
              className={styles.step}
              data-step={stepState(job, step)}
              data-failed={(job.state === 'failed' && job.step === step) || undefined}
            >
              {stepText(step)}
            </span>
          </React.Fragment>
        ))}
      </span>
      <span>{text}</span>
      {job.state === 'done' && job.bvid ? (
        <span className={styles.published}>
          <BvLink bvid={job.bvid} />
          {job.error ? <span className={styles.unknown}>{job.error}</span> : null}
        </span>
      ) : null}
      {ratio !== null ? <Progress percent={ratio} size="small" aria-label="上传进度" className={styles.jobProgress} /> : null}
      {job.state === 'done' ? null : (
        <span className={styles.jobTools}>
          {job.state === 'failed' ? (
            <Tooltip
              content={canSubmit ? (job.uploaded ? `已经传好的 ${job.uploaded} 个分 P 不会重传` : '重新排队') : NO_SUBMIT}
              className={styles.passTip}
            >
              <span className={styles.inlineWrap}>
                <Button
                  size="small"
                  theme="solid"
                  loading={busy}
                  disabled={!canSubmit}
                  onClick={() => act(() => retryJob(job), '没能重试')}
                >
                  重试发布
                </Button>
              </span>
            </Tooltip>
          ) : null}
          {job.step === 'submit' && job.state === 'running' ? null : (
            <Tooltip
              content={canSubmit ? (job.state === 'running' ? '停下正在进行的导出或上传' : '从发布队列里去掉') : NO_SUBMIT}
              className={styles.passTip}
            >
              <span className={styles.inlineWrap}>
                <Button
                  size="small"
                  theme="borderless"
                  type="tertiary"
                  disabled={!canSubmit || busy}
                  onClick={() => act(() => removeJob(job), '没能移出队列')}
                >
                  {job.state === 'running' ? '取消' : '移出队列'}
                </Button>
              </span>
            </Tooltip>
          )}
        </span>
      )}
    </div>
  )
}

/**
 * B 站返回 601 后整个发布队列暂停（不自动重试），在页面顶部说明并给「继续」。
 * `paused` 是后端给的原因（含 B 站原话）。
 */
export function QueueBanner({
  paused,
  canSubmit,
  page = false,
}: {
  paused: string | null | undefined
  canSubmit: boolean
  /** 放在页头下面、与页面内容同宽 */
  page?: boolean
}) {
  const { Text } = Typography
  const [resuming, setResuming] = useState(false)
  if (!paused) return null
  const resume = async () => {
    setResuming(true)
    try {
      await resumeQueue()
      Toast.info({ content: '发布队列继续了：从暂停的稿件接着传', duration: 3 })
    } catch (e) {
      if (!(e instanceof ReportedError)) Toast.error({ content: `没能继续：${errorText(e)}`, duration: 5 })
    } finally {
      setResuming(false)
    }
  }
  const detail = paused.match(/（(.*)）$/)?.[1]
  const banner = (
    <Banner
      type="warning"
      closeIcon={null}
      className={styles.queueBanner}
      data-queue-paused=""
      description={
        <span className={styles.queueBannerBody}>
          <span className={styles.queueBannerText}>
            <strong>{RATE_LIMITED_TITLE}</strong>
            <Text type="tertiary" size="small">
              不会自动重试，过一会儿点「继续」从暂停的稿件接着传{detail ? `。B 站原话：${detail}` : ''}
            </Text>
          </span>
          <Tooltip content={canSubmit ? '从暂停的稿件接着传' : NO_SUBMIT}>
            <span className={styles.inlineWrap}>
              <Button theme="solid" type="warning" loading={resuming} disabled={!canSubmit} onClick={resume}>
                继续
              </Button>
            </span>
          </Tooltip>
        </span>
      }
    />
  )
  return page ? <div className={styles.pageBanner}>{banner}</div> : banner
}

/** 任务状态变化时提示一次（打开页面时已经结束的不提示） */
export function useJobToasts(jobs: PublishJob[] | undefined) {
  const seen = useRef<Map<number, JobState>>(new Map())
  useEffect(() => {
    if (!jobs) return
    for (const job of jobs) {
      const before = seen.current.get(job.id)
      seen.current.set(job.id, job.state)
      if (before === undefined || before === job.state) continue
      if (job.state === 'done') {
        Toast.success({
          id: `publish-${job.id}`,
          content: `已投稿：${job.title ?? ''}（${job.bvid ?? '稿件号未知'}），等 B 站审核`,
          duration: 5,
        })
      } else if (job.state === 'failed') {
        Toast.error({
          id: `publish-${job.id}`,
          content: `发布失败：${job.error ?? '原因未知'}。在切片上点「重试发布」`,
          duration: 6,
        })
      } else if (job.state === 'paused') {
        Toast.warning({ id: 'publish-paused', content: `${RATE_LIMITED_TITLE}，点横幅上的「继续」接着传`, duration: 6 })
      }
    }
  }, [jobs])
}

/**
 * 不属于某一场的页面（直播预览页）用的暂停横幅：看整个发布队列，任务结束时提示。
 * 没有发布权限时不请求队列。
 */
export function PublishQueueBanner() {
  const { can } = useMe()
  const canSubmit = can('upload.submit')
  const queue = usePublishQueue(null, canSubmit)
  useJobToasts(queue.data?.jobs)
  return <QueueBanner paused={queue.data?.paused} canSubmit={canSubmit} page />
}
