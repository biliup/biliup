'use client'

import React, { useState } from 'react'
import { Input, Select, Typography, withField } from '@douyinfe/semi-ui'
import type { CommonFieldProps } from '@douyinfe/semi-ui/lib/es/form/interface'
import {
  SEGMENT_TIME_PRESETS,
  segmentTimeMode,
  segmentTimeSummary,
  validateSegmentTime,
  type SegmentTimeScope,
  type SegmentTimeValue,
} from '../lib/segment-time'

type InputProps = {
  value?: SegmentTimeValue
  onChange?: (value: SegmentTimeValue) => void
  onBlur?: (event: React.FocusEvent) => void
  disabled?: boolean
  id?: string
  scope?: SegmentTimeScope
  validateStatus?: 'default' | 'error' | 'warning' | 'success'
}

/** Presets and an editable duration share a single segment_time form field. */
export const SegmentTimeInput: React.FC<InputProps> = ({
  value, onChange, onBlur, disabled, id, scope = 'global', validateStatus,
}) => {
  // A custom value may happen to equal a preset. Retain the user's editing mode until an external change.
  const [customDraft, setCustomDraft] = useState<{ value: SegmentTimeValue } | null>(null)
  const mode = customDraft && customDraft.value === value ? 'custom' : segmentTimeMode(value, scope)
  const options = [
    ...(scope === 'room' ? [{ value: 'inherit', label: '跟随全局设置' }] : []),
    { value: 'off', label: '不按时长分段' },
    ...SEGMENT_TIME_PRESETS,
    { value: 'custom', label: '自定义时长…' },
  ]
  const summary = segmentTimeSummary(value)

  return (
    <div style={{ display: 'flex', flexWrap: 'wrap', gap: 8, width: '100%', alignItems: 'center' }}>
      <Select
        id={mode === 'custom' ? undefined : id}
        aria-label="录制分段时长"
        value={mode}
        disabled={disabled}
        optionList={options}
        onBlur={onBlur}
        style={{ flex: '1 1 180px', minWidth: 0 }}
        onChange={next => {
          if (next === 'custom') {
            setCustomDraft({ value })
            return
          }
          setCustomDraft(null)
          onChange?.(next === 'inherit' ? undefined : next === 'off' ? null : String(next))
        }}
      />
      {mode === 'custom' ? (
        <Input
          id={id}
          aria-label="自定义分段时长"
          value={typeof value === 'string' ? value : ''}
          placeholder="7:30 或 00:07:30 或 450"
          disabled={disabled}
          validateStatus={validateStatus}
          showClear
          onBlur={onBlur}
          onChange={text => {
            const next = text.trim() === '' ? null : text
            setCustomDraft({ value: next })
            onChange?.(next)
          }}
          style={{ flex: '2 1 230px', minWidth: 0 }}
        />
      ) : null}
      {summary ? <Typography.Text type="tertiary" size="small">每 {summary} 分一段</Typography.Text> : null}
    </div>
  )
}

const WrappedSegmentTimeField = withField(SegmentTimeInput)
type FieldProps = Omit<InputProps, 'value' | 'validateStatus'> & CommonFieldProps

/** The built-in validator also runs on save, so an invalid custom input cannot be persisted. */
export const SegmentTimeField: React.FC<FieldProps> = props => (
  <WrappedSegmentTimeField {...props} validator={props.validator ?? validateSegmentTime} />
)

export default SegmentTimeField
