'use client'
import React from 'react'
import { Form, Switch, Typography, useFormApi, useFormState } from '@douyinfe/semi-ui'
import type { AutoClipAvailability } from '@/app/lib/auto-clip'
import { SetupNote } from './AutoClipJob'

const KEY = 'auto_clip_after_live'

/**
 * 直播间编辑里的「下播后自动生成候选」：存在这个直播间的覆写配置（`override.auto_clip_after_live`）里，
 * 默认关。表单整体覆盖保存，只在拨动时改 `override`，不拨就原样交回。后端只让有 streamer.hooks 的人
 * 改覆写配置；其他角色拿不到覆写，只读展示 `/v1/streamers` 单独给的当前值（有 clip.edit 才有）。
 */
export default function AfterLiveSwitch({
  availability,
  canOverride,
}: {
  availability: AutoClipAvailability
  canOverride: boolean
}) {
  const { values } = useFormState()
  const formApi = useFormApi()
  const override = (values?.override ?? null) as Record<string, unknown> | null
  const on = canOverride ? override?.[KEY] === true : values?.[KEY] === true
  if (!availability.status || (!availability.visible && !on)) return null

  const toggle = (checked: boolean) => {
    const next: Record<string, unknown> = { ...(override ?? {}) }
    if (checked) next[KEY] = true
    else delete next[KEY]
    // 后端存回的覆写是整份 ConfigPatch（没填的项是 null），全是 null 时等同于没有覆写
    formApi.setValue('override', Object.values(next).some(v => v != null) ? next : null)
  }

  return (
    <div data-testid="auto-clip-after-live">
      <Form.Slot label="下播后自动生成候选">
        {canOverride ? (
          <>
            <Switch checked={on} onChange={toggle} aria-label="下播后自动生成候选" />
            <div className="semi-form-field-extra">
              打开后，这个直播间每次下播（等断流合并的时间过去、确认真的下播了）自动排队生成候选：会调用你配置的付费模型，
              受每场上限约束，候选要人工接受才变成切片。
            </div>
            {!availability.enabled ? (
              <SetupNote reason="自动切片的总开关现在是关的，打开后这里才会生效。" />
            ) : null}
          </>
        ) : (
          <>
            {typeof values?.[KEY] === 'boolean' ? (
              <span data-testid="auto-clip-after-live-readonly" data-on={on}>
                <Switch checked={on} disabled aria-label="下播后自动生成候选（只读）" />{' '}
                <Typography.Text type="secondary" size="small">
                  当前{on ? '已打开' : '关闭'}（只读）
                </Typography.Text>
              </span>
            ) : null}
            <div className="semi-form-field-extra">
              这个开关存在直播间的覆写配置里，只有超级管理员（streamer.hooks）能改；需要时请超级管理员在这里打开或关闭。也可以在回看页或剪辑台手动「生成候选」。
            </div>
          </>
        )}
      </Form.Slot>
    </div>
  )
}
