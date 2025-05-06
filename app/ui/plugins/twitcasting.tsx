'use client'
import React, { useEffect } from 'react'
import { Form, Select, Collapse, useFormApi } from '@douyinfe/semi-ui'

type Props = {
  entity: any
  list: any
  initValues?: Record<string, any>
}

const TwitCasting: React.FC<Props> = props => {
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
      <Collapse.Panel header="TwitCasting" itemKey="twitcasting">
        <Form.Select
          field="twitcasting_quality"
          extraText={
            <div style={{ fontSize: '14px' }}>
              TwitCasting自选画质，没有选中的画质则会自动选择更低级别清晰度，如更低级别画质依旧没有则选择最清晰的。
            </div>
          }
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
          extraText="录制TwitCasting弹幕，默认关闭"
          label="录制弹幕（twitcasting_danmaku）"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
      <Form.Input
          field="user.twitcasting_cookie"
          extraText={
            <div className="semi-form-field-extra">
              Cookie格式:
              <br />
              <code style={{ color: 'blue' }}>tc_id=xxxxxxx; tc_ss=xxxxxxx;</code>
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
          label="TwitCasting直播间密码（twitcasting_password）"
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

export default TwitCasting
