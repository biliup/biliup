'use client'
import { Suspense, useState } from 'react'
import { usePathname, useRouter, useSearchParams } from 'next/navigation'
import useSWR, { mutate as revalidate } from 'swr'
import { Button, Empty, Popconfirm, Spin, TabPane, Tabs, Tag, Toast, Typography } from '@douyinfe/semi-ui'
import { IconPlusCircle, IconServer, IconSetting } from '@douyinfe/semi-icons'
import PageHeader from '../components/PageHeader'
import dc from '@/app/ui/data-card.module.scss'
import { fetcher } from '@/app/lib/api-streamer'
import { useMe } from '@/app/lib/use-me'
import { useIsMobile } from '@/app/lib/useIsMobile'
import { humDate } from '@/app/lib/utils'
import {
  FLEET_NODES_KEY,
  FLEET_REFRESH_MS,
  FLEET_ROOMS_KEY,
  FLEET_TEMPLATES_KEY,
  FLEET_TOKENS_KEY,
  revokeAndReassign,
  revokeNode,
  voidToken,
  type FleetNode,
  type FleetNodes,
  type FleetRooms,
  type FleetTemplate,
  type JoinTokens,
} from '@/app/lib/use-fleet'
import NodeCard from './NodeCard'
import JoinDialog from './JoinDialog'
import RoomsPanel, { CreateRoomButton } from './RoomsPanel'
import TemplatesPanel from './TemplatesPanel'
import FleetTemplateModal from './FleetTemplateModal'
import FleetConfigSheet, { type ConfigTarget } from './FleetConfigSheet'
import styles from './page.module.scss'

const { Text } = Typography

type Tab = 'nodes' | 'rooms' | 'templates'

const DESCRIPTIONS: Record<Tab, string> = {
  nodes: '加入本控制面的 biliup 实例。各节点仍各自录制、各自上传，这里汇总它们的状态',
  rooms: '把直播间分派给节点录制；迁移时上一台先停录、确认释放后才交给新节点',
  templates: '房间录完按模板投稿；账号按 mid 在节点本机的凭据里找，凭据不出节点',
}

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

