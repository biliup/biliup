import {
  Form,
  Notification,
  Typography,
  Collapse,
  ArrayField,
  Button,
} from '@douyinfe/semi-ui'
import { IconPlusCircle, IconMinusCircle } from '@douyinfe/semi-icons'
import { FormApi } from '@douyinfe/semi-ui/lib/es/form'
import React, { useMemo, useRef } from 'react'
import { useState } from 'react'
import { fetcher, LiveStreamerEntity, sendRequest, StudioEntity } from '../lib/api-streamer'
import useSWR from 'swr'
import useSWRMutation from 'swr/mutation'
import { useMe } from '../lib/use-me'
import { useAutoClip } from '../lib/auto-clip'
import AfterLiveSwitch from './auto-clip/AfterLiveSwitch'
import { FormSheet } from './shell'
import ProcessingFlowField from './ProcessingFlowEditor'
import { processingFlowFromEntity, processingFlowToForm } from '../lib/processing-flow'

type TemplateModalProps = {
  visible?: boolean
  entity?: LiveStreamerEntity
  children?: React.ReactNode
  onOk: (e: any) => Promise<void>
  title?: string
  /** 投稿模板的候选项；不传时取本机的 `/v1/upload/streamers`（Fleet 控制面传自己的模板） */
  templateOptions?: { value: number; label: React.ReactNode }[]
  /** 放在投稿模板下面的额外字段（Fleet 控制面的「分派到节点」） */
  extraFields?: React.ReactNode
}

