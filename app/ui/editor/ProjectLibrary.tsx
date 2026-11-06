'use client'
import {useEffect,useState} from 'react'
import Link from 'next/link'
import {useRouter} from 'next/navigation'
import useSWR,{mutate} from 'swr'
import {Button,Input,Spin} from '@douyinfe/semi-ui'
import {Clapperboard,Plus,Search,Trash2} from 'lucide-react'
import PageHeader from '@/app/(app)/components/PageHeader'
import {FormDialog} from '@/app/ui/shell'
import {fetcher} from '@/app/lib/api-streamer'
import {deleteProject,projectsUrl,type ProjectSummary} from '@/app/lib/editor/api'
import {useMe} from '@/app/lib/use-me'
import {projectEditorHref} from '@/app/lib/workbench'
import styles from './project-library.module.scss'

const PAGE_SIZE=20
export default function ProjectLibrary(){
 const {can}=useMe(),router=useRouter()
 const [search,setSearch]=useState(''),[query,setQuery]=useState(''),[cursors,setCursors]=useState<number[]>([])
 const [deleting,setDeleting]=useState<ProjectSummary>(),[busy,setBusy]=useState(false),[deleteError,setDeleteError]=useState('')
 useEffect(()=>{const timer=setTimeout(()=>{setQuery(search.trim());setCursors([])},250);return()=>clearTimeout(timer)},[search])
 const url=`${projectsUrl}?limit=${PAGE_SIZE+1}&q=${encodeURIComponent(query)}${cursors.length?'&before='+cursors.at(-1):''}`
 const {data,error,isValidating}=useSWR<ProjectSummary[]>(url,fetcher)
 const rows=data?.slice(0,PAGE_SIZE)??[]
 const remove=async()=>{
  if(!deleting||busy)return
  setBusy(true);setDeleteError('')
  try{
   await deleteProject(deleting)
   setDeleting(undefined)
   if(rows.length===1&&cursors.length)setCursors(previous=>previous.slice(0,-1))
   await mutate(key=>typeof key==='string'&&(key===projectsUrl||key.startsWith(projectsUrl+'?')))
  }catch(cause){setDeleteError(cause instanceof Error?cause.message:String(cause))}
  finally{setBusy(false)}
 }
 return <><PageHeader title="剪辑工程" icon={<Clapperboard size={24}/>} actions={can('clip.edit')?<Button icon={<Plus size={16}/>} theme="solid" onClick={()=>router.push('/workbench?new=1')}>新建工程</Button>:undefined}/>
  <div className={styles.content}>
   <div className={styles.toolbar}><Input className={styles.search} prefix={<Search size={16}/>} aria-label="搜索工程" placeholder="搜索工程" maxLength={256} value={search} onChange={setSearch} showClear/></div>
   {error?<p role="alert" className={styles.error}>工程加载失败：{error instanceof Error?error.message:String(error)}</p>:!data?<div className={styles.empty}><Spin/></div>:!rows.length?<div className={styles.empty}>{query?'没有找到工程':'暂无工程'}</div>:<div className={styles.list}>
    <div className={styles.columns}><span>工程</span><span>更新时间</span><span/></div>
    {rows.map(project=><article className={styles.row} key={project.id}><Link className={styles.project} href={projectEditorHref(project.id)}><Clapperboard size={20}/><span>{project.name}</span></Link><time dateTime={new Date(project.updated_at).toISOString()}>{new Date(project.updated_at).toLocaleString()}</time>{can('clip.edit')&&<Button type="danger" theme="borderless" icon={<Trash2 size={16}/>} aria-label={`删除工程「${project.name}」`} title="删除工程" disabled={busy} onClick={()=>{setDeleting(project);setDeleteError('')}}/>}</article>)}
   </div>}
   {(cursors.length>0||(data?.length??0)>PAGE_SIZE)&&<div className={styles.pager}><Button disabled={busy||isValidating||!cursors.length} onClick={()=>setCursors(previous=>previous.slice(0,-1))}>上一页</Button><span>第 {cursors.length+1} 页</span><Button disabled={busy||isValidating||(data?.length??0)<=PAGE_SIZE} onClick={()=>setCursors(previous=>[...previous,rows.at(-1)!.id])}>下一页</Button></div>}
  </div>
  {deleting&&<FormDialog title={`删除「${deleting.name}」？`} size="sm" okText="删除工程" okType="danger" confirmLoading={busy} closable={!busy} onCancel={()=>{if(!busy)setDeleting(undefined)}} onOk={()=>void remove()}>
   <p>工程和保存版本将永久删除。已导出的文件保留；原始录像解除工程引用后，按现有清理策略处理。</p>
   {deleteError&&<p className={styles.error} role="alert">{deleteError}</p>}
  </FormDialog>}
 </>
}