function Nodes() {
  const { me, can } = useMe()
  const isMobile = useIsMobile()
  const router = useRouter()
  const pathname = usePathname()
  const searchParams = useSearchParams()
  const requested = searchParams.get('tab')
  const tab: Tab = requested === 'rooms' || requested === 'templates' ? requested : 'nodes'
  const controller = me?.fleet_controller === true
  const canManage = can('node.manage')
  const { data, error, isLoading, mutate } = useSWR<FleetNodes>(controller ? FLEET_NODES_KEY : null, fetcher, {
    refreshInterval: FLEET_REFRESH_MS,
  })
  const {
    data: templates,
    error: templatesError,
    isLoading: templatesLoading,
    mutate: mutateTemplates,
  } = useSWR<FleetTemplate[]>(controller ? FLEET_TEMPLATES_KEY : null, fetcher)
  const { data: roomList } = useSWR<FleetRooms>(controller ? FLEET_ROOMS_KEY : null, fetcher, {
    refreshInterval: FLEET_REFRESH_MS,
  })
  const [joining, setJoining] = useState(false)
  /** undefined 为没打开，null 为新建 */
  const [editingTemplate, setEditingTemplate] = useState<FleetTemplate | null | undefined>(undefined)
  const [configTarget, setConfigTarget] = useState<ConfigTarget | null>(null)
  const canViewConfig = can('config.view')

  const refresh = () => {
    mutate().catch(() => undefined)
  }
  const refreshRooms = () => {
    revalidate(FLEET_ROOMS_KEY).catch(() => undefined)
    refresh()
  }
  const refreshTemplates = () => {
    mutateTemplates().catch(() => undefined)
    refreshRooms()
  }
  const switchTab = (key: string) => {
    router.replace(key === 'nodes' ? pathname : `${pathname}?tab=${key}`, { scroll: false })
  }

  const revoke = async (node: FleetNode, reassign: boolean) => {
    try {
      if (!reassign) {
        await revokeNode(node.id)
        Toast.success(`已移除 ${node.name}`)
      } else {
        const { reassigned, unplaced } = await revokeAndReassign(node.id)
        Toast.success(`已移除 ${node.name}，${reassigned.length} 个房间已改派`)
        if (unplaced.length > 0) {
          Toast.warning({
            content: `${unplaced.length} 个房间没找到合适的节点，留在未分派：${unplaced[0].reason}`,
            duration: 8,
          })
        }
      }
    } catch (e) {
      Toast.error((e as Error).message)
    }
    refreshRooms()
  }

  const nodes = data?.nodes ?? []
  const rooms = roomList?.rooms ?? []
  const online = nodes.filter((n) => n.online)
  const recording = online.reduce((sum, n) => sum + (n.summary?.recording ?? 0), 0)
  const unassigned = rooms.filter((r) => r.node_id === null && r.deleted_at === null).length

  const createRoom = (
    <CreateRoomButton nodes={nodes} templates={templates ?? []} onCreated={refreshRooms}>
      <Button icon={<IconPlusCircle />} theme="solid">
        {isMobile ? '添加' : '新建房间'}
      </Button>
    </CreateRoomButton>
  )
  const createTemplate = (
    <Button icon={<IconPlusCircle />} theme="solid" onClick={() => setEditingTemplate(null)}>
      {isMobile ? '添加' : '新建模板'}
    </Button>
  )
  const addNode = (
    <Button icon={<IconPlusCircle />} theme="solid" onClick={() => setJoining(true)}>
      {isMobile ? '添加' : '添加节点'}
    </Button>
  )
  const nodeActions = (
    <>
      {canViewConfig ? (
        <Button icon={<IconSetting />} onClick={() => setConfigTarget({ mode: 'global' })}>
          {isMobile ? '配置' : 'Fleet 配置'}
        </Button>
      ) : null}
      {canManage ? addNode : null}
    </>
  )

  let nodesBody
  if (!data && (isLoading || !me)) {
    nodesBody = (
      <div className={styles.center}>
        <Spin size="large" />
      </div>
    )
  } else if (!data) {
    nodesBody = (
      <div className={styles.center}>
        <Empty title="加载失败" description={(error as Error | undefined)?.message || '无法获取节点列表'} />
        <Button onClick={refresh} style={{ marginTop: 12 }}>
          重试
        </Button>
      </div>
    )
  } else if (nodes.length === 0) {
    nodesBody = (
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
    nodesBody = (
      <div className={styles.grid}>
        {[...nodes]
          .sort((a, b) => Number(b.online) - Number(a.online) || a.id - b.id)
          .map((node) => (
            <NodeCard
              key={node.id}
              node={node}
              canManage={canManage}
              onRevoke={revoke}
              controllerVersion={data.controller_version}
              onEditConfig={canViewConfig ? (n) => setConfigTarget({ mode: 'override', nodeId: n.id }) : undefined}
            />
          ))}
      </div>
    )
  }

  const actions =
    !controller
      ? null
      : tab === 'nodes'
        ? canViewConfig || canManage
          ? nodeActions
          : null
        : !canManage
          ? null
          : tab === 'rooms'
            ? createRoom
            : createTemplate

  return (
    <>
      <PageHeader
        icon={<IconServer size="large" />}
        title="节点"
        description={DESCRIPTIONS[controller ? tab : 'nodes']}
        actions={actions}
      />
      <div className={dc.content}>
        {me && !controller ? (
          <NotController />
        ) : (
          <>
            {data && error ? (
              <div className={styles.notice} role="status">
                连接中断，下面是最后一次拿到的数据（{(error as Error).message}）
              </div>
            ) : null}
            {nodes.length > 0 || rooms.length > 0 ? (
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
                <span className={dc.kpiItem}>
                  房间 <b>{rooms.length}</b>
                </span>
                {unassigned > 0 ? (
                  <span className={dc.kpiItem}>
                    未分派 <b>{unassigned}</b>
                  </span>
                ) : null}
              </div>
            ) : null}
            <Tabs type="line" activeKey={tab} onChange={switchTab} className={styles.tabs} lazyRender>
              <TabPane tab="节点" itemKey="nodes">
                {nodesBody}
                {controller ? <PendingTokens canManage={canManage} /> : null}
              </TabPane>
              <TabPane tab={`房间${rooms.length ? ` ${rooms.length}` : ''}`} itemKey="rooms">
                {controller ? (
                  <RoomsPanel
                    nodes={nodes}
                    templates={templates ?? []}
                    canManage={canManage}
                    creating={createRoom}
                    onChanged={refresh}
                  />
                ) : null}
              </TabPane>
              <TabPane tab={`投稿模板${templates?.length ? ` ${templates.length}` : ''}`} itemKey="templates">
                {controller ? (
                  <TemplatesPanel
                    templates={templates}
                    rooms={rooms}
                    nodes={nodes}
                    loading={templatesLoading}
                    error={templatesError}
                    canManage={canManage}
                    creating={createTemplate}
                    onEdit={setEditingTemplate}
                    onChanged={refreshTemplates}
                  />
                ) : null}
              </TabPane>
            </Tabs>
          </>
        )}
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
      {configTarget ? (
        <FleetConfigSheet
          target={configTarget}
          node={configTarget.mode === 'override' ? nodes.find((n) => n.id === configTarget.nodeId) : undefined}
          canManage={canManage}
          onClose={() => setConfigTarget(null)}
        />
      ) : null}
      {editingTemplate !== undefined ? (
        <FleetTemplateModal
          template={editingTemplate}
          nodes={nodes}
          onClose={() => setEditingTemplate(undefined)}
          onSaved={() => {
            setEditingTemplate(undefined)
            refreshTemplates()
          }}
        />
      ) : null}
    </>
  )
}

export default function NodesPage() {
  return (
    <Suspense
      fallback={
        <div className={styles.center}>
          <Spin size="large" />
        </div>
      }
    >
      <Nodes />
    </Suspense>
  )
}
