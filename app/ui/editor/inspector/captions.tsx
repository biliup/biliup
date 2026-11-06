import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
import { joinWords } from '@/app/lib/editor/core/text/word-spacing';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The Inspector's captions editor (D82): mode, lines, word fixes, then Style.
import type { Item } from "@/app/lib/editor/core/timeline/types";
import { useEffect, useRef, useState } from "react";

import { frameTimecode } from "@/app/lib/editor/state/format";
import { playback, usePlayback } from "@/app/ui/editor/preview/playback";
import { type ProjectModel } from "@/app/lib/editor/state/project";
import { autoMaxChars, CAPTION_MODE_NAMES, CAPTION_MODES, captionCardAt, captionCards, captionSettings, captionsCoverMain, captionWordText, planCaptionEdit, type CaptionEdit } from "@/app/lib/editor/core/text/captions";
import { TextStyleTab } from "@/app/ui/editor/inspector/text";
import { CaptionStyleGallery } from "@/app/ui/editor/inspector/captionStyles";
import { send, NumberField, Tabs, Group } from "@/app/ui/editor/inspector/fields";
import { Tooltip } from "@/app/ui/editor/controls/Tooltip";
import {findItem} from '../timeline/edit';
import {useTextDraft} from './text/draft';
import type {GestureOrigin} from '../workspace/pointerGesture';

export function CaptionsEditor({ project, item, locked }: { project: ProjectModel; item: Item; locked: boolean }) {
  useEditorLanguage();
  const [tab, setTab] = useState<"captions" | "style">("captions");
  return (
    <div data-testid="captions-editor">
      <div className="group tabs-group">
        <Tabs
          value={tab}
          onChange={setTab}
          tabs={[
            ["captions", "Captions"],
            ["style", "Style"],
          ]}
        />
      </div>
      {tab === "captions" ? (
        <CaptionsTab project={project} item={item} disabled={locked} />
      ) : (
        <>
          <CaptionStyleGallery project={project} item={item} disabled={locked} />
          <TextStyleTab project={project} item={item} disabled={locked} />
        </>
      )}
    </div>
  );
}

/** The modes' names in the Inspector's narrow column. */
const CAPTION_SHORT = { line: "Lines", word: "Word by word", single: "One word" } as const;

