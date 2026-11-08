'use client'
import {useEffect} from 'react'
import {useRouter} from 'next/navigation'
export default function LegacyLogs(){const router=useRouter();useEffect(()=>router.replace('/runtime/logs'),[router]);return null}
