import {
  Form,
  Notification,
  Collapse,
  Select,
  Avatar,
} from '@douyinfe/semi-ui'
import { FormApi } from '@douyinfe/semi-ui/lib/es/form'
import React, { useRef } from 'react'
import { useState } from 'react'
import { LiveStreamerEntity, type MosaicConfig } from '../lib/api-streamer'
import { SupportedPlatforms } from '@/app/ui/plugins'
import { useBiliUsers } from '../lib/use-streamers'
import { FileSizeField } from './FileSizeInput'
import { FormSheet } from './shell'
import { MosaicPanel } from './MosaicPanel'
import { SegmentTimeField } from './SegmentTimeField'
import { updateSegmentTimeOverride, validateSegmentTime, type SegmentTimeValue } from '../lib/segment-time'
import { parseOverrideText, updateMosaicOverrideText, validateMosaicConfig } from '../lib/mosaic-config'

type PluginProps = {
  entity?: LiveStreamerEntity
  list?: { value: number; label: React.ReactNode }[]
  initValues?: any
}

type TemplateModalProps = {
  visible?: boolean
  entity?: LiveStreamerEntity
  children?: React.ReactNode
  initialPanel?: 'plugin' | 'mosaic'
  onOk: (e: any) => Promise<void>
}

const removeCircularReferences = (obj: any, seen = new WeakSet()): any => {
  // 处理 null 或非对象类型
  if (obj === null || typeof obj !== 'object') return obj

  // 检测循环引用
  if (seen.has(obj)) return '[Circular Reference]'
  seen.add(obj)

  if (Array.isArray(obj)) {
    return obj.map((item: any) => removeCircularReferences(item, seen))
  }

  const result: Record<string, any> = {}
  for (const [key, value] of Object.entries(obj)) {
    // 跳过 React 相关的属性
    if (key === '_context' || key === 'Provider' || key === 'Consumer') continue
    result[key] = removeCircularReferences(value, seen)
  }
  return result
}

type PlatformPattern = keyof typeof SupportedPlatforms

/** 按直播间地址找到对应平台插件在 SupportedPlatforms 里的键;没匹配到返回 undefined */
const matchPlatformPattern = (url?: string): PlatformPattern | undefined =>
  (Object.keys(SupportedPlatforms) as PlatformPattern[]).find(pattern => url?.match(new RegExp(pattern)))

