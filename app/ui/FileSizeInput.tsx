'use client'
import React, { useState } from 'react'
import { InputNumber, Select, Typography, withField } from '@douyinfe/semi-ui'

const MB = 1024 * 1024
const GB = 1024 * MB

export type SizeUnit = 'MB' | 'GB'

const UNIT_BYTES: Record<SizeUnit, number> = { MB, GB }

/** 选能用两位以内小数精确表示的最大单位；都不能精确表示时，1 GB 及以上用 GB，否则用 MB */
export const pickUnit = (bytes: number): SizeUnit => {
  for (const unit of ['GB', 'MB'] as const) {
    const size = UNIT_BYTES[unit]
    if (bytes >= size && (bytes * 100) % size === 0) return unit
  }
  return bytes >= GB ? 'GB' : 'MB'
}

/** 显示用的数值：1 以上保留两位小数，更小的保留三位有效数字 */
export const formatAmount = (bytes: number, unit: SizeUnit): number => {
  const amount = bytes / UNIT_BYTES[unit]
  return amount >= 1 ? Math.round(amount * 100) / 100 : Number(amount.toPrecision(3))
}

export const toBytes = (amount: number, unit: SizeUnit): number => Math.round(amount * UNIT_BYTES[unit])

const bytesOf = (value: unknown): number | null => {
  if (value === null || value === undefined || value === '') return null
  const bytes = typeof value === 'number' ? value : Number(value)
  return Number.isFinite(bytes) ? bytes : null
}

/** 用户最近一次输入：只对应它产生的那个字节数 */
type Draft = { bytes: number | null; unit: SizeUnit; amount: number | '' }

type Props = {
  /** 字节数；空串 / undefined / null 表示没有值 */
  value?: number | string | null
  /**
   * 清空时：外部带进来的值被清掉给 null（覆写里即「显式清空」，Semi 表单会保留这个键）；
   * 清掉的是自己刚输入的值则给空串，Semi 会把键移除，回到打开时的状态（覆写里即「跟随全局」）
   */
  onChange?: (value: number | '' | null) => void
  onBlur?: (e: React.FocusEvent) => void
  disabled?: boolean
  id?: string
  placeholder?: string
  validateStatus?: 'default' | 'error' | 'warning' | 'success'
}

/**
 * 字节数的「数值 + 单位（MB / GB，1024 进制）」输入。表单里存的仍是字节整数：
 * 没动过就原样保留（不经过换算，不丢字节），改了数值或单位才按新输入换算。
 */
const FileSizeInput: React.FC<Props> = ({
  value,
  onChange,
  onBlur,
  disabled,
  id,
  placeholder,
  validateStatus,
}) => {
  const bytes = bytesOf(value)
  const [draft, setDraft] = useState<Draft | null>(null)
  // 表单值被外部改写（读回配置、平台插件回填覆写）后草稿作废，按新值重新选单位
  const current = draft && draft.bytes === bytes ? draft : null
  const unit = current?.unit ?? (bytes === null ? 'GB' : pickUnit(bytes))
  const amount = current ? current.amount : bytes === null ? '' : formatAmount(bytes, unit)
  // 最近一次从外部带进来的值，用来区分「清空已有值」和「撤销自己的输入」
  const [loaded, setLoaded] = useState<number | null>(bytes)
  if (!current && bytes !== loaded) setLoaded(bytes)

  const commit = (nextAmount: number | '', nextUnit: SizeUnit) => {
    const nextBytes = nextAmount === '' ? null : toBytes(nextAmount, nextUnit)
    setDraft({ bytes: nextBytes, unit: nextUnit, amount: nextAmount })
    onChange?.(nextBytes !== null ? nextBytes : loaded === null ? '' : null)
  }

  return (
    <div style={{ display: 'flex', flexWrap: 'wrap', alignItems: 'center', gap: 8, width: '100%' }}>
      <InputNumber
        id={id}
        value={amount}
        min={0}
        placeholder={placeholder}
        disabled={disabled}
        validateStatus={validateStatus}
        showClear
        onBlur={onBlur}
        onChange={next => {
          if (next === '') commit('', unit)
          else if (typeof next === 'number' && Number.isFinite(next)) commit(next, unit)
        }}
        style={{ flex: '1 1 160px', minWidth: 0 }}
      />
      <Select
        aria-label="单位"
        value={unit}
        disabled={disabled}
        onChange={next => {
          const nextUnit = next as SizeUnit
          // 还没填数值时只记住单位，不写表单：保持「未设置」
          if (amount === '') setDraft({ bytes, unit: nextUnit, amount: '' })
          else commit(amount, nextUnit)
        }}
        optionList={[
          { value: 'MB', label: 'MB' },
          { value: 'GB', label: 'GB' },
        ]}
        style={{ width: 88 }}
      />
      {bytes !== null ? (
        <Typography.Text type="tertiary" size="small">
          = {bytes.toLocaleString('en-US')} 字节
        </Typography.Text>
      ) : null}
    </div>
  )
}

/** 表单字段版：`field` 存字节数 */
export const FileSizeField = withField(FileSizeInput)

export default FileSizeInput
