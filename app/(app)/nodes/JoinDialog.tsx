'use client'
import { useCallback, useEffect, useRef, useState } from 'react'
import { Banner, Button, Empty, Input, Modal, Spin, Toast, Typography } from '@douyinfe/semi-ui'
import { IconCopy } from '@douyinfe/semi-icons'
import { copyText, errorMessage, issueTicket, voidToken, type IssuedTicket } from '@/app/lib/use-fleet'
import styles from './page.module.scss'

const { Text } = Typography

function pad2(n: number): string {
  return n < 10 ? `0${n}` : String(n)
}

/** 剩余毫秒 → "23:59:08" / "6 天 23:59:08" */
function countdown(ms: number): string {
  const total = Math.max(0, Math.floor(ms / 1000))
  const days = Math.floor(total / 86400)
  const h = Math.floor((total % 86400) / 3600)
  const m = Math.floor((total % 3600) / 60)
  const s = total % 60
  const clock = `${pad2(h)}:${pad2(m)}:${pad2(s)}`
  return days > 0 ? `${days} 天 ${clock}` : clock
}

function CopyBlock({ label, hint, text }: { label: string; hint?: string; text: string }) {
  const copy = async () => {
    if (await copyText(text)) Toast.success({ id: 'fleet-copy', content: '已复制' })
    else Toast.error({ id: 'fleet-copy', content: '复制失败，请手动选中复制' })
  }
  return (
    <div className={styles.copyBlock}>
      <div className={styles.copyHead}>
        <span className={styles.copyLabel}>{label}</span>
        <Button size="small" theme="borderless" icon={<IconCopy />} onClick={copy} aria-label={`复制${label}`}>
          复制
        </Button>
      </div>
      <pre className={styles.code}>{text}</pre>
      {hint ? (
        <Text type="tertiary" size="small">
          {hint}
        </Text>
      ) : null}
    </div>
  )
}

/** 浏览器地址栏里的主机名能不能当 relay 候选地址：回环与 localhost 对节点没有意义 */
function candidateHost(hostname: string): string | null {
  const host = hostname.toLowerCase()
  if (!host) return null
  if (host === 'localhost' || host.endsWith('.localhost')) return null
  if (host === '[::1]' || host === '::1' || host === '0.0.0.0' || host === '[::]') return null
  if (/^127\./.test(host)) return null
  return host
}

function sameRelay(a: string, b: string): boolean {
  try {
    return new URL(a).href === new URL(b).href
  } catch {
    return a === b
  }
}

/**
 * 「写进票据」的候选 relay 地址：用浏览器访问控制面的主机名 + 内嵌 relay 的端口。
 * 能用这个主机名打开控制面的机器，多半也能用它连到 relay（Docker、NAT 后面时控制面自己列不出这个地址）。
 */
function CandidateRelay({
  ticket,
  busy,
  onApply,
}: {
  ticket: IssuedTicket
  busy: boolean
  onApply: (relay: string) => void
}) {
  const host = typeof window === 'undefined' ? null : candidateHost(window.location.hostname)
  const suggested = host && ticket.relay_port ? `http://${host}:${ticket.relay_port}` : null
  const [value, setValue] = useState(suggested ?? '')
  if (!suggested) return null
  const included = ticket.relays.some((relay) => sameRelay(relay, value.trim() || suggested))
  return (
    <section className={styles.candidate} aria-label="候选 relay 地址">
      <div className={styles.copyLabel}>候选地址</div>
      <Text type="tertiary" size="small">
        你是通过 <code>{host}</code> 打开控制面的。节点如果也能用这个地址访问到控制面的 relay 端口（
        {ticket.relay_port}），把它写进票据，节点加入时会先试它；控制面用 <code>--relay-url</code> 指定的地址仍然排在最前。
      </Text>
      <div className={styles.candidateRow}>
        <Input
          value={value}
          onChange={setValue}
          aria-label="候选 relay 地址"
          placeholder={suggested}
          disabled={busy}
        />
        <Button
          onClick={() => onApply(value.trim())}
          disabled={busy || !value.trim() || included}
          loading={busy}
        >
          {included ? '已在票据里' : '写进票据'}
        </Button>
      </div>
    </section>
  )
}

