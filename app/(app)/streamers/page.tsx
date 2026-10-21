'use client'
import {
  Layout,
  Button,
  SplitButtonGroup,
  Dropdown,
  Popconfirm,
  Spin,
  Empty,
  Notification,
  Typography,
  Tag,
  Tooltip,
  Banner,
  Pagination,
} from '@douyinfe/semi-ui'
import {
  IconPlusCircle,
  IconEdit2Stroked,
  IconDeleteStroked,
  IconWrench,
  IconVideoListStroked,
  IconMore,
  IconPause,
  IconPlay,
} from '@douyinfe/semi-icons'
import React, { useEffect, useMemo, useRef, useState } from 'react'
import useSWR from 'swr'
import useSWRMutation from 'swr/mutation'
import { useRouter } from 'next/navigation'
import dynamic from 'next/dynamic'
import { boundedTasks } from '@/app/lib/bounded-tasks'
import { FormDialog } from '@/app/ui/shell'
import {
  LiveStreamerEntity,
  put,
  requestDelete,
  fetcher,
  proxy,
  StreamerInfo,
} from '../../lib/api-streamer'
import { platformName, streamerStatusTag } from '@/app/lib/status'
import { hookStepListToForm, formListToHookStep } from '@/app/lib/postprocessor'
import {
  timeAgo,
  formatRate,
  liveImageUrl,
  STREAMERS_REFRESH_MS,
} from '@/app/lib/use-dashboard'
import StreamerCard, { LiveAvatar } from '@/app/ui/StreamerCard'
import { CardRateSwitch } from '@/app/ui/LiveRateChart'
import PageHeader from '../components/PageHeader'
import { pairLabel, pairPeer, useMe } from '@/app/lib/use-me'
import FleetRevokedBanner, { revokedHint } from '@/app/ui/FleetRevokedBanner'
import styles from './page.module.scss'

const { Content } = Layout
const { Text } = Typography
const OverrideModal = dynamic(() => import('../../ui/OverrideModal'), { ssr: false })

/**
 * 直播管理:卡片 / 列表双视图 + 搜索 + 平台筛选 + 批量操作。
 * 卡片与首页共用 StreamerCard;接口失败与空列表分开处理。
 */
