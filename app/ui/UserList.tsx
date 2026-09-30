import React, { useCallback, useRef, useState } from 'react'
import { requestDelete, sendRequest } from '../lib/api-streamer'
import { Button, Empty, Form, List, Notification, Radio, RadioGroup, Spin, Toast, Typography } from '@douyinfe/semi-ui'
import AvatarCard from './AvatarCard'
import { IconPlusCircle } from '@douyinfe/semi-icons'
import { FormApi } from '@douyinfe/semi-ui/lib/es/form'
import useSWRMutation from 'swr/mutation'
import { useBiliUsers } from '../lib/use-streamers'
import QRcode from '@/app/ui/QRcode'
import PairAccountsHint from './PairAccountsHint'
import { FormDialog, FormSheet } from './shell'

type UserListProps = {
  onCancel?: () => void
  visible?: boolean
}

type Method = 'cookie' | 'qrcode'

function errorText(e: any): string {
  const raw = e?.message ?? String(e)
  try {
    return JSON.parse(raw).error ?? JSON.parse(raw).message ?? raw
  } catch {
    return raw
  }
}

/**
 * 添加 B 站账号：内容少，用居中小弹窗（从账号抽屉里打开，最多叠这一层）。
 * 默认「Cookie 文件」：改造前的第一项、内容最少（一个输入框），打开时也不会先去 B 站拉二维码
 */
function AddAccountDialog({ onClose, onAdd }: { onClose: () => void; onAdd: (value: string) => Promise<boolean> }) {
  const [method, setMethod] = useState<Method>('cookie')
  const api = useRef<FormApi>(undefined)
  const submitCookie = async () => {
    const values = await api.current?.validate()
    if (!(await onAdd(values?.value?.trim()))) throw new Error('add failed')
  }
  return (
    <FormDialog
      title="添加 B 站账号"
      size="sm"
      onCancel={onClose}
      okText={method === 'cookie' ? '添加' : undefined}
      onOk={submitCookie}
      footerExtra={method === 'qrcode' ? '在 B 站 App 里扫码确认后自动添加' : undefined}
    >
      <RadioGroup type="button" value={method} onChange={(e) => setMethod(e.target.value as Method)} aria-label="添加方式">
        <Radio value="cookie">Cookie 文件</Radio>
        <Radio value="qrcode">扫码登录</Radio>
      </RadioGroup>
      {method === 'cookie' ? (
        <Form getFormApi={(formApi) => (api.current = formApi)} onSubmit={submitCookie} style={{ marginTop: 12 }}>
          <Form.Input
            field="value"
            label="Cookie 文件路径"
            placeholder="cookies.json"
            trigger="blur"
            rules={[{ required: true, message: '填写 biliup 所在机器上的凭据文件路径' }]}
            extraText="biliup login 生成的凭据文件；相对路径从 biliup 的工作目录算起"
          />
        </Form>
      ) : (
        <QRcode onSuccess={onAdd} />
      )}
    </FormDialog>
  )
}

/** 投稿管理页的「B 站账号」：可增删的账号列表用抽屉（背后留着投稿模板），「+ 添加账号」在底栏 */
const UserList: React.FC<UserListProps> = ({ onCancel, visible }) => {
  const { trigger } = useSWRMutation('/v1/users', sendRequest)
  const { trigger: deleteUser } = useSWRMutation('/v1/users', requestDelete)
  const { biliUsers: list, isLoading } = useBiliUsers()
  const [adding, setAdding] = useState(false)

  /** 成功返回 true；失败已弹出原因 */
  const addUser = useCallback(
    async (value: string) => {
      try {
        await trigger({ value })
        setAdding(false)
        Toast.success('账号已添加')
        return true
      } catch (e: any) {
        Notification.error({
          title: '添加失败',
          content: <Typography.Paragraph style={{ maxWidth: 450 }}>{errorText(e)}</Typography.Paragraph>,
          style: { width: 'min-content' },
        })
        return false
      }
    },
    [trigger],
  )

  const remove = async (id: number) => {
    try {
      await deleteUser(id)
      Toast.success('已删除')
    } catch (e: any) {
      Notification.error({
        title: '删除失败',
        content: <Typography.Paragraph style={{ maxWidth: 450 }}>{errorText(e)}</Typography.Paragraph>,
        style: { width: 'min-content' },
      })
    }
  }

  const close = () => {
    setAdding(false)
    onCancel?.()
  }

  return (
    <>
      <FormSheet
        size="sm"
        visible={visible}
        title="B 站账号"
        onCancel={close}
        cancelText={null}
        okText={list.length > 0 ? '添加账号' : undefined}
        okIcon={<IconPlusCircle />}
        okTheme="light"
        onOk={() => setAdding(true)}
      >
        <PairAccountsHint visible={visible} />
        {isLoading ? (
          <div style={{ padding: '48px 0', textAlign: 'center' }}>
            <Spin />
          </div>
        ) : list.length === 0 ? (
          <Empty title="还没有 B 站账号" description="投稿模板要选一个账号才能投稿" style={{ padding: '48px 0' }}>
            <Button icon={<IconPlusCircle />} theme="solid" onClick={() => setAdding(true)}>
              添加账号
            </Button>
          </Empty>
        ) : (
          <List
            dataSource={list}
            split={false}
            size="small"
            renderItem={(item) => (
              <AvatarCard
                url={item.face}
                abbr={item.name}
                label={item.name}
                value={item.value}
                onRemove={async () => await remove(item.id)}
              />
            )}
          />
        )}
      </FormSheet>
      {visible && adding ? <AddAccountDialog onClose={() => setAdding(false)} onAdd={addUser} /> : null}
    </>
  )
}

export default UserList
