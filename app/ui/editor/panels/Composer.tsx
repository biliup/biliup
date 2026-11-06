import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
import { isTextComposition } from '@/app/lib/editor/keyboard';
import {currentEditorStore} from '@/app/lib/editor/state/store/core';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The Agent tab's composer (D140, D184, t3code-like): a multiline box where Enter sends and ⇧Enter
// is a new line; @ opens a picker of media, items, tracks and markers that inserts an @label chip
// (Add to chat, D141, inserts the same), and a typed time (@0:12, @12s) offers that moment (D183);
// ↑ in an empty box brings back earlier prompts; while a turn runs the button is Stop and Enter
// queues the message for when it ends. D232 (issue #60): / offers the installed skills (a picked
// one goes in as /name; the server sends its instructions with the message), and since issue #111
// a Templates group above them: a picked template is filled from the project and lands here as a
// brief to edit before sending. Images come in by
// paste, drop or the paperclip. The bottom
// row picks the provider, model, effort and access, remembered per project (agentPrefs.ts). The
// text, chips and images live in the store.
import { type AgentMention, type AgentTurn } from "@/app/lib/editor/contracts";
import { mentionCandidates, momentLabel, parseMoment } from "@/app/lib/editor/core/text/mentions";
import { ArrowLeftRight, ArrowUp, AtSign, Blend, BookOpen, Clock3, FileText, Film, Flag, ListVideo, MousePointer2, Music, Paperclip, Rows3, Sparkles, Square, SquareDashed, X } from "lucide-react";
import { Fragment, useEffect, useLayoutEffect, useMemo, useRef, useState, type ReactNode } from "react";

import { usePlayback } from "@/app/ui/editor/preview/playback";
import { frameTimecode } from "@/app/lib/editor/state/format";
import { itemLabel } from "@/app/lib/editor/state/project";
import { currentFileImporter } from '@/app/lib/editor/state/store/server';
import { chooseEditorFiles, readDroppedFiles, EDITOR_FILE_ACCEPT, type BrowserFileSelection } from '@/app/lib/editor/browser-imports';
import { openMenu } from '@/app/ui/editor/timeline/ContextMenu';
import { attachComposerFiles } from '@/app/ui/editor/panels/composer/attachments';
import { actions, useApp, type Project } from "@/app/lib/editor/state/store";
import { ComposerChoices } from "@/app/ui/editor/panels/composer/Choices";
import { slashItems, type SlashItem } from "@/app/ui/editor/panels/composer/slash";
import { TokenMeter } from "@/app/ui/editor/panels/TokenMeter";
import { Tooltip } from "@/app/ui/editor/controls/Tooltip";
export { ACCESS_INFO, ProviderMark } from "@/app/ui/editor/panels/composer/Choices";

const escape = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

// D198: a path in a message (~/…, $HOME/…, ${HOME}/… or absolute), as the server reads one (a
// backslash-escaped space stays in it; trailing punctuation doesn't).
const PATH = String.raw`(?<![^\s"'(<\[])(?:(?:~|\$HOME|\$\{HOME\})\/|\/(?=[^\s/]))(?:\\ |[^\s"'\`)<>\]])*?(?=[.,;:!?]*(?:$|[\s"'\`)<>\]]))`;

/** Text with its @labels (of `labels`) and its paths (D198) as chip spans: the composer's backdrop and the turn card's prompt. */
export function withChips(text: string, labels: ReadonlyArray<string>, className = "chip"): ReactNode {
  const unique = [...new Set(labels.filter(Boolean))].sort((a, b) => b.length - a.length);
  const re = new RegExp(unique.length ? `@(?:${unique.map(escape).join("|")})|${PATH}` : PATH, "g");
  const out: Array<ReactNode> = [];
  let last = 0;
  for (const m of text.matchAll(re)) {
    if (!m[0]) continue;
    const at = m.index;
    if (at > last) out.push(<Fragment key={`t${last}`}>{text.slice(last, at)}</Fragment>);
    const path = !m[0].startsWith("@");
    out.push(
      <mark key={`m${at}`} className={path ? `${className} path` : className} data-testid={path ? "agent-path-chip" : "agent-chip"}>
        {m[0]}
      </mark>,
    );
    last = at + m[0].length;
  }
  if (!out.length) return text;
  if (last < text.length) out.push(<Fragment key={`t${last}`}>{text.slice(last)}</Fragment>);
  return out;
}

