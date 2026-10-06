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

const Twitch: React.FC<Props> = props => {
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
      <PlatformPanel header="Twitch" itemKey="twitch" bare={bare}>
        <Form.Switch
          field="twitch_danmaku"
          extraText="录制 Twitch 弹幕，默认关闭"
          label="录制弹幕（twitch_danmaku）"
        />
        <Form.Switch
          initValue={
            entity?.hasOwnProperty('twitch_disable_ads') ? entity['twitch_disable_ads'] : true
          }
          field="twitch_disable_ads"
          extraText={
            <div style={{ fontSize: '14px' }}>
              默认开启。去广告的原理是遇到广告就断开重连，因此<strong>录像会在每次广告处分段</strong>。
              <br />
              关闭后录像不再因广告分段，但广告时段会录成紫色的「Commercial Time」画面。
              <br />
              更好的办法是开通 Twitch Turbo 会员并在下方填入 Twitch Cookie，可直接免广告。
              <br />
              <strong>仅下载插件为 streamlink 或 ffmpeg 时生效</strong>；默认的 mesio 和 stream-gears 直接拉流，此开关不起作用。
            </div>
          }
          label="去除广告（twitch_disable_ads）"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Input
          field="user.twitch_cookie"
          extraText={
            <div style={{ fontSize: '14px' }}>
              <strong>仅限 Twitch Turbo 会员</strong>：填入后可大幅减少录像中的广告。
              <br />
              此处填的是 auth-token 的值，不是整段 Cookie。获取方式：浏览器打开 twitch.tv，按 F12
              打开控制台，执行：
              <br />
              <code style={{ color: 'var(--semi-color-primary)' }}>
                {`document.cookie.split("; ").find(item => item.startsWith("auth-token="))?.split("=")[1]`}
              </code>
              <br />
              该值会过期（作者实测可用四个月以上）；失效后日志会输出警告并忽略它继续录制，请及时更换。
              <br />
              <strong>仅下载插件为 streamlink 或 ffmpeg 时生效</strong>。
            </div>
          }
          label="Twitch Cookie（twitch_cookie）"
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

export default Twitch
