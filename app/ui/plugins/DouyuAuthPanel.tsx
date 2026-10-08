'use client'
import React, { useEffect, useRef, useState } from 'react'
import { Banner, Button, useFormState } from '@douyinfe/semi-ui'
import useSWR from 'swr'
import { useMe } from '../../lib/use-me'
import {
  DOUYU_LOGIN_LABELS,
  DOUYU_REFRESH_LABELS,
  douyuAuthDate,
  douyuAuthFieldsEqual,
  douyuAuthStatusKey,
  douyuManualRefreshMessage,
  fetchDouyuAuthStatus,
  refreshDouyuAuth,
} from '../../lib/douyu-auth'

/** Only mounted for local saved configuration, never a Fleet editor or room override form. */
export default function DouyuAuthPanel({ savedValues }: { savedValues: Record<string, unknown> }) {
  const { can } = useMe()
  const { values } = useFormState<Record<string, unknown>>()
  const hostRef = useRef<HTMLDivElement>(null)
  const [visible, setVisible] = useState(false)
  const [refreshing, setRefreshing] = useState(false)
  const [actionMessage, setActionMessage] = useState<string | null>(null)
  const busyRef = useRef(false)
  const previousSavedRef = useRef(savedValues)
  const allowed = can('config.edit')

  // Platform panels stay mounted while hidden. Fetch and poll only while this panel is visible.
  useEffect(() => {
    const host = hostRef.current
    if (!host || !allowed) {
      setVisible(false)
      return
    }
    if (typeof IntersectionObserver === 'undefined') {
      setVisible(host.getClientRects().length > 0)
      return
    }
    const observer = new IntersectionObserver((entries) => setVisible(entries.some((entry) => entry.isIntersecting)))
    observer.observe(host)
    return () => observer.disconnect()
  }, [allowed])

  const { data: status, error, isLoading, mutate } = useSWR(
    allowed && visible ? douyuAuthStatusKey() : null,
    fetchDouyuAuthStatus,
    { refreshInterval: 30_000, revalidateOnFocus: true, refreshWhenHidden: false, shouldRetryOnError: false },
  )

  const dirty = !douyuAuthFieldsEqual(values ?? {}, savedValues)

  useEffect(() => {
    const changed = !douyuAuthFieldsEqual(previousSavedRef.current, savedValues)
    previousSavedRef.current = savedValues
    if (changed && allowed && visible) void mutate().catch(() => undefined)
  }, [savedValues, allowed, visible, mutate])

  const handleRefresh = async () => {
    if (!allowed || dirty || !status || busyRef.current || status.refresh_state === 'refreshing') return
    const previous = status
    busyRef.current = true
    setRefreshing(true)
    setActionMessage(null)
    try {
      const next = await refreshDouyuAuth()
      await mutate(next, { revalidate: false })
      setActionMessage(douyuManualRefreshMessage(previous, next))
    } catch {
      setActionMessage('续期请求未完成，请查看状态或稍后重试。')
      await mutate().catch(() => undefined)
    } finally {
      busyRef.current = false
      setRefreshing(false)
    }
  }

  if (!allowed) return null
  const hasPair = status?.has_ltp0 && status?.has_device_id
  const busy = refreshing || status?.refresh_state === 'refreshing'

  return (
    <div ref={hostRef} style={{ margin: '12px 0 20px', padding: 16, background: 'var(--semi-color-fill-0)', borderRadius: 6 }}>
      <div style={{ fontWeight: 600, marginBottom: 10 }}>本机已保存的斗鱼登录状态</div>
      <div style={{ fontSize: 13, color: 'var(--semi-color-text-2)', marginBottom: 12 }}>
        这里显示本机已保存来源凭据的运行状态；表单修改须先保存才会用于续期。
        续期后的 Cookie 由程序单独维护，输入框保留导入的来源值。
      </div>
      {status ? (
        <div aria-live="polite" style={{ display: 'grid', gap: 6, fontSize: 14 }}>
          <div>登录：{DOUYU_LOGIN_LABELS[status.login_state]}{status.account_id ? ` · 账号 ${status.account_id}` : ''}</div>
          <div>续期：{DOUYU_REFRESH_LABELS[status.refresh_state]}</div>
          <div>上次登录校验：{douyuAuthDate(status.last_checked_at)}</div>
          <div>上次续期成功：{douyuAuthDate(status.last_success_at)}</div>
          <div>下次续期 / 重试：{status.next_refresh_at ? douyuAuthDate(status.next_refresh_at) : '尚未安排'}</div>
        </div>
      ) : (
        <div role="status">{error ? '登录状态暂时无法加载，请稍后重试。' : isLoading ? '正在读取登录状态…' : '等待读取登录状态…'}</div>
      )}
      {status?.needs_login || status?.refresh_state === 'credentials_invalid' ? (
        <Banner
          type="danger" fullMode={false} closeIcon={null} style={{ marginTop: 12 }}
          title="斗鱼登录凭据已失效"
          description="请重新登录斗鱼，导出同一账号、同一次登录的 Web Cookie、LTP0 和 dy_did，再保存配置。"
        />
      ) : status?.refresh_state === 'missing_credentials' ? (
        <Banner
          type="warning" fullMode={false} closeIcon={null} style={{ marginTop: 12 }}
          description="自动续期需要同一次登录取得的 LTP0 和 dy_did；可单独填写，也可从完整 Cookie JSON 中导入。"
        />
      ) : status?.refresh_state === 'retry' ? (
        <Banner
          type="warning" fullMode={false} closeIcon={null} style={{ marginTop: 12 }}
          description={`续期暂时失败，已安排退避重试${status.failure_count ? `（连续 ${status.failure_count} 次）` : ''}，现有已保存 Cookie 会保留。`}
        />
      ) : null}
      <div style={{ display: 'flex', alignItems: 'center', flexWrap: 'wrap', gap: 12, marginTop: 12 }}>
        <Button
          htmlType="button" loading={busy} disabled={dirty || !status || !hasPair || busy}
          onClick={handleRefresh}
        >
          手动续期
        </Button>
        {dirty ? <span style={{ fontSize: 13, color: 'var(--semi-color-warning)' }}>凭据或续期开关有未保存修改，请先保存。</span> : null}
        {error ? <Button htmlType="button" onClick={() => mutate()}>重试加载</Button> : null}
      </div>
      {actionMessage ? <div role="status" style={{ marginTop: 10, fontSize: 13 }}>{actionMessage}</div> : null}
    </div>
  )
}
