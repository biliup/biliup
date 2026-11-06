/** Browser keyboard conventions, shared by shortcuts and pointer gestures. */
export const usesCommandKey = () => typeof navigator !== 'undefined' && /Mac|iPhone|iPad|iPod/.test(navigator.platform);

export interface Modifiers {
  readonly metaKey: boolean;
  readonly ctrlKey: boolean;
}

export const primaryModifier = (event: Modifiers) => usesCommandKey() ? event.metaKey : event.ctrlKey;

export interface CompositionKey {
  readonly key: string;
  readonly isComposing?: boolean;
  readonly keyCode?: number;
  getModifierState?(key: string): boolean;
}

/** IME confirmation/cancellation and AltGr text entry never edit the timeline. */
export const isTextComposition = (event: CompositionKey) => !!event.isComposing || event.keyCode === 229
  || ['Process', 'Dead', 'Unidentified'].includes(event.key) || !!event.getModifierState?.('AltGraph');
