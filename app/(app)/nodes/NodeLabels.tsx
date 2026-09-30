'use client'
import { useRef, useState } from 'react'
import { Button, Tag, TagInput, Toast, Typography } from '@douyinfe/semi-ui'
import { IconEdit2Stroked } from '@douyinfe/semi-icons'
import { errorMessage, setNodeLabels, type FleetNode } from '@/app/lib/use-fleet'
import { FormDialog } from '@/app/ui/shell'
import styles from './labels.module.scss'

const { Text } = Typography

/** 与后端 `labels::MAX_LABEL_CHARS` / `MAX_LABELS` 一致 */
const MAX_LABEL_CHARS = 32
const MAX_LABELS = 20
const PRESETS = ['海外', '国内']

/** 节点卡片上的标签行；有 `node.manage` 时可编辑 */
export default function NodeLabels({
  node,
  known,
  canManage,
  onSaved,
}: {
  node: FleetNode
  /** 各节点已有的标签，编辑时作为候选 */
  known: string[]
  canManage: boolean
  onSaved: () => void
}) {
  const [editing, setEditing] = useState(false)
  if (!canManage && node.labels.length === 0) return null
  return (
    <div className={styles.row}>
      <span className={styles.label}>标签</span>
      {node.labels.length ? (
        node.labels.map((label) => (
          <Tag key={label} size="small" color="violet" className={styles.tag}>
            {label}
          </Tag>
        ))
      ) : (
        <Text type="tertiary" size="small">
          无
        </Text>
      )}
      {canManage && !node.removing ? (
        <Button
          size="small"
          theme="borderless"
          type="tertiary"
          icon={<IconEdit2Stroked />}
          aria-label={`编辑 ${node.name} 的标签`}
          title="编辑标签"
          onClick={() => setEditing(true)}
          className={styles.edit}
        />
      ) : null}
      {editing ? (
        <LabelsModal
          node={node}
          known={known}
          onClose={() => setEditing(false)}
          onSaved={() => {
            setEditing(false)
            onSaved()
          }}
        />
      ) : null}
    </div>
  )
}

function LabelsModal({
  node,
  known,
  onClose,
  onSaved,
}: {
  node: FleetNode
  known: string[]
  onClose: () => void
  onSaved: () => void
}) {
  const [labels, setLabels] = useState<string[]>(node.labels)
  const [saving, setSaving] = useState(false)
  const busy = useRef(false)
  const suggestions = [...new Set([...known, ...PRESETS])].filter((label) => !labels.includes(label))
  const removed = node.labels.filter((label) => !labels.includes(label))

  const save = async () => {
    if (busy.current) return
    busy.current = true
    setSaving(true)
    try {
      const saved = await setNodeLabels(node.id, labels)
      Toast.success(saved.length ? `已保存 ${node.name} 的标签` : `已清空 ${node.name} 的标签`)
      onSaved()
    } catch (e) {
      Toast.error({ content: errorMessage(e), duration: 6 })
    } finally {
      busy.current = false
      setSaving(false)
    }
  }

  return (
    <FormDialog
      title={`编辑标签「${node.name}」`}
      size="sm"
      onCancel={onClose}
      onOk={save}
      okText="保存"
      confirmLoading={saving}
    >
      <div className={styles.modal}>
        <TagInput
          value={labels}
          onChange={(next) => setLabels(next.map((label) => label.trim()).filter(Boolean))}
          separator={[',', '，']}
          addOnBlur
          allowDuplicates={false}
          maxLength={MAX_LABELS}
          onExceed={() => Toast.warning(`标签最多 ${MAX_LABELS} 个`)}
          validateStatus={labels.some((label) => label.length > MAX_LABEL_CHARS) ? 'error' : 'default'}
          placeholder="输入标签后回车，例如 海外"
          aria-label="节点标签"
          autoFocus
        />
        {suggestions.length ? (
          <div className={styles.suggest}>
            <Text type="tertiary" size="small">
              常用：
            </Text>
            {suggestions.slice(0, 12).map((label) => (
              <button
                key={label}
                type="button"
                className={styles.pick}
                onClick={() => setLabels([...labels, label])}
                aria-label={`添加标签 ${label}`}
              >
                + {label}
              </button>
            ))}
          </div>
        ) : null}
        <Text type="tertiary" size="small" className={styles.help}>
          房间可以要求节点带某些标签（例如只放在「海外」节点上），手动分派与「自动」都只会选带齐标签的节点。
          标签区分大小写，每个最多 {MAX_LABEL_CHARS} 个字，最多 {MAX_LABELS} 个。
        </Text>
        {removed.length && node.assigned_rooms > 0 ? (
          <Text type="warning" size="small" className={styles.help}>
            去掉标签不会挪走已经分派给它的房间；要求这些标签的房间会在房间列表里标成「标签不满足」，需要时手动迁移。
          </Text>
        ) : null}
      </div>
    </FormDialog>
  )
}
