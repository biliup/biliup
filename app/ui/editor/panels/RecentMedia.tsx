import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Media › Recent from other projects (issue #39, D205): captures, imports and generations from the
// server's other projects, newest first. Add reuses a registered source and the ordinary
// project save retains it independently of its previous owner; one undo takes it out.
import type { MediaRecentEntry, MediaRecentPage } from "@/app/lib/editor/contracts";
import { AudioLines, Film, Image as ImageIcon, Plus, RefreshCw, Search } from "lucide-react";
import { useContext, useEffect, useRef, useState } from "react";

import { actions, recentThumbUrl, useApp, type Project } from "@/app/lib/editor/state/store";
import { currentEditorStore, EditorStoreContext } from '@/app/lib/editor/state/store/core';
import { useEditorAction } from '../controls/useEditorAction';
import { captureEditorFocus } from '../preview/focus';

const ORIGIN: Record<MediaRecentEntry["origin"], string> = { capture: "Recording", import: "Import", generated: "Generated" };

const length = (seconds: number | undefined) => {
  if (seconds === undefined) return '';
  const total = Math.max(0, Math.round(seconds));
  return total < 60 ? `${total}${editorText(' s')}` : `${Math.floor(total / 60)}:${String(total % 60).padStart(2, '0')}`;
};

/** Project and source information, without exposing server filesystem paths. */
export const recentMeta = (e: MediaRecentEntry) => [e.project.name,editorText(ORIGIN[e.origin]),length(e.seconds), e.device].filter(Boolean).join(" · ");

export function RecentSection({ project }: { project: Project }) {
  useEditorLanguage();
  const app = useApp();
  const owner = useContext(EditorStoreContext);
  const online = app.connection === "connected";
  const { busy, error, editable, run } = useEditorAction();
  const [query, setQuery] = useState("");
  const [broken,setBroken]=useState<Record<string,boolean>>({});
  const [loadError,setLoadError]=useState('');
  const [kind,setKind]=useState('');
  const [refresh,setRefresh]=useState(0);
  const [adding,setAdding]=useState<string>();
  // Replacement and undo can change the available sources without changing the media count.
  const mediaKey=JSON.stringify(project.media.map(media=>[media.path,media.kind]));
  const search=query.trim(),listKey=JSON.stringify([search,kind,mediaKey,refresh]);
  const currentList=useRef(listKey);
  currentList.current=listKey;
  const request=useRef<object|undefined>(undefined);
  const [result,setResult]=useState<{key:string;page:MediaRecentPage}>();
  const page=result?.key===listKey?result.page:undefined;
  const [loading,setLoading]=useState(false);
  const load=async(before?:string)=>{
    const token={};request.current=token;
    const accepts=()=>request.current===token&&currentList.current===listKey&&currentEditorStore()===owner;
    setLoading(true);setLoadError('');
    try{
      const found=await actions.mediaRecent({query:search,kind,...(before?{before}:{})});
      if(!accepts())return;
      setResult(previous=>({key:listKey,page:{
        entries:before&&previous?.key===listKey?[...previous.page.entries,...found.entries]:found.entries,
        next:found.next,
      }}));
    }catch(cause){if(accepts())setLoadError(cause instanceof Error?cause.message:String(cause))}
    finally{if(accepts()){request.current=undefined;setLoading(false)}}
  };
  useEffect(() => {
    setResult(undefined);setLoadError('');setLoading(online);
    if (!online) return;
    const timer = setTimeout(() => void load(), search ? 200 : 0);
    return () => {
      clearTimeout(timer);
      request.current=undefined;
    };
  }, [online,owner,listKey]);
  const more=()=>{if(online&&!request.current)void load(page?.next??undefined)};
  const add=(entry:MediaRecentEntry)=>void run(async current=>{
    const unchanged=captureEditorFocus();
    setAdding(entry.id);
    const imported=await actions.mediaRecentImport(entry.id);
    if(!current())return;
    setRefresh(value=>value+1);
    if(unchanged())actions.showInMedia(imported.mediaId);
  });
  return (
    <div className="lib-section" data-testid="recent-media">
      <div className="lib-label media-bar">
        <span>{editorText("Recent from other projects")}</span>
        <button className="icon-btn" disabled={!online||loading} title={editorText('Refresh')} aria-label={editorText('Refresh')} onClick={()=>{setBroken({});setRefresh(value=>value+1)}}><RefreshCw className={loading?'spin':undefined}/></button>
      </div>
      <label className="recent-search">
        <Search />
        <input className="field input" data-testid="recent-media-search" value={query} aria-label={editorText("Search other projects")} placeholder={editorText("Search other projects")} onChange={(ev) => setQuery(ev.target.value)} />
      </label>
      <select className="field input" aria-label={editorText('Media type')} value={kind} onChange={e=>setKind(e.target.value)}>
        <option value="">{editorText('All media')}</option>
        {Object.entries({video:'Video',audio:'Audio',image:'Image',capture:'Recording',generated:'Generated',import:'Import'}).map(([value,label])=><option key={value} value={value}>{editorText(label)}</option>)}
      </select>
      {loadError&&<div className="import-report"><span role="alert">{loadError}</span><button className="link-btn" disabled={!online||loading} onClick={more}>{editorText('Retry')}</button></div>}
      {error&&<div className="import-line error" role="alert">{error}</div>}
      <div className="gen-results" aria-busy={loading}>
        {!loadError&&!loading&&page?.entries.length === 0 && <div className="recent-none">{editorText("Nothing matches.")}</div>}
        {page?.entries.map((e) => {
          const Icon = e.kind === "audio" ? AudioLines : e.kind === "image" ? ImageIcon : Film;
          return (
            <div key={e.id} className="gen-result" data-testid="recent-media-item" data-id={e.id}>
              <div className="gen-thumb">{e.thumb&&!broken[e.id] ? <img src={recentThumbUrl(e.id)} alt="" loading="lazy" draggable={false} onError={()=>setBroken(old=>({...old,[e.id]:true}))} /> : <Icon />}</div>
              <div className="gen-body">
                <div className="gen-title" title={e.name}>
                  {e.name}
                </div>
                <div className="gen-by" data-testid="recent-media-meta">
                  {recentMeta(e)}
                </div>
                <div className="gen-actions">
                  {project.media.some(media=>media.path===e.path&&media.kind===e.kind) ? (
                    <span className="gen-by" data-testid="recent-media-added">
                      {editorText('Added')}
                    </span>
                  ) : (
                    <button className="link-btn" data-testid="recent-media-add" disabled={!editable||busy} title={editorText("Add it to this project's media")} onClick={() => add(e)}>
                      {busy&&adding===e.id?<RefreshCw className="spin"/>:<Plus />}
                      {editorText(busy&&adding===e.id?'Adding…':"Add to this project")}</button>
                  )}
                </div>
              </div>
            </div>
          );
        })}
      </div>
      {loading&&<div className="recent-none" role="status">{editorText('Loading…')}</div>}
      {page?.next&&!loadError&&<button className="link-btn" disabled={!online||loading} onClick={more}>{editorText('Show more')}</button>}
    </div>
  );
}
