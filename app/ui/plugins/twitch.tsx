'use client'
import React, { useEffect } from 'react'
import { Form, Select, Collapse, useFormApi } from '@douyinfe/semi-ui'

type Props = {
  entity: any
  list: any
  initValues?: Record<string, any>
}

const Twitch: React.FC<Props> = props => {
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
      <Collapse.Panel header="Twitch" itemKey="twitch">
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
          extraText="去广告，默认开启。去广告会导致分段（遇广告即断）；需完整一整段可关闭（但有紫色广告屏）。或开 Turbo 会员并填下方 cookie。"
          label="去除广告（twitch_disable_ads）"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Input
          field="user.twitch_cookie"
          extraText={
            <span>
              【仅 Turbo 会员】填 cookie 可大幅减少广告。Cookie 会过期（约 4 个月以上），失效时录制忽略 Cookie。获取：twitch.tv 打开 F12 执行{' '}
              <code style={{ color: 'var(--semi-color-primary)' }}>
                document.cookie.split(&quot;; &quot;).find(i =&gt; i.startsWith(&quot;auth-token=&quot;))?.split(&quot;=&quot;)[1]
              </code>
              。需 downloader=&quot;ffmpeg&quot; 才生效。
            </span>
          }
          label="Twitch Cookie（twitch_cookie）"
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

export default Twitch
