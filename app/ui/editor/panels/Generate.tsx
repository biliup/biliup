import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The Generate tab (D31, D99, D145, redesigned for issue #63 in D234): a Picture / Video / Sound
// switch at the top, followed by the server-supported models and a prompt.
// The model's options use segmented choices and selects. Progress and Cancel stay here; results go
// to Media, marked Generated (GeneratedSection), with Add, Again and Edit prompt.
import type { KeyProvider } from "@/app/lib/editor/contracts";
import { AudioLines, Film, Image as ImageIcon, RefreshCw, Sparkles } from "lucide-react";
import { useContext, useEffect, useRef, useState } from "react";

import { actions, useApp, type GenerateDraft, type Project } from "@/app/lib/editor/state/store";
import { MediaTile } from "@/app/ui/editor/panels/library/MediaTile";
import { firstModel, modelsFor, type GenType, type Model } from "@/app/ui/editor/panels/generate/catalog";
import { LabMark } from "@/app/ui/editor/panels/generate/marks";
import { GenerationStatus } from "@/app/ui/editor/panels/generate/GenerationStatus";
import {GenerationCost} from './generate/GenerationCost';
import {generationCost} from '@/app/lib/editor/generation-pricing';
import { Tooltip } from "@/app/ui/editor/controls/Tooltip";
import {ProviderKey} from '../controls/ProviderKey';
import {EditorStoreContext} from '@/app/lib/editor/state/store/core';
import {generatedDraft,generationRunning} from '@/app/lib/editor/generation';
import {isTextComposition,primaryModifier,usesCommandKey} from '@/app/lib/editor/keyboard';

const SIZE_LABEL: Record<string, string> = { canvas: "Project canvas", "1536x1024": "1536×1024 landscape", "1024x1024": "1024×1024 square", "1024x1536": "1024×1536 portrait", "16:9": "16:9 landscape", "9:16": "9:16 portrait", "1920x1080": "1920×1080", "1080x1920": "1080×1920", "1080x1350": "1080×1350" };
const sizeLabel = (s: string) => SIZE_LABEL[s] ?? s.replace("x", "×");
const CHIP_LABEL: Record<string, string> = { canvas: "Canvas", "1536x1024": "Landscape", "1024x1024": "Square", "1024x1536": "Portrait", "16:9": "16:9", "9:16": "9:16" };
const chipLabel = (s: string) => CHIP_LABEL[s] ?? s.replace("x", "×");

const PROVIDER: Record<KeyProvider, string> = { openai: "OpenAI", google: "Google", elevenlabs: "ElevenLabs", freesound: "Freesound", brave:"Brave Search" };

const TYPES: ReadonlyArray<{ id: GenType; label: string; icon: typeof ImageIcon }> = [
  { id: "image", get label() { return editorText("Picture"); }, icon: ImageIcon },
  { id: "video", get label() { return editorText("Video"); }, icon: Film },
  { id: "audio", get label() { return editorText("Sound"); }, icon: AudioLines },
];
const PLACEHOLDER: Record<GenType, string> = {
  image: "A flat illustration of a phone on a desk, soft light",
  video: "A logo drawing itself on a dark background",
  audio: "A short whoosh as a card slides in",
};

/** Only useful readiness information; prices are determined by the provider. */
const subOf = (m: Model) => {
  const g = m.info;
  if (!g) return m.lab;
  return g.ready ? (g.model ?? m.lab) : editorText(g.needsKey===false?'Renderer required':"API key required");
};

/** Generated media (source.module generate.*), newest last as in the project. */
export const generatedMedia = (project: Project) => project.media.filter((m) => typeof m.source?.module === "string" && m.source.module.startsWith("generate."));

export function GenerateLibrary({ project }: { project: Project }) {
  useEditorLanguage();
  const app = useApp();
  const owner=useContext(EditorStoreContext);
  const online = app.connection === "connected";
  useEffect(() => {
    if (online) void actions.listGenerators();
  }, [online,owner]);
  const generators = app.generators ?? [];
  const [type, setType] = useState<GenType>("image");
  const [draft, setDraft] = useState<GenerateDraft>({ generator: "generate.gpt-image", prompt: "" });
  const prompt = useRef<HTMLTextAreaElement>(null);
  useEffect(()=>{setType('image');setDraft({generator:'generate.gpt-image',prompt:''});},[owner]);
  // "Edit prompt" on a result in Media fills the form once.
  useEffect(() => {
    const d = app.generateDraft;
    if (!d) return;
    setType(d.kind === "audio" ? "audio" : d.kind === "video" ? "video" : "image");
    setDraft(d);
    prompt.current?.focus();
  }, [owner,app.generateDraft?.seq]);
  const models = modelsFor(generators, type);
  const m = models.find((x) => x.id === draft.generator) ?? firstModel(models);
  const g = m?.info;
  const size = g && draft.size && g.sizes.includes(draft.size) ? draft.size : g?.sizes[0];
  const seconds = g?.seconds && type !== "image" ? (draft.seconds !== undefined && g.seconds.includes(draft.seconds) ? draft.seconds : g.id === "generate.sound" && g.seconds.includes(5) ? 5 : g.seconds[0]) : undefined;
  const fullPrompt = draft.prompt.trim();
  const jobs = app.generateJobs.filter((j) => j.projectDir === app.snapshot?.dir);
  // The same request already running (a double click, or Generate again while it works): not twice.
  const same = !!g&&generationRunning(jobs,{generator:g.id,prompt:fullPrompt,kind:type,size,seconds});
  const canRun = online && app.biliupCanEdit && !app.generateStarting && !!g && g.ready && fullPrompt.length > 0 && !!app.snapshot && !same;
  const run = () => {
    if (!canRun || !g) return;
    void actions.startGenerate({ generator: g.id, prompt: fullPrompt, kind: type, ...(size ? { size } : {}), ...(seconds !== undefined ? { seconds } : {}) });
  };
  const pickType = (t: GenType) => {
    setType(t);
    const first = firstModel(modelsFor(generators, t));
    setDraft((d) => ({ ...d, generator: first?.id ?? d.generator, kind: t, size: undefined, seconds: undefined }));
  };
  const shortcut=usesCommandKey()?'⌘↩':'Ctrl+Enter';
  const why = !app.snapshot ? editorText("Open a project first") : g && !g.ready ? editorText(g.needs) : same ? editorText("This one is being made now") : !fullPrompt ? editorText("Describe what to make first") : editorText('Generate ({0})',{0:shortcut});
  const modelRow = (x: Model) => (
    <button
      key={x.id}
      role="radio"
      aria-checked={x.id === m?.id}
      className={`gen-model${x.id === m?.id ? " active" : ""}${x.info.ready ? "" : " later"}`}
      data-testid="generator"
      data-id={x.id}
      data-ready={x.info?.ready ?? false}
      title={x.info?.provider ? editorText("{0} by {1}", { 0: x.name, 1: x.lab }) : editorText("{0}, uses your {1}", { 0: x.name, 1: x.lab })}
      onClick={() => setDraft((d) => ({ ...d, generator: x.id, size: undefined, seconds: undefined }))}
    >
      <LabMark brand={x.brand} lab={x.lab} />
      <span className="gen-model-text">
        <span className="gen-model-name">{editorText(x.name)}</span>
        <span className="gen-model-sub" data-testid="generator-price">
          {subOf(x)}
        </span>
      </span>
    </button>
  );
  return (
    <aside className="library gen-panel" data-testid="generate-library">
      <div className="lib-head">
        {editorText("Generate")}<Tooltip label={editorText("Check the generators and keys again")}>
          <button className="icon-btn" data-testid="generate-refresh" disabled={!online} onClick={() => void actions.listGenerators()}>
            <RefreshCw />
          </button>
        </Tooltip>
      </div>
      <div className="lib-section gen-form" data-type={type}>
        <div className="seg gen-types" role="radiogroup" aria-label={editorText("What to make")} data-testid="generate-types">
          {TYPES.map((t) => (
            <button key={t.id} role="radio" aria-checked={t.id === type} className={t.id === type ? "active" : ""} data-testid={`generate-type-${t.id}`} onClick={() => pickType(t.id)}>
              <t.icon />
              {editorText(t.label)}
            </button>
          ))}
        </div>
        <div className="gen-models" data-testid="generators" role="radiogroup" aria-label={editorText("Model")}>
          {models.map(modelRow)}
          {app.generators===undefined&&!app.generatorsError&&<RefreshCw className="spin"/>}
          {app.generators!==undefined&&models.length === 0 && <div className="small muted">{editorText("Nothing can make this yet.")}</div>}
        </div>
        {app.generatorsError&&<div className="inline-error" role="alert">{app.generatorsError}</div>}
        {g && !g.ready && g.provider && (g.needsKey===false?<div className="inline-error">{g.needs}</div>:<ProviderKey provider={g.provider} text={g.needs ?? editorText('API key required')} placeholder={editorText('{0} API key',{0:PROVIDER[g.provider]})}/>)}
        <div className="gen-box">
          <textarea
            ref={prompt}
            className="gen-prompt"
            data-testid="generate-prompt"
            placeholder={editorText(PLACEHOLDER[type])}
            value={draft.prompt}
            maxLength={4000}
            onChange={(e) => setDraft((d) => ({ ...d, prompt: e.target.value }))}
            onKeyDown={(e) => {
              if (e.key === "Enter" && primaryModifier(e) && !isTextComposition(e.nativeEvent)){e.preventDefault();run();}
            }}
          />
        </div>
        <div className="gen-options" data-testid="generate-options" data-model={m?.id}>
          {g && g.sizes.length > 1 && (
            <div className="gen-opt" data-testid="generate-sizes">
              <span className="k">{editorText("Size")}</span>
              {/* A plain select (D263): up to five sizes don't fit a segmented control in the panel. */}
              <select className="field input gen-select" data-testid="generate-size" aria-label={editorText("Size")} value={size} onChange={(e) => setDraft((d) => ({ ...d, size: e.target.value }))}>
                {g.sizes.map((s) => (
                  <option key={s} value={s} title={editorText(sizeLabel(s))}>
                    {editorText(chipLabel(s))}
                  </option>
                ))}
              </select>
            </div>
          )}
          {g?.seconds && type !== "image" && (
            <div className="gen-opt" data-testid="generate-lengths">
              <span className="k">{editorText("Length")}</span>
              <span className="seg" role="radiogroup" aria-label={editorText("Length")}>
                {g.seconds.map((n) => (
                  <button key={n} className={n === seconds ? "active" : ""} role="radio" aria-checked={n === seconds} data-testid={`generate-seconds-${n}`} data-value={n} onClick={() => setDraft((d) => ({ ...d, seconds: n }))}>
                    {n} {editorText(" s")}</button>
                ))}
              </span>
            </div>
          )}
        </div>
        <div className="gen-go">
          <button className="gen-run" data-testid="generate-run" disabled={!canRun} title={why} onClick={run}>
            <Sparkles />
            <span>{editorText(type==='image'?'Generate picture':type==='video'?'Generate video':'Generate sound')}</span>
            <kbd>{shortcut}</kbd>
          </button>
          <GenerationCost generator={g} request={{size,seconds}}/>
        </div>
        <GenerationStatus jobs={jobs} project={project} online={online}/>
      </div>
    </aside>
  );
}

/**
 * Media › Generated (D31, D234; compact in D240, #74): each result a media tile like the others
 * (same size, following the grid/list view), with a small Generated badge; + adds it at the
 * playhead, ↻ makes it again, and its menu has Edit prompt. The prompt and model are its tooltip.
 */
export function GeneratedSection({ project, onEdit, mode, used, selected, onSelect, onRemove }: { project: Project; onEdit?: () => void; mode: "grid" | "list"; used: ReadonlySet<string>; selected: string | undefined; onSelect: (id: string) => void; onRemove: (m: Project["media"][number]) => void }) {
  useEditorLanguage();
  const app = useApp();
  const owner=useContext(EditorStoreContext);
  const online = app.connection === "connected";
  const items = generatedMedia(project);
  const hasGenerated=items.length>0;
  useEffect(()=>{if(online&&hasGenerated)void actions.listGenerators();},[online,hasGenerated,owner]);
  if (items.length === 0) return null;
  const generators = app.generators ?? [];
  return (
    <div className="lib-section" data-testid="generated">
      <div className="lib-label gen-section-label">
        <Sparkles />
        {editorText("Generated")}</div>
      <div className={mode === "list" ? "media-list" : "grid2"} data-testid="generated-grid" data-view={mode}>
        {[...items].reverse().map((m) => {
          const draft = generatedDraft(m);
          const generator = generators.find((g) => g.id === draft.generator);
          const ready = generator?.ready === true;
          const cost = generator ? generationCost(generator,draft) : undefined;
          const type: GenType = m.kind === "audio" ? "audio" : m.kind === "video" ? "video" : "image";
          const by = modelsFor(generators, type).find((x) => x.id === draft.generator);
          return (
            <MediaTile
              key={m.id}
              item={m}
              fps={project.canvas.fps}
              online={online}
              used={used.has(m.id)}
              selected={selected === m.id}
              onSelect={() => onSelect(m.id)}
              onRemove={() => onRemove(m)}
              generated={{
                prompt: draft.prompt,
                by: by ? (by.credit ?? `${by.name}, ${by.lab}`) : draft.generator,
                canAgain: ready && !!app.biliupCanEdit && !app.generateStarting && !!draft.prompt&&!generationRunning(app.generateJobs,draft),
                againTitle: !draft.prompt ? "No prompt saved" : !generator ? "Check generators in Generate" : ready && cost ? `${editorText('Make another one from the same prompt')} · ${cost.label}. ${cost.detail}` : "Needs its key: Settings › Keys",
                again: () => void actions.startGenerate(draft),
                edit: () => {
                  actions.editPrompt(draft);
                  onEdit?.();
                },
              }}
            />
          );
        })}
      </div>
    </div>
  );
}
