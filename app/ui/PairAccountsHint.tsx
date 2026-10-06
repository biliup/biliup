'use client'
import useSWR from 'swr'
import { fetcher } from '../lib/api-streamer'
import { useMe } from '../lib/use-me'
import { humDate } from '../lib/utils'
import { FLEET_HA_KEY, NODE_HA_KEY, type FleetHa, type NodeHa, type Side } from '../lib/fleet-ha'
import styles from './pair-accounts-hint.module.scss'

/**
 * B 站账号侧栏顶部：本机在一主一备里时说明账号在两台之间同步，以及同步到哪了。
 * 只用接口里的个数与时刻，不显示凭据内容与路径；不在配对里时什么都不显示
 */
export default function PairAccountsHint({ visible }: { visible?: boolean }) {
  const { me, can } = useMe()
  const onNode = !!me?.fleet_node?.pair
  const key = !visible || !can('streamer.view') ? null : onNode ? NODE_HA_KEY : me?.fleet_controller ? FLEET_HA_KEY : null
  const { data } = useSWR<FleetHa | NodeHa>(key, fetcher, { refreshInterval: 10_000 })
  const sync = data?.sync
  if (!data || !sync || sync.linked === undefined) return null
  if ('active' in data && !data.active) return null
  const accounts = sync.accounts
  const self: Side = onNode ? 'node' : 'controller'
  const peer = onNode ? '控制面那台' : '配对的另一台'
  return (
    <div className={styles.hint} role="note" aria-label="账号同步">
      账号在一主一备的两台之间同步：在这里登录、刷新或删除，{peer}也跟着变。
      {accounts ? (
        <>
          {' '}现有 {accounts.count} 个
          {accounts.changed_at
            ? `；最近一次变化在 ${humDate(Math.floor(accounts.changed_at / 1000))}（${accounts.changed_on === self ? '这台' : peer}上）`
            : ''}
          ；{accounts.pending ? `${accounts.pending} 个变化还没送到${peer}` : '两台一致'}。
        </>
      ) : null}
      {sync.linked ? null : <span className={styles.warn}> 两台此刻断开：这里的变化先记下，连上后再送过去。</span>}
    </div>
  )
}
