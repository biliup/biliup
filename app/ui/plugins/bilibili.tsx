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
          extraText={
            <div style={{ fontSize: '14px' }}>
              录制画质，默认原画。
              <br />
              刚开播时通常只有原画，会先录原画；下载插件为 ffmpeg / streamlink
              时之后每次分段会重新取流并切到所选画质，stream-gears / mesio 整场沿用首次取到的流。
              <br />
              所选画质不存在时录 B 站返回的最接近的次档画质；开启「免登录原画」时录最高画质。
            </div>
          }
          label="画质等级（bili_qn）"
          placeholder="25000（原画）"
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
          extraText="录制哔哩哔哩弹幕，默认关闭。弹幕保存为与录像同名的 XML 文件，并随录像分段一起分段。"
          label="录制弹幕（bilibili_danmaku）"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Switch
          field="bilibili_danmaku_detail"
          extraText={
            <div style={{ fontSize: '14px' }}>
              弹幕文件中额外记录发送者昵称、UID，并保存醒目留言、上舰、礼物记录。默认关闭。
              <br />
              需先开启「录制弹幕」。<strong>实验性功能</strong>：可能与弹幕转 ASS 工具不兼容。
            </div>
          }
          label="录制详细弹幕（bilibili_danmaku_detail）"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Switch
          field="bilibili_danmaku_raw"
          extraText={
            <div style={{ fontSize: '14px' }}>
              额外保存 B 站服务器返回的原始弹幕数据，供有技术能力的用户做统计分析。默认关闭。
              <br />
              需先开启「录制弹幕」。<strong>实验性功能</strong>：开启后弹幕文件每 5
              分钟才写入一次，且体积可能非常大。
            </div>
          }
          label="录制完整弹幕（bilibili_danmaku_raw）"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Input
          field="user.bili_cookie"
          extraText={
            <div style={{ fontSize: '14px' }}>
              按占位符格式填入 B 站登录 Cookie，推荐使用「
              <a
                href="https://github.com/biliup/biliup-rs"
                title="「biliup-rs」 Github 项目主页"
                target="_blank"
                rel="noopener noreferrer"
                style={{ color: 'var(--semi-color-link)' }}
              >
                biliup-rs
              </a>
              」获取。未登录时部分直播间只能录到较低画质。
            </div>
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
          extraText="从已登录的 B 站账号中选择，只支持「biliup-rs」生成的 Cookie 文件。与上方「Cookie 文本」同时填写时，以 Cookie 文本为准。"
          showClear={true}
        />
        <Form.Select
          field="bili_protocol"
          extraText={
            <div style={{ fontSize: '14px' }}>
              直播流协议，默认 stream（FLV）。
              <br />
              选 hls_fmp4 时，开播后会先等待下方「hls_fmp4 转码等待时间」，仍拿不到 fmp4 流则本场回退为
              FLV。
              <br />
              <strong>stream-gears 不支持 hls_fmp4</strong>，需把下载插件改为 ffmpeg 或 streamlink。
            </div>
          }
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
          extraText="获取直播流地址时优先使用的 API，默认官方 API。填入反代地址可获取指定区域（大陆或海外）的直播流。"
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
          extraText={
            <div style={{ fontSize: '14px' }}>
              上方主要 API 不可用、受区域限制或没有所选协议的流时，改用此 API 重新获取。默认官方 API。
              <br />
              <strong>海外机器玩法</strong>：主要 API 填能取到大陆直播流的反代，回退 API 保持官方；直播流协议选
              hls_fmp4，下载插件选 streamlink，直播 CDN 填 cn-gotcha204,ov-gotcha05。大主播即可走 cn204 的
              fmp4 流稳定录制，没有 fmp4 流的小主播自动回退到 ov05 的 FLV 流。
            </div>
          }
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
          extraText="优先使用的 CDN 节点，默认不指定（使用 B 站返回的第一个节点）。可填多个，按先后顺序匹配；都匹配不上时仍用第一个节点。"
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
          extraText={
            <div style={{ fontSize: '14px' }}>
              默认关闭。开启后会先探测选中的流地址能否下载，不可用时自动改用同一协议、同一画质下的其他 CDN
              节点。
              <br />
              例：海外机器优选 ov-gotcha05，但该节点一直拉不到流，会自动回退到 ov-gotcha07。
            </div>
          }
          label="CDN 回退（bili_cdn_fallback）"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Switch
          field="bili_anonymous_origin"
          extraText={
            <div style={{ fontSize: '14px' }}>
              默认关闭。开启后改从 master playlist 接口获取 hls_fmp4 原画流，<strong>不登录也能录到原画</strong>。
              <br />
              仅在直播流协议为 hls_fmp4 时生效；特殊类型直播间（如付费直播）未填 Cookie 时不走此通道，按普通方式取流。
            </div>
          }
          label="免登录原画（bili_anonymous_origin）"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.InputNumber
          field="bili_hls_transcode_timeout"
          extraText="直播流协议为 hls_fmp4 时，从 B 站记录的开播时间起最多等待这么多秒让 fmp4 流生成；超时仍没有则本场回退为 FLV 流。单位：秒，默认 60。"
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
