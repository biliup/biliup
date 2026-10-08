'use client'
import React, { useCallback, useEffect, useRef, useState } from 'react'
import { Button, InputNumber, Select, Table, Typography } from '@douyinfe/semi-ui'
import { IconDelete } from '@douyinfe/semi-icons'
import type { ColumnProps } from '@douyinfe/semi-ui/lib/es/table'
import type { MosaicConfig, MosaicRegion } from '@/app/lib/api-streamer'
import { MAX_MOSAIC_REGIONS, mosaicRectangle, type NormalizedPoint } from '@/app/lib/mosaic-config'

const { Text } = Typography
const MAX_REGIONS = MAX_MOSAIC_REGIONS
type Rectangle = Pick<MosaicRegion, 'x' | 'y' | 'width' | 'height'>

interface MosaicEditorProps {
  config: MosaicConfig
  onChange: (config: MosaicConfig) => void
}

/** Controlled editor: configuration updates and form resets immediately reach the canvas. */
export function MosaicEditor({ config, onChange }: MosaicEditorProps) {
  const { regions, enabled } = config
  const [hoveredRegion, setHoveredRegion] = useState<string | null>(null)
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const containerRef = useRef<HTMLDivElement>(null)
  const drawStartRef = useRef<(NormalizedPoint & { pointerId: number }) | null>(null)
  const draftRef = useRef<Rectangle | null>(null)

  const drawRegions = useCallback(() => {
    const canvas = canvasRef.current
    const ctx = canvas?.getContext('2d')
    if (!canvas || !ctx) return
    ctx.clearRect(0, 0, canvas.width, canvas.height)
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
      ctx.lineWidth = hoveredRegion === region.id ? 3 : 2
      ctx.strokeRect(x, y, width, height)
      ctx.fillStyle = color
      ctx.font = '14px sans-serif'
      ctx.fillText(`${index + 1}: ${region.effectType}`, x + 5, y + 20)
    })
    if (enabled && draftRef.current) {
      const draft = draftRef.current
      ctx.strokeStyle = '#ff4d4f'
      ctx.lineWidth = 2
      ctx.setLineDash([5, 5])
      ctx.strokeRect(draft.x * canvas.width, draft.y * canvas.height,
        draft.width * canvas.width, draft.height * canvas.height)
      ctx.setLineDash([])
    }
  }, [regions, hoveredRegion, enabled])

  useEffect(() => {
    drawRegions()
  }, [drawRegions])

  // Collapse panels and drawers can resize without a browser window resize event.
  useEffect(() => {
    const canvas = canvasRef.current
    const container = containerRef.current
    if (!canvas || !container) return
    const resize = () => {
      const rect = container.getBoundingClientRect()
      canvas.width = Math.max(1, Math.round(rect.width))
      canvas.height = Math.max(1, Math.round(rect.height))
      drawRegions()
    }
    resize()
    const observer = new ResizeObserver(resize)
    observer.observe(container)
    return () => observer.disconnect()
  }, [drawRegions])

  const pointerPosition = (event: React.PointerEvent<HTMLCanvasElement>): NormalizedPoint => {
    const rect = event.currentTarget.getBoundingClientRect()
    return {
      x: Math.max(0, Math.min(1, (event.clientX - rect.left) / Math.max(1, rect.width))),
      y: Math.max(0, Math.min(1, (event.clientY - rect.top) / Math.max(1, rect.height))),
    }
  }

  const cancelDrawing = () => {
    drawStartRef.current = null
    draftRef.current = null
    drawRegions()
  }

  const handlePointerDown = (event: React.PointerEvent<HTMLCanvasElement>) => {
    if (!enabled || event.button !== 0 || regions.length >= MAX_REGIONS || drawStartRef.current) return
    event.preventDefault()
    const point = pointerPosition(event)
    drawStartRef.current = { ...point, pointerId: event.pointerId }
    draftRef.current = mosaicRectangle(point, point)
    event.currentTarget.setPointerCapture(event.pointerId)
    drawRegions()
  }

  const handlePointerMove = (event: React.PointerEvent<HTMLCanvasElement>) => {
    const start = drawStartRef.current
    if (!enabled || !start || start.pointerId !== event.pointerId) return
    draftRef.current = mosaicRectangle(start, pointerPosition(event))
    drawRegions()
  }

  const handlePointerUp = (event: React.PointerEvent<HTMLCanvasElement>) => {
    const start = drawStartRef.current
    if (!start || start.pointerId !== event.pointerId) return
    const rectangle = mosaicRectangle(start, pointerPosition(event))
    if (enabled && rectangle.width >= 0.01 && rectangle.height >= 0.01 && regions.length < MAX_REGIONS) {
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
      <div ref={containerRef} style={{
        position: 'relative', width: '100%', aspectRatio: '16/9', backgroundColor: '#000',
        borderRadius: 4, overflow: 'hidden', cursor: enabled ? 'crosshair' : 'not-allowed',
      }}>
        <canvas
          ref={canvasRef}
          onPointerDown={handlePointerDown} onPointerMove={handlePointerMove} onPointerUp={handlePointerUp}
          onPointerCancel={cancelDrawing} onLostPointerCapture={cancelDrawing}
          style={{ position: 'absolute', inset: 0, width: '100%', height: '100%', touchAction: 'none' }}
        />
        {!enabled && <div style={{ position: 'absolute', inset: 0, display: 'grid', placeItems: 'center', color: '#fff', pointerEvents: 'none' }}>
          请先启用画面遮挡功能
        </div>}
      </div>
      <Text size="small" type="tertiary">
        {regions.length >= MAX_REGIONS ? `最多可配置 ${MAX_REGIONS} 个区域。请先删除区域后再绘制。` :
          '画布为 16:9 示意图。在画布上按住鼠标左键或触屏拖拽创建区域，在表格中调整效果和强度。'}
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
