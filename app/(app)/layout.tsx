'use client'
import './bg-global.css'
import { useGlobalBackgroundInit } from '../lib/useGlobalBackground'
import { useEffect, useRef, useState } from 'react'
import type { ReactNode } from 'react'
import Link from 'next/link'
import { usePathname } from 'next/navigation'
import Image from 'next/image'
import useSWR from 'swr'
import { API_BASE, fetcher } from '../lib/api-streamer'
import { Dropdown, Empty, Spin } from '@douyinfe/semi-ui'
import { ROLE_LABELS, useMe } from '../lib/use-me'
import type { Permission } from '../lib/use-me'
import ChangePasswordModal from '../ui/ChangePasswordModal'
import ThemeButton from '../ui/ThemeButton'
import { useLocalStorageValue, useSystemTheme, useTheme } from '../lib/utils'
import { formatVersion } from '../lib/status'
import { SLOW_REFRESH_MS } from '../lib/use-dashboard'
import { useIsMobile } from '../lib/useIsMobile'
import styles from './layout.module.scss'

/* ============ 导航信息架构:5 组 10 项,按当前角色的权限点过滤 ============ */

function Ic({ d, extra }: { d: string; extra?: string }) {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d={d} />
      {extra ? <path d={extra} /> : null}
    </svg>
  )
}

/**
 * `activeOn`：从这一项进去的子页面（不在侧栏里），停在这些页面时这一项也高亮
 */
type NavItem = {
  href: string
  label: string
  icon: ReactNode
  perm: Permission
  activeOn?: string[]
}

const NAV_GROUPS: { title: string; items: NavItem[] }[] = [
  {
    title: '总览',
    items: [
      {
        href: '/',
        perm: 'streamer.view',
        label: '控制台',
        icon: <Ic d="M4 4h6v6H4zM14 4h6v6h-6zM4 14h6v6H4zM14 14h6v6h-6z" />,
      },
    ],
  },
  {
    title: '录制',
    items: [
      {
        href: '/streamers',
        perm: 'streamer.view',
        label: '直播管理',
        icon: <Ic d="M3 6h13v12H3zM16 9.5l5-2.5v10l-5-2.5" />,
      },
      {
        href: '/workbench',
        perm: 'file.view',
        label: '剪辑台',
        activeOn: ['/replay'],
        icon: <Ic d="M6 9a3 3 0 100-6 3 3 0 000 6zM6 21a3 3 0 100-6 3 3 0 000 6z" extra="M20 4L8.1 15.9M14.5 14.5L20 20M8.1 8.1L12 12" />,
      },
      {
        href: '/renders',
        perm: 'file.view',
        label: '压制任务',
        icon: <Ic d="M4 5h16v14H4z" extra="M8 9h8M8 13h5M8 16h3" />,
      },
    ],
  },
  {
    title: '投稿',
    items: [
      {
        href: '/upload-manager',
        perm: 'streamer.view',
        label: '投稿管理',
        icon: <Ic d="M12 16V4m0 0L8 8m4-4l4 4" extra="M4 17v2a1 1 0 001 1h14a1 1 0 001-1v-2" />,
      },
      {
        href: '/upload-queue',
        perm: 'file.view',
        label: '上传队列',
        icon: <Ic d="M4 6h12M4 12h12M4 18h8" extra="M19 12v8m0-8l-3 3m3-3l3 3" />,
      },
      {
        href: '/archives',
        perm: 'account.manage',
        label: 'B站稿件',
        icon: <Ic d="M4 5h16v14H4z" extra="M8 15c0-2 1.5-3 4-3s4 1 4 3M12 8a2 2 0 100 4 2 2 0 000-4z" />,
      },
    ],
  },
  {
    title: '设置',
    items: [
      {
        href: '/dashboard',
        perm: 'config.view',
        label: '空间配置',
        icon: <Ic d="M12 15a3 3 0 100-6 3 3 0 000 6z" extra="M19.4 15a1.7 1.7 0 00.3 1.9l.1.1a2 2 0 11-2.8 2.8l-.1-.1a1.7 1.7 0 00-1.9-.3 1.7 1.7 0 00-1 1.5V21a2 2 0 11-4 0v-.2a1.7 1.7 0 00-1-1.5 1.7 1.7 0 00-1.9.3l-.1.1a2 2 0 11-2.8-2.8l.1-.1a1.7 1.7 0 00.3-1.9 1.7 1.7 0 00-1.5-1H3a2 2 0 110-4h.2a1.7 1.7 0 001.5-1 1.7 1.7 0 00-.3-1.9l-.1-.1a2 2 0 112.8-2.8l.1-.1a1.7 1.7 0 001.9.3h0a1.7 1.7 0 001-1.5V3a2 2 0 114 0v.2a1.7 1.7 0 001 1.5h0a1.7 1.7 0 001.9-.3l.1-.1a2 2 0 112.8 2.8l-.1-.1a1.7 1.7 0 00-.3 1.9v0a1.7 1.7 0 001.5 1H21a2 2 0 110 4h-.2a1.7 1.7 0 00-1.5 1z" />,
      },
    ],
  },
  {
    title: '系统',
    items: [
      {
        href: '/logviewer',
        perm: 'log.view',
        label: '实时日志',
        icon: <Ic d="M4 5h16M4 12h16M4 19h10" extra="M18 15l3 3-3 3" />,
      },
      {
        href: '/status',
        perm: 'streamer.view',
        label: '任务平台',
        activeOn: ['/nodes'],
        icon: <Ic d="M4 4h16v16H4z" extra="M4 9h16M9 4v5" />,
      },
      {
        href: '/users',
        perm: 'user.manage',
        label: '用户管理',
        icon: (
          <Ic
            d="M16 19v-1a4 4 0 00-4-4H7a4 4 0 00-4 4v1M9.5 10a3.5 3.5 0 100-7 3.5 3.5 0 000 7z"
            extra="M21 19v-1a4 4 0 00-3-3.9M16 3.1a3.5 3.5 0 010 6.8"
          />
        ),
      },
    ],
  },
]

