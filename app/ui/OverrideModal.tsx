import {
  Form,
  Modal,
  Notification,
  Collapse,
  Select,
  Avatar,
} from '@douyinfe/semi-ui'
import { FormApi } from '@douyinfe/semi-ui/lib/es/form'
import React, { useRef } from 'react'
import { useState } from 'react'
import { LiveStreamerEntity } from '../lib/api-streamer'
import { SupportedPlatforms } from '@/app/ui/plugins'
import { useBiliUsers } from '../lib/use-streamers'

type PluginProps = {
  entity?: LiveStreamerEntity
  list?: { value: number; label: React.ReactNode }[]
  initValues?: any
}

type TemplateModalProps = {
  visible?: boolean
  entity?: LiveStreamerEntity
  children?: React.ReactNode
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

const OverrideModal: React.FC<TemplateModalProps> = ({ children, entity, onOk }) => {
  const [isOpen, setOpen] = useState(false)

  const toggle = () => {
    setOpen(!isOpen)
  }

  const platformSetting = () => {
    for (const [pattern, Plugin] of Object.entries(SupportedPlatforms)) {
      if (entity?.url.match(new RegExp(pattern))) {
        // console.log('匹配到平台:', pattern)
        return Plugin as React.ComponentType<PluginProps>
      }
    }
    // console.log('未匹配到平台')
    return null
  }

  const api = useRef<FormApi>()

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
    setVisible(true)
  }
  const handleOk = async () => {
    let values = await api.current?.validate()
    // 从 LiveStreamerEntity 接口定义中获取所有字段
    const entityFields = new Set([
      'id',
      'url',
      'remark',
      'filename',
      'split_time',
      'split_size',
      'upload_id',
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
      if (values.override_text) {
        try {
          values.override = JSON.parse(values.override_text)
          delete values.override_text
        } catch (e) {
          Notification.error({
            title: '错误',
            content: '配置格式不正确，请检查 JSON 格式',
          })
          return
        }
      }

      const overrideConfig = { ...(values.override || {}) }
      Object.keys(values).forEach(key => {
        console.log(key, values[key])
        if (!entityFields.has(key)) {
          if (values[key] != undefined && values[key] != null) {
            overrideConfig[key] = values[key]
          }
          delete values[key]
        }
      })
      values.override = overrideConfig

      // 处理循环引用
      const cleanValues = removeCircularReferences(values)
      await onOk(cleanValues)
      setVisible(false)
      return
    }
    setVisible(false)
  }
  const handleCancel = () => {
    setVisible(false)
  }

  const childrenWithProps = React.Children.map(children, child => {
    if (React.isValidElement<any>(child)) {
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
        placeholder="stream-gears（默认）"
        style={{ width: '100%' }}
        fieldStyle={{
          alignSelf: 'stretch',
          padding: 0,
        }}
        showClear={true}
      >
        <Select.Option value="streamlink">streamlink（hls多线程下载）</Select.Option>
        <Select.Option value="ffmpeg">ffmpeg</Select.Option>
        <Select.Option value="stream-gears">stream-gears（默认）</Select.Option>
        <Select.Option value="sync-downloader">sync-downloader（边录边传）</Select.Option>
      </Form.Select>

      <Form.InputNumber
        label="视频分段大小（file_size）"
        field="file_size"
        placeholder=""
        suffix={'Byte'}
        style={{ width: '100%' }}
        fieldStyle={{
          alignSelf: 'stretch',
          padding: 0,
        }}
      />

      <Form.Input
        field="segment_time"
        label="视频分段时长（segment_time）"
        placeholder="01:00:00"
        style={{ width: '100%' }}
        fieldStyle={{
          alignSelf: 'stretch',
          padding: 0,
        }}
        showClear={true}
        rules={[
          {
            pattern: /^[^：]*$/,
            message: '请使用英文冒号',
          },
          {
            pattern: /^[0-9:]*$/,
            message: '只接受数字和英文冒号',
          },
          {
            pattern: /^[0-9]{2,4}:[0-5][0-9]:[0-5][0-9]$/,
            message: '分或秒不符合规范',
          },
        ]}
        stopValidateWithError={true}
      />

      <Form.Input
        field="filename_prefix"
        label="文件名模板（filename_prefix）"
        placeholder="{streamer}%Y-%m-%dT%H_%M_%S"
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
      <Modal
        title="配置覆写"
        visible={visible}
        onOk={handleOk}
        style={{ width: 600 }}
        onCancel={handleCancel}
        bodyStyle={{
          overflow: 'auto',
          maxHeight: 'calc(100vh - 320px)',
          paddingLeft: 10,
          paddingRight: 10,
        }}
      >
        <Form initValues={entity} getFormApi={formApi => (api.current = formApi)}>
          <Form.TextArea
            field="override_text"
            label="配置覆写"
            placeholder="请输入 JSON 格式的配置"
            style={{ marginBottom: 12 }}
            initValue={entity?.override ? JSON.stringify(entity.override, null, 2) : ''}
            rules={[
              { required: false },
              {
                validator: (rule, value) => {
                  if (!value) return true
                  try {
                    JSON.parse(value)
                    return true
                  } catch (e) {
                    return false
                  }
                },
                message: '请输入有效的 JSON 格式',
              },
            ]}
          />
          <Form.Section>
            <Collapse defaultActiveKey={['plugin']}>
              {downloadSettings}
              {(() => {
                const Plugin = platformSetting()
                return Plugin ? (
                  <Plugin entity={entity} list={list} initValues={entity?.override} />
                ) : null
              })()}
            </Collapse>
          </Form.Section>
        </Form>
      </Modal>
    </>
  )
}

export default OverrideModal
