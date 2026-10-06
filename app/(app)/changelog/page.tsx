'use client'
import React from 'react'
import { Layout, Card, Button, Spin, Empty, Tag, Typography, Tooltip } from '@douyinfe/semi-ui'
import useSWR from 'swr'
import { IconBook, IconExport } from '@douyinfe/semi-icons'
import PageHeader from '../components/PageHeader'
import { fetcher } from '@/app/lib/api-streamer'
import { formatVersion } from '@/app/lib/status'
import { SLOW_REFRESH_MS } from '@/app/lib/use-dashboard'
import styles from './changelog.module.scss'

const { Content } = Layout
const { Title, Text } = Typography

// 数据源：biliup 官方 CHANGELOG（随版本自动更新，零后端改动）
const CHANGELOG_URL =
  'https://raw.githubusercontent.com/biliup/biliup/master/docs/content/docs/guide/CHANGELOG.md'
const MAX_VERSIONS = 25

const CHANGELOG_CACHE_KEY = 'biliup_changelog_cache'

/**
 * 拉取官方 CHANGELOG：成功则写入 localStorage 作为离线缓存；
 * 失败（离线 / GitHub 被墙 / 接口异常）时回退到上一次缓存，
 * 避免页面直接白屏成「无法加载」。仅在既失败又无缓存时才真正报错。
 */
async function fetchChangelog(url: string): Promise<{ text: string; stale: boolean }> {
  try {
    const r = await fetch(url)
    if (!r.ok) throw new Error(`http ${r.status}`)
    const text = await r.text()
    try {
      localStorage.setItem(CHANGELOG_CACHE_KEY, text)
    } catch {
      /* 隐私模式等场景忽略写入失败 */
    }
    return { text, stale: false }
  } catch {
    let cached: string | null = null
    try {
      cached = localStorage.getItem(CHANGELOG_CACHE_KEY)
    } catch {
      cached = null
    }
    if (cached) return { text: cached, stale: true }
    throw new Error('changelog fetch failed and no cache')
  }
}

export interface VersionEntry {
  version: string
  body: string
  compareUrl?: string
}

/**
 * 把 CHANGELOG markdown 解析为版本数组。
 * - 跳过文档站 frontmatter（+++ ... +++）
 * - 仅保留 `## 版本号` 段，过滤 `## 标签含义` 等噪声段
 * - 提取 `**Full Changelog**: [text](url)` 外链，并从正文移除
 * - 取最近 MAX_VERSIONS 个版本
 */
function parseChangelog(md: string): VersionEntry[] {
  const noFm = md.replace(/^\+\+\+[\s\S]*?\+\+\+\s*/, '')
  const lines = noFm.split(/\r?\n/)
  const out: VersionEntry[] = []
  let cur: { version: string; lines: string[]; compareUrl?: string } | null = null

  const fullChangelogRe = /^\s*\*\*Full Changelog\*\*\s*:\s*\[([^\]]+)\]\(([^)]+)\)\s*$/

  for (const line of lines) {
    const m = line.match(/^##\s+([\d.]+(-[\d.]+)?)\s*$/)
    if (m) {
      if (cur) out.push({ version: cur.version, body: cur.lines.join('\n').trim(), compareUrl: cur.compareUrl })
      cur = { version: m[1], lines: [] }
    } else if (/^##\s+/.test(line)) {
      // 非版本标题（如「标签含义」），结束当前收集
      cur = null
    } else if (cur) {
      const fm = line.match(fullChangelogRe)
      if (fm && /^https?:\/\//i.test(fm[2])) {
        cur.compareUrl = fm[2]
        continue
      }
      cur.lines.push(line)
    }
  }
  if (cur) out.push({ version: cur.version, body: cur.lines.join('\n').trim(), compareUrl: cur.compareUrl })
  return out.slice(0, MAX_VERSIONS)
}

// 行内渲染：将 [text](url) 渲染为外链（仅放行 http/https，防注入），**bold** 加粗
function renderInline(text: string, keyPrefix: string): React.ReactNode[] {
  const out: React.ReactNode[] = []
  const linkRe = /\[([^\]]+)\]\(([^)\s]+)\)/g
  let last = 0
  let m: RegExpExecArray | null
  let i = 0
  const pushText = (t: string, k: string) => {
    const parts = t.split(/\*\*([^*]+)\*\*/)
    parts.forEach((p, idx) => {
      if (p === '') return
      out.push(
        idx % 2 === 1 ? (
          <b key={`${k}-b${idx}`}>{p}</b>
        ) : (
          <span key={`${k}-t${idx}`}>{p}</span>
        )
      )
    })
  }
  while ((m = linkRe.exec(text)) !== null) {
    if (m.index > last) pushText(text.slice(last, m.index), `${keyPrefix}-${i}`)
    const url = m[2]
    const safe = /^https?:\/\//i.test(url)
    out.push(
      safe ? (
        <a
          key={`${keyPrefix}-l${i}`}
          href={url}
          target="_blank"
          rel="noreferrer"
          className={styles.link}
        >
          {m[1]}
        </a>
      ) : (
        <span key={`${keyPrefix}-l${i}`}>{m[1]}</span>
      )
    )
    last = m.index + m[0].length
    i++
  }
  if (last < text.length) pushText(text.slice(last), `${keyPrefix}-${i}`)
  return out
}

