'use client'

import { AutoComplete, Layout, Spin, Table, Typography } from '@douyinfe/semi-ui'
import { SortOrder } from '@douyinfe/semi-ui/lib/es/table'
import useSWR from 'swr'
import { fetcher, FileList } from '@/app/lib/api-streamer'
import {
  JSXElementConstructor,
  Key,
  ReactElement,
  ReactNode,
  ReactPortal,
  useRef,
  useState,
} from 'react'
import { IconHistory } from '@douyinfe/semi-icons'
import { humDate } from '@/app/lib/utils'
import Filter from '@/app/job/Filter'
import { AuthGuard } from '../lib/auth-guard'
import ProtectedLayout from '../lib/protected-layout'

export default function Home() {
  const { Header, Footer, Sider, Content } = Layout
  const { data: data, error, isLoading } = useSWR<any[]>('/v1/streamer-info', fetcher)
  if (isLoading) {
    return <Spin size="large" />
  }
  const { Text } = Typography
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
      render: (text: any, record: any, index: any) => {
        return <Text strong>{text}</Text>
      },
      onFilter: (value: any, record: any) => record.title.includes(value),
      renderFilterDropdown: Filter,
    },
    {
      title: '链接',
      dataIndex: 'url',
    },
    {
      title: '封面',
      dataIndex: 'live_cover_path',
    },
    {
      title: '更新日期',
      dataIndex: 'date',
      defaultSortOrder: 'descend' as SortOrder,
      sorter: (a: any, b: any) => (a.date - b.date > 0 ? 1 : -1),
      render: (time: number) => humDate(time),
    },
  ]
  const expandRowRender = (record: any, index: number | undefined) => {
    return (
      <>
        文件列表：
        {record.files.map(
          (it: {
            id: Key | null | undefined
            file:
              | string
              | number
              | boolean
              | ReactElement<any, string | JSXElementConstructor<any>>
              | Iterable<ReactNode>
              | ReactPortal
              | null
              | undefined
          }) => {
            return <div key={it.id}>&nbsp;&nbsp;文件名：{it.file}</div>
          }
        )}
      </>
    )
  }
  return (
    <AuthGuard>
      <ProtectedLayout>
        <Header style={{ backgroundColor: 'var(--semi-color-bg-1)' }}>
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              padding: '0 20px',
              height: '60px',
              backgroundColor: 'var(--semi-color-bg-1)',
              boxShadow: '0 1px 2px 0 rgb(0 0 0 / 0.05)',
            }}
          >
            <div
              style={{
                backgroundColor: 'rgb(250 102 76)',
                borderRadius: 'var(--semi-border-radius-large)',
                color: 'var(--semi-color-bg-0)',
                display: 'flex',
                padding: '6px',
              }}
            >
              <IconHistory size="large" />
            </div>
            <h4 style={{ marginLeft: '12px', margin: 0 }}>直播历史</h4>
          </div>
        </Header>
        <Content
          style={{
            paddingLeft: 12,
            paddingRight: 12,
            backgroundColor: 'var(--semi-color-bg-0)',
          }}
        >
          <main>
            <Table
              size="small"
              rowKey="id"
              columns={columns}
              dataSource={data}
              expandedRowRender={expandRowRender}
            />
          </main>
        </Content>
      </ProtectedLayout>
    </AuthGuard>
  )
}
