import { useContext, useEffect, useRef, useState } from 'react';
import { apiFetch, fetcher, handleResponse } from '@/app/lib/api-streamer';
import { useMe } from '@/app/lib/use-me';
import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
import { actions, useApp } from '@/app/lib/editor/state/store';
import { currentEditorStore, EditorStoreContext } from '@/app/lib/editor/state/store/core';
import { validAgentModel, type AgentModelProvider } from '@/app/lib/editor/state/agentPrefs';
import { useEditorAction } from '../controls/useEditorAction';

type Settings = { provider: AgentModelProvider; model: string };

export function ModelSettings() {
  useEditorLanguage();
  const { can } = useMe();
  const { connection } = useApp();
  const owner = useContext(EditorStoreContext);
  const { busy, error, setError, run } = useEditorAction();
  const [value, setValue] = useState<Settings>();
  const [loading, setLoading] = useState(false);
  const [reload, setReload] = useState(0);
  const dirty = useRef(false), drafts = useRef<Partial<Record<AgentModelProvider, string>>>({});
  const online = connection === 'connected';
  const disabled = !online || !can('config.edit') || busy || loading;
  useEffect(() => {
    if (!online) return;
    let disposed = false;
    const controller = new AbortController();
    setError('');
    setLoading(true);
    void fetcher('/v1/editor/model', { signal: controller.signal }).then((data: Settings) => {
      if (!disposed && currentEditorStore() === owner && !dirty.current) {
        drafts.current[data.provider] = data.model;
        setValue(data);
      }
    }).catch(error => {
      if (!disposed && currentEditorStore() === owner) setError(error instanceof Error ? error.message : String(error));
    }).finally(() => { if (!disposed) setLoading(false); });
    return () => { disposed = true; controller.abort(); };
  }, [online, reload, owner]);
  const change = (next: Settings) => {
    dirty.current = true;
    drafts.current[next.provider] = next.model;
    setValue(next);
  };
  const save = () => {
    if (!value || disabled || !validAgentModel(value.model.trim())) return;
    const settings = { ...value, model: value.model.trim() };
    return run(async current => {
      const response = await apiFetch('/v1/editor/model', {
        method: 'PUT', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(settings),
      });
      await handleResponse(response);
      const saved = await response.json() as Settings;
      if (!current()) return;
      dirty.current = false;
      drafts.current[saved.provider] = saved.model;
      setValue(saved);
      void actions.listGenerators();
      void actions.checkSetup();
      void actions.agentRefresh();
    }, { readOnly: true }); // This control requires config.edit, independently of clip.edit.
  };
  return <section className="model-settings">
    <div className="wiz-name">{editorText('Text and graphics model')}</div>
    {value && <div className="wiz-row">
      <select className="wiz-select" aria-label={editorText('Provider')} disabled={disabled} value={value.provider} onChange={event => {
        const provider = event.target.value as Settings['provider'];
        change({ provider, model: drafts.current[provider] ?? '' });
      }}><option value="openai">OpenAI</option><option value="google">Google</option></select>
      <input className="field input model-name" aria-label={editorText('Model')} placeholder={editorText('模型 ID')} maxLength={100} disabled={disabled} value={value.model} onChange={event => change({ ...value, model: event.target.value })} />
      <button className="btn" disabled={disabled || !validAgentModel(value.model.trim())} onClick={() => void save()}>{editorText('Save')}</button>
    </div>}
    {value?.model.trim() && !validAgentModel(value.model.trim()) && <div className="inline-error" role="alert">{editorText('Model ID may contain letters, digits, dots, underscores and hyphens.')}</div>}
    {error && <div className="inline-error" role="alert">{error}</div>}
    {!value && error && <button className="btn" disabled={!online || loading} onClick={() => setReload(n => n + 1)}>{editorText('Retry')}</button>}
  </section>;
}
