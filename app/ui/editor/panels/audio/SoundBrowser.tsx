import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import type { LibrarySearchResult } from "@/app/lib/editor/contracts";
import { type LibraryKind, type LibrarySource, MUSIC_GENRES, SFX_CATEGORIES, hitKey } from "@/app/lib/editor/core/audio/library";
import { Search, RefreshCw } from "lucide-react";
import { useState, useRef, useEffect, useContext } from "react";
import { type Project, useApp, actions } from "@/app/lib/editor/state/store";
import { usePreview } from "@/app/ui/editor/panels/audio/preview";
import { HitRow, IncludedSfx } from "@/app/ui/editor/panels/audio/rows";
import { FolderSounds } from "@/app/ui/editor/panels/audio/FolderSounds";
import { ProviderKey } from "@/app/ui/editor/controls/ProviderKey";
import { GenerateSfx } from "@/app/ui/editor/panels/audio/GenerateSfx";
import {EditorStoreContext} from '@/app/lib/editor/state/store/core';
import {isTextComposition} from '@/app/lib/editor/keyboard';

/**
 * Bundled effects, uploaded folders and online sources use the same hosted media importer.
 */
export type BrowserSource = "pack" | "folders" | LibrarySource;
const SOURCE_LABEL: Readonly<Record<BrowserSource, string>> = {
  pack: "Included sounds",
  folders: "Your folders",
  openverse: "Openverse",
  freesound: "Freesound",
};
const isOnline = (s: BrowserSource): s is LibrarySource => s === "openverse" || s === "freesound";
const SOURCES: Readonly<Record<LibraryKind, ReadonlyArray<BrowserSource>>> = {
  music: ["folders", "openverse", "freesound"],
  sfx: ["pack", "openverse", "freesound", "folders"],
};
const DEFAULT: Readonly<Record<LibraryKind, BrowserSource>> = { music: "openverse", sfx: "pack" };
const SOURCE_KEY = (kind: LibraryKind) => `biliup.library.source.${kind}`;

function savedSource(kind: LibraryKind): BrowserSource {
  try {
    const saved = localStorage.getItem(SOURCE_KEY(kind)) as BrowserSource | null;
    if (saved && SOURCES[kind].includes(saved)) return saved;
  } catch {}
  return DEFAULT[kind];
}

/**
 * The library for one kind of sound (D249): one search field and one source select (plus a mood or
 * kind select where it narrows), then the source's list. Local sources filter as you type; online
 * ones search on Enter. Nothing goes online until a search is asked for.
 */
