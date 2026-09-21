'use client'
import { SetStateAction, useEffect, useState } from 'react'
import { Button } from '@douyinfe/semi-ui'
import { IconMoon, IconSun, IconContrast } from '@douyinfe/semi-icons'
import { applyThemeMode } from '../lib/utils'

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
      // 按下按钮切换主题：一键明暗反转，避免 auto→light→dark 循环的第一下视觉无变化。
      if (typeof window !== 'undefined' && switchTrigger === true) {
        const currentMode = props.mode
        const isDarkNow =
          currentMode === 'dark' ||
          (currentMode === 'auto' && props.systemTheme === 'dark')
        const nextMode = isDarkNow ? 'light' : 'dark'
        applyThemeMode(nextMode)
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
