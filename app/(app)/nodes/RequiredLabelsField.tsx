'use client'
import { Form, Typography, useFormApi, useFormState } from '@douyinfe/semi-ui'
import { platformName } from '@/app/lib/status'
import { knownLabels, type FleetNode } from '@/app/lib/use-fleet'

const { Text } = Typography

/** 国内连不上、一般要放在海外节点录的平台；只作提示，不自动加 */
const OVERSEAS_PLATFORMS = ['YouTube', 'Twitch']
const OVERSEAS_LABEL = '海外'

/** 房间表单里的「要求的标签」：只会分派到带齐这些标签的节点 */
export default function RequiredLabelsField({ nodes }: { nodes: FleetNode[] }) {
  const api = useFormApi()
  const { values } = useFormState()
  const current: string[] = Array.isArray(values?.required_labels) ? values.required_labels : []
  const options = [...new Set([...knownLabels(nodes), ...current])].map((label) => {
    const count = nodes.filter((n) => n.labels.includes(label)).length
    return { value: label, label: count ? `${label}（${count} 台）` : `${label}（没有节点带）` }
  })
  const platform = platformName(typeof values?.url === 'string' ? values.url : undefined)
  const suggest = OVERSEAS_PLATFORMS.includes(platform) && !current.includes(OVERSEAS_LABEL)

  return (
    <Form.Select
      field="required_labels"
      label={{ text: '要求的标签', optional: true }}
      style={{ width: 'min(360px, 100%)' }}
      multiple
      filter
      allowCreate
      showClear
      optionList={options}
      renderSelectedItem={(option: { value?: unknown }) => ({ isRenderInTag: true, content: String(option.value) })}
      placeholder="不限节点"
      emptyContent="输入标签后回车"
      extraText={
        <>
          只会分派到带齐这些标签的节点（手动分派与「自动」都是）；节点的标签在「节点」页编辑。之后节点标签改了不会挪走房间，只会标成「标签不满足」。
          {suggest ? (
            <>
              <br />
              <Text type="warning" size="small">
                {platform} 在国内通常连不上，可能要放在海外节点。
              </Text>{' '}
              <Text
                link
                size="small"
                onClick={() => api.setValue('required_labels', [...current, OVERSEAS_LABEL])}
              >
                要求「{OVERSEAS_LABEL}」
              </Text>
            </>
          ) : null}
        </>
      }
    />
  )
}
