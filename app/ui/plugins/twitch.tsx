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
          extraText="录制Twitch弹幕，默认关闭"
          label="录制弹幕（twitch_danmaku）"
        />
        <Form.Switch
          initValue={
            entity?.hasOwnProperty('twitch_disable_ads') ? entity['twitch_disable_ads'] : true
          }
          field="twitch_disable_ads"
          extraText="去除Twitch广告功能，默认开启
这个功能会导致Twitch录播分段，因为遇到广告就自动断开了，这就是去广告。若需要录播完整一整段可以关闭这个，但是关了之后就会有紫色屏幕的CommercialTime
还有一个办法是去花钱开一个Turbo会员，然后下面的user里把twitch的cookie填上，也能去除广告"
          label="去除广告（twitch_disable_ads）"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Input
          field="user.twitch_cookie"
          extraText={
            <div className="semi-form-field-extra">
              【仅限Turbo会员】如录制Twitch时遇见视频流中广告过多的情况，可尝试在此填入cookie，可以大幅减少视频流中的twitch广告
              <br />
              该 Cookie 存在过期风险，Cookie过期后会在日志输出警告，请注意及时更换。
              <br />
              当 Cookie 失效，录制时将忽略 Cookie。（经作者个人测试，可保持未过期状态四个月以上）
              <br />
              twitch_cookie 获取方式：在浏览器中打开 twitch.tv ， F12
              调出控制台，在控制台中执行如下代码：
              <br />
              <code
                style={{ color: 'blue' }}
              >{`document.cookie.split("; ").find(item => item.startsWith("auth-token="))?.split("=")[1]`}</code>
              <br />
              twitch_cookie&nbsp;需要在&nbsp;downloader= &quot;ffmpeg&quot;&nbsp;时候才会生效。
            </div>
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
