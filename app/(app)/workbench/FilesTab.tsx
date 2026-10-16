'use client'
import { Pagination, Table, Typography } from '@douyinfe/semi-ui'
import { SortOrder } from '@douyinfe/semi-ui/lib/es/table'
import useSWR from 'swr'
import { fetcher, FileList } from '@/app/lib/api-streamer'
import { useEffect, useState } from 'react'
import dynamic from 'next/dynamic'
import Link from 'next/link'
import { humDate } from '@/app/lib/utils'
import { formatSize } from '@/app/lib/use-dashboard'
import { replayHref } from '@/app/lib/sessions'
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

/** 「录制文件」：已录完的文件在线回放；属于某一场的文件能按场次打开回看 */
export default function FilesTab() {
  const { Text } = Typography
  const { data: data, error, isLoading } = useSWR<FileList[]>('/v1/videos', fetcher)
  const { can } = useMe()
  const [fileName, setFileName] = useState<string>()
  const [visible, setVisible] = useState(false)
  const [pageSize, setPageSize] = useState(10)
  const [pageNumber, setPageNumber] = useState(1)

  useEffect(() => {
    const saved = Number(localStorage.getItem(PAGE_SIZE_KEY))
    if (PAGE_SIZES.includes(saved)) setPageSize(saved)
  }, [])
  useEffect(() => {
    const pageCount = Math.max(1, Math.ceil((data?.length ?? 0) / pageSize))
    if (pageNumber > pageCount) setPageNumber(pageCount)
  }, [data?.length, pageNumber, pageSize])

  const pageRows = (data ?? []).slice((pageNumber - 1) * pageSize, pageNumber * pageSize)

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
              href={replayHref(record.session_id, record.segment_start_ms)}
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
      <div className={dc.card}>
        <Table
          size="small"
          scroll={{ x: 'max-content' }}
          columns={columns}
          rowKey="key"
          dataSource={pageRows}
          loading={isLoading}
          empty={error ? '加载失败，请检查后端连接' : '暂无数据'}
        />
        {!!data?.length && <div className={styles.filePager}>
          <label>每页显示
            <select value={pageSize} onChange={event => {
              const value = Number(event.target.value)
              setPageSize(value); setPageNumber(1); localStorage.setItem(PAGE_SIZE_KEY, String(value))
            }}>
              {PAGE_SIZES.map(size => <option key={size} value={size}>{size}</option>)}
            </select>
            条
          </label>
          <Pagination currentPage={pageNumber} pageSize={pageSize} total={data.length} onPageChange={setPageNumber} />
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
