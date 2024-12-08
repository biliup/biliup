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
        const body = document.body
        const currentMode = props.mode
        let nextMode = currentMode
        switch (currentMode) {
          case 'auto':
            nextMode = 'light'
            break
          case 'light':
            nextMode = 'dark'
            break
          default:
            nextMode = 'auto'
            break
        }
        body.removeAttribute('theme-mode')
        nextMode === 'auto'
          ? body.setAttribute('theme-mode', props.systemTheme)
          : body.setAttribute('theme-mode', nextMode)
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
