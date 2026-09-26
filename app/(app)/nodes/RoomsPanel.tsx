'use client'
import React, { useMemo, useState } from 'react'
import useSWR from 'swr'
import { Button, Empty, Form, Popconfirm, Select, Spin, Tag, Toast, Tooltip, Typography } from '@douyinfe/semi-ui'
import { IconDeleteStroked, IconEdit2Stroked, IconPause, IconPlay, IconSend, IconVideo } from '@douyinfe/semi-icons'
import TemplateModal from '@/app/ui/TemplateModal'
import { fetcher } from '@/app/lib/api-streamer'
import { formListToHookStep, hookStepListToForm } from '@/app/lib/postprocessor'
import { platformName } from '@/app/lib/status'
import { useIsMobile } from '@/app/lib/useIsMobile'
import {
  DESIRED_STATE_SINCE,
  FLEET_REFRESH_MS,
  FLEET_ROOMS_KEY,
  createRoom,
  deleteRoom,
  errorMessage,
  forceRelease,
  pauseRoom,
  updateRoom,
  type FleetNode,
  type FleetRoom,
  type FleetRooms,
  type FleetTemplate,
  type RoomInput,
  type RoomStatus,
} from '@/app/lib/use-fleet'
import AssignModal from './AssignModal'
import styles from './page.module.scss'

const { Text } = Typography

const STATUS: Record<RoomStatus, { label: string; color: React.ComponentProps<typeof Tag>['color']; hint: string }> = {
  unassigned: { label: '未分派', color: 'grey', hint: '没有节点在录，分派到节点后开始监控' },
  releasing: { label: '迁移中', color: 'orange', hint: '等上一台节点停录并确认释放，之后交给新节点' },
  deleting: { label: '删除中', color: 'orange', hint: '已删除，等节点停录并确认释放' },
  offline: { label: '节点离线', color: 'grey', hint: '节点离线；它上线时按分派接着录，离线期间它仍按本地缓存在录' },
  outdated: {
    label: '节点协议旧',
    color: 'red',
    hint: `节点的 Fleet 协议版本低于 ${DESIRED_STATE_SINCE}，收不了房间，请先升级 biliup`,
  },
  syncing: { label: '下发中', color: 'blue', hint: '已下发，节点还没确认' },
  failed: { label: '落地失败', color: 'red', hint: '节点没能按分派录这个房间' },
  monitoring: { label: '监控中', color: 'green', hint: '节点在监控，未开播' },
  recording: { label: '录制中', color: 'red', hint: '节点正在录' },
  paused: { label: '已暂停', color: 'amber', hint: '已暂停监控' },
}

const ROOM_KEYS = [
  'url',
  'remark',
  'filename_prefix',
  'time_range',
  'format',
  'override',
  'preprocessor',
  'segment_processor',
  'downloaded_processor',
  'opt_args',
  'excluded_keywords',
] as const

/** 录制表单（与本机「直播管理」共用 `TemplateModal`）的值 → 房间 */
function roomFromForm(values: Record<string, any>): RoomInput {
  const room: Record<string, unknown> = {}
  for (const key of ROOM_KEYS) room[key] = values[key] ?? null
  room.postprocessor = values.postprocessor ? formListToHookStep(values.postprocessor) : null
  room.template_id = values.upload_streamers_id ?? null
  return room as unknown as RoomInput
}

function roomToForm(room: FleetRoom) {
  return {
    ...room,
    upload_streamers_id: room.template_id ?? undefined,
    postprocessor: room.postprocessor ? hookStepListToForm(room.postprocessor) : room.postprocessor,
  } as any
}

const FILTER_ALL = 'all'
const FILTER_NONE = 'none'
const FILTER_PENDING = 'pending'

