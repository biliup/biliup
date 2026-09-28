'use client'
import { useState } from 'react'
import { Button, Modal, Toast } from '@douyinfe/semi-ui'
import { errorMessage } from '@/app/lib/use-fleet'
import { configureOnNode, designate, paramsIssue, type HaMode, type HaParams } from '@/app/lib/fleet-ha'
import ModeFields from './ModeFields'
import pageStyles from '../page.module.scss'
import styles from './ha.module.scss'

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
    <Modal
      title="改模式与参数"
      visible
      onCancel={busy ? undefined : onClose}
      closeOnEsc={!busy}
      style={{ width: 'min(640px, 94vw)' }}
      footer={
        <div className={pageStyles.dialogFoot}>
          <Button onClick={onClose} disabled={busy}>
            取消
          </Button>
          <Button theme="solid" onClick={save} loading={busy} disabled={!!issue || unchanged}>
            保存
          </Button>
        </div>
      }
    >
      <div className={`${pageStyles.dialogBody} ${styles.dialogScroll}`}>
        <ModeFields mode={mode} params={params} onMode={setMode} onParams={setParams} disabled={busy} />
      </div>
    </Modal>
  )
}
