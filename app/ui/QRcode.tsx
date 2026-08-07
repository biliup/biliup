import React, { useEffect, useState } from 'react'
import { fetcher, proxy } from '@/app/lib/api-streamer'
import { QRCodeSVG } from 'qrcode.react'
import { Notification, Spin, Typography } from '@douyinfe/semi-ui'

type QrcodeProps = {
  onSuccess: (e: string) => void
}

/**
 * 扫码登录:拉取二维码 → 挂起等待扫码 → 回调 onSuccess。
 * 健壮性修复:
 *  - 响应字段安全取值(data.url 缺失时显示错误而非崩溃)
 *  - 组件卸载触发的 AbortError 静默忽略(不再弹错误提示)
 *  - 加载 / 错误 / 二维码三态清晰
 */
const Qrcode: React.FC<QrcodeProps> = ({ onSuccess }) => {
  const [url, setUrl] = useState('')
  const [error, setError] = useState('')

  useEffect(() => {
    const controller = new AbortController()
    const signal = controller.signal

    ;(async () => {
      const qrData = await fetcher('/v1/get_qrcode', undefined)
      const qrUrl: string | undefined = qrData?.['data']?.['url']
      if (!qrUrl) {
        // 带上响应内容,便于定位(后端偶发 5xx 时 data 结构会缺失)
        const dump = JSON.stringify(qrData ?? {}).slice(0, 200)
        setError(`二维码获取失败:响应中缺少 url 字段 · ${dump}`)
        return
      }
      setUrl(qrUrl)
      const res = await proxy('/v1/login_by_qrcode', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(qrData),
        signal,
      })
      const data = await res.json()
      onSuccess(data['filename'])
    })().catch((e: any) => {
      // 组件卸载/切换导致的取消,静默忽略。
      // 注意:abort(reason) 时 fetch 可能 reject 原始值(字符串)而非 DOMException,
      // e.name 检查不可靠,用 signal.aborted 判断(与错误对象形态无关)。
      if (controller.signal.aborted) return
      const detail = e?.message || String(e) || '未知错误'
      console.log(e)
      setError(`二维码获取失败:${detail}`)
      Notification.error({
        title: 'QRcode',
        content: (
          <Typography.Paragraph style={{ maxWidth: 450 }}>{detail}</Typography.Paragraph>
        ),
        style: { width: 'min-content' },
      })
    })

    return () => {
      controller.abort()
    }
  }, [onSuccess])

  if (url) {
    return (
      <div
        style={{
          marginTop: 30,
          marginLeft: 'auto',
          marginRight: 'auto',
          width: 'max-content',
        }}
      >
        <QRCodeSVG value={url} />
      </div>
    )
  }

  if (error) {
    return (
      <div style={{ marginTop: 24, textAlign: 'center' }}>
        <Typography.Text type="danger">{error}</Typography.Text>
      </div>
    )
  }

  return (
    <div style={{ marginTop: 30, textAlign: 'center' }}>
      <Spin />
      <div style={{ marginTop: 10, fontSize: 12, color: 'var(--semi-color-text-2)' }}>
        正在获取二维码…
      </div>
    </div>
  )
}

export default Qrcode