export function SoundBrowser({ kind, project, online }: { kind: LibraryKind; project: Project; online: boolean }) {
  useEditorLanguage();
  const app = useApp();
  const owner=useContext(EditorStoreContext);
  const [source, setSource] = useState<BrowserSource>(() => savedSource(kind));
  const [query, setQuery] = useState("");
  const [mood, setMood] = useState<string | undefined>();
  const [keyed, setKeyed] = useState<ReadonlyArray<LibrarySource>>(["openverse"]);
  const [result, setResult] = useState<(LibrarySearchResult & { readonly words: string }) | undefined>();
  const [error, setError] = useState<string | undefined>();
  const [busy, setBusy] = useState(false);
  const [, tick] = useState(0);
  const preview = usePreview();
  const seq = useRef(0);
  const pending=useRef<{key:string;sequence:number}|undefined>(undefined);

  useEffect(() => {
    if (!online) return;
    let live=true;
    void actions.libraryState().then(s=>{if(live&&s)setKeyed(s.sources)});
    return ()=>{live=false};
  }, [online, app.keys,owner]);
  useEffect(()=>{
    seq.current++;pending.current=undefined;setBusy(false);setResult(undefined);setError(undefined);
    return ()=>{seq.current++;pending.current=undefined};
  },[owner,online,kind]);
  // A limit's countdown.
  useEffect(() => {
    if (!result?.limit) return;
    const until=Date.parse(result.limit.until);
    if(!Number.isFinite(until)||until<=Date.now())return;
    const timer = setInterval(() => {tick(n=>n+1);if(Date.now()>=until)clearInterval(timer);}, 1000);
    return () => clearInterval(timer);
  }, [result?.limit]);

  const moods = kind === "music" ? MUSIC_GENRES : SFX_CATEGORIES;
  const moodQuery = (id: string | undefined) => moods.find((m) => m.id === id)?.query ?? "";
  const words = (q: string, m: string | undefined) => [q.trim(), kind === "music" || !q.trim() ? moodQuery(m) : ""].filter(Boolean).join(" ");
  const needsKey = (s: BrowserSource) => s === "freesound" && !keyed.includes(s);

  const search = async (text: string, opts: { page?: number; source?: BrowserSource } = {}) => {
    const s = opts.source ?? source;
    const q = text.trim();
    if (!q || !online || !isOnline(s) || needsKey(s)) return;
    if(result?.source===s&&result.limit&&Date.parse(result.limit.until)>Date.now())return;
    const page = opts.page ?? 1;
    const key=JSON.stringify([kind,s,q,page]);
    if(pending.current?.key===key)return;
    const n = ++seq.current;
    pending.current={key,sequence:n};
    setBusy(true);
    setError(undefined);
    try{
      const reply = await actions.searchLibrary({ kind, query: q, source: s, page });
      if (n !== seq.current) return;
      if ("error" in reply) return setError(reply.error);
      setResult(before=>{
        if(page>1&&before&&before.words===q&&before.source===reply.source){
          if(reply.limit)return {...before,limit:reply.limit};
          return {...reply,hits:[...new Map([...before.hits,...reply.hits].map(hit=>[hitKey(hit),hit])).values()],words:q};
        }
        return {...reply,words:q};
      });
    }catch(problem){if(n===seq.current)setError(problem instanceof Error?problem.message:String(problem));}
    finally{if(pending.current?.sequence===n)pending.current=undefined;if(n===seq.current)setBusy(false);}
  };
  const pick = (s: BrowserSource) => {
    setSource(s);
    try {
      localStorage.setItem(SOURCE_KEY(kind), s);
    } catch {}
    preview.stop();
    // D262: a search still running for the old source is dropped (its hits would show under this one).
    seq.current++;
    pending.current=undefined;
    setBusy(false);
    setResult(undefined);
    setError(undefined);
    if (isOnline(s) && (query.trim() || mood)) void search(words(query, mood), { source: s });
  };

  const searchId = kind === "music" ? "music-search" : "sfx-search";
  const placeholder = source === "pack" ? editorText('Search included sounds') : source === "folders" ? editorText("Search your folders") : editorText(kind==='music'?'Search music on {0}':'Search sound effects on {0}',{0:SOURCE_LABEL[source]});
  const showMood = source === "pack" || isOnline(source);
  const hits = result?.hits ?? [];
  const limit = result?.limit;
  const retryAt=limit?Date.parse(limit.until):NaN;
  const wait=Number.isFinite(retryAt)?Math.max(0,Math.ceil((retryAt-Date.now())/1000)):undefined;
  const canRetry=wait===0;
  const options = SOURCES[kind];

  return (
    <div className="lib-section sound-browser" data-testid={`${kind}-browser`} data-view={source}>
      <div className="lib-search">
        <Search />
        <input
          className="field input"
          data-testid={searchId}
          placeholder={placeholder}
          value={query}
          spellCheck={false}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter"&&!isTextComposition(e.nativeEvent)){e.preventDefault();void search(words(query, mood));}
          }}
        />
      </div>
      <div className="lib-selects">
        <select className="field input" data-testid="library-source" aria-label={editorText("Source")} value={source} onChange={(e) => pick(e.target.value as BrowserSource)}>
          {options.map((s) => (
            <option key={s} value={s} data-testid={`library-source-${s}`}>
              {editorText(SOURCE_LABEL[s])}
            </option>
          ))}
        </select>
        {showMood && (
          <select
            className="field input"
            data-testid="library-mood"
            aria-label={kind === "music" ? editorText("Mood") : editorText("Kind of sound")}
            value={mood ?? ""}
            onChange={(e) => {
              const m = e.target.value || undefined;
              setMood(m);
              if (isOnline(source) && (query.trim() || m)) void search(words(query, m));
            }}
          >
            <option value="">{kind === "music" ? editorText("Any mood") : editorText("Any kind")}</option>
            {moods
              .filter((m) => kind === "music" || m.id !== "meme" || source === "pack")
              .map((m) => (
                <option key={m.id} value={m.id}>
                  {editorText(m.name)}
                </option>
              ))}
          </select>
        )}
      </div>

      {preview.error&&<div className="inline-error" role="alert">{preview.error}</div>}
      {kind === "sfx" && <GenerateSfx project={project} online={online} query={query} />}

      {source === "pack" ? (
        <IncludedSfx project={project} online={online} query={query} category={mood} preview={preview} />
      ) : source === "folders" ? (
        <FolderSounds kind={kind} project={project} online={online} query={query} preview={preview} />
      ) : source === "freesound" && needsKey(source) ? (
        <ProviderKey provider={source} text={editorText('Freesound needs your API key.')} placeholder={editorText("Freesound API key")} />
      ) : isOnline(source) ? (
        <>
          <div className="lib-results" data-testid="library-results" data-busy={busy}>
            {limit && (
              <div className="lib-limit" data-testid="library-limit" data-scope={limit.scope}>
                {canRetry?<button className="btn ghost" disabled={busy||!online} onClick={()=>void search(result!.words,{page:hits.length?result!.page+1:1})}>{editorText('Retry search')}</button>
                  :wait===undefined?editorText('{0} search is temporarily unavailable.',{0:SOURCE_LABEL[result!.source]})
                  :editorText(wait<90?'{0} search limit reached. Try again in {1} seconds.':'{0} search limit reached. Try again in {1} minutes.',{0:SOURCE_LABEL[result!.source],1:wait<90?wait:Math.ceil(wait/60)})}
              </div>
            )}
            {error && (
              <div className="lib-limit error" data-testid="library-error">
                {error}
              </div>
            )}
            {result && !limit && !error && (result.note || result.searched) && (
              <div className="lib-alias small" data-testid="library-alias" data-searched={result.searched ?? ""}>
                {result.note ? (result.alias ? editorText('"{0}" names a clip; these are free sound-alikes.', {0:result.alias}) : result.note)
                  : editorText('Search terms: {0}', {0:result.searched})}
              </div>
            )}
            {!result && !busy && !error && (
              <div className="muted small" data-testid="library-idle">
                {editorText("Type what you're looking for and press Enter")}{kind === "music" ? editorText(", or pick a mood") : ""}.
              </div>
            )}
            {result && !limit && !error && hits.length === 0 && (
              <div className="muted small" data-testid="library-empty">
                {editorText("Nothing found for \"")}{result.words}{editorText("\". Try another word, or add a folder of your own sounds (Your folders).")}</div>
            )}
            {hits.map((h) => (
              <HitRow key={hitKey(h)} hit={h} kind={kind} project={project} online={online} preview={preview} adding={app.libraryAdding === hitKey(h)} />
            ))}
            {busy && <RefreshCw className="spin lib-busy" />}
            {result && !busy && !limit && result.page < result.pages && (
              <button className="btn ghost lib-more" data-testid="library-more" onClick={() => void search(result.words, { page: result.page + 1 })}>
                {editorText("More results")}</button>
            )}
          </div>
          <div className="lib-notice" data-testid="library-notice">
            {source === "openverse" ? (
              <>
                {editorText("Made with")}{" "}
                <a href="https://openverse.org" target="_blank" rel="noreferrer">
                  {editorText("Openverse")}</a>
                {editorText(", not endorsed by Openverse")}</>
            ) : (
              <>
                {editorText("Sounds from")}{" "}
                <a href="https://freesound.org" target="_blank" rel="noreferrer">
                  {editorText("Freesound")}</a>
              </>
            )}
          </div>
        </>
      ) : null}
    </div>
  );
}