/** 不在侧栏里、但同样需要权限的子页面（按前缀匹配，先匹配到的生效） */
const EXTRA_PAGE_PERMS: { prefix: string; perm: Permission }[] = [
  { prefix: '/nodes', perm: 'streamer.view' },
  { prefix: '/live', perm: 'preview.view' },
  { prefix: '/upload-manager/add', perm: 'template.edit' },
  { prefix: '/upload-manager/edit', perm: 'template.edit' },
  { prefix: '/replay', perm: 'file.view' },
]

const matches = (pathname: string, href: string) =>
  href === '/' ? pathname === '/' : pathname === href || pathname.startsWith(href + '/')

function pagePermission(pathname: string): Permission | undefined {
  const extra = EXTRA_PAGE_PERMS.find((p) => matches(pathname, p.prefix))
  if (extra) return extra.perm
  for (const group of NAV_GROUPS) {
    for (const item of group.items) {
      if (matches(pathname, item.href)) return item.perm
    }
  }
  return undefined
}

function NoAccess({
  title = '没有权限访问此页面',
  description = '当前账号的角色不包含这项功能，如需使用请联系超级管理员。',
}: {
  title?: string
  description?: string
}) {
  return (
    <div style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center', padding: 24 }}>
      <Empty
        image={
          <svg viewBox="0 0 24 24" width="64" height="64" fill="none" stroke="var(--semi-color-text-3)" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
            <rect x="5" y="11" width="14" height="10" rx="2" />
            <path d="M8 11V7a4 4 0 118 0v4M12 15v2" />
          </svg>
        }
        title={title}
        description={description}
      />
    </div>
  )
}

function initialOf(name: string) {
  return Array.from(name.trim())[0]?.toUpperCase() ?? '?'
}

/* 服务状态:布局级轻量轮询,所有页面共享 */
const SIDER_KEY = 'biliup_sider_collapsed'

