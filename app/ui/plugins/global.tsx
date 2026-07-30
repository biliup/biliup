'use client'
import React, { useEffect } from 'react'
import styles from '../../styles/dashboard.module.scss'
import SectionTitle from '../../(app)/components/SectionTitle'
import { Form, Select, Space, useFormApi } from '@douyinfe/semi-ui'
import { IconUpload, IconDownload } from '@douyinfe/semi-icons'

const Global: React.FC = () => {
  const formApi = useFormApi()

  return (
    <>
      {/* 全局下载 */}
      <div className={styles.frameDownload}>
        <SectionTitle icon={<IconDownload size="small" />} title="全局下载设置" />
        <Form.Select
          label="下载插件（downloader）"
          field="downloader"
          placeholder="stream-gears（默认）"
          // initValue="stream-gears"
          extraText={
            <span>
              全局默认下载插件：streamlink / ffmpeg 需自备 FFmpeg；stream-gears
              为默认（防 FLV 花屏）；sync-downloader 边录边传（需先设上传模板，
              <a
                href="https://github.com/biliup/biliup/wiki/%E8%BE%B9%E5%BD%95%E8%BE%B9%E4%BC%A0%E5%8A%9F%E8%83%BD"
                target="_blank"
                rel="noreferrer"
                style={{ color: 'rgb(var(--semi-color-link))' }}
              >
                详见文档
              </a>
              ）；ytarchive 仅限 YouTube Live。
            </span>
          }
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
          <Select.Option value="ytarchive">ytarchive（仅适用于 Youtube Live）</Select.Option>
          {/* <Select.Option value="mesio">mesio</Select.Option> */}
        </Form.Select>
        {formApi.getValue('downloader') === 'sync-downloader' ? (
          <>
            <Form.Input
              field="sync_save_dir"
              label="边录边传额外保存本地目录（sync_save_dir）"
              placeholder=""
              style={{ width: '100%' }}
              fieldStyle={{
                alignSelf: 'stretch',
                padding: 0,
              }}
              showClear={true}
              disabled={formApi.getValue('downloader') === 'sync-downloader' ? false : true}
              rules={[
                {
                  pattern: /^[^*|?"<>]*$/,
                  message: '路径中不能包含Windows不允许的字符 * | ? " < >',
                },
                {
                  pattern: /^(?![a-zA-Z]：).*$/,
                  message: '以字母开头时，第二个字符不能是中文冒号',
                },
                {
                  pattern: /^[^:]*$|^[a-zA-Z]:[\\/\\\\][^:]*$/,
                  message: '冒号只能出现在第二个字符位置，且后面必须连接斜杠',
                },
                {
                  pattern: /^(?!.*?\\.{3,})(?!.*?\\.{2}(?![\\/\\\\])).*$/,
                  message: '点号最多只能连续出现两次，且后面必须连接斜杠',
                },
                {
                  pattern: /^(?!.*\\/\\\\)(?!.*\\\\\\/).*$/,
                  message: '不允许连接正反斜杠',
                },
                {
                  pattern: /^(?!.*([\\\\]{3,}|[\/]{2,})).*$/,
                  message: '反斜杠最多只能连续出现两次，正斜杠最多只能连续出现一次',
                },
              ]}
              stopValidateWithError={true}
            />
          </>
        ) : null}
        <Form.InputNumber
          label="视频分段大小（file_size）"
          extraText={'单文件大小上限，超过则分割。单位 Byte（如 4294967296 ≈ 4GB）。下载回放时无效。'}
          field="file_size"
          placeholder=""
          suffix={'Byte'}
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
          showClear={true}
        />
        <Form.Input
          field="segment_time"
          extraText={'单文件时长上限，超过则分割。格式 00:00:00（时:分:秒）。'}
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
              pattern: /^$|^[0-9]{2,4}:[0-5][0-9]:[0-5][0-9]$/,
              message: '分或秒不符合规范',
            },
          ]}
          stopValidateWithError={true}
        />
        <Form.Input
          field="filename_prefix"
          extraText={'全局文件名模板，可被单主播覆盖。{streamer} 录播备注（必填）、{title} 直播标题，支持 %Y-%m-%d %H_%M_%S 时间变量。'}
          label="文件名模板（filename_prefix）"
          placeholder="{streamer}%Y-%m-%dT%H_%M_%S"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
          showClear={true}
        />
        <Form.Switch
          field="segment_processor_parallel"
          extraText={'开启后分段后处理不保证先后顺序。'}
          label="视频分段后处理并行（segment_processor_parallel）"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.InputNumber
          field="filtering_threshold"
          extraText={'小于此大小（MB）的碎片文件会被自动过滤删除。'}
          label="碎片过滤（filtering_threshold）"
          suffix={'MB'}
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
          showClear={true}
        />

        <Form.InputNumber
          field="delay"
          label="下播延迟检测（delay）"
          extraText={'检测到下播后延迟再确认的时间（秒），避免误判提前上传。默认 0。'}
          placeholder="0"
          suffix="s"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
          showClear={true}
        />
        <Form.InputNumber
          field="event_loop_interval"
          extraText={'单个主播检测间隔（秒）。'}
          label="直播事件检测间隔（event_loop_interval）"
          suffix="s"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
          showClear={true}
        />
        <Form.InputNumber
          field="pool1_size"
          extraText="负责下载事件的线程池大小，限制最大同时录制数。"
          label="下载线程池大小（pool1_size）"
          placeholder={5}
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
          showClear={true}
        />
      </div>

      <Space />

      {/* 全局上传 */}
      <div className={styles.frameUpload}>
        <SectionTitle icon={<IconUpload size="small" />} title="全局上传设置" />
        <Form.Select
          field="submit_api"
          label="提交接口（submit_api）"
          extraText="B站投稿提交接口，默认为自动选择。"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
          showClear={true}
        >
          <Form.Select.Option value="app">安卓APP（app）</Form.Select.Option>
          <Form.Select.Option value="b-cut-android">BCut安卓APP（b-cut-android）</Form.Select.Option>
          <Form.Select.Option value="web">网页（web）</Form.Select.Option>
        </Form.Select>
        <Form.Select
          field="uploader"
          label="上传插件（uploader）"
          extraText="全局默认上传插件选择。"
          placeholder="biliup-rs"
          noLabel={true}
          style={{ width: '100%', display: 'none' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
          showClear={true}
          initValue="Noop"
        >
          <Form.Select.Option value="bili_web">bili_web</Form.Select.Option>
          <Form.Select.Option value="biliup-rs">biliup-rs</Form.Select.Option>
          <Form.Select.Option value="Noop">Noop（即不上传，但会执行后处理）</Form.Select.Option>
        </Form.Select>
        <Form.Select
          field="lines"
          label="上传线路（lines）"
          extraText="B站上传线路，默认自动（AUTO）。可选 alia / bda2 / bldsa / tx / txa / estx / akbd 等。"
          placeholder="AUTO（自动，默认）"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
          showClear={true}
        >
          <Form.Select.Option value="AUTO">AUTO（自动，默认）</Form.Select.Option>
          <Form.Select.Option value="alia">alia（海外-阿里云）</Form.Select.Option>
          <Form.Select.Option value="bda2">bda2（大陆-百度云）</Form.Select.Option>
          <Form.Select.Option value="bldsa">bldsa（大陆-B站自建）</Form.Select.Option>
          <Form.Select.Option value="tx">tx（大陆-腾讯云）</Form.Select.Option>
          <Form.Select.Option value="txa">txa（海外-腾讯云）</Form.Select.Option>
          <Form.Select.Option value="estx">estx（大陆-B站自建）</Form.Select.Option>
          <Form.Select.Option value="akbd">akbd（大陆-B站自建）</Form.Select.Option>
        </Form.Select>
        <Form.InputNumber
          field="threads"
          placeholder={3}
          extraText="单文件并发上传数。未达带宽上限时可调大提速（部分线路限 8）。"
          label="上传并发（threads）"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
          showClear={true}
        />
        <Form.InputNumber
          field="max_upload_limit"
          placeholder={8}
          extraText="录播上传次数上限，防止异常时反复上传浪费带宽或被风控。重启程序会重置；默认较大，建议设 2-3。"
          label="上传重试次数限制（max_upload_limit）"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
          showClear={true}
        />

        <Form.InputNumber
          field="pool2_size"
          extraText="负责上传事件的线程池大小。根据实际带宽设置。"
          placeholder={3}
          label="上传线程池大小（pool2_size）"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Switch
          field="use_live_cover"
          extraText="用直播间封面作投稿封面（优先级低于单主播自定义封面）。支持 B站 / 克拉克拉 / Twitch / YouTube。"
          label="使用直播间封面作为投稿封面（use_live_cover）"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
      </div>
    </>
  )
}

export default Global
