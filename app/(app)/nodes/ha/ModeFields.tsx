'use client'
import { Button, Checkbox, Collapsible, InputNumber, Radio, RadioGroup, Typography } from '@douyinfe/semi-ui'
import { IconChevronDown, IconChevronRight } from '@douyinfe/semi-icons'
import { useState } from 'react'
import {
  DEFAULT_PARAMS,
  MAX_SECONDS,
  MODE_TEXT,
  PARAM_FIELDS,
  humanSeconds,
  paramsIssue,
  type HaMode,
  type HaParams,
} from '@/app/lib/fleet-ha'
import styles from './ha.module.scss'

const { Text } = Typography

/** 模式单选与参数（参数缺省收起，只列当前模式用得到的） */
export default function ModeFields({
  mode,
  params,
  onMode,
  onParams,
  disabled = false,
}: {
  mode: HaMode
  params: HaParams
  onMode: (mode: HaMode) => void
  onParams: (params: HaParams) => void
  disabled?: boolean
}) {
  const [open, setOpen] = useState(false)
  const fields = PARAM_FIELDS.filter((f) => f.modes.includes(mode))
  const changed = (Object.keys(DEFAULT_PARAMS) as (keyof HaParams)[]).some((k) => params[k] !== DEFAULT_PARAMS[k])
  const issue = paramsIssue(params)
  return (
    <>
      <div className={styles.field}>
        <span className={styles.fieldLabel}>模式</span>
        <RadioGroup
          type="card"
          direction="vertical"
          value={mode}
          disabled={disabled}
          onChange={(e) => onMode(e.target.value as HaMode)}
          className={styles.modes}
          aria-label="模式"
        >
          {([1, 2] as HaMode[]).map((m) => (
            <Radio key={m} value={m} extra={MODE_TEXT[m].hint}>
              {MODE_TEXT[m].title}
            </Radio>
          ))}
        </RadioGroup>
      </div>
      <div className={styles.field}>
        <Button
          theme="borderless"
          icon={open ? <IconChevronDown /> : <IconChevronRight />}
          onClick={() => setOpen(!open)}
          aria-expanded={open}
          style={{ alignSelf: 'flex-start', paddingLeft: 0 }}
        >
          参数{changed ? '（改过）' : '（一般不用改）'}
        </Button>
        <Collapsible isOpen={open} keepDOM>
          <div className={styles.params}>
            {fields.map((field) => (
              <label key={field.key} className={styles.param}>
                <span className={styles.fieldLabel}>{field.label}</span>
                <span className={styles.paramInput}>
                  <InputNumber
                    value={params[field.key]}
                    min={1}
                    max={MAX_SECONDS}
                    precision={0}
                    suffix="秒"
                    disabled={disabled}
                    onNumberChange={(v) => onParams({ ...params, [field.key]: Number(v) || 0 })}
                    aria-label={field.label}
                  />
                  <Text type="tertiary" size="small">
                    {params[field.key] > 0 ? humanSeconds(params[field.key]) : ''}
                  </Text>
                </span>
                <Text type="tertiary" size="small">
                  {field.hint}（默认 {humanSeconds(DEFAULT_PARAMS[field.key])}）
                </Text>
              </label>
            ))}
            {mode === 1 ? (
              <div className={styles.param}>
                <Checkbox
                  checked={params.delete_standby_copy}
                  disabled={disabled}
                  onChange={(e) => onParams({ ...params, delete_standby_copy: Boolean(e.target.checked) })}
                >
                  主机投成后备机删掉自己那份
                </Checkbox>
                <Text type="tertiary" size="small">
                  默认不删：备机那份留着，按它自己的清理规则处理
                </Text>
              </div>
            ) : null}
          </div>
          <div style={{ marginTop: 8, display: 'flex', alignItems: 'center', gap: 8, flexWrap: 'wrap' }}>
            <Button size="small" disabled={disabled || !changed} onClick={() => onParams(DEFAULT_PARAMS)}>
              恢复默认
            </Button>
            {issue ? (
              <Text type="danger" size="small">
                {issue}
              </Text>
            ) : null}
          </div>
        </Collapsible>
        {issue && !open ? (
          <Text type="danger" size="small">
            {issue}
          </Text>
        ) : null}
      </div>
    </>
  )
}