export default function Layout({ children }: { children: React.ReactNode }) {
  const pathname = usePathname()
  const isMobile = useIsMobile()
  const [mobileNavOpen, setMobileNavOpen] = useState(false)
  const [passwordOpen, setPasswordOpen] = useState(false)
  const [accountMenuOpen, setAccountMenuOpen] = useState(false)
  const asideRef = useRef<HTMLElement>(null)
  const burgerRef = useRef<HTMLButtonElement>(null)
  const wasMobileNavOpen = useRef(false)
  const previousMobileBreakpoint = useRef<boolean | null>(null)
  const accountMenuEscapeHandled = useRef(false)

  useEffect(() => {
    if (isMobile && asideRef.current?.contains(document.activeElement)) {
      burgerRef.current?.focus()
    }
  }, [isMobile])

  useEffect(() => {
    if (!passwordOpen) return
    const frame = window.requestAnimationFrame(() => {
      document.querySelector<HTMLInputElement>('input[autocomplete="current-password"]')?.focus()
    })
    return () => window.cancelAnimationFrame(frame)
  }, [passwordOpen])

  useEffect(() => {
    const syncResponsiveState = () => {
      const mobileWidth = window.innerWidth <= 640
      if (previousMobileBreakpoint.current !== null && previousMobileBreakpoint.current !== mobileWidth) {
        setMobileNavOpen(false)
        setAccountMenuOpen(false)
      }
      previousMobileBreakpoint.current = mobileWidth
    }
    syncResponsiveState()
    window.addEventListener('resize', syncResponsiveState)
    return () => window.removeEventListener('resize', syncResponsiveState)
  }, [])

  useEffect(() => {
    if (!isMobile) {
      wasMobileNavOpen.current = false
      return
    }
    if (mobileNavOpen && !wasMobileNavOpen.current) {
      const firstNavItem = asideRef.current?.querySelector<HTMLElement>('nav a[href]')
      const initialFocusTarget = firstNavItem ?? asideRef.current
      initialFocusTarget?.focus()
    } else if (!mobileNavOpen && wasMobileNavOpen.current && !passwordOpen) {
      burgerRef.current?.focus()
    }
    wasMobileNavOpen.current = mobileNavOpen
  }, [isMobile, mobileNavOpen, passwordOpen])

  useEffect(() => {
    if (!isMobile || !mobileNavOpen) return
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        // Dropdown/Modal portals handle Escape first; leave their overlay open
        // state intact instead of closing the drawer underneath them.
        if (accountMenuEscapeHandled.current) {
          accountMenuEscapeHandled.current = false
          return
        }
        if (
          event.defaultPrevented ||
          event.cancelBubble ||
          (event.target instanceof Element && event.target.closest('[role="menu"]'))
        ) {
          return
        }
        event.preventDefault()
        setMobileNavOpen(false)
        return
      }
      if (event.key !== 'Tab') return

      const aside = asideRef.current
      if (!aside) return
      // Semi Dropdown renders its menu in a body portal. The trigger's
      // aria-describedby relation identifies that popup for the focus order.
      const dropdownTrigger = aside.querySelector<HTMLElement>('button[aria-haspopup="true"][aria-describedby]')
      const popupId = dropdownTrigger?.getAttribute('aria-describedby')
      const popup = popupId ? document.getElementById(popupId) : null
      const popupOpen = !!popup && popup.getClientRects().length > 0
      const popupItems = popupOpen && popup
        ? Array.from(popup.querySelectorAll<HTMLElement>('[role="menuitem"]')).filter(
            (element) => element.getClientRects().length > 0,
          )
        : []
      const focusable = [
        ...(burgerRef.current ? [burgerRef.current] : []),
        ...Array.from(
          aside.querySelectorAll<HTMLElement>(
            'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])',
          ),
        ),
      ]
      const triggerIndex = popupOpen && dropdownTrigger ? focusable.indexOf(dropdownTrigger) : -1
      if (triggerIndex >= 0) focusable.splice(triggerIndex, 1, ...popupItems)
      else if (popupOpen) focusable.push(...popupItems)
      const visibleFocusable = focusable.filter(
        (element) => element.getAttribute('aria-hidden') !== 'true' && element.getClientRects().length > 0,
      )
      if (!visibleFocusable.length) {
        event.preventDefault()
        aside.focus()
        return
      }

      const activeIndex = visibleFocusable.indexOf(document.activeElement as HTMLElement)
      const outsideDialog =
        !aside.contains(document.activeElement) &&
        !popup?.contains(document.activeElement) &&
        document.activeElement !== burgerRef.current
      if (popupOpen && popup?.contains(document.activeElement) && activeIndex >= 0) {
        const nextIndex = activeIndex + (event.shiftKey ? -1 : 1)
        if (nextIndex >= 0 && nextIndex < visibleFocusable.length) {
          event.preventDefault()
          visibleFocusable[nextIndex].focus()
          return
        }
      }
      if (event.shiftKey && (activeIndex === 0 || outsideDialog)) {
        event.preventDefault()
        visibleFocusable[visibleFocusable.length - 1].focus()
      } else if (!event.shiftKey && (activeIndex === visibleFocusable.length - 1 || outsideDialog)) {
        event.preventDefault()
        visibleFocusable[0].focus()
      }
    }
    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [isMobile, mobileNavOpen])

  useGlobalBackgroundInit()

  // 折叠偏好持久化:localStorage 是数据源,首屏(SSR / 水合)按未折叠渲染,水合后自动切到已保存的值
  const [siderPref, setSiderPref] = useLocalStorageValue(SIDER_KEY)
  const collapsed = siderPref === '1'
  const toggleCollapsed = () => setSiderPref(collapsed ? '0' : '1')

  // 主题:同上,未保存过时为 auto
  const [savedMode, setMode] = useLocalStorageValue('mode')
  const mode = savedMode ?? 'auto'
  const systemTheme = useSystemTheme()
  useTheme(mode, systemTheme)

  // 服务状态指示(离线时变红,不影响页面)
  const { data: status, error: statusError } = useSWR('/v1/status', fetcher, {
    refreshInterval: SLOW_REFRESH_MS,
    revalidateOnFocus: false,
  })
  const online = status !== undefined && !statusError
  const version = (status as { version?: string } | undefined)?.version
  const versionText = formatVersion(version)

  const navCollapsed = !isMobile && collapsed

  const isActive = (item: NavItem) =>
    matches(pathname, item.href) || !!item.activeOn?.some((href) => matches(pathname, href))

  // 权限未加载完时整组菜单先不过滤（全部都是公开的静态页面，数据接口本身有后端拦截），
  // 避免每次进页面侧栏闪一下
  const { me, error: meError, can } = useMe()
  const visibleGroups = me
    ? NAV_GROUPS.map((group) => ({
        ...group,
        items: group.items.filter(item => can(item.perm)),
      })).filter((group) => group.items.length > 0)
    : NAV_GROUPS.map((group) => ({
        ...group,
        items: group.items.filter((item) => item.perm !== 'user.manage'),
      }))
  const requiredPerm = pagePermission(pathname)
  const denied = !!me && !!requiredPerm && !can(requiredPerm)
  // 需要权限的页面一律等权限加载完再渲染：前端不假设哪个权限点人人都有，
  // 免得没有权限的人先发出一串注定 403 的请求。/v1/me 有缓存，只有首次进入会等
  const pending = !me && !meError && !!requiredPerm

  const openPasswordDialog = () => {
    if (isMobile) setMobileNavOpen(false)
    setPasswordOpen(true)
  }
  const logout = async () => {
    try {
      await fetch(`${API_BASE}/v1/logout`, { method: 'POST', credentials: 'include' })
    } finally {
      window.location.assign('/login')
    }
  }
  const showUser = me?.auth_enabled === true
  const username = me?.username ?? ''
  const roleLabel = me ? ROLE_LABELS[me.role] : ''

  return (
    <div className={styles.app}>
      {/* 移动端汉堡按钮兼作抽屉关闭按钮，遮罩点击也可关闭 */}
      {isMobile && (
        <button
          ref={burgerRef}
          type="button"
          className={`${styles.burger} ${mobileNavOpen ? styles.burgerOpen : ''}`}
          onClick={() => setMobileNavOpen((open) => !open)}
          aria-label={mobileNavOpen ? '关闭导航' : '打开导航'}
          aria-expanded={mobileNavOpen}
          aria-controls="app-mobile-navigation"
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
            {mobileNavOpen ? <path d="M6 6l12 12M18 6L6 18" /> : <path d="M4 7h16M4 12h16M4 17h16" />}
          </svg>
        </button>
      )}
      {isMobile && mobileNavOpen && (
        <div className={styles.overlay} onClick={() => setMobileNavOpen(false)} />
      )}

      <aside
        ref={asideRef}
        id="app-mobile-navigation"
        className={`${styles.sider} ${navCollapsed ? styles.collapsed : ''} ${
          isMobile ? styles.siderMobile : ''
        } ${mobileNavOpen ? styles.open : ''}`}
        role={isMobile && mobileNavOpen ? 'dialog' : undefined}
        aria-label={isMobile && mobileNavOpen ? '主导航' : undefined}
        aria-modal={isMobile && mobileNavOpen && !accountMenuOpen ? true : undefined}
        aria-hidden={isMobile && !mobileNavOpen ? true : undefined}
        inert={isMobile && !mobileNavOpen}
        tabIndex={isMobile && mobileNavOpen ? -1 : undefined}
      >
        {/* 品牌区 */}
        <div className={styles.brand}>
          <Link
            href="/"
            prefetch={false}
            onClick={() => isMobile && setMobileNavOpen(false)}
            aria-label="回到控制台"
          >
            <Image
              src="/logo.png"
              alt="biliup"
              width={30}
              height={30}
              style={{ width: 30, height: 30, objectFit: 'contain' }}
              unoptimized
            />
          </Link>
          {!navCollapsed && (
            <span className={styles.brandText}>
              <Link
                href="/"
                prefetch={false}
                className={styles.brandName}
                onClick={() => isMobile && setMobileNavOpen(false)}
              >
                biliup
              </Link>
              <Link href="/changelog" prefetch={false} className={styles.brandVer} title="更新日志">
                v{versionText ?? '—'}
              </Link>
            </span>
          )}
        </div>

        {/* 导航 */}
        <nav className={styles.nav}>
          {visibleGroups.map((group) => (
            <div key={group.title} className={styles.group}>
              {!navCollapsed && <div className={styles.groupTitle}>{group.title}</div>}
              {group.items.map((item) => {
                const active = isActive(item)
                return (
                  <Link
                    key={item.href}
                    href={item.href}
                    prefetch={false}
                    className={`${styles.item} ${active ? styles.itemActive : ''}`}
                    title={navCollapsed ? item.label : undefined}
                    onClick={() => isMobile && setMobileNavOpen(false)}
                  >
                    <span className={styles.itemIcon}>{item.icon}</span>
                    {!navCollapsed && <span className={styles.itemText}>{item.label}</span>}
                  </Link>
                )
              })}
            </div>
          ))}
        </nav>

        {/* 底部:当前用户 + 服务状态 + 工具 */}
        <div className={styles.foot}>
          {showUser && (
            <Dropdown
              trigger="click"
              clickToHide
              visible={accountMenuOpen}
              onVisibleChange={setAccountMenuOpen}
              onEscKeyDown={() => {
                accountMenuEscapeHandled.current = true
                window.setTimeout(() => {
                  accountMenuEscapeHandled.current = false
                }, 0)
              }}
              position={navCollapsed ? 'rightBottom' : 'topLeft'}
              render={
                <Dropdown.Menu>
                  <Dropdown.Title>
                    {username} · {roleLabel}
                  </Dropdown.Title>
                  <Dropdown.Item onClick={openPasswordDialog}>修改密码</Dropdown.Item>
                  <Dropdown.Divider />
                  <Dropdown.Item type="danger" onClick={logout}>
                    退出登录
                  </Dropdown.Item>
                </Dropdown.Menu>
              }
            >
              <button
                type="button"
                className={`${styles.userBtn} ${navCollapsed ? styles.userBtnCollapsed : ''}`}
                aria-label={`当前用户 ${username}，打开账户菜单`}
                title={navCollapsed ? `${username} · ${roleLabel}` : undefined}
              >
                <span className={styles.userAvatar} aria-hidden="true">
                  {initialOf(username)}
                </span>
                {!navCollapsed && (
                  <span className={styles.userText}>
                    <span className={styles.userName}>{username}</span>
                    <span className={styles.userRole}>{roleLabel}</span>
                  </span>
                )}
              </button>
            </Dropdown>
          )}
          {!navCollapsed && (
            <div className={styles.statusRow}>
              <span className={`${styles.statusDot} ${online ? styles.online : styles.offline}`} />
              <span className={styles.statusText}>{online ? '服务运行中' : '服务未连接'}</span>
            </div>
          )}
          <div className={styles.footBtns}>
            <ThemeButton mode={mode} setMode={setMode} systemTheme={systemTheme} />
            {!isMobile && (
              <button
                className={styles.footBtn}
                onClick={toggleCollapsed}
                aria-label={navCollapsed ? '展开侧栏' : '收起侧栏'}
                title={navCollapsed ? '展开侧栏' : '收起侧栏'}
              >
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  {navCollapsed ? <path d="M9 6l6 6-6 6" /> : <path d="M15 6l-6 6 6 6" />}
                </svg>
              </button>
            )}
          </div>
        </div>
      </aside>

      <main className={styles.main} inert={isMobile && mobileNavOpen} aria-hidden={isMobile && mobileNavOpen ? true : undefined}>
        {pending ? (
          <div style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
            <Spin size="large" />
          </div>
        ) : denied ? (
          requiredPerm === 'user.manage' && me && !me.auth_enabled ? (
            <NoAccess
              title="未开启登录认证"
              description="当前以零鉴权模式运行，没有登录用户可管理。使用 biliup server --auth 启动后即可在这里添加用户、分配角色。"
            />
          ) : (
            <NoAccess />
          )
        ) : (
          children
        )}
      </main>
      {showUser && (
        <ChangePasswordModal
          visible={passwordOpen}
          onClose={() => setPasswordOpen(false)}
        />
      )}
    </div>
  )
}
