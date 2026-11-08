import type {GeneratorInfo} from './contracts'
import {editorText} from './i18n'

const dollars=(value:number)=>`US$${value.toFixed(3)}`
export type GenerationPriceChoice={readonly size?:string|undefined;readonly seconds?:number|undefined}

/** Show only the priced output or duration-equivalent amount; never imply a final invoice. */
export function generationCost(generator:GeneratorInfo,request:GenerationPriceChoice){
  const rate=generator.pricing
  let label:string,detail:string
  switch(rate.kind){
    case 'image':{
      const amount=rate.usdBySize[request.size??generator.sizes[0]??'']
      label=amount===undefined?editorText('Cost depends on the selected size'):editorText('Output ≈ {0} + input',{0:dollars(amount)})
      detail=editorText('Image output at {0} quality; prompt input is billed separately.',{0:editorText(rate.quality)})
      break
    }
    case 'video':
      label=editorText('Output ≈ {0}–{1} + other usage',{0:dollars(rate.usdPerSecond*rate.secondsMin),1:dollars(rate.usdPerSecond*rate.secondsMax)})
      detail=editorText('720p video, {0}–{1} seconds chosen by the model. Input and text output are billed separately.',{0:rate.secondsMin,1:rate.secondsMax})
      break
    case 'audio':{
      const seconds=request.seconds??5
      label=Number.isFinite(seconds)&&seconds>=0.5&&seconds<=30
        ?editorText('Reference ≈ {0}',{0:dollars(seconds*rate.usdPerMinute/60)})
        :editorText('Choose a valid sound duration')
      detail=editorText('Converted from the public {0}/minute rate. Sound effects are settled per generation; your account determines the actual charge.',{0:dollars(rate.usdPerMinute)})
      break
    }
    case 'usage':
      label=editorText('Billed by actual model usage')
      detail=editorText('The current text model determines the charge. Its output length is not known before generation.')
  }
  return {
    label,
    detail:`${detail} ${editorText('Public rates as of {0}; excludes taxes and account discounts.',{0:rate.asOf})}`,
    url:rate.url,
  }
}
