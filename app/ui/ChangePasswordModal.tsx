'use client'
import { useRef, useState } from 'react'
import { Form, Modal, Toast, Typography } from '@douyinfe/semi-ui'
import type { FormApi } from '@douyinfe/semi-ui/lib/es/form'
import { API_BASE } from '../lib/api-streamer'

export const MIN_PASSWORD_LENGTH = 8

type Values = { old_password: string; new_password: string; confirm: string }

/** 修改自己的密码。成功后当前浏览器保持登录，其它设备上的会话全部失效。 */
export default function ChangePasswordModal({
  visible,
  onClose,
}: {
  visible: boolean
  onClose: () => void
}) {
  const api = useRef<FormApi<Values>>(undefined)
  const [saving, setSaving] = useState(false)

  const submit = async () => {
    let values: Values
    try {
      values = (await api.current?.validate()) as Values
    } catch {
      return
    }
    setSaving(true)
    try {
      const res = await fetch(`${API_BASE}/v1/me/password`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          old_password: values.old_password,
          new_password: values.new_password,
        }),
      })
      if (res.status === 401) {
        window.location.assign('/login')
        return
      }
      if (!res.ok) {
        const body = await res.json().catch(() => null)
        const message = body?.message || `修改失败（HTTP ${res.status}）`
        if (res.status === 400 && message === '当前密码不正确') {
          api.current?.setError('old_password', message)
        } else {
          Toast.error(message)
        }
        return
      }
      Toast.success('密码已修改，其它设备上的登录已失效')
      onClose()
    } catch {
      Toast.error('网络错误，请稍后重试')
    } finally {
      setSaving(false)
    }
  }

  return (
    <Modal
      title="修改密码"
      visible={visible}
      onOk={submit}
      onCancel={onClose}
      okText="保存"
      confirmLoading={saving}
      style={{ width: 'min(440px, 92vw)' }}
      closeOnEsc
    >
      <Form<Values> getFormApi={(formApi) => (api.current = formApi)} labelPosition="top">
        <Form.Input
          field="old_password"
          label="当前密码"
          mode="password"
          autoComplete="current-password"
          rules={[{ required: true, message: '请输入当前密码' }]}
        />
        <Form.Input
          field="new_password"
          label="新密码"
          mode="password"
          autoComplete="new-password"
          rules={[
            { required: true, message: '请输入新密码' },
            {
              validator: (_rule: unknown, value: string) =>
                new TextEncoder().encode(value ?? '').length >= MIN_PASSWORD_LENGTH,
              message: `至少 ${MIN_PASSWORD_LENGTH} 个字符`,
            },
          ]}
        />
        <Form.Input
          field="confirm"
          label="确认新密码"
          mode="password"
          autoComplete="new-password"
          rules={[
            { required: true, message: '请再次输入新密码' },
            {
              validator: (_rule: unknown, value: string) =>
                value === api.current?.getValue('new_password'),
              message: '两次输入不一致',
            },
          ]}
        />
      </Form>
      <Typography.Text type="tertiary" size="small">
        保存后当前浏览器保持登录，其它设备需要用新密码重新登录。
      </Typography.Text>
    </Modal>
  )
}
