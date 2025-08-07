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
              哔哩哔哩自选画质。默认原画。
              <br />
              刚开播如果无选择的画质，会先录制原画， 后续视频分段时，如果下载插件为非
              stream-gears，会切换到选择的画质。
              <br />
              如果选择的画质没有提供，会使用次档画质中最接近的画质，免登录原画会使用最高画质。
            </div>
          }
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
          extraText="录制哔哩哔哩弹幕，目前不支持视频按时长分段下的弹幕文件自动分段。仅限下载插件为非 stream-gears 时生效，默认关闭。"
          label="录制弹幕（bilibili_danmaku）"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Switch
          field="bilibili_danmaku_detail"
          extraText="录制的弹幕信息中包含发送者昵称、用户UID，同时保存醒目留言、上舰、礼物信息。仅 bilibili_danmaku 开启时生效，默认关闭（实验性质：可能与弹幕转ass工具不兼容）"
          label="录制详细弹幕（bilibili_danmaku_detail）"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Switch
          field="bilibili_danmaku_raw"
          extraText="录制B站服务器返回的原始弹幕信息，方便有技术能力的用户对主播弹幕数据进行统计。仅 bilibili_danmaku 开启时生效，默认关闭，开启后弹幕文件会每隔5分钟写入一次（实验性质：可能导致弹幕文件巨大）"
          label="录制完整弹幕（bilibili_danmaku_raw）"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Input
          field="user.bili_cookie"
          extraText={
            <div className="semi-form-field-extra">
              根据格式填入cookie。推荐使用「
              <a
                href="https://github.com/biliup/biliup-rs"
                title="「biliup-rs」 Github 项目主页"
                target="_blank"
              >
                biliup-rs
              </a>
              」来获取。
              <br />
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
          extraText="只支持「biliup-rs」生成的文件。与 Cookie 文本 同时存在时，优先使用文件。"
          showClear={true}
        />
        <Form.Select
          field="bili_protocol"
          extraText={
            <div style={{ fontSize: '14px' }}>
              哔哩哔哩直播流协议。
              <br />
              该设置项遵循 hls_fmp4 转码等待时间（bili_hls_transcode_timeout）。
              <br />
              stream-gears 尚未支持 hls_fmp4，请切换为 ffmpeg 或 streamlink 来录制。
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
          {/* <Select.Option value="hls_ts">hls_ts</Select.Option> */}
          <Select.Option value="hls_fmp4">hls_fmp4</Select.Option>
        </Form.Select>
        <Form.Input
          field="bili_liveapi"
          extraText="自定义哔哩哔哩直播主要 API，用于获取指定区域（大陆或海外）的直播流链接，默认使用官方 API。"
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
              上方的主要 API 不可用或受到区域限制时，回退使用的 API。默认使用官方 API。
              <br />
              海外机器玩法：哔哩哔哩直播API（bili_liveapi）设置为能获取大陆直播流的
              API，并将哔哩哔哩直播回退API（bili_fallback_api）设置为官方
              API，然后优选「fmp4」流并使用「streamlink」下载插件（downloader），
              最后设置优选「cn-gotcha204,ov-gotcha05」两个节点。 这样大主播可以使用 cn204 的 fmp4
              流稳定录制。
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
          extraText="哔哩哔哩直播CDN，默认无。"
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
          extraText="CDN 回退（Fallback），默认为关闭。例如海外机器优选 ov05 之后，如果 ov05 流一直无法下载，将会自动回退到 ov07 进行下载。仅限相同流协议。"
          label="CDN 回退（bili_cdn_fallback）"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Switch
          field="bili_anonymous_origin"
          extraText="使用自定义API获取 master playlist 内的 hls_fmp4 原画流，无法录制特殊直播。默认关闭。"
          label="免登录原画（bili_anonymous_origin）"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        {/* <Form.Switch
          field="bili_ov2cn"
          extraText={
            <div style={{ fontSize: '14px' }}>
              将海外cdn域名替换为大陆cdn域名，默认关闭。
              <br />
              例：将海外的 ov-gotcha07 替换为 cn-gotcha07，直播 cdn 处仍然填写 ov-gotcha07。
            </div>
          }
          label="海外转大陆（bili_ov2cn）"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Switch
          field="bili_force_source"
          extraText="移除streamName的二压小尾巴（仅限 hls_fmp4 流，且画质等级 bili_qn >= 10000），默认为关闭，不保证可用性。"
          label="强制获取真原画（bili_force_source）"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Switch
          field="bili_normalize_cn204"
          extraText="去除 cn-gotcha204 后面的小尾巴（-[1-4]）"
          label="标准化 CN204（bili_normalize_cn204）"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        /> */}
        <Form.InputNumber
          field="bili_hls_transcode_timeout"
          extraText="hls_fmp4 转码等待时间，超时后回退到 flv 流。默认 60 秒。"
          label="hls_fmp4 转码等待时间（bili_hls_transcode_timeout）"
          style={{ width: '100%' }}
          placeholder="60"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
          showClear={true}
        />
        {/* <Form.TagInput
          allowDuplicates={false}
          addOnBlur={true}
          separator=","
          field="bili_replace_cn01"
          extraText="该功能在 强制获取真原画 之前生效"
          label="替换 CN01 sid (bili_replace_cn01)"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
          showClear={true}
          placeholder="可用英文逗号分隔以批量输入 sid，失焦/Enter 保存"
          // onChange={v => console.log(v)}
          rules={[
            {
              validator: (rule, value) => {
                value = value ?? []
                return (
                  Array.isArray(value) &&
                  value.every(item => /^cn-[a-z]{2,6}-[a-z]{2,4}(-[0-9]{2}){2}$/.test(item))
                )
              },
              message: '例: cn-hjlheb-cu-01-01,cn-tj-ct-01-01',
            },
          ]}
        /> */}
      </Collapse.Panel>
    </>
  )
}

export default Bilibili
