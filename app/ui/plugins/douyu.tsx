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

const Douyu: React.FC<Props> = props => {
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
      <PlatformPanel header="斗鱼" itemKey="douyu" bare={bare}>
        <Form.Select
          allowCreate={true}
          filter
          field="douyu_rate"
          extraText={
            <div style={{ fontSize: '14px' }}>
              录制画质，默认 0（最高画质）。可选：0 最高画质 / 8 蓝光 8M / 4 蓝光 4M / 3 超清 / 2
              高清，也可手动输入其他数值。
              <br />
              刚开播时可能只有原画，会先录原画；下载插件为 ffmpeg / streamlink
              时之后每次分段会重新取流并切到所选画质，stream-gears / mesio 整场沿用首次取到的流。
            </div>
          }
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
          extraText="录制卡顿时可尝试切换线路，默认 hw-h5（线路 7）。可选：tctc-h5（线路 4）、tct-h5（线路 5）、ali-h5（线路 6）、hw-h5（线路 7）、hs-h5（线路 13）。"
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
          extraText="默认关闭。开启后 hs 线路改用程序自行构造的流地址，可缓解部分海外机器频繁断流的问题。仅在访问线路（douyu_cdn）为 hs-h5 时生效。"
          label="强制 hs 流（douyu_force_hs）"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Switch
          field="douyu_disable_interactive_game"
          extraText="默认关闭。开启后，检测到主播正在运行互动游戏时按未开播处理，不会开始或继续录制。小窗运行互动游戏也算在内，请谨慎开启。"
          label="斗鱼拒绝互动游戏（douyu_disable_interactive_game）"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
      </PlatformPanel>
    </>
  )
}

export default Douyu
