import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { RefreshCw, Sparkles, X } from "lucide-react";
import { useContext, useEffect, useState } from "react";
import { type Project, actions, useApp } from "@/app/lib/editor/state/store";
import { ProviderKey } from "@/app/ui/editor/controls/ProviderKey";
import { Tooltip } from "@/app/ui/editor/controls/Tooltip";
import { GenerationStatus } from "@/app/ui/editor/panels/generate/GenerationStatus";
import {GenerationCost} from '../generate/GenerationCost';
import {isTextComposition,primaryModifier} from '@/app/lib/editor/keyboard';
import {EditorStoreContext} from '@/app/lib/editor/state/store/core';
import {generationRunning} from '@/app/lib/editor/generation';

const GENERATOR = "generate.sound";

/**
 * Generate a sound effect (D250): optional, with the user's own ElevenLabs key (the Generate tab's
 * generate.sound, D99): a description (the search words to start with), a length, Generate. The
 * result lands in Media like any generated file; + puts it at the playhead.
 */
export function GenerateSfx({ project, online, query }: { project: Project; online: boolean; query: string }) {
  useEditorLanguage();
  const app = useApp();
  const owner=useContext(EditorStoreContext);
  const [open, setOpen] = useState(false);
  const [prompt, setPrompt] = useState("");
  const [length, setLength] = useState<number>(2);
  useEffect(() => {
    if (open && online && !app.generators) void actions.listGenerators();
  }, [open, online,owner]);
  useEffect(()=>{setOpen(false);setPrompt('');setLength(2);},[owner]);
  const g = app.generators?.find((x) => x.id === GENERATOR);
  const lengths = g?.seconds ?? [];
  const seconds = lengths.includes(length) ? length : lengths[0];
  const jobs = app.generateJobs.filter((j) => j.generator === GENERATOR && j.projectDir === app.snapshot?.dir);
  const running = jobs.filter((j) => j.phase === "running" || j.phase === "importing");
  const text = prompt.trim();
  const canRun = online && app.biliupCanEdit && !app.generateStarting && !!g?.ready && seconds !== undefined && text.length > 0 && !generationRunning(jobs,{generator:GENERATOR,prompt:text,kind:'audio',seconds});
  if (!open)
    return (
      <button
        className="btn ghost lib-generate-toggle"
        data-testid="sfx-generate-toggle"
        onClick={() => {
          setOpen(true);
          if (!prompt) setPrompt(query.trim());
        }}
      >
        <Sparkles /> {editorText(" Generate a sound effect")}</button>
    );
  return (
    <div className="lib-generate" data-testid="sfx-generate">
      <div className="lib-generate-head">
        <span>{editorText("Generate a sound effect")}</span>
        <Tooltip label={editorText("Close")}>
          <button className="icon-btn" data-testid="sfx-generate-close" onClick={() => setOpen(false)}>
            <X />
          </button>
        </Tooltip>
      </div>
      {g && !g.ready ? (
        <ProviderKey provider="elevenlabs" text="Use your ElevenLabs API key for sound effects." placeholder={editorText("ElevenLabs API key")} />
      ) : (
        <>
          <textarea
            className="field input"
            data-testid="sfx-generate-prompt"
            rows={2}
            placeholder={editorText("Describe the sound: a soft whoosh into a deep hit")}
            value={prompt}
            spellCheck={false}
            onChange={(e) => setPrompt(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && primaryModifier(e) && !isTextComposition(e.nativeEvent)) {
                e.preventDefault();
                if(canRun)void actions.startGenerate({ generator: GENERATOR, prompt: text, kind: "audio", seconds });
              }
            }}
          />
          <div className="lib-selects">
            <select className="field input" data-testid="sfx-generate-seconds" aria-label={editorText("Length")} value={seconds ?? ''} onChange={(e) => setLength(Number(e.target.value))}>
              {lengths.map((s) => (
                <option key={s} value={s}>
                  {s} {editorText(" s")}</option>
              ))}
            </select>
            <button className="btn primary" data-testid="sfx-generate-run" disabled={!canRun} onClick={() => void actions.startGenerate({ generator: GENERATOR, prompt: text, kind: "audio", seconds })}>
              {running.length ? <RefreshCw className="spin" /> : <Sparkles />} {editorText(" Generate")}</button>
          </div>
        </>
      )}
      <GenerationCost generator={g} request={{seconds}}/>
      {app.generatorsError&&<div className="inline-error" role="alert">{app.generatorsError}<button className="link-btn" disabled={!online} onClick={()=>void actions.listGenerators()}>{editorText('Try again')}</button></div>}
      <GenerationStatus jobs={jobs} project={project} online={online}/>
    </div>
  );
}