function CaptionsTab({ project, item, disabled }: { project: ProjectModel; item: Item; disabled: boolean }) {
  useEditorLanguage();
  const edit = (change: CaptionEdit,origin?:GestureOrigin) => send((p) => planCaptionEdit(p, item.id, change),undefined,origin);
  const mode = item.mode === "word" || item.mode === "single" ? item.mode : "line";
  const s = captionSettings(item, project);
  const cards = captionCards(project, item);
  const n = usePlayback((st) => captionCardAt(project, item, st.frame)?.n ?? -1);
  const card = n >= 0 ? cards[n] : undefined;
  const fps = project.canvas.fps;
  const listRef = useRef<HTMLDivElement>(null);
  // Words hidden by a fix (""): always reachable here, also when a whole card is hidden.
  const hidden = (item.edits ?? []).filter((e) => e.text.trim() === "").map((e) => e.word);
  useEffect(() => {
    listRef.current?.querySelector<HTMLElement>("[data-current]")?.scrollIntoView({ block: "nearest" });
  }, [n]);
  return (
    <>
      <Group label={editorText("Show")}>
        <div className="seg" role="group">
          {CAPTION_MODES.map((m) => (
            <button key={m} className={mode === m ? "active" : ""} data-testid={`cap-mode-${m}`} disabled={disabled} title={editorText(CAPTION_MODE_NAMES[m])} onClick={() => edit({ mode: m })}>
              {editorText(CAPTION_SHORT[m])}
            </button>
          ))}
        </div>
      </Group>
      <Group label={editorText("Cards")}>
        <div className="seg" role="group">
          {[1, 2].map((k) => (
            <button key={k} className={s.maxLines === k ? "active" : ""} data-testid={`cap-lines-${k}`} disabled={disabled} onClick={() => edit({ lines: { maxLines: k } })}>
              {k === 1 ? editorText("1 line") : editorText("2 lines")}
            </button>
          ))}
        </div>
        <div className="field-row">
          <NumberField label={editorText("Characters")} testId="cap-chars" value={item.lines?.maxChars} placeholder={`${autoMaxChars(item, project)}`} min={4} max={200} disabled={disabled} onCommit={(v,origin) => edit({ lines: { maxChars: v === undefined ? null : Math.round(v) } },origin)} />
          <NumberField label={editorText("Seconds")} testId="cap-seconds" value={s.maxSeconds} step={0.5} min={0.5} max={30} disabled={disabled} onCommit={(v,origin) => edit({ lines: { maxSeconds: v ?? null } },origin)} />
          <NumberField label={editorText("Pause")} testId="cap-pause" value={s.pause} step={0.1} min={0} max={10} disabled={disabled} onCommit={(v,origin) => edit({ lines: { pause: v ?? null } },origin)} />
        </div>
        <label className="check-row">
          <input type="checkbox" data-testid="cap-fillers" checked={item.hideFillers === true} disabled={disabled} onChange={(e) => edit({ hideFillers: e.target.checked })} />
          {editorText("Hide fillers (um, uh)")}</label>
        {!captionsCoverMain(project, item) && (
          <div className="hint small">
            <span className="muted">{editorText("The edit is longer than these captions. ")}</span>
            <button className="link-btn small" data-testid="cap-cover" disabled={disabled} onClick={() => edit({ cover: true })}>
              {editorText("Cover the whole edit")}</button>
          </div>
        )}
      </Group>
      <Group label={card ? editorText("Card at {0}", { 0: frameTimecode(card.from, fps) }) : editorText("Card at the playhead")}>
        {card ? (
          <div className="cap-words" data-testid="cap-card" data-from={card.from}>
            {cardIndices(card).map((index, i) => {
              const text = captionWordText(item, index);
              const original = (item.words?.[index]?.text ?? "").trim();
              const shown = card.words.some((w) => w.index === index);
              if (!shown && text !== "") return null;
              return (
                <span key={index} className="cap-word">
                  {i > 0 && shown && (
                    <Tooltip label={editorText("Start a new card at this word")}>
                      <button className={`cap-break ${(item.breaks ?? []).includes(index) ? "on" : ""}`} data-testid="cap-break" data-index={index} disabled={disabled} onClick={() => edit({ breaks: [{ index, on: true }] })}>
                        ↵
                      </button>
                    </Tooltip>
                  )}
                  {i === 0 && (item.breaks ?? []).includes(index) && (
                    <Tooltip label={editorText("Join this card to the one before")}>
                      <button className="cap-break on" data-testid="cap-join" data-index={index} disabled={disabled} onClick={() => edit({ breaks: [{ index, on: false }] })}>
                        ↵
                      </button>
                    </Tooltip>
                  )}
                  {shown ? (
                    <CaptionWord key={`${item.id}:${index}`} project={project} item={item} index={index} value={text} original={original} disabled={disabled} />
                  ) : (
                    <button className="cap-hidden" data-testid="cap-restore" data-index={index} disabled={disabled} title={editorText("Hidden: click to show \"{0}\" again", { 0: original })} onClick={() => edit({ words: [{ index, text: null }] })}>
                      {original}
                    </button>
                  )}
                </span>
              );
            })}
          </div>
        ) : (
          <div className="muted small" data-testid="cap-no-card">
            {cards.length ? editorText("No caption under the playhead. Pick a card below to edit its words.") : editorText("No speech of this media plays on the main track.")}
          </div>
        )}
        <div className="muted small">{editorText("Type over a word to fix it; empty hides it (click a struck-out word to show it again).")}</div>
      </Group>
      {hidden.length > 0 && (
        <Group label={editorText("Hidden words ({0})", { 0: hidden.length })}>
          <div className="cap-words" data-testid="cap-hidden-list">
            {hidden.map((index) => (
              <button key={index} className="cap-hidden" data-testid="cap-restore" data-index={index} disabled={disabled} title={editorText("Show it again")} onClick={() => edit({ words: [{ index, text: null }] })}>
                {(item.words?.[index]?.text ?? "").trim()}
              </button>
            ))}
          </div>
        </Group>
      )}
      <Group label={editorText("All cards ({0})", { 0: cards.length })}>
        <div className="cap-list" ref={listRef}>
          {cards.map((c, k) => (
            <button key={`${c.from}-${c.words[0]!.index}`} className="cap-row" data-testid="cap-card-row" data-from={c.from} data-current={k === n ? "true" : undefined} onClick={() => playback.seek(c.from)}>
              <span className="mono small muted">{frameTimecode(c.from, fps)}</span>
              <span className="ellipsis">{joinWords(c.words.map((w) => w.text))}</span>
            </button>
          ))}
        </div>
      </Group>
    </>
  );
}

/** Every word index from a card's first to its last, hidden ones included. */
const cardIndices = (card: { readonly words: ReadonlyArray<{ readonly index: number }> }) => {
  const a = card.words[0]!.index;
  const b = card.words.at(-1)!.index;
  return Array.from({ length: b - a + 1 }, (_, k) => a + k);
};

const wordSnapshots=new WeakMap<NonNullable<Item['words']>,string>();
function captionWordScope(item:Item|undefined):string {
  const words=item?.words;
  if(!words)return '';
  let snapshot=wordSnapshots.get(words);
  if(snapshot===undefined){snapshot=JSON.stringify(words);wordSnapshots.set(words,snapshot)}
  return JSON.stringify([item?.source,snapshot]);
}

/** One caption word: typed over, committed on Enter, blur or unmount (playback moving on mid-typing); Esc puts it back; empty hides it. */
function CaptionWord({ project,item,index,value,original,disabled }: { project:ProjectModel;item:Item;index: number; value: string; original: string; disabled: boolean }) {
  useEditorLanguage();
  const word=(project:ProjectModel)=>{
    const found=findItem(project,item.id)?.item;
    return found?.type==='captions'&&found.words?.[index]?found:undefined;
  };
  const draft=useTextDraft({
    project,value,disabled,multiline:false,saveOnPause:false,
    normalize:text=>text.replace(/\s+/g,' ').trim(),
    read:project=>{const found=word(project);return found?captionWordText(found,index):undefined},
    scope:project=>captionWordScope(word(project)),
    plan:(project,text)=>planCaptionEdit(project,item.id,{words:[{index,text:text===original?null:text}]}),
  });
  return (
    <input
      className={`cap-input ${value !== original ? "edited" : ""}`}
      data-testid="cap-word"
      data-index={index}
      {...draft.input}
      placeholder={original}
      title={value !== original ? editorText("Fixed (the transcript says \"{0}\")", { 0: original }) : editorText("w{0}", { 0: index })}
      size={Math.max(2, draft.input.value.length || original.length)}
      spellCheck={false}
      disabled={disabled}
    />
  );
}

