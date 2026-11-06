'use client'
import { Button, Pagination, Table, Typography } from '@douyinfe/semi-ui'
import { SortOrder } from '@douyinfe/semi-ui/lib/es/table'
import useSWR from 'swr'
import { fetcher, FileList } from '@/app/lib/api-streamer'
import { useState, useSyncExternalStore } from 'react'
import dynamic from 'next/dynamic'
import Link from 'next/link'
import { humDate } from '@/app/lib/utils'
import { formatSize } from '@/app/lib/use-dashboard'
import { sessionUrl, type SessionDetail } from '@/app/lib/sessions'
import { recordingEditorHref } from '@/app/lib/workbench'
import dc from '@/app/ui/data-card.module.scss'
import { FormDialog } from '@/app/ui/shell'
import styles from './page.module.scss'
import RevealFileButton from '@/app/ui/RevealFileButton'
import { useMe } from '@/app/lib/use-me'

const Players = dynamic(() => import('@/app/ui/Player'), {
  ssr: false,
})
const PAGE_SIZE_KEY = 'biliup:recording-files-page-size'
const PAGE_SIZES = [10, 20, 50, 100]
let volatilePageSize: number | undefined
const pageSizeSnapshot = () => {
  if (volatilePageSize !== undefined) return volatilePageSize
  try { const value = Number(localStorage.getItem(PAGE_SIZE_KEY)); return PAGE_SIZES.includes(value) ? value : 10 } catch { return 10 }
}
const subscribePageSize = (notify: () => void) => {
  const storage = () => { volatilePageSize = undefined; notify() }
  window.addEventListener('storage', storage); window.addEventListener(PAGE_SIZE_KEY, notify)
  return () => { window.removeEventListener('storage', storage); window.removeEventListener(PAGE_SIZE_KEY, notify) }
}
const setPageSize = (value: number) => {
  volatilePageSize = value
  try { localStorage.setItem(PAGE_SIZE_KEY, String(value)) } catch { /* Current view still updates. */ }
  window.dispatchEvent(new Event(PAGE_SIZE_KEY))
}

/** 「录制文件」：已录完的文件在线回放；属于某一场的文件能按场次打开回看 */
export default function FilesTab({ sessionId = null }: { sessionId?: number | null }) {
  const { Text } = Typography
  const { data: data, error, isLoading } = useSWR<FileList[]>('/v1/videos', fetcher)
  const { data: source, error: sourceError } = useSWR<SessionDetail>(sessionId ? sessionUrl(sessionId) : null, fetcher)
  const [allFiles, setAllFiles] = useState(false)
  const sourceIds = source?.kind === 'group' ? source.members?.map(member => member.source_session_id) ?? [] : sessionId ? [sessionId] : []
  const files = (data ?? []).filter(file => !sessionId || allFiles || sourceIds.includes(file.session_id ?? -1))
  const { can } = useMe()
  const [fileName, setFileName] = useState<string>()
  const [visible, setVisible] = useState(false)
  const pageSize = useSyncExternalStore(subscribePageSize, pageSizeSnapshot, () => 10)
  const [pageNumber, setPageNumber] = useState(1)

  const pageCount = Math.max(1, Math.ceil(files.length / pageSize))
  if (pageNumber > pageCount) setPageNumber(pageCount)

  const pageRows = files.slice((pageNumber - 1) * pageSize, pageNumber * pageSize)
  const replay = (file: FileList) => {
    const member = source?.members?.find(member => member.source_session_id === file.session_id)
    const sameSource = sessionId && sourceIds.includes(file.session_id ?? -1)
    const sourceId = sameSource ? sessionId : file.session_id
    return sourceId ? recordingEditorHref(sourceId, (file.segment_start_ms ?? 0) + (sameSource ? member?.offset_ms ?? 0 : 0)) : '/sessions'
  }

  const columns = [
    {
      title: '标题',
      dataIndex: 'name',
      render: (text: any) => <Text strong>{text}</Text>,
    },
    {
      title: '大小',
      dataIndex: 'size',
      render: (size: number) => formatSize(size || 0),
    },
    {
      title: '更新日期',
      dataIndex: 'updateTime',
      defaultSortOrder: 'descend' as SortOrder,
      sorter: (a: any, b: any) => (a.updateTime - b.updateTime > 0 ? 1 : -1),
      render: (time: number) => humDate(time),
    },
    {
      title: '',
      dataIndex: 'operate',
      render: (text: any, record: FileList) => (
        <span className={styles.actions}>
          <Text link style={{ cursor: 'pointer' }} onClick={() => showDialog(record.name)}>
            播放
          </Text>
          {record.session_id !== undefined ? (
            <Link
              href={replay(record)}
              onClick={event => {
                if (event.button || event.ctrlKey || event.metaKey || event.shiftKey || event.altKey) return
                event.preventDefault(); window.history.pushState(null, '', replay(record))
              }}
              className={styles.replayLink}
              title="打开这一场的回看页，定位到这个文件的开头；可以拖到整场任意位置"
            >
              按场次回看
            </Link>
          ) : null}
          <RevealFileButton request={{ kind: 'recording', file_name: record.name }} allowed={can('file.view')} />
        </span>
      ),
    },
  ]

  const showDialog = (name: string) => {
    setVisible(true)
    setFileName(name)
  }

  return (
    <>
      {sessionId && <div style={{ display: 'flex', alignItems: 'center', gap: 12, marginBottom: 12 }}>
        <Text type="tertiary">{allFiles ? '全部录制文件' : `场次 #${sessionId} 的源文件`}</Text>
        <Button size="small" onClick={() => { setAllFiles(value => !value); setPageNumber(1) }}>{allFiles ? '只看当前场次' : '查看全部文件'}</Button>
      </div>}
      <div className={dc.card}>
        <Table
          size="small"
          scroll={{ x: 'max-content' }}
          columns={columns}
          rowKey="key"
          dataSource={pageRows}
          loading={isLoading || !!sessionId && !allFiles && !source && !sourceError}
          empty={error || sourceError ? '加载失败，请检查后端连接' : '暂无数据'}
        />
        {!!files.length && <div className={styles.filePager}>
          <label>每页显示
            <select value={pageSize} onChange={event => {
              const value = Number(event.target.value)
              setPageSize(value); setPageNumber(1)
            }}>
              {PAGE_SIZES.map(size => <option key={size} value={size}>{size}</option>)}
            </select>
            条
          </label>
          <Pagination currentPage={pageNumber} pageSize={pageSize} total={files.length} onPageChange={setPageNumber} />
        </div>}
      </div>
      <FormDialog title={fileName} immersive visible={visible} onCancel={() => setVisible(false)}>
        <div className={styles.playerStage}>
          {/* 文件名常带直播标题，里面的 #、% 不编码会被当成片段 / 转义，请求到的不是这个文件 */}
          <Players url={(process.env.NEXT_PUBLIC_API_SERVER ?? '') + '/static/' + encodeURIComponent(fileName ?? '')} />
        </div>
      </FormDialog>
    </>
  )
}
