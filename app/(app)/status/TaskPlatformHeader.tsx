'use client'
import type {ReactNode} from 'react'
import {IconSetting} from '@douyinfe/semi-icons'
import PageHeader from '../components/PageHeader'
export default function TaskPlatformHeader({actions}:{active:'nodes';actions?:ReactNode}){return <PageHeader icon={<IconSetting size="large"/>} title="任务平台" actions={actions}/>}
