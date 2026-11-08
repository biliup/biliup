// Biliup's adaptive shutter sampling, applied to Chromium's transparent PNG frames.
import {BLUR_REFINE,blurFarPairs,blurLevels,blurMoving,blurRefinable,blurSignature,blurStep,type BlurSignature} from '../core/visual/motionBlur'
let canvas:HTMLCanvasElement,ctx:CanvasRenderingContext2D,sum:Uint16Array,signatures:BlurSignature[]=[]
let mean:ImageData|undefined,count=0
export function resetGraphicBlur(width:number,height:number){
  if(width*height>16_777_216)throw new Error('动态图形画布超过 1600 万像素')
  canvas?.remove();canvas=document.createElement('canvas');canvas.width=width;canvas.height=height;canvas.style.cssText='position:absolute;inset:0;display:none';document.body.append(canvas)
  ctx=canvas.getContext('2d',{willReadFrequently:true})!;sum=new Uint16Array(width*height*4);mean=undefined;beginGraphicSamples()
}
export function beginGraphicSamples(){sum.fill(0);count=0;signatures=[]}
export function showGraphicSource(){canvas.style.display='none';const frame=document.querySelector('iframe');if(frame)frame.style.visibility='visible'}
export async function addGraphicSample(png:string){
  const bitmap=await createImageBitmap(await (await fetch(`data:image/png;base64,${png}`)).blob())
  try{ctx.clearRect(0,0,canvas.width,canvas.height);ctx.drawImage(bitmap,0,0)}finally{bitmap.close()}
  const pixels=ctx.getImageData(0,0,canvas.width,canvas.height).data,bgra=new Uint8Array(pixels.length)
  for(let i=0;i<pixels.length;i+=4){const a=pixels[i+3]!;bgra[i]=Math.round(pixels[i+2]!*a/255);bgra[i+1]=Math.round(pixels[i+1]!*a/255);bgra[i+2]=Math.round(pixels[i]!*a/255);bgra[i+3]=a}
  for(let i=0;i<bgra.length;i++)sum[i]!+=bgra[i]!
  signatures.push(blurSignature(bgra,canvas.width,canvas.height));count++
}
export function finishGraphicSamples(){
  if(!count)throw new Error('图形采样为空')
  const result=new ImageData(canvas.width,canvas.height),pixels=result.data
  for(let i=0;i<sum.length;i+=4){const alpha=sum[i+3]!;pixels[i+3]=Math.round(alpha/count);if(alpha){pixels[i]=Math.round(sum[i+2]!*255/alpha);pixels[i+1]=Math.round(sum[i+1]!*255/alpha);pixels[i+2]=Math.round(sum[i]!*255/alpha)}}
  mean=result
}
export function nextGraphicSamples(ts:number[]){
  if(!blurRefinable(ts)||signatures.length!==ts.length||blurStep(signatures[0]!,signatures.at(-1)!)<BLUR_REFINE.points||!blurMoving(blurFarPairs(signatures),signatures.length))return []
  return blurLevels(ts,Math.max(BLUR_REFINE.max,ts.length))[0]??[]
}
export function showGraphicMean(){if(!mean)throw new Error('图形尚未采样');ctx.putImageData(mean,0,0);const frame=document.querySelector('iframe');if(frame)frame.style.visibility='hidden';canvas.style.display='block'}
