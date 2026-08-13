'use client'

import { BiliArchive, BiliArchivePage, fetcher } from '@/app/lib/api-streamer'
import { humDate } from '@/app/lib/utils'
import { useBiliUsers } from '@/app/lib/use-streamers'
import { IconUserCardVideo } from '@douyinfe/semi-icons'
import { Banner, Layout, Nav, Pagination, Select, Table, Typography } from '@douyinfe/semi-ui'
import { useState } from 'react'
import useSWR from 'swr'

const statusOptions = [
  { label: '全部状态', value: 'all' },
  { label: '审核中', value: 'is_pubing' },
  { label: '已通过', value: 'pubed' },
  { label: '未通过', value: 'not_pubed' },
]

export default function ArchivesPage() {
  const { Header, Content } = Layout
  const { Text } = Typography
  const { biliUsers, isLoading: usersLoading } = useBiliUsers()
  const [userId, setUserId] = useState<number>()
  const [status, setStatus] = useState('all')
  const [pageNumber, setPageNumber] = useState(1)
  const key = userId
    ? `/v1/users/${userId}/archives?status=${status}&from_page=${pageNumber}&max_pages=1`
    : null
  const { data, error, isLoading } = useSWR<BiliArchivePage>(key, fetcher)

  const columns = [
    {
      title: 'BV号',
      dataIndex: 'bvid',
      render: (bvid: string) => (
        <a href={`https://www.bilibili.com/video/${bvid}`} target="_blank" rel="noreferrer">
          {bvid}
        </a>
      ),
    },
    {
      title: '标题',
      dataIndex: 'title',
      render: (title: string) => <Text strong>{title}</Text>,
    },
    {
      title: '状态',
      dataIndex: 'state_desc',
      render: (stateDescription: string, archive: BiliArchive) =>
        archive.reject_reason || stateDescription || `状态 ${archive.state}`,
    },
    {
      title: '时长',
      dataIndex: 'duration',
      render: (duration: number) => `${Math.floor(duration / 60)}:${String(duration % 60).padStart(2, '0')}`,
    },
    {
      title: '发布时间',
      dataIndex: 'ptime',
      render: (ptime: number, archive: BiliArchive) => {
        const timestamp = ptime || archive.ctime
        return timestamp ? humDate(timestamp) : '-'
      },
    },
  ]

  return (
    <>
      <Header style={{ backgroundColor: 'var(--semi-color-bg-1)' }}>
        <Nav
          style={{ border: 'none' }}
          header={
            <>
              <IconUserCardVideo size="large" />
              <h4 style={{ marginLeft: 12 }}>B站稿件</h4>
            </>
          }
          mode="horizontal"
        />
      </Header>
      <Content style={{ padding: 24, backgroundColor: 'var(--semi-color-bg-0)' }}>
        <div style={{ display: 'flex', gap: 12, flexWrap: 'wrap', marginBottom: 20 }}>
          <Select
            placeholder="请选择 B 站账号"
            loading={usersLoading}
            value={userId}
            style={{ minWidth: 260 }}
            onChange={value => {
              setUserId(Number(value))
              setPageNumber(1)
            }}
          >
            {biliUsers.map(user => (
              <Select.Option key={user.id} value={user.id}>
                {user.name}
              </Select.Option>
            ))}
          </Select>
          <Select
            value={status}
            optionList={statusOptions}
            onChange={value => {
              setStatus(String(value))
              setPageNumber(1)
            }}
          />
        </div>

        {!userId && <Banner type="info" description="请先明确选择一个账号，再读取该账号的远程稿件。" />}
        {error && <Banner type="danger" description={error.message || '读取稿件失败'} />}
        {userId && (
          <>
            <Table
              rowKey="aid"
              size="small"
              loading={isLoading}
              columns={columns}
              dataSource={data?.archives ?? []}
              pagination={false}
            />
            {!!data && data.total > 0 && (
              <Pagination
                style={{ marginTop: 16, justifyContent: 'flex-end' }}
                currentPage={pageNumber}
                pageSize={data.page_size}
                total={data.total}
                onPageChange={setPageNumber}
              />
            )}
          </>
        )}
      </Content>
    </>
  )
}
