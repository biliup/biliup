'use client'
import {useEffect} from 'react'
import {useRouter} from 'next/navigation'
export default function LegacyOutputsPage(){const router=useRouter();useEffect(()=>router.replace('/outputs'),[router]);return null}
