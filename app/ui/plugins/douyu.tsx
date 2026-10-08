'use client'
import React, { useEffect } from 'react'
import { Form, Radio, RadioGroup, Select, useFormApi, useFormState } from '@douyinfe/semi-ui'
import { usePathname } from 'next/navigation'
import { useMe } from '../../lib/use-me'
import {
  douyuCookieOverrideMode,
  douyuRoomCookieValue,
  testDouyuCookie,
  type DouyuCookieMode,
} from '../../lib/douyu-auth'
import DouyuAuthPanel from './DouyuAuthPanel'
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
  const { values } = useFormState<Record<string, unknown>>()
  const pathname = usePathname()
  const { can } = useMe()
  // Fleet editors reuse the platform forms while editing remote configuration.
  // Local credential actions must never operate on the controller's own account there.
  const fleetEditor = pathname === '/nodes' || pathname?.startsWith('/nodes/')
  const localConfig = !!bare && !fleetEditor
  const roomOverride = !bare && !fleetEditor
  const roomId = typeof entity?.id === 'number' && entity.id > 0 ? entity.id : undefined
  const canTestCookie = !fleetEditor && can('config.edit')
  const [customCookieInput, setCustomCookieInput] = React.useState(false)
  const cookieValue = values?.douyu_cookie
  const savedCookieMode = douyuCookieOverrideMode(cookieValue === undefined ? initValues?.douyu_cookie : cookieValue)
  const cookieMode: DouyuCookieMode = customCookieInput && savedCookieMode === 'inherit' ? 'custom' : savedCookieMode

  useEffect(() => {
    if (initValues) {
      Object.entries(initValues).forEach(([key, value]) => {
        formApi.setValue(key, value)
      })
    }
  }, [initValues, formApi])

  const selectCookieMode = (mode: DouyuCookieMode) => {
    // Keep an empty independent input available while typing; saving it still inherits login.
    setCustomCookieInput(mode === 'custom')
    formApi.setValue('douyu_cookie', douyuRoomCookieValue(formApi.getValue('douyu_cookie'), mode))
    setTestResult(null)
  }

  const [testingCookie, setTestingCookie] = React.useState(false)
  const [testResult, setTestResult] = React.useState<{ success: boolean; message: string } | null>(null)

  const handleTestCookie = async () => {
    if (!canTestCookie || testingCookie) return
    const cookieValue = formApi.getValue('douyu_cookie')
    if (typeof cookieValue !== 'string' || cookieValue.trim() === '') {
      setTestResult({ success: false, message: 'Cookie不能为空' })
      return
    }

    setTestingCookie(true)
    setTestResult(null)

    try {
      const valid = await testDouyuCookie(cookieValue)
      if (formApi.getValue('douyu_cookie') !== cookieValue) return
      setTestResult({
        success: valid,
        message: valid ? 'Cookie有效（已通过登录验证）' : 'Cookie无效或已过期，请重新登录并更新',
      })
    } catch {
      if (formApi.getValue('douyu_cookie') !== cookieValue) return
      setTestResult({
        success: false,
        message: '登录验证未完成，请检查网络连接或稍后重试',
      })
    } finally {
      setTestingCookie(false)
    }
  }

  return (
    <>
      <PlatformPanel header="斗鱼" itemKey="douyu" bare={bare}>
        {roomOverride && <div style={{ marginBottom: 12 }}>
          <div style={{ marginBottom: 8 }}>此房间的斗鱼登录来源</div>
          <RadioGroup value={cookieMode} onChange={event => selectCookieMode(event.target.value as DouyuCookieMode)}>
            <Radio value="inherit">继承空间配置</Radio>
            <Radio value="custom">此房间独立 Cookie</Radio>
            <Radio value="anonymous">此房间匿名取流</Radio>
          </RadioGroup>
          <div style={{ marginTop: 8, fontSize: 13, color: 'var(--semi-color-text-2)' }}>
            {cookieMode === 'inherit' ? '使用「空间配置 → 平台设置 → 斗鱼」中已保存的 Cookie，并共享其续期结果。无需在此重复填写。'
              : cookieMode === 'anonymous' ? '保存后此房间不使用空间配置中的登录凭据，平台可能限制原画画质。'
                : '仅此房间使用下面的 Cookie；输入框留空时仍继承空间配置。独立账号不会复用空间配置的续期凭据。'}
          </div>
        </div>}
        <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
          <Form.TextArea
            field="douyu_cookie"
            label="登录 Cookie（douyu_cookie）"
            placeholder={roomOverride && cookieMode === 'inherit' ? '已选择继承空间配置，无需重复填写 Cookie' : 'acf_username=xxx; acf_uid=xxx; acf_auth=xxx; acf_did=xxx; ...'}
            disabled={roomOverride && cookieMode !== 'custom'}
            convert={roomOverride ? value => douyuRoomCookieValue(value, cookieMode) : undefined}
            onChange={() => setTestResult(null)}
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
                {localConfig && <><br />完整 JSON 可同时包含 www.douyu.com 和 passport.douyu.com 的 Cookie，保存时会分离续期凭据。</>}
                <br />
                {roomOverride ? '此房间默认继承空间配置；需要匿名取流时请明确选择上方「此房间匿名取流」。' : '留空时尝试匿名取流，实际画质以平台返回为准。'}
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
          {canTestCookie && (!roomOverride || cookieMode === 'custom') && <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
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
          </div>}
        </div>
        {localConfig && <>
          <Form.Input
            field="douyu_ltp0"
            label="长期续期凭据 LTP0（douyu_ltp0）"
            mode="password"
            autoComplete="new-password"
            placeholder="passport.douyu.com 的 LTP0；完整 Cookie 导出包含时可留空"
            extraText="用于自动换取新的登录 Cookie。请与下方 dy_did 使用同一账号、同一次浏览器登录取得的值；这些敏感字段不通过 Fleet 配置下发。"
            style={{ width: '100%' }}
          />
          <Form.Input
            field="douyu_refresh_device_id"
            label="续期设备标识 dy_did（douyu_refresh_device_id）"
            autoComplete="off"
            placeholder="与 LTP0 配套的 dy_did；完整 Cookie 导出包含时可留空"
            extraText="续期使用 passport 登录对应的 dy_did。它与下方取流设备 ID 分开；请勿混用其他账号的设备号。"
            style={{ width: '100%' }}
          />
          <Form.Switch
            field="douyu_auto_refresh"
            label="自动续期登录 Cookie（douyu_auto_refresh）"
            initValue={entity?.douyu_auto_refresh ?? true}
            extraText="默认开启；保存有效 LTP0 和 dy_did 后每 3 天续期，失败时自动退避重试，重启后恢复。续期成功会更新正在运行的 Web API 请求。"
          />
          <DouyuAuthPanel savedValues={entity ?? {}} />
        </>}
        {roomOverride && roomId !== undefined && <DouyuAuthPanel savedValues={initValues ?? {}} streamerId={roomId} />}
        {roomOverride && can('config.edit') && <div style={{ fontSize: 13, color: 'var(--semi-color-text-2)', margin: '12px 0' }}>
          自动续期凭据与登录状态请在本机「空间配置 → 平台设置 → 斗鱼」中管理。
          此处测试的是当前表单里的 Cookie，尚未保存的修改不会用于自动续期。
        </div>}
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
