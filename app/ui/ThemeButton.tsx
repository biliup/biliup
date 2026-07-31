'use client'
import { SetStateAction, useEffect, useState } from 'react'
import { Button } from '@douyinfe/semi-ui'
import { IconMoon, IconSun, IconContrast } from '@douyinfe/semi-icons'

interface ThemeButtonProps {
  mode: string
  setMode: {
    (value: SetStateAction<string>): void
    (arg0: string): void
  }
  systemTheme: string
}

const ThemeButton: React.FC<ThemeButtonProps> = props => {
  const [switchTrigger, setSwitchTrigger] = useState(false)
  const [icon, setIcon] = useState(<IconContrast size="large" />)
  useEffect(() => {
    {
      // 按下按钮切换主题
      if (typeof window !== 'undefined' && switchTrigger === true) {
        const root = document.documentElement
        const currentMode = props.mode
        // 一键明暗反转：避免原 auto→light→dark 三态循环导致「白天点一下视觉无变化」的错觉。
        // 当前为 dark，或当前为 auto 且系统为暗 → 切到 light；其余（auto/light）→ 切到 dark。
        const isDarkNow =
          currentMode === 'dark' ||
          (currentMode === 'auto' && props.systemTheme === 'dark')
        const nextMode = isDarkNow ? 'light' : 'dark'
        root.setAttribute('theme-mode', nextMode)
        props.setMode(nextMode)
        setSwitchTrigger(false)
      }
      // 更新图标
      switch (props.mode) {
        case 'light':
          setIcon(<IconSun size="large" />)
          break
        case 'dark':
          setIcon(<IconMoon size="large" />)
          break
        default:
          setIcon(<IconContrast size="large" />)
          break
      }
    }
  }, [props, switchTrigger])

  const switchMode = () => {
    setSwitchTrigger(true)
  }

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
