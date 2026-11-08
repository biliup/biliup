import { useEffect, useRef } from 'react';
import { RefreshCw } from 'lucide-react';
import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
import { actions, useApp } from '@/app/lib/editor/state/store';
import { setSkipStoryboard, useSkipStoryboard } from '@/app/lib/editor/state/agentPrefs';
import { CheckRow, KeyRows, PendingRow, SpeechModelRow } from '@/app/ui/editor/onboarding/rows';
import { ModelSettings } from './ModelSettings';

/** Configuration belongs to the logged-in biliup deployment, not a second server connection. */
export function ServiceSettings() {
  useEditorLanguage();
  const { connection, settingsFocus, setupChecks, setupChecking, setupError } = useApp();
  const skip = useSkipStoryboard();
  const keys = useRef<HTMLDivElement>(null);
  const transcription = useRef<HTMLDivElement>(null);
  const online = connection === 'connected';
  useEffect(() => {
    if (online) void actions.checkSetup();
  }, [online]);
  useEffect(() => {
    const target = settingsFocus === 'keys' ? keys.current : settingsFocus === 'transcription' ? transcription.current : null;
    target?.focus({ preventScroll: true });
    target?.scrollIntoView({ block: 'nearest' });
  }, [settingsFocus]);
  const whisper = setupChecks?.find(check => check.id === 'whisper');
  return <div className="service-settings">
    <div className="group">
      <div className="group-label">{editorText('Agent')}</div>
      <ModelSettings />
      <label className="check-row">
        <input type="checkbox" checked={skip} onChange={event => setSkipStoryboard(event.target.checked)} />
        {editorText('Auto skips the storyboard approval')}
      </label>
    </div>
    <div className="group" ref={keys} tabIndex={-1} aria-label={editorText('API keys')}>
      <KeyRows />
    </div>
    <div className="group" ref={transcription} tabIndex={-1} aria-label={editorText('Transcription')}>
      <div className="group-label">{editorText('Transcription')}</div>
      <div className="service-settings-rows">
        {whisper && <CheckRow check={whisper} />}
        <SpeechModelRow />
        {setupChecking && !setupChecks && <PendingRow />}
      </div>
    </div>
    <div className="group">
      {setupError && <div className="inline-error" role="alert">{setupError}</div>}
      <button className="btn" disabled={!online || setupChecking} onClick={() => {
        void actions.checkSetup();
        void actions.listKeys();
      }}>
        <RefreshCw className={setupChecking ? 'spin' : ''} />{editorText('Refresh')}
      </button>
    </div>
  </div>;
}
