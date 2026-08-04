'use client'
import Link from 'next/link'
import useSWR from 'swr'
import { Typography, Space, Spin } from '@douyinfe/semi-ui'
import { IconExternalOpen } from '@douyinfe/semi-icons'
import {
  fetcher,
  LiveStreamerEntity,
  StreamerInfo,
  BiliupStatus,
} from '@/app/lib/api-streamer'
import { statusMeta, uploadStatusTag, platformName } from '@/app/lib/status'
import BackgroundSetter from './components/BackgroundSetter'
import styles from './page.module.scss'

const { Text } = Typography

function timeAgo(ts: number): string {
  const diff = Math.floor((Date.now() - ts * 1000) / 1000)
  if (diff < 60) return '刚刚'
  if (diff < 3600) return `${Math.floor(diff / 60)} 分钟前`
  if (diff < 86400) return `${Math.floor(diff / 3600)} 小时前`
  return `${Math.floor(diff / 86400)} 天前`
}

// 状态排序：录制中优先，让"正在发生"排在最前面
const STATUS_ORDER: Record<string, number> = {
  Working: 0,
  Pending: 1,
  Idle: 2,
  OutOfSchedule: 3,
  Pause: 4,
}

export default function Home() {
  // 前端轮询：页面常驻时保持状态新鲜
  // （录制数 / 上传状态 / 服务状态 / 直播标题都会在后台变化时自动刷新）
  const REFRESH_MS = 4000
  const swrOpts = { refreshInterval: REFRESH_MS, revalidateOnFocus: true }
  const { data: streamers, error: e1 } = useSWR<LiveStreamerEntity[]>(
    '/v1/streamers',
    fetcher,
    swrOpts
  )
  const { data: infos, error: e2 } = useSWR<StreamerInfo[]>(
    '/v1/streamer-info',
    fetcher,
    swrOpts
  )
  const { data: status, error: statusError } = useSWR<BiliupStatus>(
    '/v1/status',
    fetcher,
    swrOpts
  )

  // url -> 最近录制时间（取最新）
  const recentByUrl = new Map<string, number>()
  ;(infos ?? []).forEach((i) => {
    if (i.url) {
      recentByUrl.set(i.url, Math.max(recentByUrl.get(i.url) ?? 0, i.date))
    }
  })

  // url -> 最新直播标题（卡片主角「直播间标题」）
  const titleByUrl = new Map<string, { title: string; date: number }>()
  ;(infos ?? []).forEach((i) => {
    if (i.url && i.title) {
      const cur = titleByUrl.get(i.url)
      if (!cur || i.date > cur.date) titleByUrl.set(i.url, { title: i.title, date: i.date })
    }
  })

  const list = (streamers ?? [])
    .slice()
    .sort(
      (a, b) =>
        (STATUS_ORDER[a.status ?? ''] ?? 9) - (STATUS_ORDER[b.status ?? ''] ?? 9)
    )

  const total = list.length
  const recording = list.filter((s) => s.status === 'Working').length
  const pending = list.filter((s) => s.upload_status === 'Pending').length
  const uploading = list.filter((s) => s.upload_status === 'Working').length

  // 接口返回空数组（后端可达但无监控）也算"有响应"，因此用 undefined 判断是否存在响应
  const hasAnyResponse =
    streamers !== undefined || infos !== undefined || status !== undefined

  const loading = !hasAnyResponse && !e1 && !e2 && !statusError

  // 各接口分别判错：关键接口（streamers）失败时绝不能把缺失当成"0 个监控"
  const streamersFailed = !!e1 && streamers === undefined
  const infosFailed = !!e2 && infos === undefined
  const statusFailed = !!statusError && status === undefined

  // 后端完全不可达：所有关键数据都缺失且至少一处报错
  const connectError = !hasAnyResponse && (!!e1 || !!e2 || !!statusError)

  const isOnline = hasAnyResponse
  const isOffline = !hasAnyResponse
  const statusModifier = isOnline ? styles.online : styles.offline
  const statusText = isOnline
    ? `运行中 · ${recording} 路录制中`
    : '服务未连接'

  return (
    <div className={styles.page}>
      {/* 顶栏：状态灯 + 文档 */}
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
          <span className={`${styles.statusPill} ${statusModifier}`}>
            <span className={`${styles.dot} ${statusModifier}`} /> {statusText}
          </span>
        </Space>
      </div>

      <BackgroundSetter />

      {loading ? (
        <div style={{ padding: '80px 0', textAlign: 'center' }}>
          <Spin size="large" />
        </div>
      ) : connectError ? (
        <div className={styles.errorBox}>
          <Text>
            无法连接后端，请确认 biliup 服务已在 <Text strong>http://localhost:19159</Text>{' '}
            运行，且已登录。
          </Text>
        </div>
      ) : (
        <>
          {streamersFailed && (
            <div className={styles.warnBox}>
              <Text>
                主播列表加载失败，监控数量与卡片暂不可用，请检查后端连接或稍后重试。
              </Text>
            </div>
          )}
          {infosFailed && (
            <div className={styles.warnBox}>
              <Text>直播信息（标题 / 最近录制时间）加载失败，相关字段可能缺失。</Text>
            </div>
          )}
          {statusFailed && (
            <div className={styles.warnBox}>
              <Text>服务状态获取失败，顶部状态灯可能不准确。</Text>
            </div>
          )}
          {/* 紧凑概览条 */}
          <div className={styles.overview}>
            <span className={styles.ovItem}>
              <b>{streamersFailed ? '—' : total}</b> 个监控
            </span>
            <span className={styles.ovItem}>
              <span className={styles.ovDotRec} />
              <b>{recording}</b> 路录制中
            </span>
            <span className={styles.ovItem}>
              <b>{pending}</b> 待上传
            </span>
            <span className={styles.ovItem}>
              <b>{uploading}</b> 上传中
            </span>
          </div>

          {/* 进行中：卡片网格 */}
          <section className={styles.block}>
            <div className={styles.secHead}>
              <span className={styles.secLabel}>进行中</span>
              <Link href="/streamers" className={styles.viewAll}>
                直播管理 →
              </Link>
            </div>
            <div className={styles.grid}>
              {streamersFailed ? (
                <Text type="tertiary" className={styles.empty}>
                  主播列表加载失败，无法显示卡片。
                </Text>
              ) : (
                <>
                  {list.map((s) => {
                    const meta = statusMeta(s.status)
                    const isRec = s.status === 'Working'
                    const recent = recentByUrl.get(s.url)
                    return (
                      <div
                        key={s.id}
                        className={`${styles.card} ${isRec ? styles.rec : ''}`}
                      >
                        <div className={styles.cardHead}>
                          <span className={styles.cardStatus}>
                            <span className={`${styles.recDot} ${styles[meta.cls]}`} />
                            {s.remark ? `[${s.remark}]` : '[未命名]'}
                          </span>
                          <span className={styles.cardPlat}>
                            {platformName(s.url)}
                          </span>
                        </div>
                        <div
                          className={styles.cardName}
                          title={titleByUrl.get(s.url)?.title || s.remark || s.url}
                        >
                          {titleByUrl.get(s.url)?.title || s.remark || s.url}
                        </div>
                        <a
                          className={styles.cardSub}
                          href={s.url}
                          target="_blank"
                          rel="noreferrer"
                          title={s.url}
                        >
                          {s.url}
                        </a>
                        <div className={styles.cardMeta}>
                          {uploadStatusTag(s.upload_status)}
                          {recent ? (
                            <span className={styles.metaTime}>
                              最近录制 {timeAgo(recent)}
                            </span>
                          ) : null}
                        </div>
                      </div>
                    )
                  })}
                  {list.length === 0 && (
                    <Text type="tertiary" className={styles.empty}>
                      暂无监控中的直播间，
                      <Link href="/streamers" className={styles.emptyLink}>
                        去添加 →
                      </Link>
                    </Text>
                  )}
                </>
              )}
            </div>
          </section>
        </>
      )}
    </div>
  )
}
