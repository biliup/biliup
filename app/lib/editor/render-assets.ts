import {fetcher} from '../api-streamer'
import type {BrandState} from './contracts'
import type {RenderAssets} from './core/visual/renderAssets'
import type {ProjectFont} from './project-fonts'
export function renderAssetsKey(dir:string|undefined,brand:BrandState|undefined,fonts?:ReadonlyArray<ProjectFont>):string{
  const kit=brand?.kits.find(kit=>kit.id===brand.active.kit)
  return JSON.stringify({dir,known:brand!=null,kit:kit??null,fonts:fonts?.map(font=>[font.hash,font.family,font.weight,font.style])??null})
}
const cache=new Map<string,Promise<RenderAssets>>()
export function loadRenderAssets(dir:string,key:string):Promise<RenderAssets>{
  const id=/^biliup:(\d+)$/.exec(dir)?.[1];if(!id)return Promise.reject(new Error('工程尚未连接'))
  const cached=cache.get(key);if(cached)return cached
  const promise=(fetcher(`/v1/editor/projects/${id}/render-assets`) as Promise<RenderAssets>).catch(error=>{cache.delete(key);throw error})
  cache.set(key,promise);while(cache.size>4)cache.delete(cache.keys().next().value!)
  return promise
}
