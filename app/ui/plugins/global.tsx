'use client'
import React from 'react'
import styles from '../../styles/dashboard.module.scss'
import SectionTitle from '../../(app)/components/SectionTitle'
import { Form, Select, Space, Switch, useFormApi, useFormState } from '@douyinfe/semi-ui'
import { IconUpload, IconDownload } from '@douyinfe/semi-icons'
import { FileSizeField } from '../FileSizeInput'

/** 打开「投稿后保留录像」时默认保留的小时数 */
const DEFAULT_RETENTION_HOURS = 24

type Props = {
  /** 只读角色：表单整体禁用，不是表单字段的开关也要跟着禁用 */
  disabled?: boolean
}

const Global: React.FC<Props> = ({ disabled }) => {
  // useFormApi 不订阅表单值变化，切换下拉框后条件渲染不会刷新；useFormState 会
  const { values } = useFormState()
  const formApi = useFormApi()
  const isSyncDownloader = values?.downloader === 'sync-downloader'
  // 开关不是表单字段（整体覆盖保存时不能多出键），状态由小时数推出来：大于 0 即开
  const retentionOn = Number(values?.retention_hours) > 0

  return (
    <>
      {/* 全局下载 */}
      <div className={styles.frameDownload}>
        <SectionTitle icon={<IconDownload size="small" />} title="全局下载设置" />
        <Form.Select
          label="下载插件（downloader）"
          field="downloader"
          placeholder="mesio（默认）"
          extraText={
            <div style={{ fontSize: '14px' }}>
              全局默认的下载插件，可在单个主播的覆写设置里另选。可选：
              <br />
              1. <strong>mesio</strong>（默认）：内置 rust-srec 引擎，无需额外安装。进程内下载 FLV /
              HLS，修复时间戳（每段从 0 开始）、写入关键帧索引（onMetaData.keyframes），支持 HEVC 和
              hls_fmp4；不转封装，按源站的容器保存（FLV / TS，hls_fmp4 存为 .mp4）。详见{' '}
              <a
                href="https://github.com/hua0512/rust-srec"
                target="_blank"
                rel="noopener noreferrer"
                style={{ color: 'var(--semi-color-link)' }}
              >
                项目主页
              </a>
              。
              <br />
              2. ffmpeg：非 Docker 用户需自行安装 FFmpeg。
              <br />
              3. streamlink：多线程下载 HLS 分片，也可下载 FLV 直链；需系统中有 streamlink 命令。
              <br />
              4. sync-downloader（边录边传）：录制的同时流式上传，
              <strong>需先为主播设置上传模板</strong>；不受 pool2 / threads / segment_time
              控制，固定 3 线程上传，请确保上传带宽充足；非 Docker 用户需自行安装 FFmpeg。详见 Wiki{' '}
              <a
                href="https://github.com/biliup/biliup/wiki/%E8%BE%B9%E5%BD%95%E8%BE%B9%E4%BC%A0%E5%8A%9F%E8%83%BD"
                target="_blank"
                rel="noopener noreferrer"
                style={{ color: 'var(--semi-color-link)' }}
              >
                边录边传功能
              </a>
              。
              <br />
              5. ytarchive：仅适用于 YouTube 直播。
              <br />
              6. stream-gears：内置，无需额外安装，可防 FLV 流花屏；不支持 HEVC 编码和 hls_fmp4
              流，FLV 时间戳沿用源站的原值（不从 0 开始）。
            </div>
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
          <Select.Option value="stream-gears">stream-gears</Select.Option>
          <Select.Option value="sync-downloader">sync-downloader（边录边传）</Select.Option>
          <Select.Option value="ytarchive">ytarchive（仅适用于 Youtube Live）</Select.Option>
          <Select.Option value="mesio">mesio（默认）</Select.Option>
        </Form.Select>
        {isSyncDownloader ? (
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
                  pattern: /^[^:]*$|^[a-zA-Z]:[\/\\][^:]*$/,
                  message: '冒号只能出现在第二个字符位置，且后面必须连接斜杠',
                },
                {
                  pattern: /^(?!.*?\.{3,})(?!.*?\.{2}(?![\/\\])).*$/,
                  message: '点号最多只能连续出现两次，且后面必须连接斜杠',
                },
                {
                  pattern: /^(?!.*\/\\)(?!.*\\\/).*$/,
                  message: '不允许连接正反斜杠',
                },
                {
                  pattern: /^(?!.*([\\]{3,}|[\/]{2,})).*$/,
                  message: '反斜杠最多只能连续出现两次，正斜杠最多只能连续出现一次',
                },
              ]}
              stopValidateWithError={true}
            />
          </>
        ) : null}
        <FileSizeField
          label="视频分段大小（file_size）"
          extraText={
            <div style={{ fontSize: '14px' }}>
              录像单文件大小上限，超过后开始写下一个文件。下载回放时无法使用。留空表示不按大小分段。
              <br />按 1024 进制换算：1 GB = 1024 MB = 1073741824 字节，与 Windows
              资源管理器显示的大小一致。配置文件里存的仍是字节数。
            </div>
          }
          field="file_size"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Input
          field="segment_time"
          extraText={
            <div style={{ fontSize: '14px' }}>
              录像单文件时间限制，超过此时长触发文件分割。
              <br />
              格式：&apos;00:00:00&apos;（时:分:秒）
            </div>
          }
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
          extraText={
            <div style={{ fontSize: '14px' }}>
              全局文件名模板。可被单个主播文件名模板覆盖。可用变量如下
              <br />
              {'\u007B'}streamer{'\u007D'}: 录播备注（必须保留）
              <span style={{ margin: '0 20px' }}></span>
              {'\u007B'}title{'\u007D'}: 直播标题
              <br />
              %Y-%m-%d %H_%M_%S: 开始录制时的 年-月-日 时_分_秒
            </div>
          }
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
          extraText={<div style={{ fontSize: '14px' }}>开启后无法保证分段后处理先后执行顺序</div>}
          label="视频分段后处理并行（segment_processor_parallel)"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.InputNumber
          field="filtering_threshold"
          extraText={
            <div style={{ fontSize: '14px' }}>
              小于此大小的视频文件将会被过滤删除。
              <br />
              单位：MB
            </div>
          }
          label="碎片过滤（filtering_threshold）"
          suffix={'MB'}
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
          showClear={true}
        />
        <Form.Slot
          label="投稿后保留录像（retention_hours）"
          style={{ alignSelf: 'stretch', padding: 0 }}
        >
          <Switch
            aria-label="投稿后保留录像"
            checked={retentionOn}
            disabled={disabled}
            onChange={on => formApi.setValue('retention_hours', on ? DEFAULT_RETENTION_HOURS : 0)}
          />
        </Form.Slot>
        <Form.InputNumber
          field="retention_hours"
          noLabel={true}
          extraText={
            <div style={{ fontSize: '14px' }}>
              后处理
              rm、边录边传投稿后删除临时文件时，录像先保留这么多小时，到期后由每分钟一次的清理任务删除（连同弹幕
              XML 和 .idx 关键帧索引）。关闭或填 0 表示立即删除，与以前一样。
              <br />
              切片工作台里被标记、切片引用的片段，以及点了「保留这场」的场次，不论这里怎么设都会等引用释放后再删。
            </div>
          }
          min={0}
          precision={0}
          suffix="小时"
          placeholder="0"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <FileSizeField
          field="min_free_space"
          label="磁盘最低可用空间（min_free_space）"
          placeholder="不启用"
          extraText={
            <div style={{ fontSize: '14px' }}>
              录像所在磁盘的可用空间低于这个值时，每分钟检查一次，按「没被切片工作台引用的最旧录像 →
              被引用的最旧录像」逐个删除，直到回到这个值以上。
              正在录的分段不删；只删切片工作台记录过的录像（本版本之后录制的），其它文件不碰。
              <br />
              <strong>会删掉还没投稿的录像</strong>
              ，只作磁盘写满前的兜底。留空表示不启用（默认）。按 1024 进制换算。
            </div>
          }
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />

        <Form.InputNumber
          field="delay"
          label="下播延迟检测（delay)"
          extraText={
            <div style={{ fontSize: '14px' }}>
              当检测到主播下播后，延迟一定时间再次检测确认，避免特殊情况提早启动上传导致分稿件。
              <br />
              单位：秒
              <br />
              默认延迟时间为 0 秒
            </div>
          }
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
          extraText={
            <div style={{ fontSize: '14px' }}>
              单个主播检测间隔时间，单位：秒。比如虎牙有10个主播，每个主播会间隔10秒检测
              <br />
              单位：秒
            </div>
          }
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
          extraText="负责下载事件的线程池大小，用于限制最大同时录制数。"
          label="下载线程池大小（pool1_size）"
          placeholder={5}
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
          showClear={true}
        />
        <Form.Select
          field="preview_transport"
          label="直播预览取流方式（preview_transport）"
          extraText={
            <div style={{ fontSize: '14px' }}>
              <div>
                <strong>经 biliup 中转</strong>
                （默认）：页面里的预览复用正在录制的那一路流，不向直播平台多拉一路。 浏览器与 biliup
                在同一台机器或同一内网时选这个，不多占 CDN 带宽。
              </div>
              <div>
                <strong>浏览器直连 CDN</strong>：biliup 向平台另取一条直链（新
                token，不影响录制那条），浏览器自己去 CDN 拉，媒体流量不经过 biliup，适合 biliup
                部署在异地服务器、浏览器远程访问的情况，省服务器出口带宽。 只有 CDN
                放行跨域的平台能直连（B 站 / 抖音 / 斗鱼 / 虎牙，FLV 与 HLS 都行）；Twitch 等按
                Origin 白名单放行 的平台在直连模式下自动回落中转并在播放器角标标出原因。
              </div>
            </div>
          }
          placeholder="经 biliup 中转（relay）"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
          showClear={true}
        >
          <Form.Select.Option value="relay">经 biliup 中转（relay）</Form.Select.Option>
          <Form.Select.Option value="direct">浏览器直连 CDN（direct）</Form.Select.Option>
        </Form.Select>
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
          <Form.Select.Option value="b-cut-android">
            BCut安卓APP（b-cut-android）
          </Form.Select.Option>
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
          extraText="b站上传线路选择，默认为自动模式，可手动切换为alia, bda2, bldsa, tx, txa, estx, akbd"
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
          extraText="单文件并发上传数,未达到带宽上限时,增大此值可提高上传速度(不要设置过大,部分线路限制为8,如速度不佳优先调整上传线路)"
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
          extraText="录播上传次数上限，防止因意外情况如B站接口抽风、录播本身损坏导致录播反复上传浪费宽带或被B站风控（注：限制是记录在程序上下文中的，重启程序会重置上传次数限制；且为了保证尽量不改动老用户使用逻辑，默认将此值设置为一个较大的值，一般推荐设置为2-3）"
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
          extraText={
            <div style={{ fontSize: '14px' }}>负责上传事件的线程池大小。根据实际带宽设置。</div>
          }
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
          extraText={
            <div style={{ fontSize: '14px' }}>
              使用直播间封面作为投稿封面。此封面优先级低于单个主播指定的自定义封面，保存于cover文件夹下，上传后自动删除。
              <br />
              目前支持平台：哔哩哔哩，克拉克拉，Twitch，YouTube。
            </div>
          }
          label="使用直播间封面作为投稿封面（use_live_cover)"
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
