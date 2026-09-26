'use client'
import { Modal, Table, Typography } from '@douyinfe/semi-ui'
import { SortOrder } from '@douyinfe/semi-ui/lib/es/table'
import useSWR from 'swr'
import { fetcher, FileList } from '@/app/lib/api-streamer'
import { useState } from 'react'
import dynamic from 'next/dynamic'
import Link from 'next/link'
import { humDate } from '@/app/lib/utils'
import { formatSize } from '@/app/lib/use-dashboard'
import { replayHref } from '@/app/lib/sessions'
import dc from '@/app/ui/data-card.module.scss'
import styles from './page.module.scss'

const Players = dynamic(() => import('@/app/ui/Player'), {
  ssr: false,
})

/** 「录制文件」：已录完的文件在线回放；属于某一场的文件能按场次打开回看 */
export default function FilesTab() {
  const { Text } = Typography
  const { data: data, error, isLoading } = useSWR<FileList[]>('/v1/videos', fetcher)
  const [fileName, setFileName] = useState<string>()
  const [visible, setVisible] = useState(false)

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
          dataSource={data}
          loading={isLoading}
          empty={error ? '加载失败，请检查后端连接' : '暂无数据'}
        />
      </div>
      <Modal
        visible={visible}
        onCancel={() => setVisible(false)}
        closeOnEsc={true}
        style={{ width: 'min(600px, 90vw)' }}
        size="large"
        bodyStyle={{ height: 500 }}
        footer={null}
      >
        <Players url={(process.env.NEXT_PUBLIC_API_SERVER ?? '') + '/static/' + fileName} />
        <div id="mse" />
      </Modal>
    </>
  )
}
