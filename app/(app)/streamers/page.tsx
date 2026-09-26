'use client'
import {
  Layout,
  Button,
  ButtonGroup,
  Popconfirm,
  Spin,
  Empty,
  Notification,
  Typography,
  Tag,
  Tooltip,
  Banner,
} from '@douyinfe/semi-ui'
import {
  IconPlusCircle,
  IconEdit2Stroked,
  IconDeleteStroked,
  IconWrench,
  IconVideoListStroked,
} from '@douyinfe/semi-icons'
import React, { useMemo, useState } from 'react'
import useSWR from 'swr'
import useSWRMutation from 'swr/mutation'
import TemplateModal from '../../ui/TemplateModal'
import OverrideModal from '../../ui/OverrideModal'
import {
  LiveStreamerEntity,
  put,
  requestDelete,
  sendRequest,
  fetcher,
  proxy,
  StreamerInfo,
} from '../../lib/api-streamer'
import { PauseButton } from '@/app/ui/StreamerActions/PauseButton'
import { platformName, streamerStatusTag } from '@/app/lib/status'
import { hookStepListToForm, formListToHookStep } from '@/app/lib/postprocessor'
import {
  timeAgo,
  formatRate,
  liveImageUrl,
  STREAMERS_REFRESH_MS,
  SLOW_REFRESH_MS,
} from '@/app/lib/use-dashboard'
import StreamerCard, { LiveAvatar } from '@/app/ui/StreamerCard'
import { CardRateSwitch } from '@/app/ui/LiveRateChart'
import PageHeader from '../components/PageHeader'
import { useMe } from '@/app/lib/use-me'
import styles from './page.module.scss'

const { Content } = Layout
const { Text } = Typography

/**
 * 直播管理:卡片 / 列表双视图 + 搜索 + 平台筛选 + 批量操作。
 * 卡片与首页共用 StreamerCard;接口失败与空列表分开处理。
 */
