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
