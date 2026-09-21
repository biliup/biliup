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

const TwitCasting: React.FC<Props> = props => {
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
      <PlatformPanel header="TwitCasting" itemKey="twitcasting" bare={bare}>
        <Form.Select
          field="twitcasting_quality"
          extraText="录制画质，默认取最高可用画质。所选画质不存在时自动降到更低一档；更低的也没有时取最高可用画质。"
          label="画质等级（twitcasting_quality）"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
          showClear={true}
        >
          <Select.Option value="high">高画质（high）</Select.Option>
          <Select.Option value="medium">中画质（medium）</Select.Option>
          <Select.Option value="low">低画质（low）</Select.Option>
        </Form.Select>
        <Form.Switch
          field="twitcasting_danmaku"
          extraText="录制 TwitCasting 弹幕，默认关闭"
          label="录制弹幕（twitcasting_danmaku）"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Input
          field="user.twitcasting_cookie"
          extraText={
            <div style={{ fontSize: '14px' }}>
              TwitCasting 登录 Cookie，可选。格式：
              <code style={{ color: 'var(--semi-color-primary)' }}>tc_id=xxxxxxx; tc_ss=xxxxxxx;</code>
            </div>
          }
          label="TwitCasting Cookie（twitcasting_cookie）"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Input
          field="twitcasting_password"
          extraText="直播间设有观看密码时填写，未设密码留空。"
          label="TwitCasting直播间密码（twitcasting_password）"
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

export default TwitCasting
