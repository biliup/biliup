'use client'
import { Button } from '@douyinfe/semi-ui'
import { IconMoon, IconSun, IconContrast } from '@douyinfe/semi-icons'
import { applyThemeMode } from '../lib/utils'

interface ThemeButtonProps {
  mode: string
  setMode: (mode: string) => void
  systemTheme: string
}

const ThemeButton: React.FC<ThemeButtonProps> = ({ mode, setMode, systemTheme }) => {
  // 按下按钮切换主题:一键明暗反转,避免 auto→light→dark 循环的第一下视觉无变化。
  // 直接在事件里完成,不再经由「置标志位 → effect 里读标志位再 setState」绕一圈。
  const switchMode = () => {
    const isDarkNow = mode === 'dark' || (mode === 'auto' && systemTheme === 'dark')
    const nextMode = isDarkNow ? 'light' : 'dark'
    applyThemeMode(nextMode)
    setMode(nextMode)
  }

  // 图标由 mode 直接派生,不需要单独的 state
  const icon =
    mode === 'light' ? (
      <IconSun size="large" />
    ) : mode === 'dark' ? (
      <IconMoon size="large" />
    ) : (
      <IconContrast size="large" />
    )

  return (
    <Button
      onClick={switchMode}
      theme="borderless"
      icon={icon}
      style={{
        color: 'var(--semi-color-text-2)',
      }}
    />
  )
}

export default ThemeButton