export default function StreamersPage() {
  const { data: streamers, error, isLoading, mutate } = useSWR<LiveStreamerEntity[]>(
    '/v1/streamers',
    fetcher,
    { refreshInterval: STREAMERS_REFRESH_MS }
  )
  const { data: infos } = useSWR<StreamerInfo[]>('/v1/streamer-info', fetcher, {
    refreshInterval: SLOW_REFRESH_MS,
  })

  // url -> 最新一次录制的直播标题
  const infoByUrl = useMemo(() => {
    const map = new Map<string, StreamerInfo>()
    for (const i of infos ?? []) {
      if (!i.url) continue
      const cur = map.get(i.url)
      if (!cur || i.date > cur.date) map.set(i.url, i)
    }
    return map
  }, [infos])

  const { me, can } = useMe()
  const canEdit = can('streamer.edit')
  const canControl = can('recording.control')
  const canHooks = can('streamer.hooks')
  // 批量操作至少要能暂停或删除其一，否则连勾选框都不显示
  const canBatch = canEdit || canControl
  // 本机加入了控制面时，控制面分派下来的直播间只读（后端对它们的增删改返回 409）
  const fleet = me?.fleet_node
  const managedIds = useMemo(() => new Set(fleet?.streamers ?? []), [fleet])
  const managedHint = fleet ? `由控制面 ${fleet.controller} 管理，请到控制面修改` : undefined

  // ---- 增删改 ----
  const { trigger: deleteStreamers } = useSWRMutation('/v1/streamers', requestDelete)
  const { trigger: updateStreamers } = useSWRMutation('/v1/streamers', put)
  const { trigger } = useSWRMutation('/v1/streamers', sendRequest)

  const onConfirm = async (id: number) => {
    await deleteStreamers(id)
  }

  const handleEntityPostprocessor = (values: any) => {
    // 回显:后端 HookStep → 表单 {cmd, value}
    if (values?.postprocessor) {
      values.postprocessor = hookStepListToForm(values.postprocessor)
    }
    return values
  }

  const handleOk = async (values: any) => {
    // 提交:表单 {cmd, value} → 后端 HookStep
    if (values?.postprocessor) {
      values.postprocessor = formListToHookStep(values.postprocessor)
    }
    try {
      await trigger(values)
    } catch (e: any) {
      Notification.error({
        title: '创建失败',
        content: e?.message ?? String(e),
      })
      // 重新抛出,Modal onOk 保持打开,不丢失已填输入
      throw e
    }
  }

  const handleUpdate = async (values: any) => {
    delete values.status
    delete values.statusTag
    delete values.upload_status
    if (values?.postprocessor) {
      values.postprocessor = formListToHookStep(values.postprocessor)
    }
    try {
      await updateStreamers(values)
    } catch (e: any) {
      Notification.error({
        title: '更新失败',
        content: e?.message ?? String(e),
      })
      // 重新抛出,Modal onOk 保持打开,不丢失已填输入
      throw e
    }
  }

  // ---- 搜索 / 筛选 / 视图 ----
  const [query, setQuery] = useState('')
  const [filter, setFilter] = useState('全部')
  const [layout, setLayout] = useState<'grid' | 'list'>('grid')
  const [selected, setSelected] = useState<Set<number>>(new Set())

  const platforms = useMemo(() => {
    const set = new Set<string>()
    for (const s of streamers ?? []) set.add(platformName(s.url))
    return ['全部', ...Array.from(set).sort((a, b) => a.localeCompare(b, 'zh'))]
  }, [streamers])

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase()
    return (streamers ?? []).filter((s) => {
      const okPlat = filter === '全部' || platformName(s.url) === filter
      const okQ =
        !q ||
        (s.remark ?? '').toLowerCase().includes(q) ||
        s.url.toLowerCase().includes(q)
      return okPlat && okQ
    })
  }, [streamers, query, filter])

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
    if (ids.length === 0) return
    // 后端 PUT /pause 是 toggle:已暂停(Pause)的会被恢复为 Idle,必须过滤
    const targets = (streamers ?? []).filter((s) => selected.has(s.id) && s.status !== 'Pause')
    const skipped = ids.length - targets.length
    if (targets.length === 0) {
      Notification.warning({ title: '所选直播间均已暂停,无需操作' })
      return
    }
    const results = await Promise.allSettled(
      targets.map((s) => proxy(`/v1/streamers/${s.id}/pause`, { method: 'PUT' }))
    )
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
    if (ids.length === 0) return
    const results = await Promise.all(
      ids.map(async (id) => {
        try {
          await proxy(`/v1/streamers/${id}`, { method: 'DELETE' })
          return { id, ok: true }
        } catch {
          return { id, ok: false }
        }
      })
    )
    const failedIds = results.filter((result) => !result.ok).map((result) => result.id)
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

  // 编辑 / 暂停 / 删除 / 高级四个操作，网格卡片与列表行共用，只是外层容器不同
  // 返回数组而不是 Fragment:ButtonGroup 会对每个直接子元素 cloneElement 注入 disabled 等 props,
  // React 19 起会对带这些 props 的 Fragment 报 "Invalid prop supplied to React.Fragment"
  // 按角色只渲染有权限的按钮（只读观察者一个都没有）
  const actionButtons = (item: LiveStreamerEntity) => {
    const locked = managedIds.has(item.id)
    const lock = locked ? { disabled: true, title: managedHint } : {}
    return [
      locked && (
        <Tooltip key="managed" content={managedHint}>
          <Tag size="small" color="violet">
            托管
          </Tag>
        </Tooltip>
      ),
      canEdit && (
        <TemplateModal key="edit" onOk={handleUpdate} entity={handleEntityPostprocessor({ ...item })}>
          <Button theme="borderless" type="primary" icon={<IconEdit2Stroked />} aria-label="编辑" {...lock} />
        </TemplateModal>
      ),
      canControl && <PauseButton key="pause" streamer={item} {...lock} />,
      canEdit && (
        <Popconfirm
          key="delete"
          title="确定是否要删除？"
          content="此操作将不可逆"
          onConfirm={() => onConfirm(item.id)}
          disabled={locked}
        >
          <Button theme="borderless" type="danger" icon={<IconDeleteStroked />} aria-label="删除" {...lock} />
        </Popconfirm>
      ),
      canHooks && (
        <OverrideModal key="override" onOk={handleUpdate} entity={handleEntityPostprocessor({ ...item })}>
          <Button theme="borderless" type="tertiary" icon={<IconWrench />} aria-label="高级" {...lock} />
        </OverrideModal>
      ),
    ].filter(Boolean)
  }
  const hasActions = canEdit || canControl || canHooks
  const renderActions = (item: LiveStreamerEntity) =>
    !hasActions ? null : managedIds.has(item.id) ? (
      // ButtonGroup 会给每个子元素注入按钮属性，托管行里多了一个标签，改用普通容器
      <div className={styles.rowActions}>{actionButtons(item)}</div>
    ) : (
      <ButtonGroup theme="borderless" className={styles.cardActions}>
        {actionButtons(item)}
      </ButtonGroup>
    )
  const renderRowActions = (item: LiveStreamerEntity) => (
    <div className={styles.rowActions}>{actionButtons(item)}</div>
  )

  return (
    <>
      <PageHeader
        title="直播管理"
        description={
          canEdit ? '管理需要录制的直播间,支持新增、编辑与删除' : '查看正在监控的直播间与录制状态'
        }
        icon={<IconVideoListStroked size="large" />}
        actions={
          canEdit && (
            <TemplateModal onOk={handleOk}>
              <Button icon={<IconPlusCircle />} theme="solid">
                新建
              </Button>
            </TemplateModal>
          )
        }
      />
      <Content className={styles.content}>
        {fleet ? (
          <Banner
            type="info"
            fullMode={false}
            closeIcon={null}
            className={styles.fleetBanner}
            description={
              managedIds.size > 0
                ? `本机已加入控制面：标着「托管」的 ${managedIds.size} 个直播间由控制面 ${fleet.controller} 管理，这里只能查看，编辑、暂停、删除请到控制面操作。本机自己添加的直播间不受影响。`
                : `本机已加入控制面 ${fleet.controller}；控制面分派来的直播间会由控制面 ${fleet.controller} 管理，这里只能查看。本机自己添加的直播间不受影响。`
            }
          />
        ) : null}
        {isLoading ? (
          <div className={styles.center}>
            <Spin size="large" />
          </div>
        ) : error ? (
          /* 修复:接口失败 ≠ 空列表,给出错误态而不是 Empty */
          <div className={styles.center}>
            <Empty
              title="加载失败"
              description="无法获取主播列表,请检查后端连接"
              style={{ marginBottom: 12 }}
            />
            <Button onClick={() => mutate()}>重试</Button>
          </div>
        ) : (
          <>
            {/* 工具栏 */}
            <div className={styles.toolbar}>
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
                  placeholder="搜索主播 / URL"
                  value={query}
                  onChange={(e) => setQuery(e.target.value)}
                />
              </label>
              <div className={styles.filterChips}>
                {platforms.map((p) => (
                  <button
                    key={p}
                    className={`${styles.fchip} ${filter === p ? styles.fchipActive : ''}`}
                    onClick={() => setFilter(p)}
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
                  <Button size="small" onClick={batchPause}>
                    批量暂停
                  </Button>
                )}
                {canEdit && (
                <Popconfirm
                  title={`确定删除选中的 ${selected.size} 个直播间？`}
                  content="此操作不可逆,删除结果将逐项反馈"
                  onConfirm={batchDelete}
                >
                  <Button size="small" type="danger" theme="borderless">
                    批量删除
                  </Button>
                </Popconfirm>
                )}
                <Button size="small" theme="borderless" onClick={() => setSelected(new Set())}>
                  取消
                </Button>
              </div>
            )}

            {filtered.length === 0 ? (
              <div className={styles.center}>
                <Empty
                  title={streamers && streamers.length > 0 ? '没有匹配的直播间' : '还没有监控任何直播间'}
                  description={
                    streamers && streamers.length > 0
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
                            aria-label="全选"
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
                      const rowSelectable = canBatch && !managedIds.has(item.id)
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
                            <td onClick={(e) => e.stopPropagation()}>{renderRowActions(item)}</td>
                          )}
                        </tr>
                      )
                    })}
                  </tbody>
                </table>
              </div>
            )}
          </>
        )}
      </Content>
    </>
  )
}
