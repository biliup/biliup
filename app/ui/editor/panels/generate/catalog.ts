// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Model choices come exclusively from the current server's generator catalog.
import type { GeneratorInfo } from '@/app/lib/editor/contracts';

export type GenType = 'image' | 'video' | 'audio';
export type Brand = 'openai' | 'google' | 'elevenlabs' | 'other';
export interface Model {
  readonly id: string;
  readonly generator: string;
  readonly name: string;
  readonly lab: string;
  readonly credit?: string;
  readonly brand: Brand;
  readonly info: GeneratorInfo;
}

const PRESENTATION: Record<string, { name: Partial<Record<GenType, string>>; lab: string; credit?: string; brand: Brand }> = {
  'generate.svg': { name: { image: 'SVG drawing', video: 'Vector animation' }, lab: '', brand: 'other' },
  'generate.gpt-image': { name: { image: 'GPT Image' }, lab: 'OpenAI', brand: 'openai' },
  'generate.omni': { name: { video: 'Gemini Omni Flash' }, lab: 'Google', brand: 'google' },
  'generate.sound': { name: { audio: 'Sound effects' }, lab: 'ElevenLabs', brand: 'elevenlabs' },
};

export function modelsFor(generators: ReadonlyArray<GeneratorInfo>, type: GenType): Array<Model> {
  return generators.filter(generator => generator.kinds.includes(type))
    .sort((a, b) => Number(b.ready) - Number(a.ready))
    .map((generator): Model => {
      const look = PRESENTATION[generator.id];
      return {
        id: generator.id, generator: generator.id, info: generator,
        name: look?.name[type] ?? generator.name,
        lab: look?.lab || generator.provider || 'Other',
        ...(look?.credit ? { credit: look.credit } : {}),
        brand: generator.id === 'generate.svg'
          ? generator.provider === 'google' ? 'google' : generator.provider === 'openai' ? 'openai' : 'other'
          : look?.brand ?? 'other',
      };
    });
}

export const firstModel = (models: ReadonlyArray<Model>) => models.find(model => model.info.ready) ?? models[0];
