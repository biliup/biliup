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

const CC: React.FC<Props> = props => {
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
      <PlatformPanel header="CC" itemKey="cc" bare={bare}>
        <Form.Select
          field="cc_protocol"
          extraText="CC 直播流协议，默认 hls。录制经常异常断开、分段过多时可尝试切换为 flv。"
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
      </PlatformPanel>
    </>
  )
}

export default CC