export default function StreamersPage() {
  const [query, setQuery] = useState('')
  const [search, setSearch] = useState('')
  const [filter, setFilter] = useState('全部')
  const [page, setPage] = useState(1)
  const [pageSize, setPageSize] = useState(24)
  const [layout, setLayout] = useState<'grid' | 'list'>('grid')
  const [selectedIds, setSelected] = useState<Set<number>>(new Set())
  const [busy, setBusy] = useState(false)
  const [actionBusy, setActionBusy] = useState<number | null>(null)
  const [openActionId, setOpenActionId] = useState<number | null>(null)
  const [deleteTarget, setDeleteTarget] = useState<LiveStreamerEntity | null>(null)
  const [overrideTarget, setOverrideTarget] = useState<{
    item: LiveStreamerEntity; panel: 'plugin' | 'mosaic'
  } | null>(null)
  const toolbarRef = useRef<HTMLDivElement>(null)
  useEffect(() => {
    const timer = setTimeout(() => { setSearch(query.trim()); setPage(1); setSelected(new Set()) }, 300)
    return () => clearTimeout(timer)
  }, [query])
  const params = new URLSearchParams({ page: String(page), page_size: String(pageSize), search })
  if (filter !== '全部') params.set('platform', filter)
  const { data, error, isLoading, mutate } = useSWR<{
    items: LiveStreamerEntity[]; infos: StreamerInfo[]; total: number
    page: number; page_size: number; platforms: string[]
  }>(`/v1/streamers/page?${params}`, fetcher, { refreshInterval: STREAMERS_REFRESH_MS })
  const streamers = data?.items
  const filtered = streamers ?? []
  const platforms = ['全部', ...(data?.platforms ?? (filter === '全部' ? [] : [filter]))]
  const infoByUrl = useMemo(() => new Map((data?.infos ?? []).map(info => [info.url, info])), [data?.infos])
  // Deleting the last row may move the last page backwards. Reconcile before rendering it.
  if (data && data.page !== page) setPage(data.page)
  const selected = useMemo(() => new Set([...selectedIds].filter(id => streamers?.some(item => item.id === id))), [selectedIds, streamers])

  const { me, can } = useMe()
  const canEdit = can('streamer.edit')
  const canControl = can('recording.control')
  const canHooks = can('streamer.hooks')
  // 批量操作至少要能暂停或删除其一，否则连勾选框都不显示
  const canBatch = canEdit || canControl
  // 本机加入了控制面时，控制面分派下来的直播间只读（后端对它们的增删改返回 409）
  const fleet = me?.fleet_node
  const managedIds = useMemo(() => new Set(fleet?.streamers ?? []), [fleet])
  const managedHint = fleet
    ? fleet.local
      ? '这是分派到本机的 Fleet 房间，请到「任务平台 › 节点管理 › 房间」修改'
      : `由控制面 ${fleet.controller} 管理，请到控制面修改`
    : undefined
  // 一主一备里的节点：配对里的直播间照常增删改、暂停，改动与对端双向同步
  const pair = fleet?.pair
  const pairedIds = useMemo(() => new Set(pair?.streamers ?? []), [pair])
  const pairHint = pair
    ? `在两台之间同步：这里的修改会同步到${pairPeer(pair)} ${fleet?.controller}，那边的修改也会同步过来。run 命令与边录边传（sync-downloader）不随配对同步，请到控制面的「任务平台 › 节点管理 › 房间」设置，这里改了会被退回`
    : undefined
  // 被控制面移除后转为本机自管并暂停、等确认恢复的直播间；手动恢复后标记立即消失
  const revokedIds = useMemo(() => new Set(me?.fleet_revoked?.streamers ?? []), [me?.fleet_revoked])
  const awaitingResume = (item: LiveStreamerEntity) => revokedIds.has(item.id) && item.status === 'Pause'

  // ---- 增删改 ----
  const router = useRouter()
  const { trigger: deleteStreamers } = useSWRMutation('/v1/streamers', requestDelete)
  const { trigger: updateStreamers } = useSWRMutation('/v1/streamers', put)

  const deleteRoom = async () => {
    if (!deleteTarget || actionBusy !== null) return
    setActionBusy(deleteTarget.id)
    try {
      await deleteStreamers(deleteTarget.id)
      setDeleteTarget(null)
      await mutate()
    } catch (error: any) {
      Notification.error({ title: '删除失败', content: error?.message ?? String(error) })
    } finally {
      setActionBusy(null)
    }
  }

  const togglePause = async (item: LiveStreamerEntity) => {
    if (actionBusy !== null) return
    setActionBusy(item.id)
    try {
      await proxy(`/v1/streamers/${item.id}/pause`, { method: 'PUT' })
      await mutate()
    } catch (error: any) {
      Notification.error({ title: '操作失败', content: error?.message ?? String(error) })
    } finally {
      setActionBusy(null)
    }
  }

  const handleEntityPostprocessor = (values: any) => {
    // 回显:后端 HookStep → 表单 {cmd, value}
    if (values?.postprocessor) {
      values.postprocessor = hookStepListToForm(values.postprocessor)
    }
    return values
  }

  const handleUpdate = async (values: any) => {
    delete values.status
    delete values.statusTag
    delete values.upload_status
    try {
      if (values?.postprocessor) {
        values.postprocessor = formListToHookStep(values.postprocessor)
      }
      await updateStreamers(values)
      await mutate()
    } catch (e: any) {
      Notification.error({
        title: '更新失败',
        content: e?.message ?? String(e),
      })
      // 重新抛出,Modal onOk 保持打开,不丢失已填输入
      throw e
    }
  }

  // ---- 批量操作 ----
  const toggleSel = (id: number) => {
    setSelected((prev) => {
      const next = new Set(prev)
      if (next.has(id)) next.delete(id)
      else next.add(id)
      return next
    })
  }
  const selectable = filtered.filter((s) => !managedIds.has(s.id))
  const toggleAll = (checked: boolean) => {
    setSelected(checked ? new Set(selectable.map((s) => s.id)) : new Set())
  }

  const batchPause = async () => {
    const ids = Array.from(selected)
    if (ids.length === 0 || busy || isLoading) return
    // 后端 PUT /pause 是 toggle:已暂停(Pause)的会被恢复为 Idle,必须过滤
    const targets = (streamers ?? []).filter((s) => selected.has(s.id) && s.status !== 'Pause')
    const skipped = ids.length - targets.length
    if (targets.length === 0) {
      Notification.warning({ title: '所选直播间均已暂停,无需操作' })
      return
    }
    setBusy(true)
    const results = await boundedTasks(targets, s => proxy(`/v1/streamers/${s.id}/pause`, { method: 'PUT' }))
    setBusy(false)
    const failedIds: number[] = []
    results.forEach((r, i) => {
      if (r.status === 'rejected') failedIds.push(targets[i].id)
    })
    const ok = targets.length - failedIds.length
    setSelected(new Set(failedIds))
    await mutate().catch(() => undefined)
    if (failedIds.length === 0) {
      Notification.success({
        title: '已暂停 ' + ok + ' 个直播间' + (skipped > 0 ? ',' + skipped + ' 个已暂停已跳过' : ''),
      })
    } else {
      Notification.error({
        title: '成功暂停 ' + ok + ' 项,' + failedIds.length + ' 项失败',
        content: '失败项已保留选中,请检查后端状态后重试。',
      })
    }
  }

  const batchDelete = async () => {
    const ids = Array.from(selected)
    if (ids.length === 0 || busy || isLoading) return
    setBusy(true)
    const results = await boundedTasks(ids, id => proxy(`/v1/streamers/${id}`, { method: 'DELETE' }))
    setBusy(false)
    const failedIds = ids.filter((_, index) => results[index].status === 'rejected')
    const successCount = results.length - failedIds.length
    setSelected(new Set(failedIds))
    await mutate().catch(() => undefined)
    if (failedIds.length === 0) {
      Notification.success({ title: `已删除 ${successCount} 个直播间` })
    } else {
      Notification.error({
        title: `成功删除 ${successCount} 项,${failedIds.length} 项失败`,
        content: '失败项已保留选中,请检查后端状态后重试。',
      })
    }
  }

  // 常用编辑直接显示，其余操作通过点击菜单打开；同一时刻只挂载一个配置面板。
  const hasActions = canEdit || canControl || canHooks
  const renderActions = (item: LiveStreamerEntity) => {
    if (!hasActions) return null
    const locked = managedIds.has(item.id)
    const disabled = locked || busy || actionBusy !== null
    const paused = item.status === 'Pause'
    return (
      <div className={styles.rowActions}>
        {locked && <Tooltip content={managedHint}><Tag size="small" color="violet">托管</Tag></Tooltip>}
        {pair && pairedIds.has(item.id) && (
          <Tooltip content={pairHint}><Tag size="small" color="green">{pairLabel(pair)}</Tag></Tooltip>
        )}
        {awaitingResume(item) && (
          <Tooltip content={revokedHint(me?.fleet_revoked?.local)}><Tag size="small" color="orange">待恢复</Tag></Tooltip>
        )}
        <SplitButtonGroup>
          {canEdit ? (
            <Button theme="light" icon={<IconEdit2Stroked />} disabled={disabled} title={locked ? managedHint : '编辑'} aria-label="编辑"
              onClick={() => router.push(`/streamers/editor?id=${item.id}`)} />
          ) : canControl ? (
            <Button theme="light" icon={paused ? <IconPlay /> : <IconPause />} disabled={disabled}
              onClick={() => togglePause(item)}>{paused ? '恢复监控' : '暂停监控'}</Button>
          ) : (
            <Button theme="light" icon={<IconWrench />} disabled={disabled}
              onClick={() => setOverrideTarget({ item, panel: 'plugin' })}>配置覆写</Button>
          )}
          {(canEdit || canHooks) && <Dropdown trigger="click" position="bottomRight"
            visible={openActionId === item.id}
            onVisibleChange={visible => setOpenActionId(visible ? item.id : null)} render={
            <Dropdown.Menu>
              {canEdit && canControl && (
                <Dropdown.Item icon={paused ? <IconPlay /> : <IconPause />} disabled={disabled}
                  onClick={() => { setOpenActionId(null); void togglePause(item) }}>{paused ? '恢复监控' : '暂停监控'}</Dropdown.Item>
              )}
              {canHooks && (
                <Dropdown.Item icon={<IconVideoListStroked />} disabled={disabled}
                  onClick={() => { setOpenActionId(null); setOverrideTarget({ item, panel: 'mosaic' }) }}>画面遮挡</Dropdown.Item>
              )}
              {canHooks && (canEdit || canControl) && (
                <Dropdown.Item icon={<IconWrench />} disabled={disabled}
                  onClick={() => { setOpenActionId(null); setOverrideTarget({ item, panel: 'plugin' }) }}>配置覆写</Dropdown.Item>
              )}
              {canEdit && <Dropdown.Divider />}
              {canEdit && (
                <Dropdown.Item icon={<IconDeleteStroked />} type="danger" disabled={disabled}
                  onClick={() => { setOpenActionId(null); setDeleteTarget(item) }}>删除</Dropdown.Item>
              )}
            </Dropdown.Menu>
          }>
            <Button theme="light" icon={<IconMore />} disabled={disabled} title="更多"
              aria-label={`更多操作：${item.remark || item.url}`} />
          </Dropdown>}
        </SplitButtonGroup>
      </div>
    )
  }

  return (
    <>
      <PageHeader
        title="直播管理"
        icon={<IconVideoListStroked size="large" />}
        actions={
          canEdit && (
            <Button onClick={() => router.push('/streamers/editor')} icon={<IconPlusCircle />} theme="solid">
              新建
            </Button>
          )
        }
      />
      <Content className={styles.content}>
        {pair && fleet ? (
          <Banner
            type="info"
            fullMode={false}
            closeIcon={null}
            className={styles.fleetBanner}
            description={
              `本机与 ${fleet.controller} 组成一主一备，本机现在是${pair.primary ? '上传主机' : '备机'}：标着「${pairLabel(pair)}」的 ${pairedIds.size} 个直播间在两台之间双向同步，在哪台编辑、暂停、删除都可以，同一项两边都改过时以后改的为准；在这里新建的直播间也会加入配对。` +
              (managedIds.size > 0
                ? `标着「托管」的 ${managedIds.size} 个直播间由控制面 ${fleet.controller} 管理，这里只能查看。`
                : '') +
              '配对前本机已有的直播间不同步。'
            }
          />
        ) : fleet ? (
          <Banner
            type="info"
            fullMode={false}
            closeIcon={null}
            className={styles.fleetBanner}
            description={
              fleet.local
                ? managedIds.size > 0
                  ? `已启用「本机」节点：标着「托管」的 ${managedIds.size} 个直播间是分派到本机的 Fleet 房间，这里只能查看，编辑、暂停、删除请到「任务平台 › 节点管理 › 房间」。这里自己添加的直播间不受影响。`
                  : '已启用「本机」节点：分派到本机的 Fleet 房间会出现在这里并标为「托管」，请到「任务平台 › 节点管理 › 房间」修改。这里自己添加的直播间不受影响。'
                : managedIds.size > 0
                  ? `本机已加入控制面：标着「托管」的 ${managedIds.size} 个直播间由控制面 ${fleet.controller} 管理，这里只能查看，编辑、暂停、删除请到控制面操作。本机自己添加的直播间不受影响。`
                  : `本机已加入控制面 ${fleet.controller}；控制面分派来的直播间会由控制面 ${fleet.controller} 管理，这里只能查看。本机自己添加的直播间不受影响。`
            }
          />
        ) : null}
        <FleetRevokedBanner className={styles.fleetBanner} />
        <>
            {/* 工具栏 */}
            <div className={styles.toolbar} ref={toolbarRef}>
              <label className={styles.search}>
                <svg
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                  width="15"
                  height="15"
                >
                  <circle cx="11" cy="11" r="7" />
                  <path d="M21 21l-4.3-4.3" />
                </svg>
                <input
                  type="text"
                  placeholder="搜索全部主播 / URL"
                  aria-label="搜索直播间"
                  disabled={busy}
                  value={query}
                  onChange={(e) => setQuery(e.target.value)}
                />
              </label>
              <div className={styles.filterChips}>
                {platforms.map((p) => (
                  <button
                    key={p}
                    className={`${styles.fchip} ${filter === p ? styles.fchipActive : ''}`}
                    onClick={() => { setFilter(p); setPage(1); setSelected(new Set()) }}
                    disabled={busy}
                  >
                    {p}
                  </button>
                ))}
              </div>
              <span className={styles.toolbarSpacer} />
              {layout === 'grid' ? <CardRateSwitch className={styles.rateSwitch} /> : null}
              <div className={styles.seg}>
                <button
                  className={layout === 'grid' ? styles.segActive : ''}
                  onClick={() => setLayout('grid')}
                  aria-label="网格视图"
                >
                  ▦ 网格
                </button>
                <button
                  className={layout === 'list' ? styles.segActive : ''}
                  onClick={() => setLayout('list')}
                  aria-label="列表视图"
                >
                  ☰ 列表
                </button>
              </div>
            </div>

            {/* 批量操作条 */}
            {canBatch && selected.size > 0 && (
              <div className={styles.batchbar}>
                <span>
                  已选 <b>{selected.size}</b> 项
                </span>
                <span className={styles.batchSpacer} />
                {canControl && (
                  <Button size="small" loading={busy} disabled={isLoading} onClick={batchPause}>
                    批量暂停
                  </Button>
                )}
                {canEdit && (
                <Popconfirm
                  title={`确定删除选中的 ${selected.size} 个直播间？`}
                  content="此操作不可逆,删除结果将逐项反馈"
                  onConfirm={batchDelete}
                >
                  <Button size="small" disabled={busy || isLoading} type="danger" theme="borderless">
                    批量删除
                  </Button>
                </Popconfirm>
                )}
                <Button size="small" theme="borderless" onClick={() => setSelected(new Set())}>
                  取消
                </Button>
              </div>
            )}

            {isLoading ? <div className={styles.center}><Spin size="large" /></div> : error ? (
              <div className={styles.center}>
                <Empty title="加载失败" description="无法获取主播列表，请检查后端连接" />
                <Button onClick={() => mutate()}>重试</Button>
              </div>
            ) : filtered.length === 0 ? (
              <div className={styles.center}>
                <Empty
                  title={!!search || filter !== '全部' ? '没有匹配的直播间' : '还没有监控任何直播间'}
                  description={
                    !!search || filter !== '全部'
                      ? '调整搜索或筛选条件试试'
                      : canEdit
                        ? '点击右上角「新建」开始'
                        : '管理员添加直播间后会显示在这里'
                  }
                />
              </div>
            ) : layout === 'grid' ? (
              <div className={styles.grid}>
                {filtered.map((item) => (
                  <StreamerCard
                    key={item.id}
                    streamer={item}
                    info={infoByUrl.get(item.url)}
                    actions={renderActions(item)}
                    selectable={canBatch && !managedIds.has(item.id) && !busy}
                    selected={selected.has(item.id)}
                    onToggleSelect={toggleSel}
                  />
                ))}
              </div>
            ) : (
              <div className={styles.tableWrap}>
                <table className={styles.listTable}>
                  <thead>
                    <tr>
                      {canBatch && (
                        <th style={{ width: 36 }}>
                          <input
                            type="checkbox"
                            className={styles.chk}
                            checked={selectable.length > 0 && selectable.every((s) => selected.has(s.id))}
                            onChange={(e) => toggleAll(e.target.checked)}
                            aria-label="全选当前页"
                            disabled={busy}
                          />
                        </th>
                      )}
                      <th>状态</th>
                      <th>主播</th>
                      <th>平台</th>
                      <th>码率</th>
                      <th>最近录制</th>
                      {hasActions && <th style={{ textAlign: 'right' }}>操作</th>}
                    </tr>
                  </thead>
                  <tbody>
                    {filtered.map((item) => {
                      const info = infoByUrl.get(item.url)
                      const live = item.status === 'Working'
                      const avatarSrc = live
                        ? liveImageUrl(item.id, 'avatar', item.live_avatar_url)
                        : null
                      const rate = live ? formatRate(item.live_bytes_per_sec) : null
                      const rowSelectable = canBatch && !busy && !managedIds.has(item.id)
                      return (
                        <tr
                          key={item.id}
                          className={selected.has(item.id) ? styles.rowSel : ''}
                          onClick={rowSelectable ? () => toggleSel(item.id) : undefined}
                          style={rowSelectable ? undefined : { cursor: 'default' }}
                        >
                          {canBatch && (
                            <td onClick={(e) => e.stopPropagation()}>
                              <input
                                type="checkbox"
                                className={styles.chk}
                                checked={selected.has(item.id)}
                                disabled={!rowSelectable}
                                title={rowSelectable ? undefined : managedHint}
                                onChange={() => toggleSel(item.id)}
                                aria-label={`选择 ${item.remark}`}
                              />
                            </td>
                          )}
                          <td>{streamerStatusTag(item.status)}</td>
                          <td>
                            <div className={styles.cellNameRow}>
                              {avatarSrc ? <LiveAvatar key={avatarSrc} src={avatarSrc} /> : null}
                              <div className={styles.cellName}>{item.remark || item.url}</div>
                            </div>
                            {info?.title ? (
                              <div className={styles.cellSub}>{info.title}</div>
                            ) : null}
                          </td>
                          <td>
                            <Text type="tertiary" size="small">
                              {platformName(item.url)}
                            </Text>
                          </td>
                          <td>
                            <Text
                              type={rate ? 'secondary' : 'tertiary'}
                              size="small"
                              className={styles.cellRate}
                              title={
                                live
                                  ? rate
                                    ? '写盘速率(最近 10 秒平均)'
                                    : '写盘速率:尚无采样'
                                  : undefined
                              }
                            >
                              {live ? (rate ?? '—') : '—'}
                            </Text>
                          </td>
                          <td>
                            <Text type="tertiary" size="small">
                              {info?.date ? timeAgo(info.date) : '—'}
                            </Text>
                          </td>
                          {hasActions && (
                            <td onClick={(e) => e.stopPropagation()}>{renderActions(item)}</td>
                          )}
                        </tr>
                      )
                    })}
                  </tbody>
                </table>
              </div>
            )}
            <div className={styles.pagination}>
              <span aria-live="polite">{data ? `共 ${data.total} 个直播间` : '加载中…'}</span>
              <Pagination
                total={data?.total ?? 0}
                currentPage={data?.page ?? page}
                pageSize={pageSize}
                pageSizeOpts={[12, 24, 48]}
                showSizeChanger
                disabled={busy || isLoading}
                onPageChange={next => { setPage(next); setSelected(new Set()); toolbarRef.current?.scrollIntoView({ block: 'start' }) }}
                onPageSizeChange={size => { setPageSize(size); setPage(1); setSelected(new Set()) }}
              />
            </div>
          </>
      </Content>
      {overrideTarget && (
        <OverrideModal
          key={`${overrideTarget.item.id}-${overrideTarget.panel}`}
          visible
          initialPanel={overrideTarget.panel}
          entity={handleEntityPostprocessor({ ...overrideTarget.item })}
          onOk={handleUpdate}
          onCancel={() => setOverrideTarget(null)}
        />
      )}
      <FormDialog
        title="删除直播间"
        visible={!!deleteTarget}
        onOk={deleteRoom}
        onCancel={() => { if (actionBusy === null) setDeleteTarget(null) }}
        confirmLoading={actionBusy !== null}
        closable={actionBusy === null}
        size="sm"
        okText="删除"
        okType="danger"
      >
        确定删除「{deleteTarget?.remark || deleteTarget?.url}」？此操作不可逆。
      </FormDialog>
    </>
  )
}
