import useSWR from 'swr'
import { fetcher, ME_KEY } from './api-streamer'

export { ME_KEY }

/**
 * 与后端 `Permission` 的 serde 名一一对应（crates/biliup-cli/src/server/infrastructure/permissions.rs）。
 * 后端测试 `names_match_the_frontend_contract` 会读这个文件核对，两边增删必须同步。
 */
export type Permission =
  | 'streamer.view'
  | 'preview.view'
  | 'recording.control'
  | 'streamer.edit'
  | 'streamer.hooks'
  | 'upload.submit'
  | 'template.edit'
  | 'account.manage'
  | 'config.view'
  | 'config.edit'
  | 'log.view'
  | 'file.view'
  | 'user.manage'
  | 'clip.edit'

/** 同上，与后端 `Role` 对应 */
export type Role = 'admin' | 'operator' | 'viewer'

export const ROLE_LABELS: Record<Role, string> = {
  admin: '超级管理员',
  operator: '操作员',
  viewer: '只读观察者',
}

/** 权限点的显示名。角色拥有哪些权限点由后端下发（`/v1/web-users/roles`），前端不再手写。 */
export const PERMISSION_LABELS: Record<Permission, string> = {
  'streamer.view': '查看直播间',
  'preview.view': '观看预览',
  'recording.control': '启停录制',
  'streamer.edit': '编辑直播间',
  'streamer.hooks': '后处理命令',
  'upload.submit': '手动投稿',
  'template.edit': '编辑投稿模板',
  'account.manage': '管理 B 站账号',
  'config.view': '查看配置',
  'config.edit': '修改配置',
  'log.view': '查看日志',
  'file.view': '查看录播文件',
  'user.manage': '管理用户',
  'clip.edit': '打标记',
}

export interface Me {
  /** `--auth` 关闭时为 null */
  id: number | null
  username: string | null
  role: Role
  /** 后端授权决策点算出的实际权限（已考虑 `--auth` 是否开启等环境属性），前端只按它显隐 */
  permissions: Permission[]
  /** 未开启 `--auth` 时为 false：零鉴权，视为超级管理员 */
  auth_enabled: boolean
}

/**
 * 当前登录用户与权限点。按钮显隐只是体验，真正的拦截在后端。
 * 未加载完成时 `can` 一律返回 false，宁可晚一点出现也不先露出再收回。
 * 回到页面时、以及任何请求收到 403 或长连接断开时都会重新拉取（见 `revalidateMe`），
 * 角色被改后界面随之收起，不必手动刷新。
 */
export function useMe() {
  const { data, error, isLoading } = useSWR<Me>(ME_KEY, fetcher, {
    revalidateOnFocus: true,
    dedupingInterval: 10_000,
  })
  const permissions = data?.permissions
  const can = (permission: Permission) => permissions?.includes(permission) ?? false
  return { me: data, error, isLoading, can }
}
