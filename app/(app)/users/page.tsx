'use client'
import { useRef, useState } from 'react'
import useSWR from 'swr'
import {
  Button,
  Empty,
  Form,
  Modal,
  Popconfirm,
  Spin,
  Table,
  Tag,
  Toast,
  Typography,
} from '@douyinfe/semi-ui'
import { IconPlusCircle, IconUserGroup } from '@douyinfe/semi-icons'
import type { FormApi } from '@douyinfe/semi-ui/lib/es/form'
import PageHeader from '../components/PageHeader'
import dc from '@/app/ui/data-card.module.scss'
import { API_BASE, fetcher, revalidateMe } from '@/app/lib/api-streamer'
import { PERMISSION_LABELS, ROLE_LABELS, useMe } from '@/app/lib/use-me'
import type { Permission, Role } from '@/app/lib/use-me'
import { MIN_PASSWORD_LENGTH } from '@/app/ui/ChangePasswordModal'
import { humDate } from '@/app/lib/utils'
import { timeAgo } from '@/app/lib/use-dashboard'
import { useIsMobile } from '@/app/lib/useIsMobile'
import styles from './page.module.scss'

const { Text } = Typography

interface WebUser {
  id: number
  username: string
  role: Role
  disabled: boolean
  created_at: number
  last_login_at: number | null
}

/** 角色目录：每个角色实际拥有的权限点，由后端授权决策点给出 */
interface RoleInfo {
  role: Role
  permissions: Permission[]
}

const USERS_KEY = '/v1/web-users'
const ROLES_KEY = '/v1/web-users/roles'
const ALL_PERMISSIONS = Object.keys(PERMISSION_LABELS) as Permission[]
const ROLE_COLORS: Record<Role, 'red' | 'blue' | 'grey'> = {
  admin: 'red',
  operator: 'blue',
  viewer: 'grey',
}