const TemplateModal: React.FC<TemplateModalProps> = ({
  children,
  entity,
  onOk,
  title = entity ? `编辑直播间「${entity.remark}」` : '新建直播间',
  templateOptions,
  extraFields,
}) => {
  const { Text } = Typography
  const message = '该项为必填项'
  const api = useRef<FormApi>(undefined)
  // 各类处理器会在服务器上执行 shell 命令，只有超级管理员能看到和修改；后端也会忽略其他角色提交的这几项
  const { can } = useMe()
  const hooks = can('streamer.hooks')
  const {
    data: templates,
    error,
    isLoading,
  } = useSWR<StudioEntity[]>(templateOptions ? null : '/v1/upload/streamers', fetcher)

  const [visible, setVisible] = useState(false)
  // Fleet 控制面的直播间不在本机生成候选，不显示这个开关
  const autoClip = useAutoClip(visible && !templateOptions)
  const showDialog = () => {
    setVisible(true)
  }
  const handleOk = async () => {
    const { processing_flow, ...formValues } = await api.current!.validate()
    const values = {
      ...formValues,
      ...(hooks ? processingFlowToForm(processing_flow ?? {}) : {}),
      remark: formValues.remark?.trim(),
      url: formValues.url?.trim(),
      format: formValues.format?.trim(),
      time_range: JSON.stringify(formValues.time_range?.map((date: Date) => date.toISOString())),
    }
    await onOk(values)
    setVisible(false)
  }
  const handleCancel = () => {
    setVisible(false)
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

  const list =
    templateOptions ??
    templates?.map(template => {
      return {
        value: template.id,
        label: template.template_name,
      }
    })

  // 后端把 time_range 存成 JSON 字串,TimePicker 需要 Date[]。entity 是上层传下来的 props,
  // 渲染期不能就地改写,转换结果放进一份本地副本作为表单初始值;解析失败时保持原样(与之前一致)
  const initValues = useMemo(() => {
    const initial = { ...entity, processing_flow: processingFlowFromEntity(entity) }
    if (entity && entity.time_range && typeof entity.time_range === 'string') {
      try {
        const tr: string[] = JSON.parse(entity.time_range)
        return { ...initial, time_range: tr.map(t => new Date(t)) }
      } catch (e) {
        console.error(e)
      }
    }
    return initial
  }, [entity])

  return (
    <>
      {childrenWithProps}
      <FormSheet
        title={title}
        visible={visible}
        size={hooks ? "lg" : "md"}
        okText={entity ? '保存' : '创建'}
        onOk={handleOk}
        onCancel={handleCancel}
      >
        <Form initValues={initValues} getFormApi={formApi => (api.current = formApi)}>
          <Form.Input
            field="remark"
            label="录播备注"
            trigger="blur"
            rules={[{ required: true, message }]}
          />

          <Form.Input
            field="url"
            label="直播链接"
            trigger="blur"
            rules={[{ required: true, message }]}
          />

          <Form.Input
              field="filename_prefix"
              label="文件名模板（可选）"
              placeholder="{streamer}%Y-%m-%dT%H_%M_%S"
          />

          <Form.Select
            showClear
            field="upload_streamers_id"
            label={{ text: '投稿模板', optional: true }}
            style={{ width: 176 }}
            optionList={list}
          />

          {extraFields}

          {templateOptions ? null : <AfterLiveSwitch availability={autoClip} canOverride={hooks} />}

          {hooks ? (
            <ProcessingFlowField field="processing_flow" noLabel />
          ) : (
            <Form.Slot label={{ text: '处理流程' }}>
              <Text type="tertiary">后处理与各类处理器命令仅超级管理员可配置，保存时保持原样。</Text>
            </Form.Slot>
          )}

          <Form.Input
            field="format"
            label="视频格式"
            placeholder="flv"
            style={{ width: 176 }}
            helpText="视频保存格式（转封装）。mesio、stream-gears 下载器不转封装，按源站容器保存；YouTube 平台也不支持。"
          />

          <Collapse keepDOM>
            <Collapse.Panel header="更多设置" itemKey="processors">
              <Form.TimePicker
                field="time_range"
                type="timeRange"
                placeholder=" "
                extraText={
                  <div style={{ fontSize: '14px' }}>
                    如果设置了录制时间范围，不在时间范围内，将不进行录制<br />
                    下载器需使用ffmpeg或streamlink
                  </div>
                }
                label={{
                  text: '录制时间范围',
                  optional: true,
                  style: {
                    marginBottom: '4px',
                    paddingBottom: '8px',
                    borderBottom: '1px solid var(--semi-color-border)',
                  }
                }}
                style={{ width: 176 }}
              />

              <ArrayField field="excluded_keywords">
                {({ add, arrayFields }) => (
                  <Form.Section text="不录制关键词">
                    <div className="semi-form-field-extra">
                      如果房间名包含关键词，则停止或不录制该场直播，每个关键词需单独一行<br />
                      暂不支持<strong>cc直播</strong>、<strong>yy直播</strong>、<strong>twitch直播</strong>
                    </div>
                    <Button icon={<IconPlusCircle />} onClick={() => add()} theme="light">
                      添加关键词
                    </Button>
                    {arrayFields.map(({ field, key, remove }, i) => (
                      <div key={key} style={{ width: '100%', display: 'flex' }}>
                        <Form.Input
                          field={field}
                          label={`关键词${i + 1}`}
                          labelPosition="left"
                          rules={[{ required: true, message }]}
                        ></Form.Input>
                        <Button
                          type="danger"
                          theme="borderless"
                          icon={<IconMinusCircle />}
                          onClick={remove}
                          style={{ margin: 12 }}
                        />
                      </div>
                    ))}
                  </Form.Section>
                )}
              </ArrayField>

              <ArrayField field="opt_args">
                {({ add, arrayFields }) => (
                  <Form.Section text="ffmpeg参数">
                    <div className="semi-form-field-extra">
                      如：&quot;-ss&quot;、&quot;00:00:16&quot;，每个参数需单独一行
                    </div>
                    <Button icon={<IconPlusCircle />} onClick={() => add()} theme="light">
                      添加行
                    </Button>
                    {arrayFields.map(({ field, key, remove }, i) => (
                      <div key={key} style={{ width: '100%', display: 'flex' }}>
                        <Form.Input
                          field={field}
                          label={`参数${i + 1}`}
                          labelPosition="left"
                          rules={[{ required: true, message }]}
                        ></Form.Input>
                        <Button
                          type="danger"
                          theme="borderless"
                          icon={<IconMinusCircle />}
                          onClick={remove}
                          style={{ margin: 12 }}
                        />
                      </div>
                    ))}
                  </Form.Section>
                )}
              </ArrayField>
            </Collapse.Panel>
          </Collapse>
        </Form>
      </FormSheet>
    </>
  )
}

export default TemplateModal
