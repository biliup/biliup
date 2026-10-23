'use client'

import React, { useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react'
import { ArrayField, Button, Form, Notification, useFormApi, useFormState } from '@douyinfe/semi-ui'
import { IconPlusCircle, IconMinusCircle } from '@douyinfe/semi-icons'
import type { FormApi } from '@douyinfe/semi-ui/lib/es/form'
import useSWR from 'swr'
import { fetcher, type LiveStreamerEntity } from '../lib/api-streamer'
import { downloadFormatPolicy, savedDownloadFormat } from '../lib/download-format'
import { useMe } from '../lib/use-me'
import { useAutoClip } from '../lib/auto-clip'
import AfterLiveSwitch from './auto-clip/AfterLiveSwitch'
import { FormPage } from './shell'
import ProcessingFlowField, { ProcessingFlowGraph, FLOW_NODES, isInnerNode, type FlowNode } from './ProcessingFlowEditor'
import { PROCESSING_STAGES, processingFlowFromEntity, processingFlowToForm, validateProcessingFlow } from '../lib/processing-flow'
import styles from './ProcessingFlowEditor.module.css'

type Props = {
  entity?: LiveStreamerEntity
  title: string
  back: { href: string; label: string }
  onOk: (values: Record<string, any>) => Promise<void>
  templateOptions: { value: number; label: string }[]
  extraFields?: React.ReactNode
  fleet?: boolean
}

export default function StreamerEditor({ entity, title, back, onOk, templateOptions, extraFields, fleet }: Props) {
  const api = useRef<FormApi>(undefined)
  const [selected, setSelected] = useState<FlowNode>('start')
  const [expanded, setExpanded] = useState(false)
  const { can } = useMe()
  const hooks = can('streamer.hooks')
  const configuration = useSWR<{ downloader?: string | null }>(!fleet && can('config.view') ? '/v1/configuration' : null, fetcher)
  const inheritedDownloader = configuration.data?.downloader
  const initValues = useMemo(() => {
    const initial = { ...entity, processing_flow: processingFlowFromEntity(entity) }
    if (typeof entity?.time_range === 'string' && entity.time_range) {
      try { return { ...initial, time_range: JSON.parse(entity.time_range)?.map((t: string) => new Date(t)) } }
      catch { return { ...initial, time_range: undefined } }
    }
    return initial
  }, [entity])
  const save = async () => {
    const current = api.current!.getValues()
    if (!current.remark?.trim() || !current.url?.trim()) {
      setSelected('start')
      setExpanded(false)
      await api.current!.validate()
      Notification.error({ title: '请填写录播备注和直播链接' })
      return
    }
    if (hooks) {
      for (const stage of PROCESSING_STAGES) {
        const error = validateProcessingFlow({ [stage.key]: current.processing_flow?.[stage.key] })
        if (error) {
          setSelected(stage.key)
          setExpanded(isInnerNode(stage.key))
          Notification.error({ title: '请检查处理配置', content: error })
          return
        }
      }
    }
    let validated
    try { validated = await api.current!.validate() }
    catch (error) { setSelected('download'); setExpanded(true); throw error }
    const { processing_flow, ...values } = validated
    await onOk({
      ...values,
      ...(hooks ? processingFlowToForm(processing_flow ?? {}) : {}),
      remark: values.remark.trim(), url: values.url.trim(),
      format: savedDownloadFormat(values.format, values.override?.downloader ?? inheritedDownloader, values.url.trim(),
        hooks && (values.override?.downloader != null || configuration.data != null)),
      time_range: values.time_range?.length ? JSON.stringify(values.time_range.map((date: Date) => date.toISOString())) : null,
    })
  }
  return <FormPage title={title} back={back} breadcrumbs={fleet ? [{ href: '/status', label: '任务平台' }, { href: '/nodes', label: '节点管理' }, back] : undefined}
    canvas okText={entity ? '保存' : '创建'} onOk={save}>
    <div className={styles.workspace}>
      <Form initValues={initValues} getFormApi={formApi => { api.current = formApi }} className={styles.workspaceForm}>
        <EditorPanels selected={selected} onSelect={setSelected} expanded={expanded} onExpandedChange={setExpanded}
          hooks={hooks} fleet={fleet} templateOptions={templateOptions} extraFields={extraFields} inheritedDownloader={inheritedDownloader} configurationKnown={configuration.data != null} />
      </Form>
    </div>
  </FormPage>
}

function EditorPanels({ selected, onSelect, expanded, onExpandedChange, hooks, fleet, templateOptions, extraFields, inheritedDownloader, configurationKnown }: {
  selected: FlowNode; onSelect: (node: FlowNode) => void; hooks: boolean; fleet?: boolean
  expanded: boolean; onExpandedChange: (expanded: boolean) => void
  templateOptions: Props['templateOptions']; extraFields?: React.ReactNode
  inheritedDownloader?: string | null
  configurationKnown: boolean
}) {
  const { values } = useFormState()
  const formApi = useFormApi()
  const downloader = values.override?.downloader ?? inheritedDownloader
  // Without hook permission the API omits per-room overrides, so the effective
  // downloader is unknown. Preserve the saved suffix when editing other fields.
  const formatPolicy = downloadFormatPolicy(hooks ? downloader : undefined, values.url)
  const formatKnown = hooks && (values.override?.downloader != null || configurationKnown)
  useEffect(() => {
    const next = savedDownloadFormat(values.format, downloader, values.url, formatKnown)
    if (values.format != null && values.format !== next) formApi.setValue('format', next)
  }, [downloader, values.url, values.format, formatKnown, formApi])
  const inspector = useRef<HTMLElement>(null)
  useLayoutEffect(() => { inspector.current?.scrollTo({ top: 0 }) }, [selected])
  const autoClip = useAutoClip(!fleet)
  const message = '该项为必填项'
  const isProcessor = selected !== 'start' && selected !== 'download' && selected !== 'upload'
  return <>
    <ProcessingFlowGraph value={values.processing_flow ?? {}} selected={selected} onSelect={onSelect} expanded={expanded} onExpandedChange={onExpandedChange}
      uploadLabel={templateOptions.find(t => t.value === values.upload_streamers_id)?.label} />
    <section ref={inspector} className={styles.inspector} id="flow-node-settings" aria-label={`${FLOW_NODES[selected].title}设置`}>
      <header className={styles.inspectorHead}>
        <h2 title={FLOW_NODES[selected].description}>
          <span className={`${styles.inspectorIcon} ${styles[selected] || ''}`} aria-hidden="true">{FLOW_NODES[selected].icon}</span>
          {FLOW_NODES[selected].title}
        </h2>
        {selected === 'downloaded_processor' && <p>整场下载和分段处理完成后触发一次，接收实际文件列表；普通录制在此步骤完成后上传，边录边传仅在录制结束后执行。</p>}
        {selected === 'postprocessor' && <p>{values.upload_streamers_id ? '全部分段上传、投稿成功后，处理视频文件。' : '未配置上传：全部分段处理成功后，处理视频文件。'}</p>}
      </header>
      {/* Keep every field registered so switching nodes never drops unsaved values. */}
      <div hidden={selected !== 'start'}>
          <Form.Input
            field="remark"
            label="录播备注"
            trigger="blur"
            rules={[{ required: true, message }]}
          />

          <Form.Input
            field="url"
            label="直播链接"
            trigger="blur"
            rules={[{ required: true, message }]}
          />

          <Form.Input
              field="filename_prefix"
              label="文件名模板（可选）"
              placeholder="{streamer}%Y-%m-%dT%H_%M_%S"
          />


        {extraFields}
      </div>
      <div hidden={selected !== 'download'}>
              {hooks && <Form.Select field="override.downloader" label="下载器" showClear
                placeholder={inheritedDownloader ? `跟随全局：${inheritedDownloader}` : fleet ? '跟随录制节点设置' : '自动选择'}
                style={{ width: '100%' }} optionList={[
                  { value: 'mesio', label: 'mesio' },
                  { value: 'stream-gears', label: 'stream-gears' },
                  { value: 'streamlink', label: 'streamlink' },
                  { value: 'ffmpeg', label: 'FFmpeg（支持转封装）' },
                  { value: 'ffmpeg-internal', label: 'FFmpeg 内部分段（支持转封装）' },
                  { value: 'ffmpeg-external', label: 'FFmpeg 外部分段（支持转封装）' },
                  { value: 'yt-dlp', label: 'yt-dlp' },
                  { value: 'ytarchive', label: 'ytarchive（YouTube 直播）' },
                  ...(hooks ? [{ value: 'sync-downloader', label: 'sync-downloader（边录边传）' }] : []),
                ]} />}
              <div hidden={!formatPolicy.formats.length}>
                <Form.Select field="format" label="保存后缀（转封装）" showClear
                  disabled={!formatPolicy.formats.length} style={{ width: 200 }}
                  placeholder="默认格式"
                  optionList={formatPolicy.formats.map(value => ({ value, label: `.${value}` }))}
                  helpText={formatPolicy.help} />
              </div>
              <Form.TimePicker
                field="time_range"
                type="timeRange"
                placeholder=" "
                extraText={
                  <div style={{ fontSize: '14px' }}>
                    如果设置了录制时间范围，不在时间范围内，将不进行录制<br />
                    下载器需使用ffmpeg或streamlink
                  </div>
                }
                label={{
                  text: '录制时间范围',
                  optional: true,
                  style: {
                    marginBottom: '4px',
                    paddingBottom: '8px',
                    borderBottom: '1px solid var(--semi-color-border)',
                  }
                }}
                style={{ width: 176 }}
              />

              <ArrayField field="excluded_keywords">
                {({ add, arrayFields }) => (
                  <Form.Section text="不录制关键词">
                    <div className="semi-form-field-extra">
                      如果房间名包含关键词，则停止或不录制该场直播，每个关键词需单独一行<br />
                      暂不支持<strong>cc直播</strong>、<strong>yy直播</strong>、<strong>twitch直播</strong>
                    </div>
                    <Button icon={<IconPlusCircle />} onClick={() => add()} theme="light">
                      添加关键词
                    </Button>
                    {arrayFields.map(({ field, key, remove }, i) => (
                      <div key={key} style={{ width: '100%', display: 'flex' }}>
                        <Form.Input
                          field={field}
                          label={`关键词${i + 1}`}
                          labelPosition="left"
                          rules={[{ required: true, message }]}
                        ></Form.Input>
                        <Button
                          type="danger"
                          theme="borderless"
                          icon={<IconMinusCircle />}
                          onClick={remove}
                          style={{ margin: 12 }}
                        />
                      </div>
                    ))}
                  </Form.Section>
                )}
              </ArrayField>

              {hooks ? <ArrayField field="opt_args">
                {({ add, arrayFields }) => (
                  <Form.Section text="ffmpeg参数">
                    <div className="semi-form-field-extra">
                      如：&quot;-ss&quot;、&quot;00:00:16&quot;，每个参数需单独一行<br />
                      追加在 ffmpeg 的输出参数里（输出文件名之前），只在用 ffmpeg 录制时生效
                    </div>
                    <Button icon={<IconPlusCircle />} onClick={() => add()} theme="light">
                      添加行
                    </Button>
                    {arrayFields.map(({ field, key, remove }, i) => (
                      <div key={key} style={{ width: '100%', display: 'flex' }}>
                        <Form.Input
                          field={field}
                          label={`参数${i + 1}`}
                          labelPosition="left"
                          rules={[{ required: true, message }]}
                        ></Form.Input>
                        <Button
                          type="danger"
                          theme="borderless"
                          icon={<IconMinusCircle />}
                          onClick={remove}
                          style={{ margin: 12 }}
                        />
                      </div>
                    ))}
                  </Form.Section>
                )}
              </ArrayField>
                : <Form.Section text="ffmpeg参数">
                  <p className={styles.help}>ffmpeg 参数仅超级管理员可配置，保存时保持原样。</p>
                </Form.Section>}
      </div>
      <div hidden={selected !== 'upload'}>
        <Form.Select showClear field="upload_streamers_id" label={{ text: '投稿模板', optional: true }}
          style={{ width: '100%' }} optionList={templateOptions} placeholder="不选择投稿模板"
          extraText="投稿标题、分区、账号等由所选投稿模板决定。" />
      </div>
      <div hidden={!isProcessor}>
        {hooks ? <ProcessingFlowField field="processing_flow" noLabel stage={isProcessor ? selected : 'postprocessor'} />
          : <p className={styles.help}>后处理与各类处理器命令仅超级管理员可配置，保存时保持原样。</p>}
      </div>
      <div hidden={selected !== 'downloaded_processor'}>
        {!fleet && <AfterLiveSwitch availability={autoClip} canOverride={hooks} />}
      </div>
    </section>
  </>
}
