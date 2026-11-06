import type {GeneratorInfo} from '@/app/lib/editor/contracts'
import {generationCost,type GenerationPriceChoice} from '@/app/lib/editor/generation-pricing'
import {useEditorLanguage} from '@/app/lib/editor/i18n'

/** One price line; its linked tooltip explains the estimate and opens the official rate. */
export function GenerationCost({generator,request}:{generator:GeneratorInfo|undefined;request:GenerationPriceChoice}){
  useEditorLanguage()
  if(!generator)return null
  const cost=generationCost(generator,request)
  return <div className="gen-cost small muted" data-testid="generate-estimate"><a href={cost.url} target="_blank" rel="noreferrer" title={cost.detail} aria-label={`${cost.label}. ${cost.detail}`}>{cost.label}</a></div>
}
