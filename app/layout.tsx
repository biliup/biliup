'use client'
import './globals.css'
import styles from './page.module.css'
import { useCallback, useMemo, useState, useEffect } from 'react'
import Link from 'next/link'
import { usePathname } from 'next/navigation'
import { Button, Nav } from '@douyinfe/semi-ui'
import { OnSelectedData } from '@douyinfe/semi-ui/lib/es/navigation'
import { Layout as SeLayout } from '@douyinfe/semi-ui/lib/es/layout'
import {
  IconCloudStroked,
  IconCustomerSupport,
  IconDoubleChevronLeft,
  IconDoubleChevronRight,
  IconStar,
  IconVideoListStroked,
  IconHome,
  IconSetting,
  IconHistory,
} from '@douyinfe/semi-icons'
import Image from 'next/image'
import ThemeButton from './ui/ThemeButton'
import { useSystemTheme, useTheme } from './lib/utils'
import { useWindowSize } from 'react-use';
import { AuthProvider } from './lib/auth-context';

export default function RootLayout({ children }: { children: React.ReactNode }) {
  const { Sider } = SeLayout
  const pathname = usePathname()
  let initOpenKeys: any = []
  if (pathname.slice(1) === 'streamers' || pathname.slice(1) === 'history') {
    initOpenKeys = ['manager']
  }

  const [openKeys, setOpenKeys] = useState(initOpenKeys)
  const [selectedKeys, setSelectedKeys] = useState<any>([pathname.slice(1)])

  const { width } = useWindowSize()
  const [isCollapsed, setIsCollapsed] = useState(width <= 640)
  const [mode, setMode] = useState(
    (typeof window !== 'undefined' && localStorage.getItem('mode')) || 'auto'
  )
  const systemTheme = useSystemTheme()
  useTheme(mode, systemTheme)
  let navStyle = isCollapsed ? { height: '100%', overflow: 'visible' } : { height: '100%' }

  // 兼容 PC 切移动端
  useEffect(() => {
    if (width <= 640) {
      setIsCollapsed(true)
    }
  }, [width]);

  const items = useMemo(
    () =>
      [
        {
          itemKey: 'home',
          text: '主页',
          icon: (
            <div
              style={{
                backgroundColor: '#ffaa00ff',
                borderRadius: 'var(--semi-border-radius-medium)',
                color: 'var(--semi-color-bg-0)',
                display: 'flex',
                // justifyContent: 'center',
                padding: '4px',
              }}
            >
              <IconHome size="small" />
            </div>
          ),
        },
        {
          itemKey: 'manager',
          text: '录播管理',
          items: [
            { itemKey: 'streamers', text: '直播管理' },
            { itemKey: 'history', text: '历史记录' },
          ],
          icon: (
            <div
              style={{
                backgroundColor: '#5ac262ff',
                borderRadius: 'var(--semi-border-radius-medium)',
                color: 'var(--semi-color-bg-0)',
                display: 'flex',
                // justifyContent: 'center',
                padding: '4px',
              }}
            >
              <IconVideoListStroked size="small" />
            </div>
          ),
        },
        {
          itemKey: 'upload-manager',
          text: '投稿管理',
          icon: (
            <div
              style={{
                backgroundColor: '#885bd2ff',
                borderRadius: 'var(--semi-border-radius-medium)',
                color: 'var(--semi-color-bg-0)',
                display: 'flex',
                padding: '4px',
              }}
            >
              <IconCloudStroked size="small" />
            </div>
          ),
        },
        {
          itemKey: 'dashboard',
          text: '空间配置',
          icon: (
            <div
              style={{
                backgroundColor: '#6b6c75ff',
                borderRadius: 'var(--semi-border-radius-medium)',
                color: 'var(--semi-color-bg-0)',
                display: 'flex',
                padding: '4px',
              }}
            >
              <IconStar size="small" />
            </div>
          ),
        },
        {
          itemKey: 'job',
          text: '直播历史',
          icon: (
            <div
              style={{
                backgroundColor: 'rgb(250 102 76)',
                borderRadius: 'var(--semi-border-radius-medium)',
                color: 'var(--semi-color-bg-0)',
                display: 'flex',
                padding: '4px',
              }}
            >
              <IconHistory size="small" />
            </div>
          ),
        },
        {
          text: '实时日志',
          icon: (
            <div
              style={{
                backgroundColor: 'rgba(var(--semi-blue-4), 1)',
                borderRadius: 'var(--semi-border-radius-medium)',
                color: 'var(--semi-color-bg-0)',
                display: 'flex',
                padding: '4px',
              }}
            >
              <IconCustomerSupport size="small" />
            </div>
          ),
          itemKey: 'logViewer',
        },
        {
          text: '任务平台',
          icon: (
            <div
              style={{
                backgroundColor: 'rgba(var(--semi-lime-2), 1)',
                borderRadius: 'var(--semi-border-radius-medium)',
                color: 'var(--semi-color-bg-0)',
                display: 'flex',
                padding: '4px',
              }}
            >
              <IconSetting size="small" />
            </div>
          ),
          itemKey: 'status',
          // items: [{itemKey: 'About', text: '任务管理'}, {itemKey: 'Dashboard', text: '用户任务查询'}],
        },
      ].map((value: any) => {
        value.text = (
          <div
            style={{
              color:
                selectedKeys.some((key: string) => value.itemKey === key) ||
                (selectedKeys.some((key: string) =>
                  openKeys.some((o: string | number) => isSub(key, o))
                ) &&
                  openKeys.some((key: any) => value.itemKey === key))
                  ? 'var(--semi-color-text-0)'
                  : 'var(--semi-color-text-2)',
              fontWeight: 600,
            }}
          >
            {value.text}
          </div>
        )
        return value
      }),
    [openKeys, selectedKeys]
  )
  const renderWrapper = useCallback(({ itemElement, isSubNav, isInSubNav, props }: any) => {
    const routerMap: Record<string, string> = {
      home: '/',
      history: '/history',
      dashboard: '/dashboard',
      streamers: '/streamers',
      'upload-manager': '/upload-manager',
      job: '/job',
      status: '/status',
      logViewer: '/logviewer',
    }
    if (!routerMap[props.itemKey]) {
      return itemElement
    }
    return (
      <Link
        style={{
          textDecoration: 'none',
          fontWeight: '600 !important',
        }}
        href={routerMap[props.itemKey]}
      >
        {itemElement}
      </Link>
    )
    // return itemElement;
  }, [])

  const onSelect = (data: OnSelectedData) => {
    setSelectedKeys([...data.selectedKeys])
  }
  const onOpenChange = (data: any) => {
    setOpenKeys([...data.openKeys])
  }
  const onCollapseChange = useCallback(() => {
    setIsCollapsed(!isCollapsed)
  }, [isCollapsed])
  return (
    <html lang="zh-Hans">
      <body style={{ width: '100%' }}>
        <AuthProvider>
          <SeLayout className="components-layout-demo semi-light-scrollbar">
            <SeLayout style={{ height: '100vh' }}>
              {children}
            </SeLayout>
          </SeLayout>
        </AuthProvider>
      </body>
    </html>
  )
}

function isSub(key1: string, key2: string | number) {
  const routerMap: any = {
    manager: ['streamers', 'history'],
  }
  return routerMap[key2]?.includes(key1)
}
