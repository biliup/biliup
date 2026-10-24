'use client'
import React from 'react'
import { Button, Popconfirm, Tag, Tooltip, Typography } from '@douyinfe/semi-ui'
import { IconDeleteStroked, IconEdit2Stroked, IconSendStroked } from '@douyinfe/semi-icons'
import type { StudioEntity } from '@/app/lib/api-streamer'
import styles from './template-card.module.scss'

const { Text } = Typography

/** 定时投稿的秒数 → 「8 小时」「8 小时 30 分钟」；没设置定时返回 null */
export function formatDelay(dtime?: number | null): string | null {
  if (!dtime || dtime <= 0) return null
  const hours = Math.floor(dtime / 3600)
  const minutes = Math.round((dtime % 3600) / 60)
  if (hours > 0 && minutes > 0) return `${hours} 小时 ${minutes} 分钟`
  return hours > 0 ? `${hours} 小时` : `${minutes} 分钟`
}

/** 类型：1 自制、2 转载（与投稿表单的 Radio 一致） */
function copyrightLabel(copyright: number): string {
  if (copyright === 1) return '自制'
  if (copyright === 2) return '转载'
  return '—'
}

export interface TemplateCardProps {
  template: StudioEntity
  /** 投稿账号显示名（由 user_cookie 换成 B 站昵称）；未知时回落到 cookie 名 */
  account?: string | null
  canSubmit: boolean
  canEdit: boolean
  /** 控制面 / 本机 Fleet 下发的模板：只读，改删在后端返回 409 */
  managed?: boolean
  managedHint?: string
  /** 一主一备配对里的模板：改动两台同步，不能在这里删除 */
  pairLabel?: string
  pairHint?: string
  deleteHint?: string
  onSend: () => void
  onEdit: () => void
  onDelete: () => Promise<void> | void
}

/**
 * 投稿模板卡片：名称 + 标题模板 + 账号 / 类型 / 分区 / 定时 + 标签，底部是投稿、编辑、删除。
 * 与「节点管理」的模板卡片、直播间卡片共用同一套卡片语言（圆角、描边、hover 抬升、底栏操作）。
 * 名称最多两行，不再被右侧按钮挤成两个字。
 */
export default function TemplateCard({
  template,
  account,
  canSubmit,
  canEdit,
  managed,
  managedHint,
  pairLabel,
  pairHint,
  deleteHint,
  onSend,
  onEdit,
  onDelete,
}: TemplateCardProps) {
  const delay = formatDelay(template.dtime)
  const tags = template.tags ?? []
  const accountText = account || template.user_cookie || '—'
  const title = template.title?.trim()
  const deleteDisabled = !!managed || !!pairLabel

  return (
    <article className={styles.card} aria-label={`投稿模板 ${template.template_name}`}>
      <header className={styles.head}>
        <Text strong className={styles.name} title={template.template_name}>
          {template.template_name}
        </Text>
        {managed ? (
          <Tooltip content={managedHint}>
            <Tag size="small" color="violet" className={styles.badge}>
              托管
            </Tag>
          </Tooltip>
        ) : null}
        {pairLabel ? (
          <Tooltip content={pairHint}>
            <Tag size="small" color="green" className={styles.badge}>
              {pairLabel}
            </Tag>
          </Tooltip>
        ) : null}
      </header>

      <div className={styles.titleRow} title={title || undefined}>
        <span className={styles.label}>标题</span>
        {title ? (
          <span className={styles.titleText}>{title}</span>
        ) : (
          <span className={styles.muted}>未设置标题模板</span>
        )}
      </div>

      <div className={styles.meta}>
        <span className={styles.metaItem} title={`投稿账号：${accountText}`}>
          <span className={styles.label}>账号</span>
          <span className={styles.metaValue}>{accountText}</span>
        </span>
        <span className={styles.metaItem} title="稿件类型">
          <span className={styles.label}>类型</span>
          <span className={styles.metaValue}>{copyrightLabel(template.copyright)}</span>
        </span>
        <span className={styles.metaItem} title="投稿分区 tid">
          <span className={styles.label}>分区</span>
          <span className={styles.metaValue}>{template.tid ?? '—'}</span>
        </span>
        {delay ? (
          <span className={styles.metaItem} title="录完后再等这么久投稿">
            <span className={styles.label}>定时</span>
            <span className={styles.metaValue}>{delay}</span>
          </span>
        ) : null}
      </div>

      {tags.length > 0 ? (
        <div className={styles.tags} title={tags.join('、')}>
          {tags.slice(0, 4).map((tag) => (
            <span key={tag} className={styles.chip}>
              #{tag}
            </span>
          ))}
          {tags.length > 4 ? <span className={styles.chip}>+{tags.length - 4}</span> : null}
        </div>
      ) : null}

      {canSubmit || canEdit ? (
        <footer className={styles.foot}>
          {canSubmit ? (
            <Button
              size="small"
              theme="borderless"
              type="primary"
              icon={<IconSendStroked />}
              onClick={onSend}
            >
              投稿
            </Button>
          ) : null}
          {canEdit ? (
            <Button
              size="small"
              theme="borderless"
              icon={<IconEdit2Stroked />}
              disabled={managed}
              title={managed ? managedHint : '编辑模板'}
              onClick={onEdit}
            >
              编辑
            </Button>
          ) : null}
          {canEdit ? (
            <Popconfirm
              title="确定是否要删除？"
              content="此操作将不可逆"
              margin={50}
              disabled={deleteDisabled}
              onConfirm={() => onDelete()}
            >
              <Button
                size="small"
                theme="borderless"
                type="danger"
                icon={<IconDeleteStroked />}
                disabled={deleteDisabled}
                title={managed ? managedHint : pairLabel ? deleteHint : '删除模板'}
              >
                删除
              </Button>
            </Popconfirm>
          ) : null}
        </footer>
      ) : null}
    </article>
  )
}