/**
 * 添加节点：打开即向控制面要一张一次性加入票据，给出命令行和 Docker 两种用法。
 * 票据只在这里出现一次，关掉就拿不回来了（没用掉的可以在票据列表里作废）。
 */
export default function JoinDialog({ onClose }: { onClose: () => void }) {
  const [ticket, setTicket] = useState<IssuedTicket | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)
  const [extra, setExtra] = useState<string[]>([])
  const [now, setNow] = useState(() => Date.now())
  const requested = useRef(false)

  const generate = useCallback(async (relays: string[], replaces?: string) => {
    setLoading(true)
    setError(null)
    try {
      const issued = await issueTicket(relays)
      setTicket(issued)
      setExtra(relays)
      setNow(Date.now())
      // 换了一张带候选地址的票，上一张没用过，顺手作废，免得票据列表里多一张
      if (replaces) voidToken(replaces).catch(() => undefined)
    } catch (e) {
      const message = errorMessage(e)
      if (replaces) Toast.error({ content: message, duration: 6 })
      else setError(message)
    } finally {
      setLoading(false)
    }
  }, [])

  // StrictMode 下 effect 会跑两遍，一次打开只发一张票
  useEffect(() => {
    if (requested.current) return
    requested.current = true
    generate([])
  }, [generate])

  useEffect(() => {
    if (!ticket) return
    const timer = setInterval(() => setNow(Date.now()), 1000)
    return () => clearInterval(timer)
  }, [ticket])

  const remaining = ticket ? ticket.expires_at - now : 0
  const expired = ticket !== null && remaining <= 0

  let body
  if (loading && !ticket) {
    body = (
      <div className={styles.dialogCenter}>
        <Spin />
      </div>
    )
  } else if (error && !ticket) {
    body = (
      <div className={styles.dialogCenter}>
        <Empty title="生成票据失败" description={error} />
      </div>
    )
  } else if (ticket) {
    body = (
      <div className={styles.dialogBody}>
        {ticket.private_only ? (
          <Banner
            type="warning"
            fullMode={false}
            closeIcon={null}
            title="只有局域网内的节点能加入"
            description={
              <>
                票据里的 relay 地址都是内网地址（{ticket.relays.join('、')}）。要让外网的机器加入，
                请给控制面准备一个公网能访问的 TCP 端口（转发到 relay 端口），把这个地址填进下面的「候选地址」写进票据，
                或用 <code>--relay-url</code> 启动控制面。
              </>
            }
          />
        ) : null}
        <CandidateRelay
          ticket={ticket}
          busy={loading}
          onApply={(relay) => generate([relay], ticket.id)}
        />
        <div className={`${styles.expiry} ${expired ? styles.expired : ''}`} role="timer" aria-live="off">
          {expired ? (
            '票据已过期，请重新生成'
          ) : (
            <>
              有效期还剩 <b>{countdown(remaining)}</b>，只能使用一次
            </>
          )}
        </div>
        <CopyBlock label="在节点上执行" text={ticket.command} hint="节点加入后重启 biliup server 即开始向控制面上报。" />
        <CopyBlock
          label="Docker：已在运行的容器"
          text={ticket.docker_command}
          hint="把 <容器名> 换成节点的容器名，然后重启该容器。"
        />
        <CopyBlock
          label="Docker：新建容器时"
          text={ticket.docker_env}
          hint="作为环境变量传入（-e 或 compose 的 environment），首次启动时自动加入；data 目录需要挂载持久卷。"
        />
        <Text type="tertiary" size="small" className={styles.relays}>
          票据里的 relay：{ticket.relays.join('、')}
        </Text>
      </div>
    )
  }

  return (
    <Modal
      title="添加节点"
      visible
      onCancel={onClose}
      closeOnEsc
      style={{ width: 'min(640px, 94vw)' }}
      // 内容比矮屏高时只滚正文，「完成」始终露在外面
      bodyStyle={{ maxHeight: 'calc(100dvh - 200px)', overflowY: 'auto' }}
      footer={
        <div className={styles.dialogFoot}>
          <Button onClick={() => generate(extra)} loading={loading} disabled={loading}>
            {ticket || error ? '重新生成' : '生成'}
          </Button>
          <Button theme="solid" onClick={onClose}>
            完成
          </Button>
        </div>
      }
    >
      {body}
    </Modal>
  )
}
