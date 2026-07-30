'use client'
import styles from './page.module.css'
import './bg-global.css'
import { useGlobalBackgroundInit } from '../lib/useGlobalBackground'
import { useCallback, useMemo, useState, useEffect } from 'react'
import type { ReactNode } from 'react'
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
  IconUserCardVideo,
  IconBook,
  IconMenu,
} from '@douyinfe/semi-icons'
import Image from 'next/image'
import ThemeButton from '../ui/ThemeButton'
import { useSystemTheme, useTheme } from '../lib/utils'
import { useWindowSize } from 'react-use'

/**
 * 导航项强调色 —— 统一收口到一处，避免各页面各自硬编码颜色。
 * 仅保留少量语义色，符合"设计语言"而非"随意配色"。
 */
const NAV_ACCENT: Record<string, string> = {
  home: '#ffaa00',
  manager: '#5ac262',
  'upload-manager': '#885bd2',
  archives: '#00a1d6',
  dashboard: '#6b6c75',
  changelog: 'rgb(var(--semi-cyan-4))',
  job: 'rgb(250 102 76)',
  logViewer: 'rgb(var(--semi-blue-4))',
  status: 'rgba(var(--semi-lime-2), 1)',
}

function navIcon(accent: string, icon: ReactNode) {
  return (
    <div
      style={{
        backgroundColor: accent,
        borderRadius: 'var(--semi-border-radius-medium)',
        color: 'var(--semi-color-bg-0)',
        display: 'flex',
        padding: '4px',
      }}
    >
      {icon}
    </div>
  )
}

export default function Layout({ children }: { children: React.ReactNode }) {
  const { Sider } = SeLayout
  const pathname = usePathname()
  let initOpenKeys: any = []
  if (pathname.slice(1) === 'streamers' || pathname.slice(1) === 'history') {
    initOpenKeys = ['manager']
  }

  const [openKeys, setOpenKeys] = useState(initOpenKeys)
  const [selectedKeys, setSelectedKeys] = useState<any>([pathname.slice(1)])
  useGlobalBackgroundInit()

  const { width } = useWindowSize()
  const isMobile = width <= 640
  const [isCollapsed, setIsCollapsed] = useState(isMobile)
  const [mobileNavOpen, setMobileNavOpen] = useState(false)
  const [mode, setMode] = useState(
    (typeof window !== 'undefined' && localStorage.getItem('mode')) || 'auto'
  )
  const systemTheme = useSystemTheme()
  useTheme(mode, systemTheme)
  const navCollapsed = isMobile ? false : isCollapsed
  let navStyle = navCollapsed ? { height: '100%', overflow: 'visible' } : { height: '100%' }

  // 兼容 PC 切移动端
  useEffect(() => {
    if (width <= 640) {
      setIsCollapsed(true)
    }
  }, [width])

  const items = useMemo(
    () =>
      [
        {
          itemKey: 'home',
          text: '主页',
          icon: navIcon(NAV_ACCENT.home, <IconHome size="small" />),
        },
        {
          itemKey: 'manager',
          text: '录播管理',
          items: [
            { itemKey: 'streamers', text: '直播管理' },
            { itemKey: 'history', text: '历史记录' },
          ],
          icon: navIcon(NAV_ACCENT.manager, <IconVideoListStroked size="small" />),
        },
        {
          itemKey: 'upload-manager',
          text: '投稿管理',
          icon: navIcon(NAV_ACCENT['upload-manager'], <IconCloudStroked size="small" />),
        },
        {
          itemKey: 'archives',
          text: 'B站稿件',
          icon: navIcon(NAV_ACCENT.archives, <IconUserCardVideo size="small" />),
        },
        {
          itemKey: 'dashboard',
          text: '空间配置',
          icon: navIcon(NAV_ACCENT.dashboard, <IconStar size="small" />),
        },
        {
          itemKey: 'job',
          text: '直播历史',
          icon: navIcon(NAV_ACCENT.job, <IconHistory size="small" />),
        },
        {
          itemKey: 'logViewer',
          text: '实时日志',
          icon: navIcon(NAV_ACCENT.logViewer, <IconCustomerSupport size="small" />),
        },
        {
          itemKey: 'status',
          text: '任务平台',
          icon: navIcon(NAV_ACCENT.status, <IconSetting size="small" />),
        },
        {
          itemKey: 'changelog',
          text: '更新日志',
          icon: navIcon(NAV_ACCENT.changelog, <IconBook size="small" />),
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
      changelog: '/changelog',
      streamers: '/streamers',
      'upload-manager': '/upload-manager',
      archives: '/archives',
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
  }, [])

  const onSelect = (data: OnSelectedData) => {
    setSelectedKeys([...data.selectedKeys])
    if (isMobile) setMobileNavOpen(false)
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
        <SeLayout className="components-layout-demo semi-light-scrollbar">
          {isMobile && (
            <Button
              type="tertiary"
              theme="borderless"
              icon={<IconMenu />}
              onClick={() => setMobileNavOpen(true)}
              style={{
                position: 'fixed',
                top: 12,
                left: 12,
                zIndex: 1001,
                backgroundColor: 'var(--semi-color-bg-0)',
                boxShadow: 'var(--semi-shadow-elevated)',
              }}
            />
          )}
          {isMobile && mobileNavOpen && (
            <div
              onClick={() => setMobileNavOpen(false)}
              style={{
                position: 'fixed',
                inset: 0,
                backgroundColor: 'rgba(0,0,0,0.35)',
                zIndex: 999,
              }}
            />
          )}
          <Sider
            style={
              isMobile
                ? {
                    display: mobileNavOpen ? 'flex' : 'none',
                    position: 'fixed',
                    left: 0,
                    top: 0,
                    height: '100vh',
                    zIndex: 1000,
                  }
                : {}
            }
          >
            <Nav
              style={navStyle}
              openKeys={openKeys}
              selectedKeys={selectedKeys}
              isCollapsed={navCollapsed}
              renderWrapper={renderWrapper}
              items={items}
              onOpenChange={onOpenChange}
              onSelect={onSelect}
            >
              <Nav.Header
                logo={<Image src="/logo.png" alt="{}" height={10} width={20}></Image>}
                style={
                  navCollapsed
                    ? { flexDirection: 'column', paddingLeft: 0, paddingRight: 0, paddingBottom: 0, gap: '8px' }
                    : { justifyContent: 'flex-start' }
                }
                text="BILIUP"
              >
                <div
                  style={{
                    flexGrow: 1,
                    display: isMobile ? 'none' : 'flex',
                    flexDirection: 'row-reverse',
                    zIndex: 2,
                  }}
                >
                  <Button
                    onClick={onCollapseChange}
                    type="tertiary"
                    className={styles.shadow}
                    theme="borderless"
                    icon={isCollapsed ? <IconDoubleChevronRight /> : <IconDoubleChevronLeft />}
                  />
                </div>
              </Nav.Header>
              <Nav.Footer collapseButton={false}>
                <ThemeButton mode={mode} setMode={setMode} systemTheme={systemTheme} />
              </Nav.Footer>
            </Nav>
          </Sider>
          <SeLayout style={{ height: '100vh' }}>{children}</SeLayout>
        </SeLayout>
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
