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

const Kilakila: React.FC<Props> = props => {
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
      <PlatformPanel header="克拉克拉" itemKey="kilakila" bare={bare}>
        <Form.Select
          field="kila_protocol"
          extraText="直播流协议，默认 hls"
          label="直播流协议（kila_protocol）"
          placeholder="hls"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
          showClear={true}
        >
          <Select.Option value="hls">hls</Select.Option>
          <Select.Option value="flv">flv</Select.Option>
        </Form.Select>
      </PlatformPanel>
    </>
  )
}

export default Kilakila
