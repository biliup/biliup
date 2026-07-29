'use client'
import React, { useEffect } from 'react'
import { Form, Select, Collapse, useFormApi } from '@douyinfe/semi-ui'

type Props = {
  entity: any
  list: any
  initValues?: Record<string, any>
}

const Douyu: React.FC<Props> = props => {
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
      <Collapse.Panel header="斗鱼" itemKey="douyu">
        <Form.Select
          allowCreate={true}
          filter
          field="douyu_rate"
          extraText="画质等级，默认原画。0 原画 / 8 蓝光8M / 4 蓝光4M / 3 超清 / 2 高清；开播无选档先录原画，分段后（ffmpeg/streamlink）切换。"
          label="画质等级（douyu_rate）"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
          rules={[
            {
              pattern: /^\d*$/,
              message: '请仅输入纯数字',
            },
          ]}
          showClear={true}
        >
          <Select.Option value={0}>最高画质（0）</Select.Option>
          <Select.Option value={8}>蓝光8M（8）</Select.Option>
          <Select.Option value={4}>蓝光4M（4）</Select.Option>
          <Select.Option value={3}>超清（3）</Select.Option>
          <Select.Option value={2}>高清（2）</Select.Option>
        </Form.Select>
        <Form.Switch
          field="douyu_danmaku"
          extraText="录制斗鱼弹幕，默认关闭"
          label="录制弹幕（douyu_danmaku）"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Select
          allowCreate={true}
          filter
          field="douyu_cdn"
          extraText="遇卡顿可切换线路：tctc-h5(4)/tct-h5(5)/ali-h5(6)/hw-h5(7)/hs-h5(13)。"
          label="访问线路（douyu_cdn）"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
          showClear={true}
        >
          <Select.Option value="tctc-h5">线路4（tctc-h5）</Select.Option>
          <Select.Option value="tct-h5">线路5（tct-h5）</Select.Option>
          <Select.Option value="ali-h5">线路6（ali-h5）</Select.Option>
          <Select.Option value="hw-h5">线路7（hw-h5）</Select.Option>
          <Select.Option value="hs-h5">线路13（hs-h5）</Select.Option>
        </Form.Select>
        <Form.Switch
          field="douyu_force_hs"
          extraText="强制 hs 流构造链接防断流，需 douyu_cdn 设为 hs-h5。"
          label="强制 hs 流（douyu_force_hs）"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Switch
          field="douyu_disable_interactive_game"
          extraText="主播运行互动游戏时下个分段拒绝录制（小窗也算），谨慎开启。"
          label="斗鱼拒绝互动游戏（douyu_disable_interactive_game）"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
      </Collapse.Panel>
    </>
  )
}

export default Douyu