const OverrideModal: React.FC<TemplateModalProps> = ({ children, entity, initialPanel = 'plugin', onOk }) => {
  const [activePanels, setActivePanels] = useState<string[]>([initialPanel])

  // 平台插件组件从模块级常量表里按键取出,渲染间始终是同一个引用,不会因为重渲染而重置内部状态
  const platformPattern = matchPlatformPattern(entity?.url)
  const PlatformPlugin = platformPattern
    ? (SupportedPlatforms[platformPattern] as React.ComponentType<PluginProps>)
    : null

  const api = useRef<FormApi>(undefined)

  const { biliUsers } = useBiliUsers()
  const list = biliUsers?.map(item => {
    return {
      value: item.value,
      label: (
        <>
          <Avatar size="extra-small" src={item.face} />
          <span style={{ marginLeft: 8 }}>{item.name}</span>
        </>
      ),
    }
  })

  const [visible, setVisible] = useState(false)
  const showDialog = () => {
    setActivePanels([initialPanel])
    setVisible(true)
  }
  const handleOk = async () => {
    const submitted = await api.current?.validate()
    const values = submitted ? { ...submitted } : undefined
    // 从 LiveStreamerEntity 接口定义中获取所有字段
    const entityFields = new Set([
      'id',
      'url',
      'remark',
      'filename',
      'filename_prefix',
      'split_time',
      'split_size',
      'upload_id',
      'upload_streamers_id',
      'status',
      'format',
      'time_range',
      'excluded_keywords',
      'preprocessor',
      'segment_processor',
      'downloaded_processor',
      'postprocessor',
      'opt_args',
      'override',
    ])

    if (values) {
      // 处理 override_text
      if (typeof values.override_text === 'string') {
        try {
          values.override = parseOverrideText(values.override_text)
        } catch (e) {
          Notification.error({
            title: '错误',
            content: '配置格式不正确，请检查 JSON 格式',
          })
          return
        }
      }
      delete values.override_text

      const overrideConfig = { ...(values.override || {}) }
      Object.keys(values).forEach(key => {
        if (!entityFields.has(key)) {
          if (values[key] !== undefined) {
            overrideConfig[key] = values[key] === '' && key !== 'douyu_cookie' ? null : values[key]
          }
          delete values[key]
        }
      })
      const mosaicError = validateMosaicConfig(overrideConfig.mosaic_config)
      if (mosaicError) {
        Notification.error({ title: '画面遮挡配置错误', content: mosaicError })
        return
      }
      const durationError = validateSegmentTime(overrideConfig.segment_time)
      if (durationError) {
        Notification.error({ title: '分段时长配置错误', content: durationError })
        return
      }
      values.override = overrideConfig

      // PUT /v1/streamers 会按整行覆盖。漏掉 upload_streamers_id 会被写成 NULL，
      // 之后录像走默认 rm 且不再投稿。以当前行打底，再叠表单字段。
      const payload = {
        ...entity,
        ...values,
        override: overrideConfig,
        upload_streamers_id:
          values.upload_streamers_id ?? entity?.upload_streamers_id ?? null,
      }

      // 处理循环引用
      const cleanValues = removeCircularReferences(payload)
      await onOk(cleanValues)
      setVisible(false)
      return
    }
    setVisible(false)
  }
  const handleCancel = () => {
    setVisible(false)
  }

  const syncMosaicConfig = (config: MosaicConfig) => {
    try {
      const text = api.current?.getValue('override_text')
      api.current?.setValue('override_text', updateMosaicOverrideText(typeof text === 'string' ? text : '', config))
    } catch {
      // Retain incomplete JSON edits; the form validator will report them on save.
    }
  }

  const syncSegmentTime = (value: SegmentTimeValue) => {
    try {
      const text = api.current?.getValue('override_text')
      const override = parseOverrideText(typeof text === 'string' ? text : '')
      api.current?.setValue('override_text', JSON.stringify(updateSegmentTimeOverride(override, value), null, 2))
    } catch {
      // Incomplete JSON is retained for validation when saving.
    }
  }

  const childrenWithProps = React.Children.map(children, child => {
    if (React.isValidElement<{ onClick?: () => void }>(child)) {
      return React.cloneElement(child, {
        onClick: () => {
          showDialog()
          child.props.onClick?.()
        },
      })
    }
  })

  const downloadSettings = (
    <Collapse.Panel header="下载设置" itemKey="download">
      <div style={{ marginBottom: 12 }}>
        请到
        <a href="/dashboard" style={{ textDecoration: 'none', color: 'var(--semi-color-primary)' }}>
          空间配置
        </a>
        查看选项说明
      </div>
      <Form.Select
        label="下载插件（downloader）"
        field="downloader"
        placeholder="mesio（默认）"
        style={{ width: '100%' }}
        fieldStyle={{
          alignSelf: 'stretch',
          padding: 0,
        }}
        showClear={true}
      >
        <Select.Option value="streamlink">streamlink（hls多线程下载）</Select.Option>
        <Select.Option value="ffmpeg">ffmpeg</Select.Option>
        <Select.Option value="stream-gears">stream-gears</Select.Option>
        <Select.Option value="sync-downloader">sync-downloader（边录边传）</Select.Option>
        <Select.Option value="mesio">mesio（默认）</Select.Option>
      </Form.Select>

      <FileSizeField
        label="视频分段大小（file_size）"
        field="file_size"
        extraText="按 1024 进制：1 GB = 1024 MB。没填过的留空即跟随全局设置；把已有的值清空，则这个主播不按大小分段（边录边传仍约 2 GB 一段）。"
        fieldStyle={{
          alignSelf: 'stretch',
          padding: 0,
        }}
      />

      <SegmentTimeField
        field="segment_time"
        scope="room"
        initValue={entity?.override?.segment_time}
        onChange={syncSegmentTime}
        label="视频分段时长（segment_time）"
        extraText="可选择预设或自定义时长。跟随全局、不按时长分段和独立时长分别保存；分段较短时请同时检查碎片过滤阈值，低于阈值的分段会被过滤。"
        fieldStyle={{ alignSelf: 'stretch', padding: 0 }}
      />

      <Form.InputNumber
        field="filtering_threshold"
        label="碎片过滤（filtering_threshold）"
        suffix={'MB'}
        style={{ width: '100%' }}
        fieldStyle={{
          alignSelf: 'stretch',
          padding: 0,
        }}
        showClear={true}
      />
    </Collapse.Panel>
  )

  return (
    <>
      {childrenWithProps}
      <FormSheet
        title={`${initialPanel === 'mosaic' ? '画面遮挡' : '配置覆写'}${entity?.remark ? `「${entity.remark}」` : ''}`}
        visible={visible}
        size="md"
        okText="保存"
        onOk={handleOk}
        onCancel={handleCancel}
      >
        <Form initValues={entity} getFormApi={formApi => (api.current = formApi)}>
          <Form.TextArea
            field="override_text"
            label="配置覆写"
            placeholder="请输入 JSON 格式的配置"
            style={{ marginBottom: 12 }}
            initValue={entity?.override ? JSON.stringify(entity.override, null, 2) : ''}
            onChange={text => {
              // Keep the visual editor aligned with valid JSON edits, including removing the override.
              try {
                const override = parseOverrideText(text)
                api.current?.setValue('mosaic_config', override.mosaic_config)
                api.current?.setValue('segment_time', override.segment_time)
                api.current?.setValue('douyu_cookie', override.douyu_cookie ?? null)
              } catch {
                // While a JSON edit is incomplete, retain the last usable configuration.
              }
            }}
            rules={[
              { required: false },
              {
                validator: (rule, value) => {
                  if (!value) return true
                  try {
                    parseOverrideText(value)
                    return true
                  } catch (e) {
                    return false
                  }
                },
                message: '请输入有效的 JSON 对象',
              },
            ]}
          />
          <Form.Section>
            <Collapse
              activeKey={activePanels}
              onChange={keys => setActivePanels(Array.isArray(keys) ? keys : keys ? [keys] : [])}
              keepDOM
              lazyRender={false}
            >
              {downloadSettings}
              <MosaicPanel entity={entity} active={visible && activePanels.includes('mosaic')} initValues={entity?.override} onChange={syncMosaicConfig} />
              {PlatformPlugin ? (
                <PlatformPlugin entity={entity} list={list} initValues={entity?.override} />
              ) : null}
            </Collapse>
          </Form.Section>
        </Form>
      </FormSheet>
    </>
  )
}

export default OverrideModal
