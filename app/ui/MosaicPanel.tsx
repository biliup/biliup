'use client'
import React from 'react'
import { Collapse, Switch, Typography, withField } from '@douyinfe/semi-ui'
import type { LiveStreamerEntity, MosaicConfig } from '@/app/lib/api-streamer'
import { EMPTY_MOSAIC_CONFIG, isMosaicConfig, validateMosaicConfig } from '@/app/lib/mosaic-config'
import { MosaicEditor } from './MosaicEditor'

interface MosaicPanelProps {
  entity?: LiveStreamerEntity
  initValues?: Record<string, unknown>
  onChange?: (config: MosaicConfig) => void
}

interface MosaicInputProps {
  value?: MosaicConfig | null
  onChange?: (config: MosaicConfig) => void
}

/** The form owns the configuration; changing a switch or a region commits an object directly. */
function MosaicInput({ value, onChange }: MosaicInputProps) {
  if (value != null && !isMosaicConfig(value)) {
    return <Typography.Text type="danger">画面遮挡配置格式不正确，请在上方配置 JSON 中修正。</Typography.Text>
  }
  const config = value ?? EMPTY_MOSAIC_CONFIG
  return (
    <>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 16 }}>
        <Switch
          checked={config.enabled}
          onChange={enabled => onChange?.({ ...config, enabled })}
          aria-label="启用画面遮挡"
        />
        <span>启用画面遮挡功能</span>
      </div>
      {config.enabled && (
        <div style={{ padding: 12, borderRadius: 4, marginBottom: 16, background: 'var(--semi-color-warning-light-default)' }}>
          分段录制完成后使用 FFmpeg 处理遮挡，再进入上传流程。启用后会增加 CPU 负载和处理时间，需安装 FFmpeg。
        </div>
      )}
      <MosaicEditor config={config} onChange={next => onChange?.(next)} />
      {value == null && (
        <Typography.Text type="tertiary" size="small">尚未设置主播遮挡覆写；如有全局遮挡配置，将继续跟随全局设置。</Typography.Text>
      )}
    </>
  )
}

const MosaicField = withField(MosaicInput)

/** Keep the field registered when the collapse panel is closed, so saving still validates it. */
export function MosaicPanel({ initValues, onChange }: MosaicPanelProps) {
  return (
    <Collapse.Panel header="画面遮挡（实验性）" itemKey="mosaic">
      <MosaicField
        field="mosaic_config"
        initValue={initValues?.mosaic_config}
        noLabel
        validator={validateMosaicConfig}
        onChange={onChange}
      />
    </Collapse.Panel>
  )
}