/** 调用用户管理接口；失败时抛出带服务端中文提示的 Error */
async function call(method: string, url: string, body?: unknown) {
  const res = await fetch(API_BASE + url, {
    method,
    headers: body === undefined ? undefined : { 'Content-Type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  })
  if (res.status === 401) {
    window.location.assign(`/login?next=${encodeURIComponent(window.location.pathname)}`)
    throw new Error('登录已失效')
  }
  // 自己的用户管理权限被收回了：刷新权限点，页面随之换成无权限提示
  if (res.status === 403) revalidateMe()
  if (!res.ok) {
    const data = await res.json().catch(() => null)
    throw new Error(data?.message || `操作失败（HTTP ${res.status}）`)
  }
  return res
}

const passwordRules = [
  { required: true, message: '请输入密码' },
  {
    validator: (_rule: unknown, value: string) =>
      new TextEncoder().encode(value ?? '').length >= MIN_PASSWORD_LENGTH,
    message: `至少 ${MIN_PASSWORD_LENGTH} 个字符`,
  },
]

/** 按后端给的权限点描述角色，前端不再手写「哪个角色能做什么」 */
function describe(permissions: Permission[]) {
  if (ALL_PERMISSIONS.every((p) => permissions.includes(p))) return '全部权限'
  return permissions.map((p) => PERMISSION_LABELS[p] ?? p).join('、')
}

function RoleOptions({ roles }: { roles: RoleInfo[] }) {
  return (
    <>
      {roles.map(({ role, permissions }) => (
        <Form.Radio key={role} value={role} className={styles.roleOption}>
          <span className={styles.roleName}>{ROLE_LABELS[role]}</span>
          <span className={styles.roleDesc}>{describe(permissions)}</span>
        </Form.Radio>
      ))}
    </>
  )
}

type Dialog =
  | { kind: 'create' }
  | { kind: 'role'; user: WebUser }
  | { kind: 'password'; user: WebUser }
  | null

type DialogValues = { username?: string; password?: string; role?: Role }

function UserDialog({
  dialog,
  roles,
  onClose,
  onDone,
}: {
  dialog: Exclude<Dialog, null>
  roles: RoleInfo[]
  onClose: () => void
  onDone: () => void
}) {
  const api = useRef<FormApi<DialogValues>>(undefined)
  const [saving, setSaving] = useState(false)

  const title =
    dialog.kind === 'create'
      ? '新建用户'
      : dialog.kind === 'role'
        ? `修改角色：${dialog.user.username}`
        : `重置密码：${dialog.user.username}`

  const submit = async () => {
    let values: DialogValues
    try {
      values = (await api.current?.validate()) as DialogValues
    } catch {
      return
    }
    setSaving(true)
    try {
      if (dialog.kind === 'create') {
        await call('POST', USERS_KEY, {
          username: values.username?.trim(),
          password: values.password,
          role: values.role,
        })
        Toast.success(`已创建用户 ${values.username?.trim()}`)
      } else if (dialog.kind === 'role') {
        await call('PUT', `${USERS_KEY}/${dialog.user.id}`, { role: values.role })
        Toast.success('角色已更新，对方下一次操作即按新角色生效')
      } else {
        await call('PUT', `${USERS_KEY}/${dialog.user.id}`, { password: values.password })
        Toast.success('密码已重置，该用户在所有设备上需要重新登录')
      }
      onDone()
      onClose()
    } catch (e) {
      const message = (e as Error).message
      if (dialog.kind === 'create' && message === '用户名已存在') {
        api.current?.setError('username', message)
      } else {
        Toast.error(message)
      }
    } finally {
      setSaving(false)
    }
  }

  return (
    <Modal
      title={title}
      visible
      onOk={submit}
      onCancel={onClose}
      okText={dialog.kind === 'create' ? '创建' : '保存'}
      confirmLoading={saving}
      style={{ width: 'min(520px, 94vw)' }}
      closeOnEsc
    >
      <Form<DialogValues>
        getFormApi={(formApi) => (api.current = formApi)}
        labelPosition="top"
        initValues={{ role: dialog.kind === 'role' ? dialog.user.role : 'viewer' }}
      >
        {dialog.kind === 'create' && (
          <Form.Input
            field="username"
            label="用户名"
            autoComplete="off"
            maxLength={32}
            rules={[
              { required: true, message: '请输入用户名' },
              {
                validator: (_rule: unknown, value: string) => !!value && value.trim() === value,
                message: '首尾不能有空格',
              },
            ]}
          />
        )}
        {dialog.kind !== 'role' && (
          <Form.Input
            field="password"
            label={dialog.kind === 'create' ? '初始密码' : '新密码'}
            mode="password"
            autoComplete="new-password"
            rules={passwordRules}
          />
        )}
        {dialog.kind !== 'password' && (
          <Form.RadioGroup field="role" label="角色" direction="vertical" rules={[{ required: true }]}>
            <RoleOptions roles={roles} />
          </Form.RadioGroup>
        )}
      </Form>
    </Modal>
  )
}

export default function UsersPage() {
  const { me } = useMe()
  const isMobile = useIsMobile()
  // 六列表格放在侧栏旁边至少要 960 左右的视口；更窄时改成卡片，操作按钮不会被裁掉
  const compact = useIsMobile(960)
  const { data: users, error, isLoading, mutate } = useSWR<WebUser[]>(USERS_KEY, fetcher)
  const { data: roles, error: rolesError, isLoading: rolesLoading, mutate: mutateRoles } = useSWR<RoleInfo[]>(
    ROLES_KEY,
    fetcher,
    { revalidateOnFocus: false },
  )
  const [dialog, setDialog] = useState<Dialog>(null)

  const refresh = () => {
    mutate().catch(() => undefined)
    if (rolesError) mutateRoles().catch(() => undefined)
  }

  const run = async (action: () => Promise<unknown>, success: string) => {
    try {
      await action()
      Toast.success(success)
      refresh()
    } catch (e) {
      Toast.error((e as Error).message)
    }
  }

  const toggleDisabled = (user: WebUser) =>
    run(
      () => call('PUT', `${USERS_KEY}/${user.id}`, { disabled: !user.disabled }),
      user.disabled ? `已启用 ${user.username}` : `已禁用 ${user.username}，其登录会话立即失效`,
    )
  const logoutAll = (user: WebUser) =>
    run(() => call('POST', `${USERS_KEY}/${user.id}/logout-all`), `${user.username} 已在所有设备下线`)
  const remove = (user: WebUser) =>
    run(() => call('DELETE', `${USERS_KEY}/${user.id}`), `已删除 ${user.username}`)

  const isSelf = (user: WebUser) => me?.id === user.id

  const actions = (user: WebUser) => {
    const self = isSelf(user)
    return (
      <div className={styles.actions}>
        <Button size="small" theme="borderless" disabled={self} onClick={() => setDialog({ kind: 'role', user })}>
          角色
        </Button>
        <Button size="small" theme="borderless" onClick={() => setDialog({ kind: 'password', user })}>
          重置密码
        </Button>
        <Popconfirm
          title={user.disabled ? `启用 ${user.username}？` : `禁用 ${user.username}？`}
          content={user.disabled ? '启用后可以重新登录' : '禁用后无法登录，已登录的会话立即失效'}
          onConfirm={() => toggleDisabled(user)}
          disabled={self}
        >
          <Button size="small" theme="borderless" type={user.disabled ? 'primary' : 'warning'} disabled={self}>
            {user.disabled ? '启用' : '禁用'}
          </Button>
        </Popconfirm>
        <Popconfirm
          title={`让 ${user.username} 在所有设备下线？`}
          content={self ? '包括当前这个浏览器' : '对方需要重新登录'}
          onConfirm={() => logoutAll(user)}
        >
          <Button size="small" theme="borderless" type="tertiary">
            强制下线
          </Button>
        </Popconfirm>
        <Popconfirm
          title={`删除 ${user.username}？`}
          content="此操作不可恢复"
          okType="danger"
          onConfirm={() => remove(user)}
          disabled={self}
        >
          <Button size="small" theme="borderless" type="danger" disabled={self}>
            删除
          </Button>
        </Popconfirm>
      </div>
    )
  }

  const nameCell = (user: WebUser) => (
    <span className={styles.nameCell}>
      <Text strong ellipsis={{ showTooltip: true }} style={{ maxWidth: 220 }}>
        {user.username}
      </Text>
      {isSelf(user) && (
        <Tag size="small" color="green">
          当前账号
        </Tag>
      )}
    </span>
  )
  const roleCell = (user: WebUser) => (
    <Tag size="small" color={ROLE_COLORS[user.role]}>
      {ROLE_LABELS[user.role]}
    </Tag>
  )
  const statusCell = (user: WebUser) =>
    user.disabled ? <Text type="danger">已禁用</Text> : <Text type="success">正常</Text>
  const lastLogin = (user: WebUser) =>
    user.last_login_at ? (
      <span title={humDate(user.last_login_at)}>{timeAgo(user.last_login_at)}</span>
    ) : (
      <Text type="tertiary">从未登录</Text>
    )

  const columns = [
    { title: '用户名', dataIndex: 'username', render: (_: unknown, u: WebUser) => nameCell(u) },
    { title: '角色', dataIndex: 'role', width: 130, render: (_: unknown, u: WebUser) => roleCell(u) },
    { title: '状态', dataIndex: 'disabled', width: 90, render: (_: unknown, u: WebUser) => statusCell(u) },
    { title: '最后登录', dataIndex: 'last_login_at', width: 130, render: (_: unknown, u: WebUser) => lastLogin(u) },
    {
      title: '创建时间',
      dataIndex: 'created_at',
      width: 180,
      render: (t: number) => <Text type="tertiary">{humDate(t)}</Text>,
    },
    { title: '操作', dataIndex: 'id', width: 360, render: (_: unknown, u: WebUser) => actions(u) },
  ]

  let body
  if (isLoading || rolesLoading) {
    body = (
      <div className={styles.center}>
        <Spin size="large" />
      </div>
    )
  } else if (error || rolesError || !roles) {
    body = (
      <div className={styles.center}>
        <Empty
          title="加载失败"
          description={((error || rolesError) as Error | undefined)?.message || '无法获取用户列表'}
        />
        <Button onClick={refresh} style={{ marginTop: 12 }}>
          重试
        </Button>
      </div>
    )
  } else if (!users || users.length === 0) {
    body = (
      <div className={styles.center}>
        <Empty title="还没有用户" description="点击右上角「新建用户」添加第一个账号" />
      </div>
    )
  } else if (compact) {
    body = (
      <div className={styles.cards}>
        {users.map((user) => (
          <div key={user.id} className={`${dc.card} ${styles.userCard}`}>
            <div className={styles.cardHead}>
              {nameCell(user)}
              {roleCell(user)}
            </div>
            <div className={styles.cardMeta}>
              {statusCell(user)}
              <span>最后登录 {lastLogin(user)}</span>
            </div>
            {actions(user)}
          </div>
        ))}
      </div>
    )
  } else {
    body = (
      <div className={dc.card}>
        <Table columns={columns} dataSource={users} rowKey="id" pagination={false} size="middle" scroll={{ x: 1060 }} />
      </div>
    )
  }

  const userCount = users?.length ?? 0
  const adminCount = users?.filter((u) => u.role === 'admin' && !u.disabled).length ?? 0

  return (
    <>
      <PageHeader
        icon={<IconUserGroup size="large" />}
        title="用户管理"
        description="为其他人开设账号并分配角色。改动即时生效，至少保留一个启用中的超级管理员"
        actions={
          <Button
            icon={<IconPlusCircle />}
            theme="solid"
            disabled={!roles}
            onClick={() => setDialog({ kind: 'create' })}
          >
            {isMobile ? '新建' : '新建用户'}
          </Button>
        }
      />
      <div className={dc.content}>
        {userCount > 0 && (
          <div className={dc.kpiStrip}>
            <span className={dc.kpiItem}>
              用户 <b>{userCount}</b>
            </span>
            <span className={dc.kpiItem}>
              启用中的超级管理员 <b>{adminCount}</b>
            </span>
          </div>
        )}
        {body}
        <div className={styles.legend}>
          {roles?.map(({ role, permissions }) => (
            <div key={role} className={styles.legendItem}>
              {roleCell({ role } as WebUser)}
              <Text type="tertiary" size="small">
                {describe(permissions)}
              </Text>
            </div>
          ))}
        </div>
      </div>
      {dialog && roles && (
        <UserDialog dialog={dialog} roles={roles} onClose={() => setDialog(null)} onDone={refresh} />
      )}
    </>
  )
}
