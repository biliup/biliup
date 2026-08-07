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
import { timeAgo } from '@/app/lib/use-dashboard'
import StreamerCard from '@/app/ui/StreamerCard'
import PageHeader from '../components/PageHeader'
import styles from './page.module.scss'

const { Content } = Layout
const { Text } = Typography

/**
 * 直播管理:卡片 / 列表双视图 + 搜索 + 平台筛选 + 批量操作。
 * 相对 PR 版本的增强:
 *  - 修复 useStreamers 吞错误导致"接口失败被误判为空列表"的问题
 *  - 卡片与主页共用 StreamerCard,样式不再两处重复
 *  - 新增列表视图(主播多时可用)、搜索、筛选、批量暂停/删除
 */
export default function StreamersPage() {
  const { data: streamers, error, isLoading, mutate } = useSWR<LiveStreamerEntity[]>(
    '/v1/streamers',
    fetcher,
    { refreshInterval: 10000 }
  )
  const { data: infos } = useSWR<StreamerInfo[]>('/v1/streamer-info', fetcher, {
    refreshInterval: 30000,
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

  // ---- 增删改(保留 PR 逻辑) ----
  const { trigger: deleteStreamers } = useSWRMutation('/v1/streamers', requestDelete)
  const { trigger: updateStreamers } = useSWRMutation('/v1/streamers', put)
  const { trigger } = useSWRMutation('/v1/streamers', sendRequest)

  const onConfirm = async (id: number) => {
    await deleteStreamers(id)
  }

  const handleEntityPostprocessor = (values: any) => {
    if (values?.postprocessor) {
      values.postprocessor = values.postprocessor.map((element: any) => {
        if (element?.mv) {
          return { ...element, run: `mv ${element.mv}` }
        }
        return element
      })
    }
    return values
  }

  const handleOk = async (values: any) => {
    if (values?.postprocessor) {
      values.postprocessor = values.postprocessor.map((element: any) => {
        if (element?.mv) {
          return { ...element, run: `mv ${element.mv}` }
        }
        return element
      })
    }
    try {
      await trigger(values)
    } catch (e: any) {
      Notification.error({
        title: '创建失败',
        content: e?.message ?? String(e),
      })
    }
  }

  const handleUpdate = async (values: any) => {
    delete values.status
    delete values.statusTag
    delete values.upload_status
    if (values?.postprocessor) {
      values.postprocessor = values.postprocessor.map((element: any) => {
        if (element?.mv) {
          return { ...element, run: `mv ${element.mv}` }
        }
        return element
      })
    }
    try {
      await updateStreamers(values)
    } catch (e: any) {
      Notification.error({
        title: '更新失败',
        content: e?.message ?? String(e),
      })
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
  const toggleAll = (checked: boolean) => {
    setSelected(checked ? new Set(filtered.map((s) => s.id)) : new Set())
  }

  const batchPause = async () => {
    const ids = Array.from(selected)
    if (ids.length === 0) return
    try {
      await Promise.all(ids.map((id) => proxy(`/v1/streamers/${id}/pause`, { method: 'PUT' })))
      Notification.success({ title: `已暂停 ${ids.length} 个直播间` })
      setSelected(new Set())
      mutate()
    } catch (e: any) {
      Notification.error({ title: '批量暂停失败', content: e?.message ?? String(e) })
    }
  }

  const batchDelete = async () => {
    const ids = Array.from(selected)
    if (ids.length === 0) return
    try {
      await Promise.all(ids.map((id) => proxy(`/v1/streamers/${id}`, { method: 'DELETE' })))
      Notification.success({ title: `已删除 ${ids.length} 个直播间` })
      setSelected(new Set())
      mutate()
    } catch (e: any) {
      Notification.error({ title: '批量删除失败', content: e?.message ?? String(e) })
    }
  }

  // ---- 卡片操作区(网格) ----
  const renderActions = (item: LiveStreamerEntity) => (
    <ButtonGroup theme="borderless" className={styles.cardActions}>
      <TemplateModal onOk={handleUpdate} entity={handleEntityPostprocessor({ ...item })}>
        <Button theme="borderless" type="primary" icon={<IconEdit2Stroked />} aria-label="编辑" />
      </TemplateModal>
      <PauseButton streamer={item} />
      <Popconfirm title="确定是否要删除？" content="此操作将不可逆" onConfirm={() => onConfirm(item.id)}>
        <Button theme="borderless" type="danger" icon={<IconDeleteStroked />} aria-label="删除" />
      </Popconfirm>
      <OverrideModal onOk={handleUpdate} entity={handleEntityPostprocessor({ ...item })}>
        <Button theme="borderless" type="tertiary" icon={<IconWrench />} aria-label="高级" />
      </OverrideModal>
    </ButtonGroup>
  )

  const renderRowActions = (item: LiveStreamerEntity) => (
    <div className={styles.rowActions}>
      <TemplateModal onOk={handleUpdate} entity={handleEntityPostprocessor({ ...item })}>
        <Button theme="borderless" type="primary" icon={<IconEdit2Stroked />} aria-label="编辑" />
      </TemplateModal>
      <PauseButton streamer={item} />
      <Popconfirm title="确定是否要删除？" content="此操作将不可逆" onConfirm={() => onConfirm(item.id)}>
        <Button theme="borderless" type="danger" icon={<IconDeleteStroked />} aria-label="删除" />
      </Popconfirm>
      <OverrideModal onOk={handleUpdate} entity={handleEntityPostprocessor({ ...item })}>
        <Button theme="borderless" type="tertiary" icon={<IconWrench />} aria-label="高级" />
      </OverrideModal>
    </div>
  )

  return (
    <>
      <PageHeader
        title="直播管理"
        description="管理需要录制的直播间,支持新增、编辑与删除"
        icon={<IconVideoListStroked size="large" />}
        actions={
          <TemplateModal onOk={handleOk}>
            <Button icon={<IconPlusCircle />} theme="solid">
              新建
            </Button>
          </TemplateModal>
        }
      />
      <Content className={styles.content}>
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
            {selected.size > 0 && (
              <div className={styles.batchbar}>
                <span>
                  已选 <b>{selected.size}</b> 项
                </span>
                <span className={styles.batchSpacer} />
                <Button size="small" onClick={batchPause}>
                  批量暂停
                </Button>
                <Button size="small" type="danger" theme="borderless" onClick={batchDelete}>
                  批量删除
                </Button>
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
                      : '点击右上角「新建」开始'
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
                      <th style={{ width: 36 }}>
                        <input
                          type="checkbox"
                          className={styles.chk}
                          checked={filtered.length > 0 && filtered.every((s) => selected.has(s.id))}
                          onChange={(e) => toggleAll(e.target.checked)}
                          aria-label="全选"
                        />
                      </th>
                      <th>状态</th>
                      <th>主播</th>
                      <th>平台</th>
                      <th>最近录制</th>
                      <th style={{ textAlign: 'right' }}>操作</th>
                    </tr>
                  </thead>
                  <tbody>
                    {filtered.map((item) => {
                      const info = infoByUrl.get(item.url)
                      return (
                        <tr
                          key={item.id}
                          className={selected.has(item.id) ? styles.rowSel : ''}
                          onClick={() => toggleSel(item.id)}
                        >
                          <td onClick={(e) => e.stopPropagation()}>
                            <input
                              type="checkbox"
                              className={styles.chk}
                              checked={selected.has(item.id)}
                              onChange={() => toggleSel(item.id)}
                              aria-label={`选择 ${item.remark}`}
                            />
                          </td>
                          <td>{streamerStatusTag(item.status)}</td>
                          <td>
                            <div className={styles.cellName}>{item.remark || item.url}</div>
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
                            <Text type="tertiary" size="small">
                              {info?.date ? timeAgo(info.date) : '—'}
                            </Text>
                          </td>
                          <td onClick={(e) => e.stopPropagation()}>{renderRowActions(item)}</td>
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
