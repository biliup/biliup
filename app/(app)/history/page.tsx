'use client'
import { Modal, Table, Typography } from '@douyinfe/semi-ui'
import { IconVideoListStroked } from '@douyinfe/semi-icons'
import { SortOrder } from '@douyinfe/semi-ui/lib/es/table'
import useSWR from 'swr'
import { fetcher, FileList } from '@/app/lib/api-streamer'
import { useState } from 'react'
import dynamic from 'next/dynamic'
import { humDate } from '@/app/lib/utils'
import { formatSize } from '@/app/lib/use-dashboard'
import PageHeader from '../components/PageHeader'
import dc from '@/app/ui/data-card.module.scss'

const Players = dynamic(() => import('@/app/ui/Player'), {
  ssr: false,
})

export default function History() {
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
      render: (text: any, record: any) => (
        <Text link style={{ cursor: 'pointer' }} onClick={() => showDialog(record.name)}>
          播放
        </Text>
      ),
    },
  ]

  const showDialog = (name: string) => {
    setVisible(true)
    setFileName(name)
  }

  return (
    <>
      <PageHeader
        icon={<IconVideoListStroked size="large" />}
        title="历史记录"
        description="已录制的视频文件,可在线回放"
      />
      <div className={dc.content}>
        <div className={dc.card}>
          <Table
            size="small"
            scroll={{ x: 'max-content' }}
            columns={columns}
            dataSource={data}
            loading={isLoading}
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
      </div>
    </>
  )
}
