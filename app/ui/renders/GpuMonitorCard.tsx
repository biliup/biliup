'use client'
import { useRenderGpus, type RenderGpuDevice } from '@/app/lib/renders'
import styles from './renders.module.scss'

const percent = (value: number | null) => value == null ? '不可用' : `${value.toFixed(1)}%`
const memory = (used: number | null, total: number | null) => used == null || total == null
  ? '不可用' : `${(used / 1024 ** 3).toFixed(1)} / ${(total / 1024 ** 3).toFixed(1)} GiB`
const vendors: Record<RenderGpuDevice['vendor'], string> = { nvidia: 'NVIDIA', intel: 'Intel', amd: 'AMD', unknown: 'GPU' }

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
          <span><small>整体利用率</small><strong>{percent(device.utilization_percent)}</strong></span>
          <span><small>{device.vendor === 'intel' && !device.id.startsWith('windows:') ? '视频引擎（编码/解码）' : '编码引擎'}</small><strong>{percent(device.encoder_utilization_percent)}</strong></span>
          <span><small>显存使用</small><strong>{memory(device.memory_used_bytes, device.memory_total_bytes)}</strong></span>
        </div>
        {device.error && <p className={styles.jobNotice}>{device.error}</p>}
      </li>)}</ul> : <p className={styles.empty}>未发现可监测的 GPU 设备。</p>}
    {data && <small className={styles.sampledAt}>采样时间：{new Date(data.sampled_at).toLocaleTimeString()}</small>}
  </section>
}
