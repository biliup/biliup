'use client'
import React from 'react'
import { Form, Select, Collapse } from '@douyinfe/semi-ui'

type Props = {
  entity: any
}

const Kilakila: React.FC = () => {
  return (
    <>
      <Collapse.Panel header="克拉克拉" itemKey="kilakila">
        <Form.Select
          field="kila_protocol"
          extraText="直播流协议，默认hls"
          label="直播流协议(kila_protocol)"
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
      </Collapse.Panel>
    </>
  )
}

export default Kilakila
