'use client'

import { Layout, Nav, Spin, Typography } from '@douyinfe/semi-ui'
import useSWR from 'swr'
import { fetcher } from '@/app/lib/api-streamer'
import {
  IconCustomerSupport,
  IconSearch,
  IconSetting,
  IconVideoListStroked,
} from '@douyinfe/semi-icons'
import { JSONTree } from 'react-json-tree'

export default function Home() {
  const { Header, Footer, Sider, Content } = Layout
  const { data: data, error, isLoading } = useSWR<any[]>('/v1/status', fetcher)

  if (isLoading) {
    return <Spin size="large" />
  }
  return (
    <>
      <Header style={{ backgroundColor: 'var(--semi-color-bg-1)' }}>
        <Nav
          style={{ border: 'none' }}
          header={
            <>
              <div
                style={{
                  backgroundColor: 'rgba(var(--semi-lime-2), 1)',
                  borderRadius: 'var(--semi-border-radius-large)',
                  color: 'var(--semi-color-bg-0)',
                  display: 'flex',
                  padding: '6px',
                }}
              >
                <IconSetting size="large" />
              </div>
              <h4 style={{ marginLeft: '12px' }}>任务平台</h4>
            </>
          }
          mode="horizontal"
        ></Nav>
      </Header>
      <Content
        style={{
          paddingLeft: 12,
          paddingRight: 12,
          backgroundColor: 'var(--semi-color-bg-0)',
        }}
      >
        <main>
          <JSONTree data={data} />
        </main>
      </Content>
    </>
  )
}
