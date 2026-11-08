import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The Elements tab's stickers (F22, D165; sections in D251): core's bundled set (drawn SVG stickers
// and emoji with keywords, no network). Without a search they're in sections (Reactions, Arrows &
// marks, Shapes & bursts, Bubbles, Badges, Social, Emoji); with one, the matches in one grid. A
// click adds a sticker component with that emoji or art at the playhead (one edit); a drag drops it
// where it's let go, on the canvas or the timeline.
import { STICKER_ART, STICKER_GROUPS, STICKERS, stickerArt, stickerProps, type StickerEntry } from "@/app/lib/editor/core/visual/stickers";
import {stickerLabel} from '@/app/lib/editor/sticker-library';
import { useState } from "react";

import { actions, useApp } from "@/app/lib/editor/state/store";
import {beginEditorDrag,endEditorDrag} from '@/app/ui/editor/timeline/dnd';
import { setDragChip } from "@/app/ui/editor/panels/dragChip";

/** How many stickers a long list (the emoji, a search) shows before "More". */
const PAGE = 48;
const STICKER = "component.sticker";

function StickerButton({ s, online }: { s: StickerEntry; online: boolean }) {
  useEditorLanguage();
  const art = s.art ? stickerArt(s.art) : undefined;
  const add = () => void actions.addComponent(STICKER, { props: stickerProps(s) });
  return (
    <button
      className={`sticker-btn ${art?.mark ? "mark" : ""}`}
      data-testid="sticker"
      data-key={s.key}
      title={stickerLabel(s)}
      aria-label={editorText("Add {0}", { 0: stickerLabel(s) })}
      disabled={!online}
      draggable={online}
      onClick={(e) => {
        e.currentTarget.blur();
        add();
      }}
      onDragStart={(e) => {
        if(!beginEditorDrag({kind:'component',id:STICKER,props:stickerProps(s)},e.dataTransfer)){e.preventDefault();return;}
        setDragChip(e, stickerLabel(s));
      }}
      onDragEnd={() => {
        endEditorDrag();
      }}
    >
      {s.art ? <span className="sticker-art" style={art?.size ? { aspectRatio: `${art.size[0]} / ${art.size[1]}` } : undefined} dangerouslySetInnerHTML={{ __html: STICKER_ART[s.art]!.svg }} /> : <span className="sticker-emoji">{s.emoji}</span>}
    </button>
  );
}

/** A grid of stickers, the first PAGE of a long one with "More". */
function StickerGrid({ list, online, testId = "sticker-grid", more }: { list: ReadonlyArray<StickerEntry>; online: boolean; testId?: string; more: string }) {
  useEditorLanguage();
  const [all, setAll] = useState(false);
  const shown = all ? list : list.slice(0, PAGE);
  return (
    <>
      <div className="sticker-grid" data-testid={testId}>
        {shown.map((s) => (
          <StickerButton key={s.key} s={s} online={online} />
        ))}
      </div>
      {!all && list.length > PAGE && (
        <button className="link-btn small" data-testid="sticker-more" data-more={more} onClick={() => setAll(true)}>
          {editorText('More ({0})',{0:list.length-PAGE})}
        </button>
      )}
    </>
  );
}

/** The stickers for the Elements tab's search (empty: every section). */
export function StickerPicker({ query, results }: { query: string; results: ReadonlyArray<StickerEntry> }) {
  useEditorLanguage();
  const { connection,biliupCanEdit } = useApp();
  const online = connection === "connected"&&!!biliupCanEdit;
  if (query.trim()) {
    return <StickerGrid key={query} list={results} online={online} more="search" />;
  }
  return (
    <>
      {STICKER_GROUPS.map((g) => {
        const list = STICKERS.filter((s) => s.group === g);
        return (
          <div key={g} className="sticker-group" data-testid="sticker-group" data-group={g}>
            <div className="sticker-group-label">{editorText(g)}</div>
            <StickerGrid list={list} online={online} more={g} />
          </div>
        );
      })}
    </>
  );
}
