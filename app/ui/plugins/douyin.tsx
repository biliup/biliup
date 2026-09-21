'use client'
import React, { useEffect } from 'react'
import { Form, Select, useFormApi } from '@douyinfe/semi-ui'
import PlatformPanel from './PlatformPanel'

type Props = {
  entity: any
  list: any
  initValues?: Record<string, any>
  bare?: boolean
}

const Douyin: React.FC<Props> = props => {
  const { entity, list, initValues, bare } = props
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
      <PlatformPanel header="抖音" itemKey="douyin" bare={bare}>
        <Form.Select
          field="douyin_quality"
          extraText={
            <div style={{ fontSize: '14px' }}>
              录制画质，默认原画。所选画质不存在时自动选最接近的一档，<strong>优先更低</strong>的清晰度。
              <br />
              刚开播时可能只有原画，会先录原画；下载插件为 ffmpeg / streamlink
              时之后每次分段会重新取流并切到所选画质，stream-gears / mesio 整场沿用首次取到的流。
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
            <div style={{ fontSize: '14px' }}>
              录制 www.douyin.com/user/… 形式的主页链接，或遇到风控时，需要在此填入 Cookie。
              <br />
              只需 __ac_nonce、__ac_signature、sessionid 三项的值，<strong>不要把全部 Cookie 粘进来</strong>。
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
              主播开双屏直播时录哪一路拼接流，默认关闭。
              <br />
              关闭：录横向像素不变的缩放拼接流，画面被压缩，可能有画质损失。
              <br />
              开启：录纵向像素不变的原始（raw）双屏拼接流。
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
          extraText={
            <div style={{ fontSize: '14px' }}>
              默认关闭。开启后录制抖音的真原画流，<strong>仅在画质为原画、直播流协议为 flv 时生效</strong>。
              <br />
              真原画可能是 HEVC 编码，默认下载插件 stream-gears 不支持 HEVC 会导致录制失败，请先把下载插件换成
              ffmpeg 或 streamlink。
            </div>
          }
          label="抖音真原画（douyin_true_origin）"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
      </PlatformPanel>
    </>
  )
}

export default Douyin
