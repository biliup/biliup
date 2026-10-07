'use client'
import React, { useEffect } from 'react'
import { Form, Radio, Select, useFormApi } from '@douyinfe/semi-ui'
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
      const response = await fetch('/api/v1/douyu/validate-cookie', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ cookie: cookieValue }),
      })

      const data = await response.json()
      setTestResult({
        success: data.valid,
        message: data.message || (data.valid ? 'Cookie有效' : 'Cookie无效'),
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
          <Form.Input
            field="douyu_cookie"
            label="登录 Cookie（douyu_cookie）"
            placeholder="acf_username=xxx; acf_uid=xxx; acf_auth=xxx; acf_did=xxx; ..."
            extraText={
              <div style={{ fontSize: '14px' }}>
                斗鱼网页版登录 Cookie（www.douyu.com 的完整 Cookie）。
                <br />
                <strong>自 2026 年 9 月起，原画（1080P60/2K）和蓝光4M等高码率需要登录才能获取。</strong>
                <br />
                必需字段：<code>acf_uid</code> 和 <code>acf_auth</code>（用于身份验证）
                <br />
                登录斗鱼账号后，从浏览器开发者工具的 Network 面板中复制完整 Cookie 字符串粘贴到这里。
                <br />
                留空时只能获取较低画质（最高超清）。
                <br />
                <a href="/docs/tutorials/douyu-cookie-guide" target="_blank" style={{ color: '#1890ff' }}>
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
          extraText="在浏览器登录自己的斗鱼账号后，从 Cookie 中复制 acf_did 的值粘贴到这里；留空时使用默认设备 ID。"
          style={{ width: '100%' }}
        />
        <Form.RadioGroup
          field="douyu_codec"
          label="视频编码（douyu_codec）"
          mode="advanced"
          extraText={
            <div style={{ fontSize: '14px' }}>
              默认 AVC。
              <br />
              <strong style={{ color: '#ff4d4f' }}>
                ⚠️ 重要提示：HEVC 编码仅在使用 mesio 或 ffmpeg 下载器时有效。
              </strong>
              <br />
              使用 stream-gears 下载器时，即使选择 HEVC，如果收到 HEVC 流也会导致录制失败。
              <br />
              建议：除非明确需要 HEVC 且已配置兼容的下载器，否则请选择 AVC。
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
              录制画质，默认 0（最高画质）。可选：0 最高画质 / 8 蓝光 8M / 4 蓝光 4M / 3 超清 / 2
              高清，也可手动输入其他数值。
              <br />
              <strong style={{ color: '#1890ff' }}>
                💡 提示：原画（0）和蓝光4M（4）需要登录 Cookie 才能获取。
              </strong>
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
          <Select.Option value={0}>最高画质/原画（0）⭐ 需要登录</Select.Option>
          <Select.Option value={8}>蓝光8M（8）</Select.Option>
          <Select.Option value={4}>蓝光4M（4）⭐ 需要登录</Select.Option>
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
