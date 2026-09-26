'use client'
import React, { useMemo, useRef, useState } from 'react'
import { Form, Modal, Spin, Toast, Typography } from '@douyinfe/semi-ui'
import type { FormApi, FormFCChild } from '@douyinfe/semi-ui/lib/es/form'
import TemplateFields from '@/app/ui/TemplateFields'
import type { BiliType } from '@/app/lib/api-streamer'
import { useTypeTree } from '@/app/lib/use-streamers'
import { useIsMobile } from '@/app/lib/useIsMobile'
import {
  createTemplate,
  errorMessage,
  updateTemplate,
  type FleetNode,
  type FleetTemplate,
  type TemplateInput,
} from '@/app/lib/use-fleet'
import styles from './page.module.scss'

const { Text } = Typography

type TypeTree = { value: number; children: BiliType[] }[]

/** 模板 → 表单初始值，与「投稿管理 / 编辑模板」同一套换算 */
function toForm(template: FleetTemplate | null, typeTree: TypeTree | undefined) {
  if (!template) return {}
  const parent = typeTree?.find((t) => t.children.some((c) => c.id === template.tid))?.value
  return {
    ...template,
    tid: typeTree && template.tid !== null ? [parent, template.tid] : (template.tid ?? undefined),
    sound: (template.dolby === 1 ? ['dolby'] : []).concat(template.hires === 1 ? ['hires'] : []),
    interaction: (template.up_close_danmu === 1 ? ['up_close_danmu'] : [])
      .concat(template.up_close_reply === 1 ? ['up_close_reply'] : [])
      .concat(template.up_selection_reply === 1 ? ['up_selection_reply'] : []),
    charging_pay: template.charging_pay === 1,
    no_reprint: template.no_reprint === 1,
    is_only_self: template.is_only_self === 1,
    isDtime: Boolean(template.dtime),
  }
}

function fromForm(values: Record<string, any>): TemplateInput {
  const flag = (list: unknown, key: string) => (Array.isArray(list) && list.includes(key) ? 1 : 0)
  return {
    template_name: values.template_name,
    account_mid: values.account_mid ?? null,
    title: values.title ?? '',
    tid: Array.isArray(values.tid) ? values.tid[1] : (values.tid ?? null),
    tid_v2: values.tid_v2 ?? null,
    copyright: values.copyright ?? null,
    copyright_source: values.copyright_source ?? '',
    cover_path: values.cover_path ?? '',
    description: values.description ?? '',
    dynamic: values.dynamic ?? '',
    tags: values.tags ?? [],
    dolby: flag(values.sound, 'dolby'),
    hires: flag(values.sound, 'hires'),
    up_selection_reply: flag(values.interaction, 'up_selection_reply'),
    up_close_reply: flag(values.interaction, 'up_close_reply'),
    up_close_danmu: flag(values.interaction, 'up_close_danmu'),
    charging_pay: values.charging_pay ? 1 : 0,
    no_reprint: values.no_reprint ? 1 : 0,
    is_only_self: values.is_only_self ? 1 : 0,
    dtime: values.isDtime ? (values.dtime ?? null) : null,
    credits: values.credits ?? null,
    uploader: values.uploader ?? null,
    extra_fields: values.extra_fields ?? '',
  }
}

/**
 * 控制面上的投稿模板表单：字段与本机「投稿管理」共用 `TemplateFields`，
 * 只是「投稿账号」按节点上报的 mid 选（凭据文件留在各节点上）。
 */
export default function FleetTemplateModal({
  template,
  nodes,
  onClose,
  onSaved,
}: {
  /** null 为新建 */
  template: FleetTemplate | null
  nodes: FleetNode[]
  onClose: () => void
  onSaved: () => void
}) {
  const isMobile = useIsMobile()
  const api = useRef<FormApi>(undefined)
  const [saving, setSaving] = useState(false)
  const { typeTree, isLoading } = useTypeTree()

  const accounts = useMemo(() => {
    const byMid = new Map<number, { uname: string; nodes: string[] }>()
    for (const node of nodes) {
      for (const account of node.accounts) {
        const entry = byMid.get(account.mid) ?? { uname: account.uname, nodes: [] }
        entry.uname ||= account.uname
        entry.nodes.push(node.name)
        byMid.set(account.mid, entry)
      }
    }
    const options = [...byMid.entries()]
      .sort((a, b) => b[1].nodes.length - a[1].nodes.length || a[0] - b[0])
      .map(([mid, { uname, nodes }]) => ({
        value: mid,
        label: (
          <span className={styles.accountOption}>
            <span>{uname || `mid ${mid}`}</span>
            <Text type="tertiary" size="small">
              {uname ? `mid ${mid} · ` : ''}
              {nodes.join('、')}
            </Text>
          </span>
        ),
      }))
    const current = template?.account_mid
    if (current && !byMid.has(current)) {
      options.push({
        value: current,
        label: (
          <span className={styles.accountOption}>
            <span>mid {current}</span>
            <Text type="danger" size="small">
              没有节点登记这个账号
            </Text>
          </span>
        ),
      })
    }
    return options
  }, [nodes, template])

  const initValues = useMemo(() => toForm(template, typeTree), [template, typeTree])

  const save = async () => {
    const values = await api.current?.validate().catch(() => undefined)
    if (!values) return
    setSaving(true)
    try {
      const input = fromForm(values)
      if (template) await updateTemplate(template.id, input)
      else await createTemplate(input)
      Toast.success(template ? '模板已保存，已下发到用它的节点' : '模板已创建')
      onSaved()
    } catch (e) {
      Toast.error({ content: errorMessage(e), duration: 6 })
    } finally {
      setSaving(false)
    }
  }

  const accountField = (
    <Form.Select
      field="account_mid"
      label={{ text: '投稿账号', optional: true }}
      style={{ width: 320 }}
      optionList={accounts}
      showClear
      placeholder={accounts.length ? '选择节点上报的账号' : '还没有节点上报 B 站账号'}
      extraText="房间只能派到登记了这个账号的节点；不选则节点上投稿没有账号（配合 Noop 上传插件只录不投）"
    />
  )

  return (
    <Modal
      title={template ? `编辑投稿模板「${template.template_name}」` : '新建投稿模板'}
      visible
      fullScreen={isMobile}
      width={isMobile ? undefined : 'min(820px, 94vw)'}
      onCancel={onClose}
      onOk={save}
      okText="保存"
      confirmLoading={saving}
      bodyStyle={{ overflow: 'auto', maxHeight: isMobile ? undefined : 'calc(100vh - 260px)' }}
    >
      {isLoading ? (
        <div className={styles.dialogCenter}>
          <Spin />
        </div>
      ) : (
        <Form
          className={styles.templateForm}
          initValues={initValues}
          getFormApi={(formApi) => (api.current = formApi)}
          labelPosition={isMobile ? 'top' : 'left'}
          labelWidth="140px"
          render={(props: FormFCChild<any>) => <TemplateFields {...props} accountField={accountField} plainTid={!typeTree} />}
        />
      )}
    </Modal>
  )
}
