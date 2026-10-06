'use client'
import { useState } from 'react'
import { Toast } from '@douyinfe/semi-ui'
import { errorMessage } from '@/app/lib/use-fleet'
import { configureOnNode, designate, paramsIssue, type HaMode, type HaParams } from '@/app/lib/fleet-ha'
import ModeFields from './ModeFields'
import { FormDialog } from '@/app/ui/shell'
import pageStyles from '../page.module.scss'

/** 改模式与参数：控制面上直接改，配对节点上经控制面提交 */
export default function ModeDialog({
  mode: initialMode,
  params: initialParams,
  standby,
  onNode,
  onClose,
  onSaved,
}: {
  mode: HaMode
  params: HaParams
  /** 控制面上改时要带上配对里的节点 id */
  standby: number
  onNode: boolean
  onClose: () => void
  onSaved: () => void
}) {
  const [mode, setMode] = useState(initialMode)
  const [params, setParams] = useState(initialParams)
  const [busy, setBusy] = useState(false)
  const issue = paramsIssue(params)
  const unchanged = mode === initialMode && JSON.stringify(params) === JSON.stringify(initialParams)
  const save = async () => {
    if (busy || issue) return
    setBusy(true)
    try {
      if (onNode) await configureOnNode(mode, params)
      else await designate({ standby, mode, params })
      Toast.success({ id: 'fleet-ha-mode', content: '已保存，两台都按新的模式与参数工作' })
      onSaved()
    } catch (e) {
      Toast.error({ id: 'fleet-ha-mode', content: errorMessage(e), duration: 6 })
      setBusy(false)
    }
  }
  return (
    <FormDialog
      title="改模式与参数"
      size="md"
      closable={!busy}
      onCancel={onClose}
      okText="保存"
      onOk={save}
      confirmLoading={busy}
      okDisabled={!!issue || unchanged}
    >
      <div className={pageStyles.dialogBody}>
        <ModeFields mode={mode} params={params} onMode={setMode} onParams={setParams} disabled={busy} />
      </div>
    </FormDialog>
  )
}
