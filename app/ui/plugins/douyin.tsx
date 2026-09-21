'use client'
import React, { useEffect } from 'react'
import { Form, Select, Collapse, useFormApi } from '@douyinfe/semi-ui'

type Props = {
  entity: any
  list: any
  initValues?: Record<string, any>
}

const Douyin: React.FC<Props> = props => {
  const { entity, list, initValues } = props
  const formApi = useFormApi()

  useEffect(() => {
    if (initValues) {
      Object.entries(initValues).forEach(([key, value]) => {
        formApi.setValue(key, value)
      })
    }
  }, [initValues, formApi])

  return (
    <>
      <Collapse.Panel header="抖音" itemKey="douyin">
        <Form.Select
          field="douyin_quality"
          extraText="自选画质，默认原画。开播无选档时先录原画，分段后（ffmpeg/streamlink）录制设定档。"
          label="画质等级（douyin_quality）"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
          showClear={true}
        >
          <Select.Option value="origin">原画（origin）</Select.Option>
          <Select.Option value="uhd">蓝光（uhd）</Select.Option>
          <Select.Option value="hd">超清（hd）</Select.Option>
          <Select.Option value="sd">高清（sd）</Select.Option>
          <Select.Option value="ld">标清（ld）</Select.Option>
          <Select.Option value="md">流畅（md）</Select.Option>
        </Form.Select>
        <Form.Switch
          field="douyin_danmaku"
          extraText="录制抖音弹幕，默认关闭。"
          label="录制弹幕（douyin_danmaku）"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Input
          field="user.douyin_cookie"
          extraText="如需录 user/ 类型链接或遇风控请填 Cookie。需 __ac_nonce/__ac_signature/sessionid，勿填全部。"
          placeholder="__ac_nonce=none;__ac_signature=none;sessionid=none;"
          label="抖音 Cookie（douyin_cookie）"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
          showClear={true}
        />
        <Form.Select
          field="douyin_protocol"
          extraText="hls 仅供测试，请谨慎切换。"
          label="直播流协议（douyin_protocol）"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
          showClear={true}
        >
          <Select.Option value="flv">flv（默认）</Select.Option>
          <Select.Option value="hls">hls</Select.Option>
        </Form.Select>
        <Form.Switch
          field="douyin_double_screen"
          extraText="录制双屏拼接流，默认关闭。开启为纵像素不变 raw 流；关闭为横像素不变缩放流（可能画质损失）。"
          label="双屏直播录制方式（douyin_double_screen）"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Switch
          field="douyin_true_origin"
          extraText="仅 FLV 生效，默认关闭。可能录到 HEVC，stream-gears 不支持，需换下载器。"
          label="抖音真原画（douyin_true_origin）"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
      </Collapse.Panel>
    </>
  )
}

export default Douyin
