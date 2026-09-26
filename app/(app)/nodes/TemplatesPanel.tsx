'use client'
import React, { useMemo } from 'react'
import { Button, Empty, Popconfirm, Spin, Tag, Toast, Tooltip, Typography } from '@douyinfe/semi-ui'
import { IconDeleteStroked, IconEdit2Stroked, IconUpload } from '@douyinfe/semi-icons'
import {
  deleteTemplate,
  errorMessage,
  type FleetNode,
  type FleetRoom,
  type FleetTemplate,
} from '@/app/lib/use-fleet'
import styles from './page.module.scss'

const { Text } = Typography

/** 控制面上的投稿模板：房间引用它们，节点收到时按 `account_mid` 换成本机的凭据文件 */
export default function TemplatesPanel({
  templates,
  rooms,
  nodes,
  loading,
  error,
  canManage,
  creating,
  onEdit,
  onChanged,
}: {
  templates: FleetTemplate[] | undefined
  rooms: FleetRoom[]
  nodes: FleetNode[]
  loading: boolean
  error: unknown
  canManage: boolean
  creating: React.ReactNode
  onEdit: (template: FleetTemplate) => void
  onChanged: () => void
}) {
  const usage = useMemo(() => {
    const map = new Map<number, number>()
    for (const room of rooms) {
      if (room.template_id !== null && room.deleted_at === null) {
        map.set(room.template_id, (map.get(room.template_id) ?? 0) + 1)
      }
    }
    return map
  }, [rooms])

  const accountNodes = (mid: number) => nodes.filter((n) => n.accounts.some((a) => a.mid === mid))
  const uname = (mid: number) =>
    nodes.flatMap((n) => n.accounts).find((a) => a.mid === mid && a.uname)?.uname ?? null

  const remove = async (template: FleetTemplate) => {
    try {
      await deleteTemplate(template.id)
      Toast.success(`已删除「${template.template_name}」`)
    } catch (e) {
      Toast.error({ content: errorMessage(e), duration: 6 })
    }
    onChanged()
  }

  if (!templates && loading) {
    return (
      <div className={styles.center}>
        <Spin size="large" />
      </div>
    )
  }
  if (!templates) {
    return (
      <div className={styles.center}>
        <Empty title="加载失败" description={errorMessage(error) || '无法获取投稿模板'} />
        <Button onClick={onChanged} style={{ marginTop: 12 }}>
          重试
        </Button>
      </div>
    )
  }
  if (templates.length === 0) {
    return (
      <div className={styles.center}>
        <Empty
          image={<IconUpload size="extra-large" style={{ color: 'var(--semi-color-text-3)' }} />}
          title="还没有投稿模板"
          description={
            canManage
              ? '房间录完按模板投稿；模板里选的 B 站账号必须在节点上登记过（节点会上报它登记了哪些账号）'
              : '管理员添加投稿模板后会显示在这里'
          }
        />
        {canManage ? <div style={{ marginTop: 12 }}>{creating}</div> : null}
      </div>
    )
  }

  return (
    <div className={styles.templateGrid}>
      {templates.map((template) => {
        const mid = template.account_mid
        const holders = mid ? accountNodes(mid) : []
        const used = usage.get(template.id) ?? 0
        return (
          <article key={template.id} className={styles.templateCard} aria-label={`投稿模板 ${template.template_name}`}>
            <div className={styles.templateHead}>
              <Text strong ellipsis={{ showTooltip: true }} className={styles.templateName}>
                {template.template_name}
              </Text>
              {template.uploader ? (
                <Tag size="small" color={template.uploader === 'Noop' ? 'grey' : 'blue'}>
                  {template.uploader}
                </Tag>
              ) : null}
            </div>
            <div className={styles.templateMeta}>
              {mid ? (
                <Tooltip
                  content={
                    holders.length
                      ? `登记了这个账号的节点：${holders.map((n) => n.name).join('、')}`
                      : '没有节点登记这个账号，用这个模板的房间派不出去'
                  }
                >
                  <span>
                    账号 <b>{uname(mid) ?? mid}</b>
                    <Text type={holders.length ? 'tertiary' : 'danger'} size="small">
                      {' '}
                      · {holders.length} 个节点
                    </Text>
                  </span>
                </Tooltip>
              ) : (
                <Text type="tertiary" size="small">
                  不指定账号
                </Text>
              )}
              <span>
                <b>{used}</b> 个房间在用
              </span>
            </div>
            {canManage ? (
              <div className={styles.templateActions}>
                <Button
                  theme="borderless"
                  type="primary"
                  icon={<IconEdit2Stroked />}
                  aria-label="编辑"
                  title="编辑"
                  onClick={() => onEdit(template)}
                />
                <Popconfirm
                  title={`删除模板「${template.template_name}」？`}
                  content={used ? `还有 ${used} 个房间在用，先改掉它们的模板` : '此操作不可逆'}
                  okType="danger"
                  okButtonProps={{ disabled: used > 0 }}
                  onConfirm={() => remove(template)}
                >
                  <Button theme="borderless" type="danger" icon={<IconDeleteStroked />} aria-label="删除" title="删除" />
                </Popconfirm>
              </div>
            ) : null}
          </article>
        )
      })}
    </div>
  )
}
