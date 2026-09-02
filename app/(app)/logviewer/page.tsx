'use client'
import { useEffect, useState, useRef } from 'react'
import { useIsMobile } from '../../lib/useIsMobile'
import { Button, Spin, Typography, Tabs, TabPane, Toast } from '@douyinfe/semi-ui'
import { IconCustomerSupport, IconRefresh, IconClear, IconSave } from '@douyinfe/semi-icons'
import PageHeader from '../components/PageHeader'
import dc from '@/app/ui/data-card.module.scss'

// 日志内容组件
interface LogContentProps {
  logs: string[]
  logContainerRef: React.RefObject<HTMLDivElement>
  isLoading: boolean
}

const LogContent = ({ logs, logContainerRef, isLoading }: LogContentProps) => {
  // 判断滚动条是否接近底部
  const isScrolledToBottom = () => {
    const containers = document.getElementsByClassName('log-container')
    if (containers.length === 0) return false
    const container = containers[0] as HTMLElement
    const diff = container.scrollHeight - container.scrollTop
    return diff - container.clientHeight <= 50
  }

  const scrollToBottom = () => {
    const containers = document.getElementsByClassName('log-container')
    if (containers.length > 0) {
      const container = containers[0] as HTMLElement
      container.scrollTop = container.scrollHeight
    }
  }

  useEffect(() => {
    if (logs.length > 0 && isScrolledToBottom()) {
      scrollToBottom()
    }
  }, [logs])

  if (isLoading) {
    return (
      <div style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', height: '100%' }}>
        <Spin size="large" />
      </div>
    )
  }

  return (
    <div
      className="log-container"
      ref={logContainerRef}
      style={{
        height: 'calc(100vh - 220px)',
        minHeight: 320,
        overflow: 'auto',
        padding: 14,
        backgroundColor: 'var(--semi-color-fill-0)',
        borderRadius: 8,
        whiteSpace: 'pre-wrap',
        wordBreak: 'break-all',
        fontFamily: 'ui-monospace, SFMono-Regular, Consolas, "Liberation Mono", Menlo, monospace',
        fontSize: 12.5,
        lineHeight: 1.7,
      }}
    >
      {logs.length > 0 ? (
        logs.map((log, index) => (
          <div key={index} style={{ marginBottom: 1, color: 'var(--semi-color-text-1)' }}>
            {log}
          </div>
        ))
      ) : (
        <div style={{ color: 'var(--semi-color-text-2)', textAlign: 'center', marginTop: 24 }}>
          暂无日志内容
        </div>
      )}
    </div>
  )
}

export default function LogViewer() {
  const [logs, setLogs] = useState<string[]>([])
  const [isConnected, setIsConnected] = useState(false)
  const [isLoading, setIsLoading] = useState(true)
  const [activeTab, setActiveTab] = useState('ds_update')
  const wsRef = useRef<WebSocket | null>(null)
  const logContainerRef = useRef<HTMLDivElement>(null)
  const isMobile = useIsMobile()

  const connectWebSocket = () => {
    setIsLoading(true)
    setLogs([])

    if (wsRef.current) {
      wsRef.current.close()
    }

    const isDev = process.env.NODE_ENV === 'development'
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
    const server = isDev
      ? process.env.NEXT_PUBLIC_API_SERVER?.replace(/^http/, 'ws')
      : `${protocol}//${window.location.host}`
    const wsUrl = `${server}/v1/ws/logs?file=${activeTab}.log`

    const ws = new WebSocket(wsUrl)
    wsRef.current = ws

    ws.onopen = () => {
      setIsConnected(true)
      setIsLoading(false)
      Toast.success('日志连接已建立')
    }

    ws.onmessage = (event) => {
      setLogs((prev) => [...prev, event.data])
    }

    ws.onerror = () => {
      if (ws.readyState === WebSocket.CLOSED || ws.readyState === WebSocket.CLOSING) {
        // 连接建立前已关闭,忽略
      } else {
        Toast.error('连接错误，请重试')
      }
      setIsLoading(false)
    }

    ws.onclose = () => {
      setIsConnected(false)
    }
  }

  useEffect(() => {
    connectWebSocket()
    return () => {
      if (wsRef.current) {
        wsRef.current.close()
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeTab])

  const handleRefresh = () => connectWebSocket()
  const handleClear = () => setLogs([])

  const actions = (
    <>
      <Button icon={<IconSave />} onClick={() => (window.location.href = `/static/${activeTab}.log`)} type="primary" theme="solid" size="small">
        {isMobile ? null : '下载'}
      </Button>
      <Button icon={<IconRefresh />} onClick={handleRefresh} theme="light" size="small">
        {isMobile ? null : '刷新'}
      </Button>
      <Button icon={<IconClear />} onClick={handleClear} theme="light" size="small">
        {isMobile ? null : '清空'}
      </Button>
      <Typography.Text type={isConnected ? 'success' : 'danger'} style={{ marginLeft: 6 }}>
        {isConnected ? '● 已连接' : '○ 未连接'}
      </Typography.Text>
    </>
  )

  return (
    <>
      <PageHeader
        icon={<IconCustomerSupport size="large" />}
        title="实时日志"
        description="主程序运行与下载上传日志,WebSocket 实时推送"
        actions={actions}
      />
      <div className={dc.content}>
        <div className={dc.card} style={{ padding: 16 }}>
          <Tabs type="line" activeKey={activeTab} onChange={setActiveTab}>
            <TabPane tab="主程序运行日志" itemKey="ds_update">
              <LogContent logs={logs} logContainerRef={logContainerRef} isLoading={isLoading} />
            </TabPane>
            <TabPane tab="biliup下载和上传日志" itemKey="download">
              <LogContent logs={logs} logContainerRef={logContainerRef} isLoading={isLoading} />
            </TabPane>
          </Tabs>
        </div>
      </div>
    </>
  )
}
