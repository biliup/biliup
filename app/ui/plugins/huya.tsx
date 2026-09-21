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
          extraText={
            <div style={{ fontSize: '14px' }}>
              录制码率上限，默认 0 表示不限制、录原画。
              <br />
              设为 10000 则录制码率不超过 10000 的最高一档画质，可避免录 20M 码率原画时每小时约 8 GB、上传和转码过慢的问题。
              <br />
              参考：20000 蓝光 20M / 10000 蓝光 10M / 8000 蓝光 8M / 2000 超清 / 500 流畅。
            </div>
          }
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
          extraText={
            <div style={{ fontSize: '14px' }}>
              录制卡顿时可尝试切换线路，默认不指定（使用虎牙返回的首个可用线路）。
              <br />
              可选：AL（阿里云，线路 3）、TX（腾讯云，线路 5）、HW（华为云，线路 6）、WS（网宿）、HS（火山引擎，线路
              14）、AL13（阿里云）、TX15（腾讯云）、HW16（华为云）。
              <br />
              HY、HYZJ（虎牙自建，线路 66）已被屏蔽，填了也会改用首个可用线路。
            </div>
          }
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
          extraText="默认关闭。开启后会先探测所选访问线路（huya_cdn）能否拉流，不可用时按顺序尝试其他线路。"
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
          extraText={
            <div style={{ fontSize: '14px' }}>
              是否录制虎牙二次编码后的直播流，默认开启。
              <br />
              关闭后改取未二次编码的流，部分直播间可能无法下载；2K / 4K 超分和 HDR 画质都依赖二次编码，<strong>请谨慎关闭</strong>。
            </div>
          }
          label="虎牙二次编码（huya_imgplus）"
          initValue={entity?.hasOwnProperty('huya_imgplus') ? entity['huya_imgplus'] : true}
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Switch
          field="huya_mobile_api"
          extraText="默认关闭。开启后改用移动端 API 请求直播间信息，可能解决部分直播分区每 2 分钟分段的问题。"
          label="使用移动端 API（huya_mobile_api）"
          fieldStyle={{
            alignSelf: 'stretch',
            padding: 0,
          }}
        />
        <Form.Switch
          field="huya_use_wup"
          extraText="使用 WUP 协议获取直播流，可能解决部分直播分区每 2 分钟分段的问题。"
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
