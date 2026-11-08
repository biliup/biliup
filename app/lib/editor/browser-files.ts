/** Browser file selections stay in this window until the owning command consumes them. */
import {selectBrowserFiles} from './browser-imports'
const picked=new Map<string,ReadonlyArray<File>>()
export async function selectBrandFiles(bundle=false,directory=false,accept='.svg,.png,.jpg,.jpeg,.webp,.gif,.ttf,.otf,.woff,.woff2,.mp4,.mov,.webm',multiple=true):Promise<string[]>{
  const files=(await selectBrowserFiles({directory,accept,multiple})).sort((a,b)=>(a.webkitRelativePath||a.name).localeCompare(b.webkitRelativePath||b.name,undefined,{numeric:true}))
  if(!files.length)return []
  const groups=bundle?[files]:files.map(file=>[file])
  return groups.map(files=>{const id=`browser-brand:${crypto.randomUUID()}`;picked.set(id,files);return id})
}
export function selectedBrandFiles(id:string){return picked.get(id)}
export function releaseBrandFiles(id:string){picked.delete(id)}
