import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Browser capture reuses the portable MediaRecorder implementation.
import { CaptureBrowser } from './CaptureBrowser'
export function CaptureLibrary() {
  useEditorLanguage(); return <aside className="library" data-testid="capture-library"><div className="lib-head">{editorText("Capture")}</div><CaptureBrowser /></aside> }
