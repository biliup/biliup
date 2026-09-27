'use client'
import Link from 'next/link'
import useSWR from 'swr'
import { IconAlertTriangle, IconChevronRight, IconServer } from '@douyinfe/semi-icons'
import { fetcher } from '@/app/lib/api-streamer'
import { useMe } from '@/app/lib/use-me'
import { FLEET_SUMMARY_KEY, type FleetSummary as Summary } from '@/app/lib/fleet-alerts'
import styles from './fleet-summary.module.scss'

const REFRESH_MS = 10_000

/**
 * 控制台上的 Fleet 汇总，只在以 `--controller` 启动的实例上出现（普通实例不发请求、不渲染）。
 * 整块点进节点页。
 */
export default function FleetSummary({ className }: { className?: string }) {
  const { me } = useMe()
  const controller = me?.fleet_controller === true
  const { data, error, isLoading, mutate } = useSWR<Summary>(controller ? FLEET_SUMMARY_KEY : null, fetcher, {
    refreshInterval: REFRESH_MS,
  })
  if (!controller) return null

  if (!data) {
    return (
      <div className={`${styles.bar} ${className ?? ''}`} aria-busy={isLoading} aria-label="Fleet 汇总">
        <span className={styles.title}>
          <IconServer aria-hidden="true" /> Fleet
        </span>
        {error && !isLoading ? (
          <>
            <span className={styles.error}>汇总加载失败</span>
            <button type="button" className={styles.retry} onClick={() => mutate().catch(() => undefined)}>
              重试
            </button>
          </>
        ) : (
          <span className={styles.loading}>加载中…</span>
        )}
      </div>
    )
  }

  const alerting = data.alerts_open > 0
  return (
    <Link
      href={alerting ? '/nodes?tab=alerts' : '/nodes'}
      prefetch={false}
      className={`${styles.bar} ${styles.link} ${className ?? ''}`}
      aria-label={`Fleet 汇总：在线节点 ${data.nodes_online}/${data.nodes_total}，录制中 ${data.recording}，未清除告警 ${data.alerts}。查看节点`}
    >
      <span className={styles.title}>
        <IconServer aria-hidden="true" /> Fleet
      </span>
      <span className={styles.item}>
        在线节点{' '}
        <b className={data.nodes_online < data.nodes_total ? styles.warn : undefined}>{data.nodes_online}</b>
        <span className={styles.of}>/{data.nodes_total}</span>
      </span>
      <span className={styles.item}>
        {data.recording > 0 ? <span className={styles.rec} aria-hidden="true" /> : null}
        <b>{data.recording}</b> 路录制中
      </span>
      <span className={`${styles.item} ${alerting ? styles.alerting : ''}`}>
        {data.alerts > 0 ? <IconAlertTriangle size="small" aria-hidden="true" /> : null}
        未清除告警 <b>{data.alerts}</b>
        {data.alerts > 0 && data.alerts_open < data.alerts ? (
          <span className={styles.of}>（{data.alerts - data.alerts_open} 条已恢复）</span>
        ) : null}
      </span>
      <span className={styles.go}>
        {alerting ? '查看告警' : '节点'} <IconChevronRight size="small" aria-hidden="true" />
      </span>
    </Link>
  )
}
