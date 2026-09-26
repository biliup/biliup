'use client'
import { useState } from 'react'
import { Button, Checkbox, Modal, Toast, Typography } from '@douyinfe/semi-ui'
import { IconServer } from '@douyinfe/semi-icons'
import { enableLocalNode, errorMessage } from '@/app/lib/use-fleet'
import styles from './page.module.scss'

const { Text } = Typography

function LocalNodeDialog({ onClose, onEnabled }: { onClose: () => void; onEnabled: () => void }) {
  const [allowHooks, setAllowHooks] = useState(false)
  const [busy, setBusy] = useState(false)
  const enable = async () => {
    if (busy) return
    setBusy(true)
    try {
      await enableLocalNode(allowHooks)
      Toast.success({ id: 'fleet-local-node', content: '已启用「本机」节点，连上后出现在列表里' })
      onEnabled()
    } catch (e) {
      Toast.error({ id: 'fleet-local-node', content: errorMessage(e) })
      setBusy(false)
    }
  }
  return (
    <Modal
      title="启用「本机」节点"
      visible
      onCancel={busy ? undefined : onClose}
      closeOnEsc={!busy}
      style={{ width: 'min(520px, 94vw)' }}
      footer={
        <div className={styles.dialogFoot}>
          <Button onClick={onClose} disabled={busy}>
            取消
          </Button>
          <Button theme="solid" onClick={enable} loading={busy}>
            启用
          </Button>
        </div>
      }
    >
      <ul className={styles.localList}>
        <li>节点列表里多一台「本机」：房间可以手动分派给它，自动选节点时它也是候选。</li>
        <li>
          录制用这台机器自己的「空间配置」，不收 Fleet 配置；「直播管理」里已有的直播间照常录，不会被改动，也不会被导入。
        </li>
        <li>分派来的房间在「直播管理」里标为「托管」、只读；与已有直播间同地址的房间不收，不会重复录制。</li>
        <li>关闭时先交出房间、等它停录确认后再关，不留暂停的直播间。</li>
      </ul>
      <Checkbox checked={allowHooks} onChange={(e) => setAllowHooks(Boolean(e.target.checked))}>
        允许钩子
      </Checkbox>
      <Text type="tertiary" size="small" className={styles.localHint}>
        与 <code>biliup node join --allow-hooks</code> 相同：处理器里带 run 命令（能执行任意命令）的房间才能派到这台机器
      </Text>
    </Modal>
  )
}

/** 控制面还没启用「本机」节点时，节点页顶部的入口 */
export default function LocalNodeOffer({ onEnabled }: { onEnabled: () => void }) {
  const [open, setOpen] = useState(false)
  return (
    <section className={styles.localOffer} aria-label="本机节点">
      <IconServer className={styles.localIcon} />
      <div className={styles.localText}>
        <Text strong>让控制面这台机器也录 Fleet 房间</Text>
        <Text type="tertiary" size="small">
          启用后列表里多一台「本机」，可以手动或自动分派房间给它；已有的直播间不受影响
        </Text>
      </div>
      <Button onClick={() => setOpen(true)}>启用本机节点</Button>
      {open ? (
        <LocalNodeDialog
          onClose={() => setOpen(false)}
          onEnabled={() => {
            setOpen(false)
            onEnabled()
          }}
        />
      ) : null}
    </section>
  )
}
