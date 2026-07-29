'use client'
import React, { useEffect } from 'react'
import { Form, Select, Collapse, useFormApi } from '@douyinfe/semi-ui'

type Props = {
  entity: any
  list: any
  initValues?: Record<string, any>
}

const CC: React.FC<Props> = props => {
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
      <Collapse.Panel header="CC" itemKey="cc">
        <Form.Select
          field="cc_protocol"
          extraText="直播流协议；录制经常分段时可尝试切换。"
          label="直播流协议（cc_protocol）"
          placeholder="hls（默认）"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
          showClear={true}
        >
          <Select.Option value="flv">flv</Select.Option>
          <Select.Option value="hls">hls（默认）</Select.Option>
        </Form.Select>
      </Collapse.Panel>
    </>
  )
}

export default CC
