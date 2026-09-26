'use client'
import React from 'react'
import { Button, Tag, Tooltip, Typography } from '@douyinfe/semi-ui'
import { formatVersion } from '@/app/lib/status'
import type { FleetNode } from '@/app/lib/use-fleet'
import type { NodeConfigState } from '@/app/lib/fleet-config'
import styles from './fleet-config.module.scss'

const { Text } = Typography

/** 在线节点的配置同步情况；离线不显示 */
export function ConfigSyncTag({ state }: { state: NodeConfigState }) {
  switch (state.sync) {
    case 'unsupported':
      return (
        <Tooltip content="这台节点的 biliup 版本只收房间、不收配置（协议次版本低于 2），升级后才会应用 Fleet 配置">
          <Tag size="small" color="orange">
            不收配置
          </Tag>
        </Tooltip>
      )
    case 'pending':
      return (
        <Tooltip content="已下发最新的配置，节点还没确认">
          <Tag size="small" color="blue">
            配置下发中
          </Tag>
        </Tooltip>
      )
    case 'failed':
      return (
        <Tooltip content="节点没有应用最近下发的配置，仍按原来的配置运行">
          <Tag size="small" color="red">
            配置未生效
          </Tag>
        </Tooltip>
      )
    case 'applied':
      return (
        <Tag size="small" color="green">
          配置已生效
        </Tag>
      )
    default:
      return null
  }
}

/** 节点卡片上的配置行：同步情况、失败原因、版本提示、覆盖了几项，以及「节点覆盖」入口 */
export default function NodeConfigStatus({
  node,
  controllerVersion,
  onOpen,
}: {
  node: FleetNode
  controllerVersion?: string
  /** 没有查看配置的权限时不给入口 */
  onOpen?: () => void
}) {
  const state = node.config
  if (!state) return null
  const overrides = state.override_keys
  return (
    <div className={styles.status} role="group" aria-label="配置">
      <span className={styles.statusLabel}>配置</span>
      {node.online ? (
        <ConfigSyncTag state={state} />
      ) : (
        <Text type="tertiary" size="small">
          离线，上线后下发
        </Text>
      )}
      {state.outdated && state.sync !== 'unsupported' ? (
        <Tooltip
          content={`节点${node.version ? ` v${formatVersion(node.version)}` : ''} 比控制面${
            controllerVersion ? ` v${formatVersion(controllerVersion)}` : ''
          } 旧，建议升级到相同版本`}
        >
          <Tag size="small" color="orange">
            可升级
          </Tag>
        </Tooltip>
      ) : null}
      {overrides.length ? (
        <Tooltip content={`覆盖了：${overrides.join('、')}`}>
          <Tag size="small" color="white">
            覆盖 {overrides.length} 项
          </Tag>
        </Tooltip>
      ) : null}
      {onOpen ? (
        <Button size="small" theme="borderless" className={styles.statusAction} onClick={onOpen}>
          节点覆盖
        </Button>
      ) : null}
      {node.online && state.sync === 'failed' && state.error ? (
        <span className={styles.statusError} role="alert">
          {state.error}
        </span>
      ) : null}
    </div>
  )
}