export default function RoomsPanel({
  nodes,
  templates,
  canManage,
  creating,
  onChanged,
}: {
  nodes: FleetNode[]
  templates: FleetTemplate[]
  canManage: boolean
  /** 空列表里的「新建房间」按钮（同页头那个） */
  creating: React.ReactNode
  /** 分派变了：节点卡片上的房间数跟着刷新 */
  onChanged: () => void
}) {
  /** 表格要约 640px 宽；窗口窄到侧栏展开后放不下时改用卡片 */
  const compact = useIsMobile(960)
  const { data, error, isLoading, mutate } = useSWR<FleetRooms>(FLEET_ROOMS_KEY, fetcher, {
    refreshInterval: FLEET_REFRESH_MS,
  })
  const [query, setQuery] = useState('')
  const [filter, setFilter] = useState<string>(FILTER_ALL)
  const [assigning, setAssigning] = useState<FleetRoom | null>(null)

  const nodeById = useMemo(() => new Map(nodes.map((n) => [n.id, n])), [nodes])
  const templateById = useMemo(() => new Map(templates.map((t) => [t.id, t])), [templates])
  const templateOptions = useMemo(
    () => templates.map((t) => ({ value: t.id, label: t.template_name })),
    [templates]
  )
  const refresh = () => {
    mutate().catch(() => undefined)
    onChanged()
  }

  const rooms = useMemo(() => data?.rooms ?? [], [data])
  const counts = useMemo(() => {
    const byNode = new Map<number, number>()
    let none = 0
    let pending = 0
    for (const room of rooms) {
      if (room.node_id === null) none++
      else byNode.set(room.node_id, (byNode.get(room.node_id) ?? 0) + 1)
      if (room.releasing_node_id !== null) pending++
    }
    return { byNode, none, pending }
  }, [rooms])

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase()
    return rooms.filter((room) => {
      const okFilter =
        filter === FILTER_ALL ||
        (filter === FILTER_NONE && room.node_id === null) ||
        (filter === FILTER_PENDING && room.releasing_node_id !== null) ||
        String(room.node_id) === filter ||
        String(room.releasing_node_id) === filter
      const okQuery = !q || room.remark.toLowerCase().includes(q) || room.url.toLowerCase().includes(q)
      return okFilter && okQuery
    })
  }, [rooms, query, filter])

  const nodeName = (id: number | null) => (id === null ? null : (nodeById.get(id)?.name ?? `节点 ${id}`))

  const run = async (action: () => Promise<unknown>, ok: string) => {
    try {
      await action()
      Toast.success(ok)
    } catch (e) {
      Toast.error({ content: errorMessage(e), duration: 6 })
    }
    refresh()
  }

  const handleUpdate = (room: FleetRoom) => async (values: Record<string, any>) => {
    try {
      await updateRoom(room.id, roomFromForm(values))
      Toast.success('已保存，已下发到节点')
      refresh()
    } catch (e) {
      Toast.error({ content: errorMessage(e), duration: 6 })
      throw e
    }
  }

  const remove = async (room: FleetRoom, force: boolean) => {
    try {
      const pending = await deleteRoom(room.id, force)
      if (pending) Toast.info(`已删除，等 ${nodeName(pending.releasing_node_id) ?? '节点'} 停录后清掉`)
      else Toast.success('已删除')
    } catch (e) {
      Toast.error({ content: errorMessage(e), duration: 6 })
    }
    refresh()
  }

  const statusTag = (room: FleetRoom) => {
    const s = STATUS[room.status]
    const hint = room.status === 'failed' && room.error ? `${s.hint}：${room.error}` : s.hint
    return (
      <Tooltip content={hint}>
        <Tag size="small" color={s.color}>
          {s.label}
        </Tag>
      </Tooltip>
    )
  }

  const nodeCell = (room: FleetRoom) => {
    const target = nodeName(room.node_id)
    if (room.releasing_node_id !== null) {
      const from = nodeName(room.releasing_node_id)
      return (
        <div className={styles.roomNode}>
          <span>
            {from}
            {room.deleted_at === null ? <> → {target ?? '不分派'}</> : null}
          </span>
          <Text type={room.releasing_online === false ? 'warning' : 'tertiary'} size="small">
            {room.releasing_online === false ? `${from} 离线，会一直等它上线释放` : `等 ${from} 停录`}
          </Text>
        </div>
      )
    }
    if (target === null) return <Text type="tertiary">—</Text>
    const node = room.node_id !== null ? nodeById.get(room.node_id) : undefined
    return (
      <span className={styles.roomNodeName}>
        <span className={`${styles.dot} ${node?.online ? styles.dotOn : ''}`} aria-hidden="true" />
        {target}
      </span>
    )
  }

  const templateName = (room: FleetRoom) =>
    room.template_id === null ? '不投稿' : (templateById.get(room.template_id)?.template_name ?? `模板 ${room.template_id}`)

  const actions = (room: FleetRoom) => {
    if (!canManage) return null
    if (room.deleted_at !== null) {
      return (
        <Popconfirm
          title="不等节点释放，直接删掉？"
          content={`${nodeName(room.releasing_node_id)} 如果其实还在录，它会一直录到重新连上控制面`}
          okType="danger"
          onConfirm={() => remove(room, true)}
        >
          <Button size="small" type="danger" theme="borderless">
            强制删除
          </Button>
        </Popconfirm>
      )
    }
    return (
      <>
        {room.releasing_node_id !== null ? (
          <Popconfirm
            title="强制迁移？"
            content={`不等 ${nodeName(room.releasing_node_id)} 确认释放就交给 ${nodeName(room.node_id) ?? '—'}。它如果其实还在录，两边会同时录这个房间（重复录制与投稿），直到它重新连上控制面才停`}
            okType="danger"
            okText="强制迁移"
            onConfirm={() => run(() => forceRelease(room.id), '已强制迁移')}
          >
            <Button size="small" type="danger" theme="borderless">
              强制
            </Button>
          </Popconfirm>
        ) : null}
        <TemplateModal
          title={`编辑房间「${room.remark}」`}
          entity={roomToForm(room)}
          templateOptions={templateOptions}
          onOk={handleUpdate(room)}
        >
          <Button theme="borderless" type="primary" icon={<IconEdit2Stroked />} aria-label="编辑" title="编辑" />
        </TemplateModal>
        <Button
          theme="borderless"
          type="primary"
          icon={<IconSend />}
          aria-label={room.node_id !== null ? '迁移' : '分派'}
          title={room.node_id !== null ? '迁移到其他节点' : '分派到节点'}
          onClick={() => setAssigning(room)}
        />
        <Button
          theme="borderless"
          type="tertiary"
          icon={room.paused ? <IconPlay /> : <IconPause />}
          aria-label={room.paused ? '恢复' : '暂停'}
          title={room.paused ? '恢复监控' : '暂停监控'}
          onClick={() => run(() => pauseRoom(room.id, !room.paused), room.paused ? '已恢复' : '已暂停')}
        />
        <Popconfirm
          title={`删除房间「${room.remark}」？`}
          content={
            room.node_id !== null
              ? '节点会停录并删掉本机的这一行（已录的文件保留）'
              : '此操作不可逆'
          }
          okType="danger"
          onConfirm={() => remove(room, false)}
        >
          <Button theme="borderless" type="danger" icon={<IconDeleteStroked />} aria-label="删除" title="删除" />
        </Popconfirm>
      </>
    )
  }

  let body: React.ReactNode
  if (!data && isLoading) {
    body = (
      <div className={styles.center}>
        <Spin size="large" />
      </div>
    )
  } else if (!data) {
    body = (
      <div className={styles.center}>
        <Empty title="加载失败" description={errorMessage(error) || '无法获取房间列表'} />
        <Button onClick={refresh} style={{ marginTop: 12 }}>
          重试
        </Button>
      </div>
    )
  } else if (rooms.length === 0) {
    body = (
      <div className={styles.center}>
        <Empty
          image={<IconVideo size="extra-large" style={{ color: 'var(--semi-color-text-3)' }} />}
          title="还没有房间"
          description={
            canManage
              ? '在这里添加直播间并分派给节点，节点会自动开始监控录制；控制面本机「直播管理」里的直播间不受影响'
              : '管理员添加房间后会显示在这里'
          }
        />
        {canManage ? <div style={{ marginTop: 12 }}>{creating}</div> : null}
      </div>
    )
  } else if (filtered.length === 0) {
    body = (
      <div className={styles.center}>
        <Empty title="没有匹配的房间" description="调整搜索或筛选条件试试" />
      </div>
    )
  } else if (compact) {
    body = (
      <div className={styles.roomCards}>
        {filtered.map((room) => (
          <article key={room.id} className={styles.roomCard} aria-label={`房间 ${room.remark}`}>
            <div className={styles.roomCardHead}>
              {statusTag(room)}
              <Text strong ellipsis={{ showTooltip: true }} className={styles.roomName}>
                {room.remark}
              </Text>
            </div>
            <Text type="tertiary" size="small" className={styles.roomUrl}>
              {room.url}
            </Text>
            <div className={styles.roomCardMeta}>
              <span>节点：{nodeCell(room)}</span>
              <span>模板：{templateName(room)}</span>
            </div>
            {canManage ? <div className={styles.roomCardActions}>{actions(room)}</div> : null}
          </article>
        ))}
      </div>
    )
  } else {
    body = (
      <div className={styles.tableWrap}>
        <table className={styles.roomTable}>
          <thead>
            <tr>
              <th>状态</th>
              <th>房间</th>
              <th>节点</th>
              <th>投稿模板</th>
              {canManage ? <th style={{ textAlign: 'right' }}>操作</th> : null}
            </tr>
          </thead>
          <tbody>
            {filtered.map((room) => (
              <tr key={room.id} aria-label={`房间 ${room.remark}`}>
                <td>{statusTag(room)}</td>
                <td className={styles.roomMain}>
                  <div className={styles.roomName}>{room.remark}</div>
                  <div className={styles.roomSub}>
                    <Text type="tertiary" size="small">
                      {platformName(room.url)}
                    </Text>{' '}
                    <Text type="tertiary" size="small" className={styles.roomUrl}>
                      {room.url}
                    </Text>
                  </div>
                </td>
                <td>{nodeCell(room)}</td>
                <td>
                  <Text size="small">{templateName(room)}</Text>
                </td>
                {canManage ? (
                  <td>
                    <div className={styles.rowActions}>{actions(room)}</div>
                  </td>
                ) : null}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    )
  }

  const filterOptions = [
    { value: FILTER_ALL, label: `全部节点（${rooms.length}）` },
    { value: FILTER_NONE, label: `未分派（${counts.none}）` },
    ...(counts.pending > 0 ? [{ value: FILTER_PENDING, label: `迁移 / 删除中（${counts.pending}）` }] : []),
    ...nodes.map((n) => ({ value: String(n.id), label: `${n.name}（${counts.byNode.get(n.id) ?? 0}）` })),
  ]

  return (
    <>
      {rooms.length > 0 ? (
        <div className={styles.toolbar}>
          <label className={styles.search}>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" width="15" height="15">
              <circle cx="11" cy="11" r="7" />
              <path d="M21 21l-4.3-4.3" />
            </svg>
            <input
              type="text"
              placeholder="搜索备注 / 直播间地址"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              aria-label="搜索房间"
            />
          </label>
          <Select
            value={filter}
            onChange={(v) => setFilter(String(v))}
            optionList={filterOptions}
            className={styles.nodeFilter}
            aria-label="按节点筛选"
          />
        </div>
      ) : null}
      {data && error ? (
        <div className={styles.notice} role="status">
          连接中断，下面是最后一次拿到的数据（{errorMessage(error)}）
        </div>
      ) : null}
      {body}
      {assigning ? (
        <AssignModal
          room={rooms.find((r) => r.id === assigning.id) ?? assigning}
          nodes={nodes}
          template={assigning.template_id !== null ? templateById.get(assigning.template_id) : undefined}
          onClose={() => setAssigning(null)}
          onDone={() => {
            setAssigning(null)
            refresh()
          }}
        />
      ) : null}
    </>
  )
}

/** 「分派到节点」里的「自动」，由控制面按负载选 */
const AUTO_NODE = -1

/** 页头的「新建房间」：录制设置与本机共用表单，外加「分派到节点」 */
export function CreateRoomButton({
  nodes,
  templates,
  onCreated,
  children,
}: {
  nodes: FleetNode[]
  templates: FleetTemplate[]
  onCreated: () => void
  children: React.ReactElement
}) {
  const templateOptions = templates.map((t) => ({ value: t.id, label: t.template_name }))
  const nodeOptions = [
    ...(nodes.length > 0 ? [{ value: AUTO_NODE, label: '自动（按负载选）' }] : []),
    ...[...nodes]
      .sort((a, b) => Number(b.online) - Number(a.online) || a.id - b.id)
      .map((n) => ({
        value: n.id,
        label: `${n.name}${n.online ? '' : '（离线）'}`,
      })),
  ]
  const create = async (values: Record<string, any>) => {
    const auto = values.node_id === AUTO_NODE
    try {
      const room = await createRoom({
        ...roomFromForm(values),
        node_id: auto ? null : (values.node_id ?? null),
        auto_node: auto,
      })
      const picked = nodes.find((n) => n.id === room?.node_id)?.name
      Toast.success(
        room?.node_id == null
          ? '已添加，还没分派到节点'
          : auto
            ? `已添加，自动分派到 ${picked ?? '节点'}`
            : '已添加，已下发到节点',
      )
      onCreated()
    } catch (e) {
      Toast.error({ content: errorMessage(e), duration: 6 })
      throw e
    }
  }
  return (
    <TemplateModal
      title="新建房间"
      templateOptions={templateOptions}
      onOk={create}
      extraFields={
        <Form.Select
          field="node_id"
          label={{ text: '分派到节点', optional: true }}
          style={{ width: 240 }}
          optionList={nodeOptions}
          showClear
          placeholder="暂不分派"
          extraText="模板要用的 B 站账号必须在节点上登记过；处理器里带 run 命令的房间只能派给允许钩子的节点（rm、mv 等文件操作不算）。「自动」在满足这些条件的在线节点里挑空闲下载位多、房间少、磁盘余量大的一台，之后不会因负载变化挪走"
        />
      }
    >
      {children}
    </TemplateModal>
  )
}
