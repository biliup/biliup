'use client'
import { useRenderGpus, type RenderGpuDevice } from '@/app/lib/renders'
import styles from './renders.module.scss'

const percent = (value: number | null) => value == null ? '不可用' : `${value.toFixed(1)}%`
const memory = (device: RenderGpuDevice) => {
  if (device.memory_kind === 'shared') return '独立显存不适用'
  const { memory_used_bytes: used, memory_total_bytes: total } = device
  if (total == null) return used == null ? '不可用' : `${(used / 1024 ** 3).toFixed(2)} GiB 已用 · 总量不可用`
  return used == null ? `${(total / 1024 ** 3).toFixed(2)} GiB 总量 · 用量不可用`
    : `${(used / 1024 ** 3).toFixed(2)} / ${(total / 1024 ** 3).toFixed(2)} GiB`
}
const vendors: Record<RenderGpuDevice['vendor'], string> = { nvidia: 'NVIDIA', intel: 'Intel', amd: 'AMD', unknown: 'GPU' }
const memoryLabel = (device: RenderGpuDevice) => device.memory_kind === 'shared'
  ? '共享系统内存' : device.memory_kind === 'dedicated' ? '独立显存' : '显存'

export default function GpuMonitorCard({ allowed }: { allowed: boolean }) {
  const { data, error, isLoading } = useRenderGpus(allowed)
  return <section className={styles.gpuCard} aria-label="GPU 使用情况">
    <div className={styles.sectionHeading}><strong>GPU 使用情况</strong><span>每 2 秒更新</span></div>
    {!allowed ? <p className={styles.empty}>需要查看录像的权限。</p>
      : error ? <p className={styles.jobError}>GPU 状态读取失败：{error instanceof Error ? error.message : String(error)}</p>
      : isLoading && !data ? <p className={styles.empty}>正在读取 GPU 状态…</p>
      : data?.devices.length ? <ul className={styles.gpuList}>{data.devices.map(device => <li key={device.id}>
        <div className={styles.gpuHeading}><strong>{vendors[device.vendor]} · {device.name}</strong><span>{device.render_node ?? '设备节点不可用'}</span></div>
        <div className={styles.gpuMetrics}>
          {(device.vendor !== 'intel' || device.id.startsWith('windows:')) && <>
            <span><small>{device.id.startsWith('windows:') ? '最忙引擎利用率' : '整体利用率'}</small><strong>{percent(device.utilization_percent)}</strong></span>
            <span><small>编码引擎</small><strong>{percent(device.encoder_utilization_percent)}</strong></span>
          </>}
          <span><small>{memoryLabel(device)}</small><strong>{memory(device)}</strong></span>
        </div>
        {device.engine_utilization.length > 0 && <div className={styles.gpuMetrics} aria-label="GPU 各引擎利用率">
          {device.engine_utilization.map(engine => <span key={engine.name}>
            <small>{engine.name}</small><strong>{percent(engine.utilization_percent)}</strong>
          </span>)}
        </div>}
        {device.vendor === 'intel' && !device.id.startsWith('windows:') && <p className={styles.jobNotice}>
          Video 引擎包含编码与解码活动。{device.engine_utilization.length === 0 ? '各引擎利用率不可用。' : ''}
        </p>}
        {device.error && <p className={styles.jobNotice}>{device.error}</p>}
      </li>)}</ul> : <p className={styles.empty}>未发现可监测的 GPU 设备。</p>}
    {data && <small className={styles.sampledAt}>采样时间：{new Date(data.sampled_at).toLocaleTimeString()}</small>}
  </section>
}