// 把一个版本的正文渲染成：小标题 / 列表 / 普通段落
function renderBody(body: string, versionKey: string): React.ReactNode {
  const lines = body.split(/\r?\n/)
  const blocks: React.ReactNode[] = []
  let listBuf: string[] = []
  const flush = (k: string) => {
    if (listBuf.length === 0) return
    const items = listBuf
    listBuf = []
    blocks.push(
      <ul key={`${k}-ul`} className={styles.ul}>
        {items.map((it, idx) => (
          <li key={idx} className={styles.li}>
            {renderInline(it.replace(/^[-*]\s+/, ''), `${k}-${idx}`)}
          </li>
        ))}
      </ul>
    )
  }
  lines.forEach((line, idx) => {
    const h = line.match(/^###\s+(.+)$/)
    if (h) {
      flush(`${versionKey}-${idx}`)
      const isContributors = /New Contributors/i.test(h[1])
      blocks.push(
        <div key={`${versionKey}-h${idx}`} className={isContributors ? styles.subheadMuted : styles.subhead}>
          {h[1]}
        </div>
      )
    } else if (/^[-*]\s+/.test(line)) {
      listBuf.push(line)
    } else if (line.trim() === '') {
      flush(`${versionKey}-${idx}`)
    } else {
      flush(`${versionKey}-${idx}`)
      blocks.push(
        <div key={`${versionKey}-p${idx}`} className={styles.para}>
          {renderInline(line, `${versionKey}-${idx}`)}
        </div>
      )
    }
  })
  flush(`${versionKey}-end`)
  return <>{blocks}</>
}

/** 比较 "1.2.5" 与 "1.2.1" 这类点分版本号，返回 a - b 的符号 */
function compareVersion(a: string, b: string): number {
  const pa = a.split(/[.-]/).map(Number)
  const pb = b.split(/[.-]/).map(Number)
  for (let i = 0; i < Math.max(pa.length, pb.length); i++) {
    const d = (pa[i] ?? 0) - (pb[i] ?? 0)
    if (d !== 0) return d
  }
  return 0
}

export default function Changelog() {
  const { data, error, isLoading } = useSWR(CHANGELOG_URL, fetchChangelog)
  const result = data as { text: string; stale: boolean } | undefined
  const versions = result ? parseChangelog(result.text) : []

  // 与侧栏共用 /v1/status 的缓存，只为拿到正在运行的版本号
  const { data: status } = useSWR('/v1/status', fetcher, {
    refreshInterval: SLOW_REFRESH_MS,
    revalidateOnFocus: false,
  })
  const running = formatVersion((status as { version?: string } | undefined)?.version)
  const top = versions[0]?.version
  // CHANGELOG.md 未必随每次发版更新；运行版本比日志里最新一条还新时如实标出，
  // 不把旧条目标成「最新」
  const runningIsNewer = !!running && !!top && compareVersion(running, top) > 0

  return (
    <>
      {/* PageHeader 自带 sticky 与 z-index，不要再包一层带 position / z-index 的容器，
          否则会形成更低层级的 stacking context，滚动时被下方内容盖住 */}
      <PageHeader
        icon={<IconBook size="large" />}
        title="更新日志"
        description="biliup 版本更新记录"
      />
      <Content>
        <main className={styles.content}>
          {isLoading && (
            <div className={styles.center}>
              <Spin size="large" />
            </div>
          )}
          {error && (
            <Empty
              title="无法加载更新日志"
              description="请检查网络连接（更新日志内容来自 GitHub）"
            />
          )}
          {!isLoading && !error && versions.length > 0 && (
            <div className={styles.feed}>
              {result?.stale && (
                <div
                  style={{
                    fontSize: 12,
                    color: 'var(--semi-color-text-2)',
                    textAlign: 'center',
                    padding: '4px 0 12px',
                  }}
                >
                  当前展示的是离线缓存版本，联网后将自动更新。
                </div>
              )}
              {runningIsNewer && (
                <div
                  style={{
                    fontSize: 12,
                    color: 'var(--semi-color-text-2)',
                    textAlign: 'center',
                    padding: '4px 0 12px',
                  }}
                >
                  当前运行的 v{running} 尚未收录到更新日志，以下为已发布的历史版本。
                </div>
              )}
              {versions.map((v, idx) => (
                <Card
                  key={v.version}
                  className={styles.card}
                  shadows="hover"
                  bodyStyle={{ padding: '20px 24px' }}
                  title={
                    <div className={styles.cardHeader}>
                      <div className={styles.versionBlock}>
                        <Title heading={5} className={styles.versionTitle}>
                          {v.version}
                        </Title>
                        {idx === 0 && !runningIsNewer && (
                          <Tag color="green" type="light" size="small">
                            最新
                          </Tag>
                        )}
                        {running && v.version === running && (
                          <Tag color="blue" type="light" size="small">
                            当前运行
                          </Tag>
                        )}
                      </div>
                      {v.compareUrl && (
                        <Tooltip content="完整变更" position="top">
                          <Button
                            type="tertiary"
                            theme="borderless"
                            icon={<IconExport />}
                            onClick={() => window.open(v.compareUrl, '_blank', 'noopener,noreferrer')}
                          />
                        </Tooltip>
                      )}
                    </div>
                  }
                >
                  {renderBody(v.body, v.version)}
                </Card>
              ))}
            </div>
          )}
        </main>
      </Content>
    </>
  )
}
