'use client'
import { Modal, Table, Tabs, TabPane, Typography } from '@douyinfe/semi-ui'
import { IconVideoListStroked } from '@douyinfe/semi-icons'
import { SortOrder } from '@douyinfe/semi-ui/lib/es/table'
import useSWR from 'swr'
import { fetcher, FileList } from '@/app/lib/api-streamer'
import { useState } from 'react'
import dynamic from 'next/dynamic'
import { humDate } from '@/app/lib/utils'
import { formatSize } from '@/app/lib/use-dashboard'
import PageHeader from '../components/PageHeader'
import LiveMonitor from '@/app/ui/LiveMonitor'
import dc from '@/app/ui/data-card.module.scss'
import styles from './page.module.scss'

const Players = dynamic(() => import('@/app/ui/Player'), {
  ssr: false,
})

type HistoryTab = 'files' | 'monitor'

/**
 * 历史记录：「录制文件」（已录完的文件回放）与「实时监视」（正在录制的直播间多路同屏）两个 Tab。
 * 监视器是独立组件 <LiveMonitor />，要挪到别的页面只需换个挂载点。
 */
export default function History() {
  const { Text } = Typography
  const [tab, setTab] = useState<HistoryTab>('files')
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
        description="已录制的视频文件可在线回放；「实时监视」同屏查看正在录制的直播间"
      />
      <div className={dc.content}>
        <Tabs
          type="line"
          activeKey={tab}
          onChange={(key) => setTab(key as HistoryTab)}
          className={styles.tabs}
          keepDOM={false}
        >
          <TabPane tab="录制文件" itemKey="files">
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
          </TabPane>
          <TabPane tab="实时监视" itemKey="monitor">
            {/* keepDOM=false：切走即卸载播放器、断开全部预览连接 */}
            <LiveMonitor />
          </TabPane>
        </Tabs>
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
