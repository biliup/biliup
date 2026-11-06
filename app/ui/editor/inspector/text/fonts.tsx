import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The font picker (D129): a button showing the font in itself, opening a searchable list where each
// name is drawn in its own font. Bundled and active-brand faces come first. Browser-discovered
// local faces are real families on this device; no platform-specific fallback list is invented.
import { useMemo, useRef, useState } from "react";

import { Popover } from "@/app/ui/editor/inspector/controls";
import {useApp} from '@/app/lib/editor/state/store/core';
import {canReadLocalFonts,FONT_FILE_ACCEPT,localFamilyFiles,localFonts,type LocalFont} from '@/app/lib/editor/project-fonts';
import {getPlayback} from '@/app/ui/editor/preview/playback';
import {chooseEditorFiles} from '@/app/lib/editor/browser-imports';
import {currentFileImporter,type FileImportResult} from '@/app/lib/editor/state/store/server';
import {useEditorAction} from '../../controls/useEditorAction';
import {editorGestureOrigin,ownsEditorGesture,type GestureOrigin} from '../../workspace/pointerGesture';
import {isTextComposition} from '@/app/lib/editor/keyboard';
import {focusEditor} from '@/app/lib/editor/environment';

/** A family name the renderer accepts (core resolveFont's SAFE_FONT). */
const SAFE = /^[\p{L}\p{N} _.-]{1,80}$/u;

/** What each special value is called, and how its option is drawn. */
const SPECIAL: ReadonlyArray<readonly [string, string, string]> = [
  ["", "Default", "system-ui, sans-serif"],
  ["DM Sans", "DM Sans", "'DM Sans'"],
  ["DM Mono", "DM Mono", "'DM Mono'"],
  ["mono", "System mono", "ui-monospace, monospace"],
];

export const fontLabel = (font: string) => {const special=SPECIAL.find(([v])=>v===font);return special?editorText(special[1]):font};
const fontCss = (font: string) => SPECIAL.find(([v]) => v === font)?.[2] ?? `'${font}'`;

const selection=()=>JSON.stringify([getPlayback().selected,...getPlayback().also]);
const importProblem=(result:FileImportResult|undefined)=>result?.failed.length?result.failed.map(failure=>`${failure.path}: ${editorText(failure.message)}`).join('\n'):undefined;

export function FontPicker({ value, onPick, testId, disabled }: { value: string; onPick: (font: string,origin:GestureOrigin) => void; testId: string; disabled?: boolean }) {
  const language=useEditorLanguage();
  const app=useApp(),{brand}=app;
  const {busy,error,editable,run}=useEditorAction();
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [local, setLocal] = useState<LocalFont[]|undefined>();
  const anchor = useRef<HTMLButtonElement>(null);
  const others = useMemo(()=>[...new Set([...(app.projectFonts?.map(font=>font.family)??[]),...(brand?.kits.find(kit=>kit.id===brand.active.kit)?.fonts.map(font=>font.family)??[]),...(local?.map(font=>font.family)??[])])].filter(font=>SAFE.test(font)).sort((a,b)=>a.localeCompare(b)),[brand,local,app.projectFonts]);
  const q = query.trim().toLowerCase();
  const list = useMemo(() => {
    const special = SPECIAL.filter(([, label]) => !q || label.toLowerCase().includes(q)||editorText(label).toLowerCase().includes(q)).map(([v]) => v);
    const rest = others.filter((f) => !SPECIAL.some(([v]) => v === f) && (!q || f.toLowerCase().includes(q)));
    return { special, rest };
  }, [q, others, language]);
  const pick = (font: string) => {
    if(disabled)return
    const origin=editorGestureOrigin(),selected=selection(),importer=currentFileImporter();if(!origin||!importer)return
    void run(async current=>{
      const kit=brand?.kits.find(kit=>kit.id===brand.active.kit)
      const bundled=SPECIAL.some(([family])=>family===font)
      const inKit=kit?.fonts.some(face=>face.family===font&&face.file&&brand?.files[kit.id]?.[face.file])
      const known=origin.owner.getSnapshot().projectFonts??[]
      if(!bundled&&!inKit&&local?.some(face=>face.family===font)){
        const prepared=await localFamilyFiles(font,local,known,current)
        if(!prepared||!current())return
        if(prepared.files.length){
          const result=await importer(prepared.files,{fontMetadata:prepared.fontMetadata})
          if(!current()||!result)return
          const problem=importProblem(result);if(problem)throw new Error(problem)
        }
      }else if(!bundled&&!inKit&&!known.some(face=>face.family===font))throw new Error(editorText('The font files are unavailable. Import a font file.'))
      if(!current()||selection()!==selected)return
      if(!ownsEditorGesture(origin))throw new Error(editorText('The project changed. Try the action again.'))
      if(font!==value)onPick(font,origin)
      setOpen(false);setQuery('');focusEditor()
    })
  };
  const importFiles=()=>{
    if(disabled)return
    const importer=currentFileImporter();if(!importer)return
    void run(async current=>{
      const picked=await chooseEditorFiles(false,FONT_FILE_ACCEPT)
      if(!current())return
      const result=picked.files.length?await importer(picked.files):undefined
      const problem=[...picked.problems,importProblem(result)].filter(Boolean).join('\n')
      if(problem)throw new Error(problem)
    })
  }
  return (
    <>
      <button
        ref={anchor}
        className="pselect font-btn"
        data-testid={testId}
        data-font={value}
        disabled={disabled||!editable||busy}
        aria-expanded={open}
        style={{ fontFamily: fontCss(value) }}
        onClick={() => setOpen(!open)}
      >
        {fontLabel(value)}
      </button>
      {open && (
        <Popover anchor={anchor.current} onClose={() => {if(!busy)setOpen(false)}} testId={`${testId}-popover`} width={260}>
          <input className="field input ppop-search" data-testid={`${testId}-search`} aria-label={editorText("Search fonts")} placeholder={editorText("Search fonts")} autoFocus value={query} onChange={(e) => setQuery(e.target.value)} onKeyDown={(e) => {
              if(isTextComposition(e.nativeEvent))return;
              const first = [...list.special, ...list.rest][0];
              if(e.key==='Enter'){e.preventDefault();e.stopPropagation();if(first!==undefined)pick(first)}
            }} />
          <div className="font-list">
            {[...list.special, ...list.rest].map((f, i) => (
              <button key={f || "default"} disabled={busy||disabled||!editable} aria-pressed={f===value} className={`font-opt ${f === value ? "active" : ""} ${i === list.special.length && list.special.length ? "sep" : ""}`} data-testid={`${testId}-option`} data-font={f} style={{ fontFamily: fontCss(f) }} onClick={() => pick(f)}>
                {fontLabel(f)}
              </button>
            ))}
          </div>
          {!list.special.length&&!list.rest.length&&<div className="muted small">{editorText('No matching fonts')}</div>}
          {!local&&canReadLocalFonts()&&<button className="btn ghost" disabled={busy||disabled||!editable} onClick={()=>void run(async current=>{try{const faces=await localFonts();if(current())setLocal(faces)}catch{throw new Error(editorText('无法读取本机字体，请导入字体文件'))}})}>{editorText('Use local fonts…')}</button>}
          <button className="btn ghost" disabled={busy||disabled||!editable} onClick={importFiles}>{busy?editorText('Working…'):editorText('导入字体')}</button>
          {error&&<div className="error font-error" role="alert">{editorText(error)}</div>}
        </Popover>
      )}
    </>
  );
}