const KIND_ICON = { media: Film, item: ListVideo, track: Rows3, marker: Flag, moment: Clock3, range: ArrowLeftRight, effect: Sparkles, transition: Blend, region: SquareDashed } as const;

export function Composer({ project, online, blocked, turns, running, provider }: { project: Project; online: boolean; blocked: boolean; turns: ReadonlyArray<AgentTurn>; running: string | null; provider: string }) {
  const language = useEditorLanguage();
  const { agentDraft: draft, agentMentions: mentions, agentQueued: queued, agentFocus, agentImages: images, snapshot, brand, templates, agentSending, biliupCanEdit, uploading } = useApp();
  const sending=useRef(false);
  const [attachError, setAttachError] = useState<string | undefined>(undefined);
  const [dropping, setDropping] = useState(false);
  const [attaching, setAttaching] = useState(false);
  const attachBusy = useRef(false);
  const mounted = useRef(true);
  useEffect(() => { mounted.current = true; return () => { mounted.current = false; }; }, []);
  const fileInput = useRef<HTMLInputElement>(null);
  const addFiles = async (selection: ReadonlyArray<File> | Promise<BrowserFileSelection>) => {
    const owner = currentEditorStore();
    const importer = currentFileImporter();
    if (!owner || !online || blocked || !biliupCanEdit || attachBusy.current || sending.current || agentSending) return;
    attachBusy.current = true;
    setAttaching(true);
    setAttachError(undefined);
    try {
      const picked = Array.isArray(selection) ? { files: selection, problems: [] } : await (selection as Promise<BrowserFileSelection>);
      const problems = [...picked.problems, ...await attachComposerFiles(picked.files, owner, importer)];
      if (mounted.current) setAttachError(problems.length ? problems.join('\n') : undefined);
    } catch (error) {
      if (mounted.current) setAttachError(error instanceof Error ? error.message : String(error));
    } finally {
      attachBusy.current = false;
      if (mounted.current) setAttaching(false);
    }
  };
  const input = useRef<HTMLTextAreaElement>(null);
  const backdrop = useRef<HTMLDivElement>(null);
  const [caret, setCaret] = useState(0);
  const [pick, setPick] = useState(0);
  // The @query position the user closed the picker at (Esc), until the text changes.
  const [dismissed, setDismissed] = useState<number | undefined>(undefined);
  const history = useRef<{ at: number; saved: string }>({ at: -1, saved: "" });
  const caretTo = useRef<number | undefined>(undefined);

  // Add to chat: the caret goes to the end of the box.
  useEffect(() => {
    if (!agentFocus) return;
    const el = input.current;
    if (!el) return;
    el.focus();
    el.setSelectionRange(el.value.length, el.value.length);
    setCaret(el.value.length);
  }, [agentFocus]);
  // A caret position chosen by an edit (a picked mention, a deleted chip) once React has the new text.
  useLayoutEffect(() => {
    const at = caretTo.current;
    if (at === undefined || !input.current) return;
    caretTo.current = undefined;
    input.current.setSelectionRange(at, at);
    setCaret(at);
  }, [draft]);

  const candidates = useMemo(() => mentionCandidates(project as never), [project]);
  const before = draft.slice(0, caret);
  const query = /(?:^|\s)@([^\s@]{0,40})$/.exec(before);
  const at = query ? caret - query[1]!.length - 1 : -1;
  const fps = project.canvas.fps;
  const matches = useMemo(() => {
    if (!query) return [];
    const q = query[1]!.toLowerCase();
    // D183: a typed time (@0:12, @12s, @1:02.5) offers that moment first.
    const frame = parseMoment(q, fps);
    const moment = frame !== undefined ? [{ kind: "moment" as const, id: `frame:${frame}`, label: momentLabel(frame, fps), frame, detail: `Moment, frame ${frame}` }] : [];
    return [...moment, ...candidates.filter((c) => c.label.toLowerCase().includes(q)).slice(0, 8 - moment.length)];
  }, [candidates, query?.[1], fps]);
  const pickerOpen = !!query && dismissed !== at && matches.length > 0 && !blocked;
  // D232: /name calls a skill; the picker lists the installed ones (a path closes it). Issue #111:
  // the templates come first as their own group, the skills after them.
  useEffect(() => {
    if (online && !templates) void actions.templatesGet();
  }, [online, !templates]);
  const slash = !query ? /(?:^|\s)\/([\p{L}\p{N}-]{0,40})$/u.exec(before) : null;
  const slashAt = slash ? caret - slash[1]!.length - 1 : -1;
  const skillMatches = useMemo(() => (slash ? slashItems(slash[1]!, brand?.skills ?? [], templates ?? [], entry => entry.builtin ? editorText(entry.name) : entry.name) : []), [brand?.skills, templates, slash?.[1], language]);
  const skillOpen = !!slash && dismissed !== slashAt && skillMatches.length > 0 && !blocked;
  useEffect(() => setPick(0), [query?.[1], slash?.[1]]);

  const setDraft = (text: string, next?: ReadonlyArray<AgentMention>) => {
    if (text !== draft) setDismissed(undefined);
    actions.setAgentDraft(text, next);
  };
  const choose = (c: (typeof matches)[number]) => {
    const insert = `@${c.label} `;
    const text = draft.slice(0, at) + insert + draft.slice(caret);
    const mention: AgentMention = { kind: c.kind, id: c.id, label: c.label, ...(c.frame !== undefined ? { frame: c.frame } : {}) };
    caretTo.current = at + insert.length;
    setDraft(text, [...mentions.filter((m) => !(m.kind === c.kind && m.id === c.id && m.label === c.label)), mention]);
  };

  const chooseSkill = (x: SlashItem) => {
    if (x.kind === "template") {
      // The /query goes at once; the filled brief comes in where it was when the server answers.
      caretTo.current = slashAt;
      setDraft(draft.slice(0, slashAt) + draft.slice(caret));
      void actions.useTemplate(x.id, slashAt);
      return;
    }
    const insert = `/${x.id} `;
    caretTo.current = slashAt + insert.length;
    setDraft(draft.slice(0, slashAt) + insert + draft.slice(caret));
  };

  const send = async () => {
    if (!draft.trim() || !online || blocked || sending.current || agentSending || attachBusy.current) return;
    setAttachError(undefined);
    sending.current=true;
    try {
      if (await actions.agentSubmitDraft()) history.current = { at: -1, saved: "" };
    } finally { sending.current=false; }
  };

  // Earlier prompts, newest first, for ↑ / ↓.
  const prompts = useMemo(() => [...new Set([...turns].reverse().map((t) => t.prompt))], [turns]);
  const recall = (index: number) => {
    const h = history.current;
    if (h.at === -1) h.saved = draft;
    h.at = index;
    const text = index === -1 ? h.saved : prompts[index]!;
    // The chips of a recalled prompt, found again by their labels.
    const turn = turns.find((t) => t.prompt === text);
    const again = (turn?.mentions ?? []).flatMap((m) => {
      const c = candidates.find((x) => x.kind === m.kind && x.label === m.label);
      return c ? [{ kind: c.kind, id: c.id, label: c.label }] : [];
    });
    caretTo.current = text.length;
    setDraft(text, again);
  };

  const labels = mentions.map((m) => m.label);
  return (
    <div
      className={`composer ${dropping ? "dropping" : ""}`}
      data-testid="composer"
      onDragOver={(e) => {
        if (![...e.dataTransfer.items].some((i) => i.kind === "file")) return;
        e.preventDefault();
        if (!online || blocked || attaching || !biliupCanEdit) { e.dataTransfer.dropEffect = 'none'; return; }
        setDropping(true);
      }}
      onDragLeave={(e) => {
        if (!e.currentTarget.contains(e.relatedTarget as Node | null)) setDropping(false);
      }}
      onDrop={(e) => {
        if (![...e.dataTransfer.items].some(item => item.kind === 'file')) return;
        e.preventDefault();
        setDropping(false);
        void addFiles(readDroppedFiles(e.dataTransfer));
      }}
    >
      <ContextLine project={project} />
      {queued && (
        <div className="composer-queued" data-testid="agent-queued">
          <span className="ellipsis">{editorText(queued.paused ? "Unsent: " : "Queued: ")}{queued.request.prompt}</span>
          {queued.paused && <Tooltip label={editorText('Retry')}><button className="icon-btn" aria-label={editorText('Retry')} disabled={!online||blocked||!!running||!!agentSending||attaching} onClick={()=>void actions.agentRetryQueued()}><ArrowUp/></button></Tooltip>}
          <Tooltip label={editorText('Edit message')}><button className="icon-btn" aria-label={editorText('Edit message')} disabled={!!draft.trim()||images.length>0||!!agentSending||attaching} onClick={()=>actions.agentEditQueued()}><FileText/></button></Tooltip>
          <Tooltip label={editorText("Don't send it")}>
            <button className="icon-btn" data-testid="agent-unqueue" aria-label={editorText("Don't send it")} onClick={() => actions.agentClearQueued()}>
              <X />
            </button>
          </Tooltip>
        </div>
      )}
      {pickerOpen && (
        <div className="mention-picker" role="listbox" data-testid="mention-picker">
          {matches.map((c, i) => {
            const Icon = c.kind === "media" && c.detail === "Sound" ? Music : KIND_ICON[c.kind];
            return (
              <button
                key={`${c.kind}:${c.id}`}
                role="option"
                aria-selected={i === pick}
                className={`mention-option ${i === pick ? "active" : ""}`}
                data-testid="mention-option"
                data-kind={c.kind}
                onMouseDown={(e) => {
                  e.preventDefault();
                  choose(c);
                }}
                onMouseEnter={() => setPick(i)}
              >
                <Icon />
                <span className="ellipsis grow">{editorText(c.label)}</span>
                <span className="mention-detail">{editorText(c.detail)}</span>
              </button>
            );
          })}
        </div>
      )}
      {skillOpen && (
        <div className="mention-picker" role="listbox" data-testid="skill-picker">
          {skillMatches.map((x, i) => (
            <Fragment key={`${x.kind}:${x.id}`}>
              {x.kind !== skillMatches[i - 1]?.kind && (
                <div className="slash-group" role="presentation" data-testid={x.kind === "template" ? "template-group" : "skill-group"}>
                  {x.kind === "template" ? editorText("Templates") : editorText("Skills")}
                </div>
              )}
              <button
                role="option"
                aria-selected={i === pick}
                className={`mention-option ${i === pick ? "active" : ""}`}
                data-testid={x.kind === "template" ? "template-option" : "skill-option"}
                {...(x.kind === "template" ? { "data-template": x.id } : { "data-skill": x.id })}
                title={x.builtin ? editorText(x.description) : x.description}
                onMouseDown={(e) => {
                  e.preventDefault();
                  chooseSkill(x);
                }}
                onMouseEnter={() => setPick(i)}
              >
                {x.kind === "template" ? <FileText /> : <BookOpen />}
                <span className="ellipsis grow">{x.builtin ? editorText(x.name) : x.name}</span>
                <span className="skill-call">{x.kind === "template" ? (x.builtin ? editorText("template") : editorText("yours")) : `/${x.id}`}</span>
              </button>
            </Fragment>
          ))}
        </div>
      )}
      {images.length > 0 && (
        <div className="composer-images" data-testid="agent-attachments">
          {images.map((img, i) => (
            <div key={i} className="composer-image" data-testid="agent-attachment" title={img.name}>
              <img src={`data:${img.mimeType};base64,${img.data}`} alt={img.name} />
              <Tooltip label={editorText("Remove")}>
                <button className="icon-btn" data-testid="agent-attachment-remove" onClick={() => actions.setAgentImages(images.filter((_, j) => j !== i))}>
                  <X />
                </button>
              </Tooltip>
            </div>
          ))}
        </div>
      )}
      {attachError && (
        <div className="composer-attach-error small" data-testid="agent-attach-error" role="alert">
          {attachError}
        </div>
      )}
      {attaching && <div className="composer-attach-progress small" role="status">{uploading ? editorText('Importing {0} ({1}/{2})', { 0: uploading.name, 1: uploading.done + 1, 2: uploading.total }) : editorText('Preparing attachments…')}</div>}
      <div className="composer-box">
        <div className="composer-hl" ref={backdrop} aria-hidden="true">
          {withChips(draft, labels)}
          {"​"}
        </div>
        <textarea
          ref={input}
          id="agent-input"
          data-testid="agent-input"
          rows={3}
          value={draft}
          spellCheck={false}
          disabled={!online || blocked}
          placeholder={!biliupCanEdit ? editorText('当前账号没有剪辑权限') : blocked ? editorText("AI 编辑服务尚未配置") : running ? editorText("Write the next message; it's sent when this turn ends") : editorText("Ask for an edit; @ to point at media or a clip, / for a skill or a template")}
          onChange={(e) => {
            history.current.at = -1;
            setCaret(e.target.selectionStart);
            setDraft(e.target.value);
          }}
          onSelect={(e) => setCaret((e.target as HTMLTextAreaElement).selectionStart)}
          onPaste={(e) => {
            const files = [...e.clipboardData.files];
            if (!files.length) return;
            e.preventDefault();
            void addFiles(files);
          }}
          onScroll={(e) => backdrop.current && (backdrop.current.scrollTop = (e.target as HTMLTextAreaElement).scrollTop)}
          onKeyDown={(e) => {
            if (isTextComposition(e.nativeEvent)) return;
            const el = e.currentTarget;
            if (skillOpen) {
              if (e.key === "ArrowDown" || e.key === "ArrowUp") {
                e.preventDefault();
                setPick((p) => (p + (e.key === "ArrowDown" ? 1 : skillMatches.length - 1)) % skillMatches.length);
                return;
              }
              if (e.key === "Enter" || e.key === "Tab") {
                e.preventDefault();
                chooseSkill(skillMatches[Math.min(pick, skillMatches.length - 1)]!);
                return;
              }
              if (e.key === "Escape") {
                e.preventDefault();
                setDismissed(slashAt);
                return;
              }
            }
            if (pickerOpen) {
              if (e.key === "ArrowDown" || e.key === "ArrowUp") {
                e.preventDefault();
                setPick((p) => (p + (e.key === "ArrowDown" ? 1 : matches.length - 1)) % matches.length);
                return;
              }
              if (e.key === "Enter" || e.key === "Tab") {
                e.preventDefault();
                choose(matches[pick]!);
                return;
              }
              if (e.key === "Escape") {
                e.preventDefault();
                setDismissed(at);
                return;
              }
            }
            // A chip goes whole with one Backspace.
            if (e.key === "Backspace" && el.selectionStart === el.selectionEnd) {
              const head = draft.slice(0, el.selectionStart);
              const chip = labels.filter((l) => head.endsWith(`@${l}`)).sort((a, b) => b.length - a.length)[0];
              if (chip) {
                e.preventDefault();
                const from = el.selectionStart - chip.length - 1;
                caretTo.current = from;
                setDraft(draft.slice(0, from) + draft.slice(el.selectionStart));
                return;
              }
            }
            // ↑ in an empty box (or at its start) brings back the last prompt; ↓ goes forward again.
            if (e.key === "ArrowUp" && !e.shiftKey && prompts.length && el.selectionStart === 0 && el.selectionEnd === 0 && (draft === "" || history.current.at >= 0)) {
              e.preventDefault();
              recall(Math.min(prompts.length - 1, history.current.at + 1));
              return;
            }
            if (e.key === "ArrowDown" && history.current.at >= 0 && el.selectionStart === draft.length) {
              e.preventDefault();
              recall(history.current.at - 1);
              return;
            }
            // Enter sends (or queues while a turn runs), ⇧Enter is a new line.
            if (e.key === "Enter" && !e.shiftKey) {
              e.preventDefault();
              void send();
            }
          }}
        />
      </div>
      <div className="composer-foot">
        <input
          ref={fileInput}
          type="file"
          accept={EDITOR_FILE_ACCEPT}
          multiple
          hidden
          data-testid="agent-attach-input"
          onChange={(e) => {
            void addFiles([...(e.target.files ?? [])]);
            e.target.value = "";
          }}
        />
        <Tooltip label={editorText("Attach files")}>
          <button className="icon-btn composer-at" data-testid="agent-attach" aria-label={editorText('Attach files')} aria-haspopup="menu" disabled={!online || blocked || attaching || !biliupCanEdit || agentSending} onClick={event => {
            const rect = event.currentTarget.getBoundingClientRect();
            openMenu(rect.left, rect.bottom, [
              { id: 'attach-files', label: editorText('Choose files'), run: () => fileInput.current?.click() },
              { id: 'attach-folder', label: editorText('Choose folder'), run: () => void addFiles(chooseEditorFiles(true)) },
            ], editorText('Attach files'));
          }}>
            <Paperclip />
          </button>
        </Tooltip>
        <Tooltip label={editorText("Point at media, a clip, a track or a marker")}>
          <button
            className="icon-btn composer-at"
            data-testid="agent-at"
            disabled={!online || blocked}
            onClick={() => {
              const el = input.current;
              if (!el) return;
              const pos = el.selectionStart;
              const pad = pos > 0 && !/\s/.test(draft[pos - 1]!) ? " @" : "@";
              caretTo.current = pos + pad.length;
              setDraft(draft.slice(0, pos) + pad + draft.slice(pos));
              el.focus();
            }}
          >
            <AtSign />
          </button>
        </Tooltip>
        {snapshot && <ComposerChoices key={`${snapshot.dir}:${provider}`} dir={snapshot.dir} provider={provider} />}
        <span className="composer-spacer" />
        {running ? (
          <Tooltip label={editorText("Stop")}>
            <button className="send stop" data-testid="agent-stop" disabled={!online} onClick={() => void actions.agentCancel()}>
              <Square />
            </button>
          </Tooltip>
        ) : (
          <Tooltip label={editorText("Send")} keys={["↩"]}>
            <button className="send" data-testid="agent-send" aria-busy={agentSending || attaching} disabled={!draft.trim() || !online || blocked || agentSending || attaching} onClick={() => void send()}>
              <ArrowUp />
            </button>
          </Tooltip>
        )}
      </div>
    </div>
  );
}

/** What the agent is told about where you are: the playhead and the selected item. */
function ContextLine({ project }: { project: Project }) {
  useEditorLanguage();
  const frame = usePlayback((s) => Math.floor(s.frame));
  const selected = usePlayback((s) => s.selected);
  const item = selected ? project.tracks.flatMap((t) => t.items).find((i) => i.id === selected) : undefined;
  return (
    <div className="composer-ctx" data-testid="agent-context">
      <MousePointer2 />
      <span className="ellipsis">
        {editorText("Playhead ")}{frameTimecode(frame, project.canvas.fps)}
        {item ? editorText(", selected {0}", { 0: itemLabel(project, item) }) : ""}
      </span>
      <TokenMeter />
    </div>
  );
}
