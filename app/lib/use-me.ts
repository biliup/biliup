import useSWR from 'swr'
import { fetcher } from './api-streamer'

/** 与后端 `Permission` 的 serde 名一一对应（crates/biliup-cli/src/server/infrastructure/permissions.rs） */
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

export type Role = 'admin' | 'operator' | 'viewer'

export const ROLE_LABELS: Record<Role, string> = {
  admin: '超级管理员',
  operator: '操作员',
  viewer: '只读观察者',
}

export const ROLE_DESCRIPTIONS: Record<Role, string> = {
  admin: '全部权限，包括用户管理、全局配置、B 站账号与后处理命令',
  operator: '管理直播间、控制录制、编辑投稿模板和手动投稿',
  viewer: '只能查看录制状态、预览、日志和录播文件',
}

export interface Me {
  /** `--auth` 关闭时为 null */
  id: number | null
  username: string | null
  role: Role
  permissions: Permission[]
  /** 未开启 `--auth` 时为 false：零鉴权，视为超级管理员 */
  auth_enabled: boolean
}

export const ME_KEY = '/v1/me'

/**
 * 当前登录用户与权限点。按钮显隐只是体验，真正的拦截在后端。
 * 未加载完成时 `can` 一律返回 false，宁可晚一点出现也不先露出再收回。
 */
export function useMe() {
  const { data, error, isLoading } = useSWR<Me>(ME_KEY, fetcher, {
    revalidateOnFocus: false,
    dedupingInterval: 10_000,
  })
  const permissions = data?.permissions
  const can = (permission: Permission) => permissions?.includes(permission) ?? false
  return { me: data, error, isLoading, can }
}
