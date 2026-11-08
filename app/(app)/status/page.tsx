'use client'
import {useEffect} from 'react'
import {useRouter} from 'next/navigation'
export default function TaskPlatform(){const router=useRouter();useEffect(()=>router.replace('/nodes'),[router]);return null}
