'use client'
import React, { useState } from 'react'
import Link from 'next/link'
import { Button, Popconfirm, Popover, Progress, Tag, Toast, Tooltip, Typography } from '@douyinfe/semi-ui'
import { IconAlertTriangle, IconSetting } from '@douyinfe/semi-icons'
import {
  AUTO_CLIP_SETTINGS_HREF,
  type AutoClipAvailability,
  type AutoClipJob,
  autoClipError,
  cancelAutoClip,
  isActive,
  jobBadge,
  type JobHint,
  jobHints,
} from '@/app/lib/auto-clip'
import { ReportedError } from '@/app/lib/markers'
import { useMe } from '@/app/lib/use-me'
import { useNowSec } from '@/app/lib/use-dashboard'
import EstimateModal from './EstimateModal'
import styles from './auto-clip.module.scss'

export const EDIT_REASON = '只读观察者不能生成或接受候选：需要 clip.edit 权限，请让管理员把你的角色改成操作员'

/** 任务状态标签；`detail` 为 true 时后面跟一句进度 / 用量 */
export function JobBadge({ job, detail = false }: { job: AutoClipJob; detail?: boolean }) {
  const now = useNowSec() * 1000
  const badge = jobBadge(job, now)
  return (
    <span className={styles.badgeLine} data-job-state={job.state} data-testid="auto-clip-badge">
      <Tag size="small" color={badge.tone} className={styles.badge}>
        {badge.label}
      </Tag>
      {detail && badge.detail ? <span className={styles.badgeDetail}>{badge.detail}</span> : null}
    </span>
  )
}

/** 任务进度条：运行中且知道总量时才有 */
export function JobProgress({ job }: { job: AutoClipJob }) {
  if (job.state !== 'running' || job.progress_total <= 0) return null
  const percent = Math.round((Math.min(job.progress_done, job.progress_total) / job.progress_total) * 100)
  return <Progress percent={percent} size="small" aria-label="生成进度" className={styles.progress} />
}

function SettingsLink() {
  const { can } = useMe()
  if (!can('config.view')) return <span className={styles.hintTodo}>（设置要请管理员改）</span>
  return (
    <Link href={AUTO_CLIP_SETTINGS_HREF} className={styles.settingsLink}>
      <IconSetting size="small" aria-hidden="true" /> 去空间配置
    </Link>
  )
}

/** 失败原因与警告，每条附一句下一步 */
export function JobHints({ hints }: { hints: JobHint[] }) {
  if (hints.length === 0) return null
  return (
    <ul className={styles.hints} aria-label="自动切片提示" data-testid="auto-clip-hints">
      {hints.map((h, i) => (
        <li key={i} data-level={h.level}>
          <span className={styles.hintText}>{h.text}</span>
          {h.todo ? <span className={styles.hintTodo}>{h.todo}</span> : null}
          {h.settings ? <SettingsLink /> : null}
        </li>
      ))}
    </ul>
  )
}

/** 设置层面不能生成时的说明（没开启、没填转写模型），带去设置页的入口 */
export function SetupNote({ reason }: { reason: string }) {
  return (
    <Typography.Text type="tertiary" size="small" className={styles.setupNote}>
      {reason} <SettingsLink />
    </Typography.Text>
  )
}

/**
 * 「生成候选」按钮（点开先看预估再确认）与运行中任务的「取消」。没有 clip.edit、自动切片没开、
 * 场次还在录时按钮禁用并说明原因。
 */
export function AutoClipTrigger({
  sessionId,
  job,
  availability,
  recording,
  size = 'small',
}: {
  sessionId: number
  job: AutoClipJob | null
  availability: AutoClipAvailability
  /** 场次还在录；不知道时传 false，后端会拒绝 */
  recording: boolean
  size?: 'small' | 'default'
}) {
  const { can, isLoading } = useMe()
  const canEdit = can('clip.edit')
  const [open, setOpen] = useState(false)
  const [canceling, setCanceling] = useState(false)
  const active = isActive(job)

  if (active) {
    const cancel = async () => {
      setCanceling(true)
      try {
        await cancelAutoClip(sessionId)
        Toast.info({ content: '已取消。已转写的部分会保留，下次生成时沿用', duration: 3 })
      } catch (e) {
        if (!(e instanceof ReportedError)) Toast.error({ content: `取消失败：${autoClipError(e)}`, duration: 4 })
      } finally {
        setCanceling(false)
      }
    }
    return canEdit ? (
      <Popconfirm
        title="取消生成？"
        content="正在进行的转写或分析会停下；已经转写完的音频块会保留，重新生成时沿用"
        okText="取消生成"
        cancelText="继续"
        okType="danger"
        onConfirm={cancel}
      >
        <Button size={size} type="danger" theme="light" loading={canceling}>
          取消
        </Button>
      </Popconfirm>
    ) : (
      <Tooltip content={EDIT_REASON}>
        <span className={styles.inlineWrap}>
          <Button size={size} type="danger" theme="light" disabled>
            取消
          </Button>
        </span>
      </Tooltip>
    )
  }

  const reason = isLoading
    ? '正在读取权限…'
    : !canEdit
      ? EDIT_REASON
      : availability.setupReason ?? (recording ? '这一场还在录，下播后再生成' : null)
  const label = job && job.state !== 'canceled' ? '重新生成' : '生成候选'
  return (
    <>
      <Tooltip
        content={
          reason ??
          (job ? '重新让模型挑一遍；已接受或丢弃过的片段不会重复出现' : '先看要转写多少分钟、大概多少 token，确认后才开始')
        }
      >
        <span className={styles.inlineWrap}>
          <Button
            size={size}
            theme={job ? 'light' : 'solid'}
            disabled={reason !== null}
            onClick={() => setOpen(true)}
            data-testid="auto-clip-trigger"
          >
            {label}
          </Button>
        </span>
      </Tooltip>
      {open ? (
        <EstimateModal sessionId={sessionId} status={availability.status} onClose={() => setOpen(false)} />
      ) : null}
    </>
  )
}

/** 工作台场次行里的一格：状态标签 + 提示 + 按钮 */
export function JobCell({
  sessionId,
  job,
  availability,
  loading,
}: {
  sessionId: number
  job: AutoClipJob | null
  availability: AutoClipAvailability
  loading: boolean
}) {
  const hints = job ? jobHints(job) : []
  const worst = hints.some((h) => h.level === 'error') ? 'error' : hints.length ? 'warning' : null
  return (
    <span className={styles.cell}>
      {job ? (
        <Popover
          trigger="click"
          position="bottomLeft"
          showArrow
          content={
            <div className={styles.cellTip}>
              <JobBadge job={job} detail />
              <JobProgress job={job} />
              {hints.length ? <JobHints hints={hints} /> : null}
            </div>
          }
        >
          <button
            type="button"
            className={styles.cellBadge}
            aria-label={`自动切片任务详情${worst ? `（${hints.length} 条提示）` : ''}`}
          >
            <JobBadge job={job} />
            {worst ? <IconAlertTriangle size="small" className={styles.cellWarn} data-level={worst} /> : null}
          </button>
        </Popover>
      ) : loading ? null : (
        <span className={styles.cellNone}>—</span>
      )}
      <AutoClipTrigger sessionId={sessionId} job={job} availability={availability} recording={false} />
    </span>
  )
}
