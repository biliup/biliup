'use client'
import { Form, useFormState } from '@douyinfe/semi-ui'
import { missingLabels, quoteLabels, type FleetNode } from '@/app/lib/use-fleet'
const AUTO_NODE = -1

/** 新建房间的「分派到节点」：缺少所填标签的节点标出来、不能选 */
export default function TargetNodeField({ nodes }: { nodes: FleetNode[] }) {
  const { values } = useFormState()
  const required: string[] = Array.isArray(values?.required_labels) ? values.required_labels : []
  const nodeOptions = [
    ...(nodes.length > 0 ? [{ value: AUTO_NODE, label: '自动（按负载选）' }] : []),
    ...[...nodes]
      .sort((a, b) => Number(b.online) - Number(a.online) || a.id - b.id)
      .map((n) => {
        const missing = missingLabels(n.labels, required)
        return {
          value: n.id,
          disabled: missing.length > 0,
          label: `${n.name}${n.online ? '' : '（离线）'}${missing.length ? `（缺少标签${quoteLabels(missing)}）` : ''}`,
        }
      }),
  ]
  return (
    <Form.Select
      field="node_id"
      label={{ text: '分派到节点', optional: true }}
      style={{ width: 'min(320px, 100%)' }}
      optionList={nodeOptions}
      showClear
      placeholder="暂不分派"
      extraText="模板要用的 B 站账号必须在节点上登记过；处理器里带 run 命令或设了 ffmpeg 参数的房间只能派给允许钩子的节点（rm、mv 等文件操作不算）；节点要带齐房间要求的标签。「自动」在满足这些条件的在线节点里挑空闲下载位多、房间少、磁盘余量大的一台，之后不会因负载变化挪走"
    />
  )
}
