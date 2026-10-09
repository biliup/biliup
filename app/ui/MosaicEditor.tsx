'use client'
import React, { useCallback, useEffect, useRef, useState } from 'react'
import { Button, InputNumber, Select, Spin, Table, Typography } from '@douyinfe/semi-ui'
import { IconDelete } from '@douyinfe/semi-icons'
import type { ColumnProps } from '@douyinfe/semi-ui/lib/es/table'
import type { MosaicConfig, MosaicRegion } from '@/app/lib/api-streamer'
import { apiFetch, handleResponse, revalidateMe } from '@/app/lib/api-streamer'
import { MAX_MOSAIC_REGIONS, mosaicRectangle, type NormalizedPoint } from '@/app/lib/mosaic-config'
import { frameAspectRatio, mosaicFrameFailure, normalizedFramePoint } from '@/app/lib/mosaic-frame'

const { Text } = Typography
const MAX_REGIONS = MAX_MOSAIC_REGIONS
type Rectangle = Pick<MosaicRegion, 'x' | 'y' | 'width' | 'height'>

interface MosaicEditorProps {
  config: MosaicConfig
  streamerId?: number
  active?: boolean
  onChange: (config: MosaicConfig) => void
}

type FrameState =
  | { status: 'idle' | 'loading' }
  | { status: 'error'; streamerId: number; captureIndex: number; message: string }
  | { status: 'ready'; streamerId: number; captureIndex: number; image: HTMLImageElement; width: number; height: number; capturedAt: number }

