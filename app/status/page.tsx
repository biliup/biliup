'use client'

import { Layout, Spin, Typography } from '@douyinfe/semi-ui'
import useSWR from 'swr'
import { fetcher } from '@/app/lib/api-streamer'
import {
  IconCustomerSupport,
  IconSearch,
  IconSetting,
  IconVideoListStroked,
} from '@douyinfe/semi-icons'
import { JSONTree } from 'react-json-tree'
import { AuthGuard } from '../lib/auth-guard'
import ProtectedLayout from '../lib/protected-layout'

export default function Home() {
  const { Header, Footer, Sider, Content } = Layout
  const { data: data, error, isLoading } = useSWR<any[]>('/v1/status', fetcher)

  if (isLoading) {
    return <Spin size="large" />
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
                backgroundColor: 'rgba(var(--semi-lime-2), 1)',
                borderRadius: 'var(--semi-border-radius-large)',
                color: 'var(--semi-color-bg-0)',
                display: 'flex',
                padding: '6px',
              }}
            >
              <IconSetting size="large" />
            </div>
            <h4 style={{ marginLeft: '12px', margin: 0 }}>任务平台</h4>
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
            <JSONTree data={data} />
          </main>
        </Content>
      </ProtectedLayout>
    </AuthGuard>
  )
}
