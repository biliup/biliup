'use client'
import Link from 'next/link'
import { Typography, Space, Spin } from '@douyinfe/semi-ui'
import { IconExternalOpen } from '@douyinfe/semi-icons'
import {
  useDashboard,
  formatSize,
} from '@/app/lib/use-dashboard'
import StreamerCard from '../ui/StreamerCard'
import EventTimeline from '../ui/EventTimeline'
import BackgroundSetter from './components/BackgroundSetter'
import styles from './page.module.scss'

const { Text } = Typography

/**
 * 控制台主页 —— Monitor 表面:2 秒内回答"几个在录、有没有问题"。
 * 数据全部来自现有后端接口(见 use-dashboard),零后端改动。
 */
export default function Home() {
  const d = useDashboard()

  const streamers = d.streamers ?? []
  const live = streamers.filter((s) => s.status === 'Working')
  const offline = streamers.filter((s) => s.status !== 'Working')

  return (
    <div className={styles.page}>
      {/* 顶栏:状态 + 文档 */}
      <div className={styles.head}>
        <Space>
          <a
            className={styles.docLink}
            href="https://doc.biliup.rs/"
            target="_blank"
            rel="noreferrer"
          >
            <IconExternalOpen size="small" /> 文档
          </a>
          {d.version ? <span className={styles.version}>v{d.version}</span> : null}
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
          <Text>
            无法连接后端,请确认 biliup 服务已在{' '}
            <Text strong>http://localhost:19159</Text> 运行,且已登录。
          </Text>
        </div>
      ) : (
        <>
          {d.streamersFailed && (
            <div className={styles.warnBox}>
              <Text>
                主播列表加载失败,监控数量与卡片暂不可用,请检查后端连接或稍后重试。
              </Text>
            </div>
          )}
          {d.infosFailed && (
            <div className={styles.warnBox}>
              <Text>直播信息(标题 / 录制时长)加载失败,相关字段可能缺失。</Text>
            </div>
          )}

          {/* 概览条:全部来自真实聚合(无虚构指标) */}
          <div className={styles.overview}>
            <span className={styles.ovItem}>
              <b>{d.streamersFailed ? '—' : d.total}</b> 个监控
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
            <span className={styles.ovItem}>
              录制文件 <b>{formatSize(d.totalSize)}</b>
            </span>
            <span className={styles.ovItem}>
              今日新增 <b>{formatSize(d.todaySize)}</b>
            </span>
          </div>

          {/* 进行中 */}
          <section className={styles.section}>
            <div className={styles.secHead}>
              <span className={styles.secLabel}>
                进行中<span className={styles.secCount}>{live.length}</span>
              </span>
              <Link href="/streamers" className={styles.secLink}>
                直播管理 →
              </Link>
            </div>
            {d.streamersFailed ? (
              <Text type="tertiary">主播列表加载失败,无法显示卡片。</Text>
            ) : live.length === 0 ? (
              <Text type="tertiary">当前没有正在录制的直播间。</Text>
            ) : (
              <div className={styles.grid}>
                {live.map((s) => (
                  <StreamerCard key={s.id} streamer={s} info={d.infoByUrl.get(s.url)} />
                ))}
              </div>
            )}
          </section>

          {/* 未开播 */}
          <section className={styles.section}>
            <div className={styles.secHead}>
              <span className={styles.secLabel}>
                未开播<span className={styles.secCount}>{offline.length}</span>
              </span>
            </div>
            {offline.length === 0 ? (
              <Text type="tertiary">全部直播间均在录制中。</Text>
            ) : (
              <div className={styles.grid}>
                {offline.map((s) => (
                  <StreamerCard key={s.id} streamer={s} info={d.infoByUrl.get(s.url)} />
                ))}
              </div>
            )}
          </section>

          {/* 最近事件:前端合成的 24h 活动流 */}
          <section className={styles.section}>
            <div className={styles.secHead}>
              <span className={styles.secLabel}>最近事件</span>
              <span className={styles.secNote}>最近 24h</span>
            </div>
            <div className={styles.timelineCard}>
              <EventTimeline events={d.events} />
            </div>
          </section>
        </>
      )}
    </div>
  )
}
