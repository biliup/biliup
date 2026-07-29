'use client'
import React, { useEffect } from 'react'
import { Form, Select, Collapse, useFormApi } from '@douyinfe/semi-ui'

type Props = {
  entity: any
  list: any
  initValues?: Record<string, any>
}

const YouTube: React.FC<Props> = props => {
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
      <Collapse.Panel header="YouTube" itemKey="youtube">
        <Form.Input
          field="user.youtube_cookie"
          extraText="登录 YouTube 账号，用于下载会限/私享等未登录不可访问内容。可用 Chrome 插件「Get cookies.txt」生成 Netscape 格式文件。"
          label="YouTube Cookie（youtube_cookie）"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Switch
          initValue={
            entity?.hasOwnProperty('youtube_enable_download_live')
              ? entity['youtube_enable_download_live']
              : true
          }
          field="youtube_enable_download_live"
          extraText="下载直播，默认开启。关闭后忽略直播（可下载回放），降低风控；多直播同时仅录最新一个。"
          label="下载直播（youtube_enable_download_live）"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Switch
          initValue={
            entity?.hasOwnProperty('youtube_enable_download_playback')
              ? entity['youtube_enable_download_playback']
              : true
          }
          field="youtube_enable_download_playback"
          extraText="下载回放，默认开启。关闭后忽略回放（不影响普通视频）；录直播时无法下回放。"
          label="下载回放（youtube_enable_download_playback）"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Input
          field="youtube_after_date"
          extraText="仅下载该日期之后的视频，默认不限制。"
          label="下载起始日期（youtube_after_date）"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Input
          field="youtube_before_date"
          extraText="仅下载该日期之前的视频，可与起始日期配合成区间，默认不限制。"
          label="下载截止日期（youtube_before_date）"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Input
          field="youtube_max_videosize"
          extraText="单个视频大小上限，默认不限制（直播无此功能）。不含音频；优先级高于分辨率；部分视频不支持，推荐用分辨率限制。"
          label="视频大小上限（youtube_max_videosize）"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.InputNumber
          field="youtube_max_resolution"
          extraText="偏好最高纵向分辨率，默认不限制。如 1080 即最高 1080P。"
          label="视频分辨率上限（youtube_max_resolution）"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Input
          field="youtube_prefer_vcodec"
          label="偏好视频封装格式（youtube_prefer_vcodec）"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Input
          field="youtube_prefer_acodec"
          extraText="偏好音频/视频封装格式，默认不限制。需装 ffmpeg；录制直播时多数 mp4 不可用。推荐：mp4:avc+mp4a; mkv:vp9+mp4a/avc+opus; webm:av01+opus/vp9+opus。avc≤1080p, vp9≤4k, av01 多 8k。"
          label="偏好音频封装格式（youtube_prefer_acodec）"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
      </Collapse.Panel>
    </>
  )
}
export default YouTube
