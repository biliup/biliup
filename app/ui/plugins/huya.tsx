'use client'
import React, { useEffect } from 'react'
import { Form, Select, Collapse, useFormApi } from '@douyinfe/semi-ui'

type Props = {
  entity: any
  list: any
  initValues?: Record<string, any>
}

const Huya: React.FC<Props> = props => {
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
      <Collapse.Panel header="虎牙" itemKey="huya">
        <Form.Select
          allowCreate={true}
          filter
          field="huya_max_ratio"
          extraText="录制码率上限，避免过高码率体积过大。0 原画 / 20000 蓝光20M / 10000 蓝光10M / 8000 蓝光8M / 2000 超清 / 500 流畅。"
          label="画质等级（huya_max_ratio）"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
          rules={[
            {
              pattern: /^\d*$/,
              message: '请输入纯数字',
            },
          ]}
          showClear={true}
        >
          <Select.Option value={0}>原画（0）</Select.Option>
          <Select.Option value={20000}>蓝光20M（20000）</Select.Option>
          <Select.Option value={10000}>蓝光10M（10000）</Select.Option>
          <Select.Option value={8000}>蓝光8M（8000）</Select.Option>
          <Select.Option value={2000}>超清（2000）</Select.Option>
          <Select.Option value={500}>流畅（500）</Select.Option>
        </Form.Select>
        <Form.Switch
          field="huya_danmaku"
          extraText="录制虎牙弹幕，默认关闭"
          label="录制弹幕（huya_danmaku）"
        />
        <Form.Select
          allowCreate={true}
          filter
          field="huya_cdn"
          extraText="遇卡顿可切换线路：AL(3)/TX(5)/HW(6)/WS/HS(14)/AL13/TX15/HW16；HY/HYZJ已屏蔽将切首个可用。"
          label="访问线路（huya_cdn）"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
          showClear={true}
        >
          <Select.Option value="AL">直播线路3（AL）</Select.Option>
          <Select.Option value="TX">直播线路5（TX）</Select.Option>
          <Select.Option value="AL13">直播线路13（AL13）</Select.Option>
          <Select.Option value="HS">直播线路14（HS）</Select.Option>
          <Select.Option value="TX15">直播线路15（TX15）</Select.Option>
        </Form.Select>
        <Form.Switch
          field="huya_cdn_fallback"
          extraText="访问线路不可用时尝试其他线路。"
          label="CDN 回退（huya_cdn_fallback）"
        />
        <Form.Select
          field="huya_protocol"
          extraText="Hls 仅供测试，请谨慎切换。"
          label="直播流协议（huya_protocol）"
          style={{ width: '100%' }}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
          showClear={true}
        >
          <Select.Option value="Flv">Flv（默认）</Select.Option>
          <Select.Option value="Hls">Hls</Select.Option>
        </Form.Select>
        <Form.Switch
          field="huya_imgplus"
          extraText="录制二次编码流，默认开启。关闭可能无法下载；2k/4k/HDR 依赖二次编码，谨慎关。"
          label="虎牙二次编码（huya_imgplus）"
          initValue={entity?.hasOwnProperty('huya_imgplus') ? entity['huya_imgplus'] : true}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Switch
          field="huya_mobile_api"
          extraText="移动端 API 请求直播间信息，可能解决部分分区 2 分钟分段问题"
          label="使用移动端 API（huya_mobile_api）"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Switch
          field="huya_use_wup"
          extraText="使用 WUP 协议获取直播流，可能解决部分分区 2 分钟分段问题"
          label="使用 WUP 协议（huya_use_wup）"
          initValue={entity?.hasOwnProperty('huya_use_wup') ? entity['huya_use_wup'] : true}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
      </Collapse.Panel>
    </>
  )
}

export default Huya
