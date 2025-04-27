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
          extraText={
            <div style={{ fontSize: '14px' }}>
              抖音自选画质，没有选中的画质则会自动选择相近的画质优先低清晰度。
              <br />
              刚开播可能没有除了原画之外的画质，会先录制原画。当使用 ffmpeg 或 streamlink
              时，后续视频分段将会录制设置的画质。
            </div>
          }
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
          extraText={
            <div className="semi-form-field-extra">
              如需要录制抖音 www.douyin.com/user/ 类型链接，或遭到风控，请在此填入 Cookie。
              <br />
              需要__ac_nonce、__ac_signature、sessionid的值，请不要将所有 Cookie 填入。
            </div>
          }
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
          extraText={
            <div style={{ fontSize: '14px' }}>
              是否录制抖音双屏直播的原像素拼接流，默认关闭。
              <br />
              关闭时录制 横像素不变的 缩放拼接流，可能存在画质损失；开启时录制 纵像素不变的 raw
              双屏拼接流。
            </div>
          }
          label="双屏直播录制方式（douyin_double_screen）"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Switch
          field="douyin_true_origin"
          extraText="仅限直播流协议为 FLV 时生效，默认关闭。开启后可能录制到 HEVC 编码，而 stream-gears（默认下载器）暂不支持，请切换下载器后录制。"
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
