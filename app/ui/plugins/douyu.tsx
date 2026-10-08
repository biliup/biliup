'use client'
import React, { useEffect } from 'react'
import { Form, Radio, Select, useFormApi } from '@douyinfe/semi-ui'
import { fetcher } from '../../lib/api-streamer'
import PlatformPanel, { digitsToNumber } from './PlatformPanel'

type Props = {
  entity: any
  list: any
  initValues?: Record<string, any>
  bare?: boolean
}

const Douyu: React.FC<Props> = props => {
  const { entity, list, initValues, bare } = props
  const formApi = useFormApi()

  useEffect(() => {
    if (initValues) {
      Object.entries(initValues).forEach(([key, value]) => {
        formApi.setValue(key, value)
      })
    }
  }, [initValues, formApi])

  const [testingCookie, setTestingCookie] = React.useState(false)
  const [testResult, setTestResult] = React.useState<{ success: boolean; message: string } | null>(null)

  const handleTestCookie = async () => {
    const cookieValue = formApi.getValue('douyu_cookie')
    if (!cookieValue || cookieValue.trim() === '') {
      setTestResult({ success: false, message: 'Cookie不能为空' })
      return
    }

    setTestingCookie(true)
    setTestResult(null)

    try {
      const data: unknown = await fetcher('/v1/douyu/validate-cookie', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ cookie: cookieValue }),
      })

      if (!data || typeof data !== 'object' || !('valid' in data) || typeof data.valid !== 'boolean') {
        throw new Error('Cookie 验证接口返回了无效响应，请检查前后端版本是否一致')
      }
      setTestResult({
        success: data.valid,
        message:
          'message' in data && typeof data.message === 'string' && data.message
            ? data.message
            : data.valid
              ? 'Cookie有效'
              : 'Cookie无效或已过期',
      })
    } catch (error) {
      setTestResult({
        success: false,
        message: `测试失败: ${error instanceof Error ? error.message : '未知错误'}`,
      })
    } finally {
      setTestingCookie(false)
    }
  }

  return (
    <>
      <PlatformPanel header="斗鱼" itemKey="douyu" bare={bare}>
        <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
          <Form.TextArea
            field="douyu_cookie"
            label="登录 Cookie（douyu_cookie）"
            placeholder="acf_username=xxx; acf_uid=xxx; acf_auth=xxx; acf_did=xxx; ..."
            autosize={{ minRows: 2, maxRows: 6 }}
            extraText={
              <div style={{ fontSize: '14px' }}>
                斗鱼网页版登录 Cookie（www.douyu.com 的完整 Cookie）。
                <br />
                部分直播间的原画需要登录；匿名请求可能被平台降到较低档位。
                <br />
                登录凭据应包含 <code>acf_uid</code> 和 <code>acf_auth</code>。
                <br />
                支持 Network 面板里的 Cookie 字符串，也支持浏览器插件导出的 Cookie JSON 数组。
                <br />
                留空时尝试匿名取流，实际画质以平台返回为准。
                <br />
                <a
                  href="https://biliup.github.io/biliup/docs/tutorials/douyu-cookie-guide/"
                  target="_blank"
                  rel="noopener noreferrer"
                  style={{ color: '#1890ff' }}
                >
                  📖 查看详细Cookie获取教程
                </a>
              </div>
            }
            style={{ width: '100%' }}
          />
          <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
            <button
              type="button"
              onClick={handleTestCookie}
              disabled={testingCookie}
              style={{
                padding: '6px 16px',
                backgroundColor: '#1890ff',
                color: 'white',
                border: 'none',
                borderRadius: '4px',
                cursor: testingCookie ? 'not-allowed' : 'pointer',
                opacity: testingCookie ? 0.6 : 1,
                fontSize: '14px',
              }}
            >
              {testingCookie ? '测试中...' : '🔍 测试Cookie'}
            </button>
            {testResult && (
              <span
                style={{
                  fontSize: '14px',
                  color: testResult.success ? '#52c41a' : '#ff4d4f',
                }}
              >
                {testResult.success ? '✅' : '❌'} {testResult.message}
              </span>
            )}
          </div>
        </div>
        <Form.Input
          field="douyu_deviceId"
          label="设备 ID（douyu_deviceId）"
          placeholder="10000000000000000000000000001511"
          extraText="可填写 Cookie 中的设备 ID；留空时优先使用 dy_did，其次 acf_did，再使用默认设备 ID。"
          style={{ width: '100%' }}
        />
        <Form.RadioGroup
          field="douyu_codec"
          label="视频编码（douyu_codec）"
          mode="advanced"
          extraText={
            <div style={{ fontSize: '14px' }}>
              默认 AVC。HEVC 需直播间提供对应视频流，并使用 mesio 或支持该格式的新版 FFmpeg。
              <br />
              stream-gears 不支持 HEVC；平台未提供 HEVC 地址时会回退到 AVC，并在日志中提示。
            </div>
          }
          initValue={entity?.douyu_codec ?? ''}
        >
          <Radio value="AVC">AVC (H.264) - 兼容所有下载器</Radio>
          <Radio value="HEVC">HEVC (H.265) - 需要 mesio/ffmpeg 下载器</Radio>
        </Form.RadioGroup>
        <Form.Select
          allowCreate={true}
          filter
          field="douyu_rate"
          convert={digitsToNumber}
          extraText={
            <div style={{ fontSize: '14px' }}>
              默认 0（请求最高画质）。常见档位：0 原画 / 8 蓝光 8M / 4 蓝光 4M / 3 超清 / 2
              高清；档位编号随房间变化，也可手动输入其他数值。平台可能限制匿名用户的画质，
              降档时会在日志中显示请求档位和实际档位。
              <br />
              刚开播时可能只有原画，会先录原画；下载插件为 ffmpeg / streamlink
              时之后每次分段会重新取流并切到所选画质，stream-gears / mesio 整场沿用首次取到的流。
            </div>
          }
          label="画质等级（douyu_rate）"
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
          <Select.Option value={0}>最高画质/原画（0）</Select.Option>
          <Select.Option value={8}>蓝光8M（8）</Select.Option>
          <Select.Option value={4}>蓝光4M（4）</Select.Option>
          <Select.Option value={3}>超清（3）</Select.Option>
          <Select.Option value={2}>高清（2）</Select.Option>
        </Form.Select>
        <Form.Switch
          field="douyu_danmaku"
          extraText="录制斗鱼弹幕，默认关闭"
          label="录制弹幕（douyu_danmaku）"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Select
          allowCreate={true}
          filter
          field="douyu_cdn"
          extraText="录制卡顿时可尝试切换线路，默认 hw-h5（线路 7）。可选：tct-h5（线路 5）hw-h5（线路 7）、hs-h5（线路 13）。"
          label="访问线路（douyu_cdn）"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
          showClear={true}
        >
          <Select.Option value="tct-h5">线路5（tct-h5）</Select.Option>
          <Select.Option value="hw-h5">线路7（hw-h5）</Select.Option>
          <Select.Option value="hs-h5">线路13（hs-h5）</Select.Option>
        </Form.Select>
        <Form.Switch
          field="douyu_force_hs"
          extraText="默认关闭，不保证可用性。开启后由 biliup 重新生成流地址，可缓解部分海外机器频繁断流的问题。仅在访问线路（douyu_cdn）为 hs-h5 时生效。"
          label="强制 火山引擎CDN 流（douyu_force_hs）"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Switch
          field="douyu_disable_interactive_game"
          extraText="默认关闭。开启后，检测到主播正在运行互动游戏时按未开播处理，不会开始或继续录制。小窗运行互动游戏也算在内，请谨慎开启。"
          label="斗鱼拒绝互动游戏（douyu_disable_interactive_game）"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
      </PlatformPanel>
    </>
  )
}

export default Douyu
