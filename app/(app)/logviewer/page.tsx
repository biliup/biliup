'use client'

import { useEffect, useState, useRef } from 'react'
import { Layout, Nav, Spin, Typography, Select, Card, Button, Toast, Tabs, TabPane } from '@douyinfe/semi-ui'
import {
  IconCustomerSupport,
  IconRefresh,
  IconClear,
  IconSave,
} from '@douyinfe/semi-icons'

// 日志内容组件
interface LogContentProps {
  logs: string[];
  logContainerRef: React.RefObject<HTMLDivElement>;
  isLoading: boolean;
}

const LogContent = ({ logs, logContainerRef, isLoading }: LogContentProps) => {
  if (isLoading) {
    return (
      <div style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', height: '100%' }}>
        <Spin size="large" />
      </div>
    )
  }

  // 判断滚动条是否接近底部
  const isScrolledToBottom = () => {
    const containers = document.getElementsByClassName('log-container');
    if (containers.length === 0) return false;

    const container = containers[0] as HTMLElement;
    // 50px 容许
    const diff = container.scrollHeight - container.scrollTop;
    return diff - container.clientHeight <= 50;
  };

  // 滚动到底部
  const scrollToBottom = () => {
    const containers = document.getElementsByClassName('log-container');
    if (containers.length > 0) {
      const container = containers[0] as HTMLElement;
      container.scrollTop = container.scrollHeight;
    }
  };

  useEffect(() => {
    // 如果已经滚动到底部，那么在日志更新时自动滚动到底部
    if (logs.length > 0 && isScrolledToBottom()) {
      console.log("自动滚动到底部")
      scrollToBottom();
    }
  }, [logs]);

  return (
    <div
      className="log-container"
      ref={logContainerRef}
      style={{
        height: 'calc(100% - 40px)', // 减去 tabs 的高度
        maxHeight: 'calc(100vh - 180px)', // 设置最大高度
        overflow: 'auto',
        padding: 12,
        backgroundColor: 'var(--semi-color-bg-1)',
        borderRadius: 4,
        whiteSpace: 'pre-wrap',
        wordBreak: 'break-all'
      }}
    >
      {logs.length > 0 ? (
        logs.map((log, index) => (
          <div key={index} style={{ marginBottom: 2 }}>{log}</div>
        ))
      ) : (
        <div style={{ color: 'var(--semi-color-text-2)', textAlign: 'center', marginTop: 20 }}>
          暂无日志内容
        </div>
      )}
    </div>
  )
}

export default function LogViewer() {
  const { Header, Content } = Layout
  const [logs, setLogs] = useState<string[]>([])
  const [isConnected, setIsConnected] = useState(false)
  const [isLoading, setIsLoading] = useState(true)
  const [activeTab, setActiveTab] = useState('ds_update')
  const wsRef = useRef<WebSocket | null>(null)
  const logContainerRef = useRef<HTMLDivElement>(null)

  const connectWebSocket = () => {
    setIsLoading(true)
    setLogs([])

    // 关闭现有连接
    if (wsRef.current) {
      wsRef.current.close()
    }

    // 创建新的WebSocket连接
    const isDev = process.env.NODE_ENV === 'development';
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const server = isDev
      ? process.env.NEXT_PUBLIC_API_SERVER?.replace(/^http/, 'ws') // 使用环境变量中配置的API服务器地址
      : `${protocol}//${window.location.host}`;
    const wsUrl = `${server}/v1/ws/logs?file=${activeTab}.log`;

    const ws = new WebSocket(wsUrl)
    wsRef.current = ws

    ws.onopen = () => {
      setIsConnected(true)
      setIsLoading(false)
      Toast.success('日志连接已建立')
    }

    ws.onmessage = (event) => {
      setLogs(prev => [...prev, event.data])
    }

    ws.onerror = (error) => {
      console.error('WebSocket错误:', error)

      // 检查是否是连接建立前WebSocket已关闭的错误
      // 这种情况通常发生在组件卸载或用户切换标签时
      if (ws.readyState === WebSocket.CLOSED || ws.readyState === WebSocket.CLOSING) {
        console.log('WebSocket在连接建立前已关闭')
      } else {
        // 其他错误仍然显示Toast提示
        Toast.error('连接错误，请重试')
      }

      setIsLoading(false)
    }

    ws.onclose = () => {
      setIsConnected(false)
      console.log('WebSocket连接已关闭')
    }
  }

  useEffect(() => {
    connectWebSocket()

    return () => {
      // 组件卸载时关闭WebSocket连接
      if (wsRef.current) {
        console.log('主动关闭WebSocket连接')
        wsRef.current.close()
      }
    }
  }, [activeTab])

  const handleFileChange = (value: string) => {
    setActiveTab(value)
  }

  const handleRefresh = () => {
    connectWebSocket()
  }

  const handleClear = () => {
    setLogs([])
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
                  backgroundColor: 'rgba(var(--semi-blue-4), 1)',
                  borderRadius: 'var(--semi-border-radius-large)',
                  color: 'var(--semi-color-bg-0)',
                  display: 'flex',
                  padding: '6px',
                }}
              >
                <IconCustomerSupport size="large" />
              </div>
              <h4 style={{ marginLeft: '12px' }}>实时日志</h4>
            </>
          }
          mode="horizontal"
        ></Nav>
      </Header>
      <Content
        style={{
          padding: 12,
          backgroundColor: 'var(--semi-color-bg-0)',
          height: 'calc(100vh - 60px)',
          display: 'flex',
          flexDirection: 'column',
        }}
      >
        <Card
          style={{ flex: 1, overflow: 'hidden' }}
          bodyStyle={{ height: '100%', overflow: 'hidden', boxSizing: 'border-box' }}
        >
          <Tabs
            type="line"
            style={{
              marginTop: -20,
              marginBottom: -20
            }}
            activeKey={activeTab}
            onChange={handleFileChange}
            tabBarExtraContent={
              <div style={{ display: 'flex', gap: 8, alignItems: 'center'}}>
                <Button
                  icon={<IconSave />}
                  onClick={() => (window.location.href = `/static/${activeTab}.log`)}
                  type="primary"
                  theme="solid"
                  size="small"
                >
                  下载
                </Button>
                <Button
                  icon={<IconRefresh />}
                  onClick={handleRefresh}
                  theme="light"
                  size="small"
                >
                  刷新
                </Button>
                <Button
                  icon={<IconClear />}
                  onClick={handleClear}
                  theme="light"
                  size="small"
                >
                  清空
                </Button>
                <Typography.Text
                  type={isConnected ? 'success' : 'danger'}
                  style={{ marginLeft: 8, display: 'flex', alignItems: 'center' }}
                >
                  {isConnected ? '已连接' : '未连接'}
                </Typography.Text>
              </div>
            }
          >
            <TabPane tab="主程序运行日志" itemKey="ds_update">
              <LogContent logs={logs} logContainerRef={logContainerRef} isLoading={isLoading} />
            </TabPane>
            <TabPane tab="biliup下载和上传日志" itemKey="download">
              <LogContent logs={logs} logContainerRef={logContainerRef} isLoading={isLoading} />
            </TabPane>
            {/*<TabPane tab="biliup-rs上传日志" itemKey="upload">*/}
            {/*  <LogContent logs={logs} logContainerRef={logContainerRef} isLoading={isLoading} />*/}
            {/*</TabPane>*/}
          </Tabs>
        </Card>
      </Content>
    </>
  )
}