/** Coordinates refer to a frozen live frame, using its actual aspect without fitted-image margins. */
export function MosaicEditor({ config, streamerId, active = false, onChange }: MosaicEditorProps) {
  const { regions, enabled } = config
  const [frame, setFrame] = useState<FrameState>({ status: 'idle' })
  const [refreshIndex, setRefreshIndex] = useState(0)
  const [hoveredRegion, setHoveredRegion] = useState<string | null>(null)
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const containerRef = useRef<HTMLDivElement>(null)
  const drawStartRef = useRef<(NormalizedPoint & { pointerId: number }) | null>(null)
  const draftRef = useRef<Rectangle | null>(null)
  const currentFrame = active && frame.status === 'ready' && frame.streamerId === streamerId && frame.captureIndex === refreshIndex ? frame : null
  const currentError = active && frame.status === 'error' && frame.streamerId === streamerId && frame.captureIndex === refreshIndex ? frame : null
  const canCapture = active && Number.isSafeInteger(streamerId) && (streamerId ?? 0) > 0
  const frameLoading = canCapture && !currentFrame && !currentError

  // A single capture per opening/room/refresh. Closing or switching rooms cancels the old request.
  useEffect(() => {
    drawStartRef.current = null
    draftRef.current = null
    if (!active || !Number.isSafeInteger(streamerId) || (streamerId ?? 0) <= 0) {
      let cancelled = false
      queueMicrotask(() => {
        if (!cancelled) setFrame({ status: 'idle' })
      })
      return () => { cancelled = true }
    }
    const sourceId = streamerId as number
    const controller = new AbortController()
    let stale = false
    let objectUrl: string | null = null
    let capturedImage: HTMLImageElement | null = null
    let responseStatus: number | undefined

    const capture = async () => {
      try {
        const response = await apiFetch(`/v1/streamers/${sourceId}/mosaic-frame`, {
          signal: controller.signal,
          cache: 'no-store',
          headers: { Accept: 'image/jpeg' },
        })
        responseStatus = response.status
        if (stale) return
        if (response.status === 401) await handleResponse(response)
        if (response.status === 403) revalidateMe()
        if (!response.ok) throw new Error('Frame unavailable')
        if (response.headers.get('Content-Type')?.split(';')[0].trim().toLowerCase() !== 'image/jpeg') {
          throw new Error('Invalid frame response')
        }
        const blob = await response.blob()
        if (stale) return
        if (blob.size === 0) throw new Error('Empty frame')
        objectUrl = URL.createObjectURL(blob)
        const image = new Image()
        capturedImage = image
        await new Promise<void>((resolve, reject) => {
          const finish = (error?: Error) => {
            image.onload = null
            image.onerror = null
            controller.signal.removeEventListener('abort', onAbort)
            if (error) reject(error)
            else resolve()
          }
          const onAbort = () => finish(new DOMException('Frame capture cancelled', 'AbortError'))
          image.onload = () => finish()
          image.onerror = () => finish(new Error('Invalid JPEG'))
          controller.signal.addEventListener('abort', onAbort, { once: true })
          if (controller.signal.aborted) onAbort()
          else image.src = objectUrl as string
        })
        if (stale) return
        if (!frameAspectRatio({ width: image.naturalWidth, height: image.naturalHeight })) {
          throw new Error('Invalid image dimensions')
        }
        setFrame({
          status: 'ready', streamerId: sourceId, captureIndex: refreshIndex, image,
          width: image.naturalWidth, height: image.naturalHeight, capturedAt: Date.now(),
        })
      } catch {
        if (stale || controller.signal.aborted) return
        if (objectUrl) {
          URL.revokeObjectURL(objectUrl)
          objectUrl = null
        }
        setFrame({ status: 'error', streamerId: sourceId, captureIndex: refreshIndex, message: mosaicFrameFailure(responseStatus ?? 0) })
      }
    }
    queueMicrotask(() => {
      if (stale) return
      setFrame({ status: 'loading' })
      void capture()
    })
    return () => {
      stale = true
      controller.abort()
      if (capturedImage) {
        capturedImage.onload = null
        capturedImage.onerror = null
        capturedImage.src = ''
      }
      if (objectUrl) URL.revokeObjectURL(objectUrl)
    }
  }, [active, streamerId, refreshIndex])

  const drawRegions = useCallback(() => {
    const canvas = canvasRef.current
    const ctx = canvas?.getContext('2d')
    if (!canvas || !ctx || !currentFrame) return
    ctx.clearRect(0, 0, canvas.width, canvas.height)
    ctx.drawImage(currentFrame.image, 0, 0, canvas.width, canvas.height)
    const scale = canvas.width / Math.max(1, canvas.getBoundingClientRect().width)
    regions.forEach((region, index) => {
      const color = hoveredRegion === region.id ? '#ff4d4f' :
        region.effectType === 'mosaic' ? '#1890ff' : region.effectType === 'blur' ? '#52c41a' : '#faad14'
      const x = region.x * canvas.width
      const y = region.y * canvas.height
      const width = region.width * canvas.width
      const height = region.height * canvas.height
      ctx.fillStyle = region.effectType === 'solid' ? region.color ?? '#000000' : `${color}33`
      ctx.fillRect(x, y, width, height)
      ctx.strokeStyle = color
      ctx.lineWidth = (hoveredRegion === region.id ? 3 : 2) * scale
      ctx.strokeRect(x, y, width, height)
      ctx.fillStyle = color
      ctx.font = `${14 * scale}px sans-serif`
      ctx.fillText(`${index + 1}: ${region.effectType}`, x + 5 * scale, y + 20 * scale)
    })
    if (enabled && draftRef.current) {
      const draft = draftRef.current
      ctx.strokeStyle = '#ff4d4f'
      ctx.lineWidth = 2 * scale
      ctx.setLineDash([5 * scale, 5 * scale])
      ctx.strokeRect(draft.x * canvas.width, draft.y * canvas.height,
        draft.width * canvas.width, draft.height * canvas.height)
      ctx.setLineDash([])
    }
  }, [regions, hoveredRegion, enabled, currentFrame])

  useEffect(() => {
    if (!enabled || !currentFrame) {
      drawStartRef.current = null
      draftRef.current = null
    }
    drawRegions()
  }, [drawRegions, enabled, currentFrame])

  // Collapse panels and drawers can resize without a browser window resize event.
  useEffect(() => {
    const canvas = canvasRef.current
    const container = containerRef.current
    if (!canvas || !container || !currentFrame) return
    const resize = () => {
      const rect = container.getBoundingClientRect()
      const pixelRatio = window.devicePixelRatio || 1
      canvas.width = Math.max(1, Math.round(rect.width * pixelRatio))
      canvas.height = Math.max(1, Math.round(rect.height * pixelRatio))
      drawRegions()
    }
    resize()
    const observer = new ResizeObserver(resize)
    observer.observe(container)
    return () => observer.disconnect()
  }, [drawRegions, currentFrame])

  const pointerPosition = (event: React.PointerEvent<HTMLCanvasElement>) =>
    normalizedFramePoint(event.clientX, event.clientY, event.currentTarget.getBoundingClientRect())

  const cancelDrawing = () => {
    drawStartRef.current = null
    draftRef.current = null
    drawRegions()
  }

  const handlePointerDown = (event: React.PointerEvent<HTMLCanvasElement>) => {
    if (!enabled || !currentFrame || event.button !== 0 || regions.length >= MAX_REGIONS || drawStartRef.current) return
    event.preventDefault()
    const point = pointerPosition(event)
    if (!point) return
    drawStartRef.current = { ...point, pointerId: event.pointerId }
    draftRef.current = mosaicRectangle(point, point)
    event.currentTarget.setPointerCapture(event.pointerId)
    drawRegions()
  }

  const handlePointerMove = (event: React.PointerEvent<HTMLCanvasElement>) => {
    const start = drawStartRef.current
    const point = pointerPosition(event)
    if (!enabled || !currentFrame || !start || start.pointerId !== event.pointerId || !point) return
    draftRef.current = mosaicRectangle(start, point)
    drawRegions()
  }

  const handlePointerUp = (event: React.PointerEvent<HTMLCanvasElement>) => {
    const start = drawStartRef.current
    const point = pointerPosition(event)
    if (!start || start.pointerId !== event.pointerId) return
    const rectangle = point ? mosaicRectangle(start, point) : null
    if (enabled && currentFrame && rectangle && rectangle.width >= 0.01 && rectangle.height >= 0.01 && regions.length < MAX_REGIONS) {
      const region: MosaicRegion = {
        id: `region-${globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(36).slice(2)}`}`,
        ...rectangle,
        effectType: 'mosaic',
        strength: 16,
      }
      onChange({ ...config, regions: [...regions, region] })
    }
    cancelDrawing()
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId)
    }
  }

  const updateRegion = (id: string, updates: Partial<MosaicRegion>) => {
    onChange({ ...config, regions: regions.map(region => region.id === id ? { ...region, ...updates } : region) })
  }

  const columns: ColumnProps<MosaicRegion>[] = [
    { title: '#', width: 40, render: (_value, _record, index) => index + 1 },
    {
      title: '效果', dataIndex: 'effectType', width: 110,
      render: (_effectType, region) => (
        <Select
          value={region.effectType}
          onChange={value => {
            if (value !== 'mosaic' && value !== 'blur' && value !== 'solid') return
            const min = value === 'mosaic' ? 4 : 1
            const max = value === 'mosaic' ? 64 : 100
            updateRegion(region.id, { effectType: value, strength: Math.min(max, Math.max(min, region.strength)) })
          }}
          style={{ width: '100%' }} size="small"
        >
          <Select.Option value="mosaic">马赛克</Select.Option>
          <Select.Option value="blur">模糊</Select.Option>
          <Select.Option value="solid">纯色</Select.Option>
        </Select>
      ),
    },
    {
      title: '强度 / 颜色', width: 115,
      render: (_value, region) => region.effectType === 'solid' ? (
        <input
          type="color" aria-label="遮挡颜色" value={region.color && /^#[\da-f]{6}$/i.test(region.color) ? region.color : '#000000'}
          onChange={event => updateRegion(region.id, { color: event.target.value })}
          style={{ width: 56, height: 28 }}
        />
      ) : (
        <InputNumber
          value={region.strength}
          onChange={value => {
            if (typeof value !== 'number' || !Number.isFinite(value)) return
            const min = region.effectType === 'mosaic' ? 4 : 1
            const max = region.effectType === 'mosaic' ? 64 : 100
            updateRegion(region.id, { strength: Math.min(max, Math.max(min, Math.round(value))) })
          }}
          min={region.effectType === 'mosaic' ? 4 : 1} max={region.effectType === 'mosaic' ? 64 : 100}
          step={1} size="small" style={{ width: '100%' }}
        />
      ),
    },
    {
      title: '位置 / 大小', width: 150,
      render: (_value, region) => <Text size="small" type="tertiary">
        ({(region.x * 100).toFixed(1)}%, {(region.y * 100).toFixed(1)}%)<br />
        {(region.width * 100).toFixed(1)}% × {(region.height * 100).toFixed(1)}%
      </Text>,
    },
    {
      title: '操作', width: 60,
      render: (_value, region) => <Button
        type="danger" theme="borderless" icon={<IconDelete />} size="small" aria-label="删除遮挡区域"
        onClick={() => onChange({ ...config, regions: regions.filter(item => item.id !== region.id) })}
      />,
    },
  ]

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
      <div style={{ display: 'flex', alignItems: 'center', flexWrap: 'wrap', gap: 8 }}>
        <Button
          size="small"
          loading={frameLoading}
          disabled={!canCapture || frameLoading}
          onClick={() => setRefreshIndex(index => index + 1)}
        >
          {currentError ? '重试抓帧' : '刷新画面'}
        </Button>
        {currentFrame && <Text size="small" type="tertiary">
          {currentFrame.width} × {currentFrame.height} · 获取于 {new Date(currentFrame.capturedAt).toLocaleTimeString('zh-CN')}
        </Text>}
      </div>
      {currentFrame ? <div ref={containerRef} style={{
        position: 'relative', width: '100%', aspectRatio: frameAspectRatio(currentFrame) ?? undefined,
        borderRadius: 4, overflow: 'hidden', cursor: enabled ? 'crosshair' : 'not-allowed',
      }}>
        <canvas
          ref={canvasRef}
          aria-label="当前直播截帧，可拖拽圈选遮挡区域"
          onPointerDown={handlePointerDown} onPointerMove={handlePointerMove} onPointerUp={handlePointerUp}
          onPointerCancel={cancelDrawing} onLostPointerCapture={cancelDrawing}
          style={{ position: 'absolute', inset: 0, width: '100%', height: '100%', touchAction: 'none' }}
        />
        {!enabled && <div style={{ position: 'absolute', inset: 0, display: 'grid', placeItems: 'center', background: '#0006', color: '#fff', pointerEvents: 'none' }}>
          先开启画面遮挡，再在直播画面上拖拽圈选
        </div>}
      </div> : <div
        role="status"
        aria-live="polite"
        style={{ minHeight: 160, display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', gap: 12, padding: 16, border: '1px solid var(--semi-color-border)', borderRadius: 4 }}
      >
        {frameLoading ? <><Spin /><Text>正在抓取当前直播画面…</Text></> :
          <Text type={currentError ? 'danger' : 'tertiary'}>
            {!active ? '展开画面遮挡后获取当前直播画面。' : !canCapture ? '请先保存直播间，再获取直播画面。' :
              currentError ? currentError.message : '准备获取当前直播画面…'}
          </Text>}
        <Text size="small" type="tertiary">取得直播画面后才能新增圈选区域；已有区域配置会保留。</Text>
      </div>}
      <Text size="small" type="tertiary">
        {regions.length >= MAX_REGIONS ? `最多可配置 ${MAX_REGIONS} 个区域。请先删除区域后再绘制。` :
          '在直播截帧上按住鼠标左键或触屏拖拽圈选矩形区域，在表格中调整效果和强度。画面在圈选时保持固定，点击「刷新画面」可重新抓帧。'}
      </Text>
      {regions.length > 0 && <Table<MosaicRegion>
        columns={columns} dataSource={regions} rowKey="id" pagination={false} size="small"
        scroll={{ x: 475 }}
        onRow={region => ({ onMouseEnter: () => setHoveredRegion(region?.id ?? null), onMouseLeave: () => setHoveredRegion(null) })}
      />}
      <Text size="small" type="tertiary">
        区域位置和大小按画面百分比保存，宽高至少为画面的 1%。配置应用于后续分段；处理失败时会保留原始文件。
      </Text>
    </div>
  )
}
