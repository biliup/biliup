import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The Audio and Sound effects tabs (D33, D54, D134, D249, D250): the online library (Openverse
// without a key, Freesound with the user's key: CC0 and CC BY only), uploaded folders,
// the bundled CC0 sound-effects pack, and Copy credits for what needs it (D135).
import { creditsOf, creditsText } from "@/app/lib/editor/core/audio/library";
import { Check, Copy, Plus, RefreshCw } from "lucide-react";
import { useContext, useEffect, useRef, useState } from "react";

import { copyText } from "@/app/lib/editor/clipboard";
import { actions, activeJobs, useApp, type Project } from "@/app/lib/editor/state/store";
import { SoundBrowser } from "@/app/ui/editor/panels/audio/SoundBrowser";
import {currentEditorStore,EditorStoreContext} from '@/app/lib/editor/state/store/core';
import {useEditorAction} from '../controls/useEditorAction';

/**
 * The Audio tab (D33, D54, D249): music from uploaded folders or online sources
 * (one search field and a source select). The project's own
 * sounds live in Media; + imports a file there.
 */
export function AudioLibrary({ project }: { project: Project }) {
  useEditorLanguage();
  const app = useApp();
  const online = app.connection === "connected";
  const importing = activeJobs(app).length > 0;
  return (
    <aside className="library" data-testid="audio-library">
      <div className="lib-head">
        {editorText("Audio")}<span className="lib-head-actions">
          <CopyCredits project={project} />
          <button className="icon-btn" data-testid="audio-import" disabled={!online||!app.biliupCanEdit} title={editorText("Import music into the project (it shows in Media)")} onClick={() => void actions.importMedia()}>
            {importing ? <RefreshCw className="spin" /> : <Plus />}
          </button>
        </span>
      </div>
      <SoundBrowser kind="music" project={project} online={online} />
    </aside>
  );
}

/** The Sound effects tab (D134): the online library's sound effects by category, and the bundled pack. */
export function SfxLibrary({ project }: { project: Project }) {
  useEditorLanguage();
  const online = useApp().connection === "connected";
  return (
    <aside className="library" data-testid="sfx-library">
      <div className="lib-head">
        {editorText("Sound effects")}<span className="lib-head-actions">
          <CopyCredits project={project} />
        </span>
      </div>
      <SoundBrowser kind="sfx" project={project} online={online} />
    </aside>
  );
}

/** Copies the project's credits (D135: CC BY attributions, then the CC0 sounds) for the video's description. */
function CopyCredits({ project }: { project: Project }) {
  useEditorLanguage();
  const [copied, setCopied] = useState(false);
  const owner=useContext(EditorStoreContext),timer=useRef<ReturnType<typeof setTimeout>|undefined>(undefined);
  const {busy,error,run}=useEditorAction();
  useEffect(()=>{
    setCopied(false);
    return ()=>clearTimeout(timer.current);
  },[owner]);
  useEffect(()=>{if(error&&owner&&currentEditorStore()===owner)owner.set({notice:{text:error,transport:false}});},[error,owner]);
  const credits = creditsOf(project);
  const count = credits.attribution.length + credits.publicDomain.length;
  if (!count) return null;
  return (
    <button
      className={`icon-btn${copied ? " done" : ""}`}
      data-testid="copy-credits"
      data-attribution={credits.attribution.length}
      disabled={busy}
      title={credits.attribution.length ? editorText("Copy credits ({0} need{1} attribution)", { 0: credits.attribution.length, 1: credits.attribution.length === 1 ? "s" : "" }) : editorText("Copy credits")}
      onClick={(e) => {
        e.currentTarget.blur();
        clearTimeout(timer.current);setCopied(false);
        void run(()=>copyText(creditsText(project)),{readOnly:true,onSuccess:()=>{
          setCopied(true);timer.current=setTimeout(()=>setCopied(false),1500);
        }});
      }}
    >
      {copied ? <Check /> : <Copy />}
    </button>
  );
}
