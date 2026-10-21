'use client'
import Link from 'next/link'
import { Typography, Space, Spin } from '@douyinfe/semi-ui'
import { IconExternalOpen } from '@douyinfe/semi-icons'
import {
  useDashboard,
} from '@/app/lib/use-dashboard'
import { formatVersion } from '@/app/lib/status'
import StreamerCard from '../ui/StreamerCard'
import { CardRateSwitch } from '../ui/LiveRateChart'
import EventTimeline from '../ui/EventTimeline'
import SystemStatsPanel from '../ui/SystemStatsPanel'
import BackgroundSetter from './components/BackgroundSetter'
import FleetRevokedBanner from '../ui/FleetRevokedBanner'
import FleetSummary from '../ui/FleetSummary'
import styles from './page.module.scss'

const { Text } = Typography

/**
 * 概览：系统资源、录制状态与最近事件。
 * 概览汇总数据见 use-dashboard,系统状态(CPU / 内存 / 磁盘 / 网速)见 use-system-stats。
 */
export default function Home() {
  const d = useDashboard()

  const streamers = d.streamers ?? []
  const live = streamers.filter((s) => s.status === 'Working')
  const versionText = formatVersion(d.version)

  return (
    <div className={styles.page}>
      <div className={styles.workspace} role="region" aria-label="概览内容" tabIndex={0}>
      {/* 顶栏:状态 + 文档 */}
      <div className={styles.head}>
        <h1 className={styles.heading}>概览</h1>
        <Space>
          <a
            className={styles.docLink}
            href="https://biliup.github.io/biliup/docs/guide/introduction/"
            target="_blank"
            rel="noreferrer"
          >
            <IconExternalOpen size="small" /> 文档
          </a>
          {versionText ? <span className={styles.version}>v{versionText}</span> : null}
          <span
            className={`${styles.statusPill} ${
              d.connectError ? styles.offline : styles.online
            }`}
          >
            <span
              className={`${styles.dot} ${
                d.connectError ? styles.offline : styles.online
              }`}
            />
            {d.connectError
              ? '服务未连接'
              : d.loading
                ? '连接中…'
                : `运行中 · ${d.recording} 路录制中`}
          </span>
        </Space>
      </div>

      <BackgroundSetter />

      {d.loading ? (
        <div style={{ padding: '80px 0', textAlign: 'center' }}>
          <Spin size="large" />
        </div>
      ) : d.connectError ? (
        <div className={styles.errorBox}>
          <Text>无法连接 biliup 后端,请确认服务正在运行且当前会话已登录。</Text>
        </div>
      ) : (
        <>
          <FleetRevokedBanner className={styles.revokedBanner} linkToStreamers />
          <FleetSummary />
          {d.stale && <div className={styles.warnBox}>更新失败，暂时显示上次数据。</div>}
          {/* 概览条:全部来自真实聚合(无虚构指标) */}
          <div className={styles.overview}>
            <span className={styles.ovItem}>
              <b>{d.total}</b> 个监控
            </span>
            <span className={styles.ovItem}>
              <span className={`${styles.ovDot} ${styles.rec}`} />
              <b>{d.recording}</b> 路录制中
            </span>
            <span className={styles.ovItem}>
              <b>{d.pending}</b> 待上传
            </span>
            <span className={styles.ovItem}>
              <b>{d.uploading}</b> 上传中
            </span>
          </div>

          <SystemStatsPanel />
          {/* 进行中 */}
          <section className={styles.section}>
            <div className={styles.secHead}>
              <span className={styles.secLabel}>
                进行中<span className={styles.secCount}>{d.recording}</span>
              </span>
              <span className={styles.secTools}>
                {live.length > 0 ? <CardRateSwitch /> : null}
                <Link href="/streamers" prefetch={false} className={styles.secLink}>
                  直播管理 →
                </Link>
              </span>
            </div>
            {live.length === 0 ? (
              <Text type="tertiary">当前没有正在录制的直播间。</Text>
            ) : (
              <div className={styles.grid}>
                {live.map((s) => (
                  <StreamerCard key={s.id} streamer={s} info={d.infoByUrl.get(s.url)} />
                ))}
              </div>
            )}
          </section>

          {d.recording > live.length && <Link href="/streamers" prefetch={false} className={styles.secLink}>查看全部 {d.recording} 路录制 →</Link>}
        </>
      )}
      </div>
      <aside className={styles.activity} aria-label="最近事件" tabIndex={0}>
        <div className={styles.secHead}>
          <span className={styles.secLabel}>最近事件</span>
          <span className={styles.secNote}>最新 12 条</span>
        </div>
        <EventTimeline events={d.events} error={d.activityFailed} />
      </aside>
    </div>
  )
}
