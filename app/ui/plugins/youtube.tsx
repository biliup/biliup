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

const YouTube: React.FC<Props> = props => {
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
      <PlatformPanel header="YouTube" itemKey="youtube" bare={bare}>
        <Form.Input
          field="user.youtube_cookie"
          extraText={
            <div style={{ fontSize: '14px' }}>
              填 Cookie <strong>文件的路径</strong>（不是 Cookie 内容），用于以登录状态下载会员限定、私享等未登录无法访问的内容。
              <br />
              文件需为 Netscape 格式的 cookies.txt，可用 Chrome 插件「Get cookies.txt」导出。
            </div>
          }
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
          extraText={
            <div style={{ fontSize: '14px' }}>
              是否录制正在进行的直播，默认开启。
              <br />
              关闭后跳过正在直播的条目，只下载回放和普通视频。有些网络环境只能下回放、拉直播会被风控，且大量下载时直播极易触发风控——<strong>对实时性要求不高时建议关闭</strong>。
              <br />
              同一频道同时开多场直播时只录最新的一场；正在录直播时不会同时下载回放。
            </div>
          }
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
          extraText={
            <div style={{ fontSize: '14px' }}>
              是否下载直播回放，默认开启。
              <br />
              关闭后跳过直播回放，只录直播和下载普通视频；只想录直播的可以关闭。正在下载回放时不会同时录直播。
            </div>
          }
          label="下载回放（youtube_enable_download_playback）"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Input
          field="youtube_after_date"
          extraText="只下载发布日期不早于该日的视频，默认不限制。格式 YYYYMMDD，如 20220201。"
          label="下载起始日期（youtube_after_date）"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Input
          field="youtube_before_date"
          extraText="只下载发布日期不晚于该日的视频，默认不限制。格式 YYYYMMDD，如 20230501；与「下载起始日期」一起用可限定一个日期区间。"
          label="下载截止日期（youtube_before_date）"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Input
          field="youtube_max_videosize"
          extraText={
            <div style={{ fontSize: '14px' }}>
              单个视频的大小上限，默认不限制；对直播无效。格式如 100M、5G、10G。
              <br />
              只计视频轨、不含音频；优先级高于下方「视频分辨率上限」。部分视频没有大小信息会导致匹配不到可用格式，<strong>推荐改用分辨率上限来控制画质</strong>。
            </div>
          }
          label="视频大小上限（youtube_max_videosize）"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.InputNumber
          field="youtube_max_resolution"
          extraText="下载的最高纵向分辨率，默认不限制。例如填 1080，最高只下载 1080P。"
          label="视频分辨率上限（youtube_max_resolution）"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Input
          field="youtube_prefer_vcodec"
          extraText={
            <div style={{ fontSize: '14px' }}>
              只下载指定视频编码，默认不限制。可填 avc、vp9、av01，多个用 | 分隔（如
              av01|vp9|avc），会在指定编码中自动选画质最好的。
              <br />
              avc 最高约 1080p；vp9 最高 4K、少数 8K；av01 不是所有视频都有，但多数 8K 视频只有 av01。
              <br />
              需要安装 FFmpeg 合并音视频。无特殊需求不建议筛选，尤其录直播时多数 mp4 格式不可用；B 站支持
              mp4 / mkv / webm，不筛选也能正常上传。
            </div>
          }
          label="偏好视频编码（youtube_prefer_vcodec）"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Input
          field="youtube_prefer_acodec"
          extraText={
            <div style={{ fontSize: '14px' }}>
              只下载指定音频编码，默认不限制。可填 opus、mp4a，多个用 | 分隔（如 opus|mp4a）。
              <br />
              opus 最高 48 kHz 采样，mp4a（AAC）最高 44.1 kHz，理论上 opus 音质更好。
              <br />
              想得到特定封装格式时按此搭配视频 / 音频编码：mp4 → avc+mp4a 或 av01+mp4a；mkv → vp9+mp4a 或
              avc+opus；webm → av01+opus 或 vp9+opus。
            </div>
          }
          label="偏好音频编码（youtube_prefer_acodec）"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
      </PlatformPanel>
    </>
  )
}
export default YouTube
