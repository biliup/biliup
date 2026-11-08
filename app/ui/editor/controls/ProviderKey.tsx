import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
import {useMe} from '@/app/lib/use-me';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import type { KeyProvider } from "@/app/lib/editor/contracts";
import { useState, useRef, useEffect, useContext } from "react";
import { actions, useApp } from "@/app/lib/editor/state/store";
import {EditorStoreContext} from '@/app/lib/editor/state/store/core';
import {isTextComposition} from '@/app/lib/editor/keyboard';

/**
 * A missing key or client ID, pasted right here (D99: it goes to the server once, is kept in the
 * deployment's configured secret store and never comes back), as the Generate tab does.
 */
export function ProviderKey({ provider, text, placeholder, link }: { provider: KeyProvider; text: string; placeholder: string; link?: { readonly href: string; readonly label: string } | undefined }) {
  useEditorLanguage();
  const app = useApp();
  const owner=useContext(EditorStoreContext);
  const {can}=useMe();
  const [key, setKey] = useState("");
  const [saving, setSaving] = useState(false);
  const pending=useRef(false),epoch=useRef(0);
  useEffect(()=>{
    epoch.current++;pending.current=false;setSaving(false);setKey('');
    return ()=>{epoch.current++};
  },[owner,provider]);
  const save = async () => {
    if (!key.trim() || !can('config.edit') || app.connection!=='connected' || pending.current) return;
    const request=epoch.current;
    pending.current=true;
    setSaving(true);
    try{const ok=await actions.setKey(provider,key.trim());if(ok&&request===epoch.current)setKey('');}
    finally{if(request===epoch.current){pending.current=false;setSaving(false);}}
  };
  return (
    <div className="lib-key small" data-testid={`key-needed-${provider}`}>
      <span>
        {editorText(text)}
        {link && (
          <>
            {" "}
            <a href={link.href} target="_blank" rel="noreferrer">
              {editorText(link.label)}
            </a>
          </>
        )}
      </span>
      <div className="lib-key-row">
        <input
          className="field input"
          type="password"
          disabled={saving||!can('config.edit')}
          data-testid={`lib-key-input-${provider}`}
          placeholder={placeholder}
          autoComplete="off"
          spellCheck={false}
          value={key}
          onChange={(e) => setKey(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter"&&!isTextComposition(e.nativeEvent)){e.preventDefault();void save();}
          }}
        />
        <button className="btn" data-testid={`lib-key-save-${provider}`} disabled={!key.trim() || saving || app.connection !== "connected" || !can('config.edit')} onClick={() => void save()}>
          {editorText("Save")}</button>
      </div>
      {app.keysError && <div className="inline-error small">{app.keysError}</div>}
      <button className="link-btn" onClick={() => actions.openKeys()}>
        {editorText("Settings › Keys")}</button>
    </div>
  );
}
