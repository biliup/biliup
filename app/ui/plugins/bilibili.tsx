'use client'
import React, { useEffect } from 'react'
import { Form, Select, Collapse, useFormApi } from '@douyinfe/semi-ui'

type Props = {
  entity: any
  list: any
  initValues?: Record<string, any>
}

const Bilibili: React.FC<Props> = props => {
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
      <Collapse.Panel header="哔哩哔哩" itemKey="bilibili">
        <Form.Select
          allowCreate={true}
          filter
          field="bili_qn"
          extraText="自选画质，默认原画。刚开播无此画质会先录原画，分段时（非 stream-gears）切换；未提供则取最接近档。"
          label="画质等级（bili_qn）"
          placeholder="10000（原画）"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
          rules={[
            {
              pattern: /^\d*$/,
              message: '请仅输入纯数字',
            },
          ]}
          showClear={true}
        >
          <Select.Option value={30000}>30000（杜比）</Select.Option>
          <Select.Option value={20000}>20000（4k）</Select.Option>
          <Select.Option value={10000}>10000（原画）</Select.Option>
          <Select.Option value={401}>401（蓝光-杜比）</Select.Option>
          <Select.Option value={400}>400（蓝光）</Select.Option>
          <Select.Option value={250}>250（超清）</Select.Option>
          <Select.Option value={150}>150（高清）</Select.Option>
          <Select.Option value={80}>80（流畅）</Select.Option>
          <Select.Option value={0}>0（最低画质）</Select.Option>
        </Form.Select>
        <Form.Switch
          field="bilibili_danmaku"
          extraText="录制弹幕，默认关闭。仅非 stream-gears 时生效；按时长分段时弹幕文件不自动分段。"
          label="录制弹幕（bilibili_danmaku）"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Switch
          field="bilibili_danmaku_detail"
          extraText="弹幕含昵称/UID/醒目留言/上舰/礼物，默认关闭。需开启弹幕，可能与弹幕转ass工具不兼容。"
          label="录制详细弹幕（bilibili_danmaku_detail）"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Switch
          field="bilibili_danmaku_raw"
          extraText="录制原始弹幕数据，默认关闭。需开启弹幕；每5分钟写入，文件可能极大。"
          label="录制完整弹幕（bilibili_danmaku_raw）"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Input
          field="user.bili_cookie"
          extraText={
            <span>
              按格式填入 Cookie，推荐用{' '}
              <a
                href="https://github.com/biliup/biliup-rs"
                title="biliup-rs Github"
                target="_blank"
              >
                biliup-rs
              </a>{' '}
              获取。
            </span>
          }
          placeholder="SESSDATA=none;bili_jct=none;DedeUserID__ckMd5=none;DedeUserID=none;"
          label="哔哩哔哩 Cookie 文本（bili_cookie）"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Select
          field="user.bili_cookie_file"
          label="哔哩哔哩 Cookie 文件（bili_cookie_file）"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
          optionList={list}
          extraText="仅支持 biliup-rs 生成的文件；与文本同时存在时优先用文件。"
          showClear={true}
        />
        <Form.Select
          field="bili_protocol"
          extraText="直播流协议。遵循 hls_fmp4 转码等待时间；stream-gears 不支持 hls_fmp4，需改用 ffmpeg/streamlink。"
          label="直播流协议（bili_protocol）"
          placeholder="stream（flv，默认）"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
          showClear={true}
        >
          <Select.Option value="stream">stream（flv，默认）</Select.Option>
          <Select.Option value="hls_fmp4">hls_fmp4</Select.Option>
        </Form.Select>
        <Form.Input
          field="bili_liveapi"
          extraText="自定义主 API，用于获取指定区域直播流，默认官方。"
          label="哔哩哔哩直播主要API（bili_liveapi）"
          style={{ width: '100%' }}
          placeholder="https://api.live.bilibili.com"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
          showClear={true}
          rules={[
            {
              pattern:
                /^https?:\/\/(?:[\w-]+(?::[\w-]+)?@)?([\w-]+\.)+[\w-]+(?::\d+)?(?:\/[\w-/.]*)?$/,
              message: '请输入有效的API地址，必须以 http:// 或 https:// 开头',
            },
          ]}
        />
        <Form.Input
          field="bili_fallback_api"
          extraText="主 API 不可用或受区域限制时的回退，默认官方。海外机可配 fmp4+streamlink 稳定录制大主播。"
          label="哔哩哔哩直播回退API（bili_fallback_api）"
          style={{ width: '100%' }}
          placeholder="https://api.live.bilibili.com"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
          showClear={true}
          rules={[
            {
              pattern:
                /^https?:\/\/(?:[\w-]+(?::[\w-]+)?@)?([\w-]+\.)+[\w-]+(?::\d+)?(?:\/[\w-/.]*)?$/,
              message: '请输入有效的API地址，必须以 http:// 或 https:// 开头',
            },
          ]}
        />
        <Form.TagInput
          field="bili_cdn"
          extraText="直播 CDN，默认无。"
          label="直播CDN（bili_cdn）"
          placeholder="例: cn-gotcha204,ov-gotcha05。用英文逗号分隔以批量输入，失焦/Enter保存"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
          showClear={true}
          rules={[
            {
              validator: (rule, value) => {
                value = value ?? []
                return Array.isArray(value) && value.every(item => /^(cn|ov)-gotcha\d+$/.test(item))
              },
              message: '例: cn-gotcha204,ov-gotcha05',
            },
          ]}
        />
        <Form.Switch
          field="bili_cdn_fallback"
          extraText="CDN 回退，默认关闭。同协议下首选流不可用时自动切其他节点。"
          label="CDN 回退（bili_cdn_fallback）"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Switch
          field="bili_anonymous_origin"
          extraText="用自定义 API 取 hls_fmp4 原画，无法录特殊直播，默认关闭。"
          label="免登录原画（bili_anonymous_origin）"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.InputNumber
          field="bili_hls_transcode_timeout"
          extraText="hls_fmp4 转码等待，超时回退 flv，默认 60 秒。"
          label="hls_fmp4 转码等待时间（bili_hls_transcode_timeout）"
          style={{ width: '100%' }}
          placeholder="60"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
          showClear={true}
        />
      </Collapse.Panel>
    </>
  )
}

export default Bilibili
