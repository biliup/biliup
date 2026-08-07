'use client'
import { Spin, Table, Typography } from '@douyinfe/semi-ui'
import { SortOrder } from '@douyinfe/semi-ui/lib/es/table'
import useSWR from 'swr'
import { fetcher } from '@/app/lib/api-streamer'
import { IconHistory } from '@douyinfe/semi-icons'
import { humDate } from '@/app/lib/utils'
import Filter from '@/app/(app)/job/Filter'
import { useIsMobile } from '../../lib/useIsMobile'
import PageHeader from '../components/PageHeader'
import dc from '@/app/ui/data-card.module.scss'

export default function Job() {
  const isMobile = useIsMobile()
  const { Text } = Typography
  const { data: data, error, isLoading } = useSWR<any[]>('/v1/streamer-info', fetcher)

  if (isLoading) {
    return (
      <>
        <PageHeader
          icon={<IconHistory size="large" />}
          title="直播历史"
          description="按主播查看历史直播记录"
        />
        <div style={{ padding: '80px 0', textAlign: 'center' }}>
          <Spin size="large" />
        </div>
      </>
    )
  }

  const columns = [
    {
      title: '名称',
      dataIndex: 'name',
      onFilter: (value: any, record: any) => record.name.includes(value),
      renderFilterDropdown: Filter,
    },
    {
      title: '标题',
      dataIndex: 'title',
      render: (text: any) => (
        <Text strong style={{ whiteSpace: 'nowrap' }}>
          {text}
        </Text>
      ),
      onFilter: (value: any, record: any) => record.title.includes(value),
      renderFilterDropdown: Filter,
    },
    ...(isMobile
      ? []
      : [
          {
            title: '链接',
            dataIndex: 'url',
          },
          {
            title: '封面',
            dataIndex: 'live_cover_path',
          },
        ]),
    {
      title: '更新日期',
      dataIndex: 'date',
      defaultSortOrder: 'descend' as SortOrder,
      sorter: (a: any, b: any) => (a.date - b.date > 0 ? 1 : -1),
      render: (time: number) => humDate(time),
    },
  ]

  return (
    <>
      <PageHeader
        icon={<IconHistory size="large" />}
        title="直播历史"
        description="按主播查看历史直播记录"
      />
      <div className={dc.content}>
        <div className={dc.card}>
          <Table
            size="small"
            rowKey="id"
            scroll={{ x: 'max-content' }}
            columns={columns}
            dataSource={data}
            expandedRowRender={expandRowRender}
          />
        </div>
      </div>
    </>
  )
}

// 展开子行:该次直播的录制文件列表
const FileLists = ({ recordId }: { recordId: string }) => {
  const { data: files, isLoading } = useSWR(`/v1/streamer-info/files/${recordId}`, fetcher)

  if (isLoading) return <div>加载中...</div>
  if (!files || files.length === 0) return <div>暂无文件</div>

  return (
    <div style={{ padding: '4px 8px', fontSize: 13, color: 'var(--semi-color-text-1)' }}>
      文件列表:
      {files.map((it: any) => (
        <div key={it.id} style={{ padding: '2px 0 2px 24px', fontVariantNumeric: 'tabular-nums' }}>
          {it.file}
        </div>
      ))}
    </div>
  )
}

const expandRowRender = (record: any) => {
  return <FileLists recordId={record.id} />
}
