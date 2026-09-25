'use client'
import { useState } from 'react'
import useSWR, { mutate as revalidate } from 'swr'
import { Button, Empty, Popconfirm, Spin, Tag, Toast, Typography } from '@douyinfe/semi-ui'
import { IconPlusCircle, IconServer } from '@douyinfe/semi-icons'
import PageHeader from '../components/PageHeader'
import dc from '@/app/ui/data-card.module.scss'
import { fetcher } from '@/app/lib/api-streamer'
import { useMe } from '@/app/lib/use-me'
import { useIsMobile } from '@/app/lib/useIsMobile'
import { humDate } from '@/app/lib/utils'
import {
  FLEET_NODES_KEY,
  FLEET_REFRESH_MS,
  FLEET_TOKENS_KEY,
  revokeNode,
  voidToken,
  type FleetNode,
  type FleetNodes,
  type JoinTokens,
} from '@/app/lib/use-fleet'
import NodeCard from './NodeCard'
import JoinDialog from './JoinDialog'
import styles from './page.module.scss'

const { Text } = Typography

function NotController() {
  return (
    <div className={styles.center}>
      <Empty
        image={<IconServer size="extra-large" style={{ color: 'var(--semi-color-text-3)' }} />}
        title="本机不是控制面"
        description="用 biliup server --controller 启动后，才能在这里添加和查看节点"
      />
    </div>
  )
}

function PendingTokens({ canManage }: { canManage: boolean }) {
  const { data, mutate } = useSWR<JoinTokens>(canManage ? FLEET_TOKENS_KEY : null, fetcher, {
    refreshInterval: 30_000,
  })
  const active = data?.tokens.filter((t) => t.state === 'active') ?? []
  if (!canManage || active.length === 0) return null
  const remove = async (id: string) => {
    try {
      await voidToken(id)
      Toast.success('票据已作废')
    } catch (e) {
      Toast.error((e as Error).message)
    }
    mutate().catch(() => undefined)
  }
  return (
    <section className={styles.tokens} aria-label="尚未使用的加入票据">
      <div className={styles.tokensHead}>尚未使用的加入票据</div>
      {active.map((t) => (
        <div key={t.id} className={styles.tokenRow}>
          <Tag size="small" color="blue" className={styles.tokenId}>
            {t.id}
          </Tag>
          <Text type="tertiary" size="small" className={styles.tokenTime}>
            {humDate(Math.floor(t.created_at / 1000))} 生成 · {humDate(Math.floor(t.expires_at / 1000))} 过期
          </Text>
          <Popconfirm title="作废这张票据？" content="作废后用它加入会被拒绝" onConfirm={() => remove(t.id)}>
            <Button size="small" theme="borderless" type="danger">
              作废
            </Button>
          </Popconfirm>
        </div>
      ))}
    </section>
  )
}

export default function NodesPage() {
  const { me, can } = useMe()
  const isMobile = useIsMobile()
  const controller = me?.fleet_controller === true
  const canManage = can('node.manage')
  const { data, error, isLoading, mutate } = useSWR<FleetNodes>(controller ? FLEET_NODES_KEY : null, fetcher, {
    refreshInterval: FLEET_REFRESH_MS,
  })
  const [joining, setJoining] = useState(false)

  const refresh = () => {
    mutate().catch(() => undefined)
  }

  const revoke = async (node: FleetNode) => {
    try {
      await revokeNode(node.id)
      Toast.success(`已移除 ${node.name}`)
    } catch (e) {
      Toast.error((e as Error).message)
    }
    refresh()
  }

  const nodes = data?.nodes ?? []
  const online = nodes.filter((n) => n.online)
  const recording = online.reduce((sum, n) => sum + (n.summary?.recording ?? 0), 0)

  let body
  if (me && !controller) {
    body = <NotController />
  } else if (!data && (isLoading || !me)) {
    body = (
      <div className={styles.center}>
        <Spin size="large" />
      </div>
    )
  } else if (!data) {
    body = (
      <div className={styles.center}>
        <Empty title="加载失败" description={(error as Error | undefined)?.message || '无法获取节点列表'} />
        <Button onClick={refresh} style={{ marginTop: 12 }}>
          重试
        </Button>
      </div>
    )
  } else if (nodes.length === 0) {
    body = (
      <div className={styles.center}>
        <Empty
          image={<IconServer size="extra-large" style={{ color: 'var(--semi-color-text-3)' }} />}
          title="还没有节点"
          description={
            canManage
              ? '点击右上角「添加节点」生成一张加入票据，在另一台运行 biliup 的机器上执行给出的命令'
              : '还没有机器加入这个控制面'
          }
        />
        {canManage ? (
          <Button theme="solid" icon={<IconPlusCircle />} onClick={() => setJoining(true)} style={{ marginTop: 12 }}>
            添加节点
          </Button>
        ) : null}
      </div>
    )
  } else {
    body = (
      <div className={styles.grid}>
        {[...nodes]
          .sort((a, b) => Number(b.online) - Number(a.online) || a.id - b.id)
          .map((node) => (
            <NodeCard key={node.id} node={node} canManage={canManage} onRevoke={revoke} />
          ))}
      </div>
    )
  }

  return (
    <>
      <PageHeader
        icon={<IconServer size="large" />}
        title="节点"
        description="加入本控制面的 biliup 实例。各节点仍各自录制、各自上传，这里汇总它们的状态"
        actions={
          controller && canManage ? (
            <Button icon={<IconPlusCircle />} theme="solid" onClick={() => setJoining(true)}>
              {isMobile ? '添加' : '添加节点'}
            </Button>
          ) : null
        }
      />
      <div className={dc.content}>
        {data && error ? (
          <div className={styles.notice} role="status">
            连接中断，下面是最后一次拿到的数据（{(error as Error).message}）
          </div>
        ) : null}
        {controller && nodes.length > 0 ? (
          <div className={dc.kpiStrip}>
            <span className={dc.kpiItem}>
              节点 <b>{nodes.length}</b>
            </span>
            <span className={dc.kpiItem}>
              在线 <b>{online.length}</b>
            </span>
            <span className={dc.kpiItem}>
              录制中 <b>{recording}</b>
            </span>
          </div>
        ) : null}
        {body}
        {controller ? <PendingTokens canManage={canManage} /> : null}
      </div>
      {joining ? (
        <JoinDialog
          onClose={() => {
            setJoining(false)
            refresh()
            revalidate(FLEET_TOKENS_KEY).catch(() => undefined)
          }}
        />
      ) : null}
    </>
  )
}
