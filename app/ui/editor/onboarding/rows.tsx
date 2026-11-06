import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
import {useMe} from '@/app/lib/use-me';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import type { SetupCheck, KeyInfo } from "@/app/lib/editor/contracts";
import { Terminal, Check, CircleAlert, Copy, AudioLines, X, Download, TriangleAlert, KeyRound, RefreshCw } from "lucide-react";
import { useEffect, useState, useRef, type ReactElement, type SVGProps } from "react";
import { actions, useApp } from "@/app/lib/editor/state/store";
import { OpenAIMark } from "@/app/ui/editor/panels/composer/providerMarks";
import { ElevenLabsMark, GoogleMark } from "@/app/ui/editor/panels/generate/marks";
import { FFmpegMark, WhisperCppMark } from "@/app/ui/editor/onboarding/toolMarks";
import {useEditorAction} from '../controls/useEditorAction';

// Reuse the editor's provider and media-tool marks.
type Mark = (props: SVGProps<SVGSVGElement>) => ReactElement;
const CHECK_MARKS: Record<string, Mark> = { ffmpeg: FFmpegMark, whisper: WhisperCppMark };
const KEY_MARKS: Partial<Record<KeyInfo["provider"], Mark>> = { openai: OpenAIMark, google: GoogleMark, elevenlabs: ElevenLabsMark };
const RowMark = ({ mark: M, id }: { mark: Mark; id: string }) => <M className="wiz-mark" data-testid={`row-mark-${id}`} aria-hidden focusable="false" />;

const CHIP: Record<SetupCheck["status"], { label: string; className: string; icon: typeof Check }> = {
  ready: { get label() { return editorText("Ready"); }, className: "ok", icon: Check },
  "needs-login": { get label() { return editorText("Needs login"); }, className: "warn", icon: CircleAlert },
  warning: { get label() { return editorText("Found"); }, className: "warn", icon: CircleAlert },
  missing: { get label() { return editorText("Not installed"); }, className: "dim", icon: CircleAlert },
  error: { get label() { return editorText("Error"); }, className: "error", icon: CircleAlert },
};

export function CheckRow({ check }: { check: SetupCheck }) {
  useEditorLanguage();
  const mark = CHECK_MARKS[check.id];
  const chip = CHIP[check.status];
  const ChipIcon = chip.icon;
  const found = check.status !== "missing" && check.path !== undefined;
  const version = check.version ? `Version ${check.version}.` : found ? "Version unknown." : undefined;
  const sub = [version, check.detail].filter(Boolean).join(" ");
  return (
    <div className="wiz-check" data-testid={`check-${check.id}`} data-status={check.status}>
      <div className="wiz-row">
        {mark ? <RowMark mark={mark} id={check.id} /> : <Terminal />}
        <div className="grow">
          <div className="wiz-name">
            {check.name}
            {check.optional && <span className="wiz-optional">{editorText("Optional")}</span>}
          </div>
          <div className="wiz-sub" title={check.path}>
            {sub}
          </div>
        </div>
        <span className={`wiz-chip ${chip.className}`} data-testid={`chip-${check.id}`}>
          <ChipIcon />
          {editorText(chip.label)}
        </span>
      </div>
      {check.fix && check.status !== 'ready' && (
        <div className="wiz-fix">
          <span className="wiz-fix-text">{check.fix.text}</span>
          {check.fix.command && (
            <div className="wiz-cmd">
              <code>{check.fix.command}</code>
              <button className="link-btn" data-testid={`copy-${check.id}`} onClick={() => void actions.copy(check.fix!.command!)} title={editorText("Copy the command")}>
                <Copy />
                {editorText("Copy")}</button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

const mb = (bytes: number) => `${Math.round(bytes / 1e6)} MB`;

/** The speech model for transcripts (D68): which one, and its download (on demand, never at start). */
export function SpeechModelRow() {
  useEditorLanguage();
  const { can } = useMe();
  const { transcription, transcriptionError, transcriptionPending, connection } = useApp();
  const {run,busy}=useEditorAction();
  const retry=<button className="link-btn" disabled={busy||transcriptionPending||connection!=='connected'} onClick={()=>void run(async()=>{await actions.refreshTranscription()},{readOnly:true})}>{editorText('Retry')}</button>;
  if (!transcription) return transcriptionError ? <div className="inline-error" role="alert">{transcriptionError} {retry}</div> : <PendingRow/>;
  const model = transcription.models.find((m) => m.id === transcription.selected);
  if (!model) return null;
  const choices = transcription.models.filter((m) => m.offered || m.id === model.id);
  const download = transcription.downloads.find((d) => d.model === model.id);
  const downloading = download?.phase === "downloading";
  const total = download?.total ?? model.bytes;
  const fraction = downloading && total ? Math.min(1, download.received / total) : 0;
  const offline = connection !== "connected" || !can('clip.edit') || transcriptionPending || busy;
  const sub = model.installed
    ? editorText("Downloaded ({0})", {0:mb(model.bytes)})
    : downloading
      ? `${Math.round(fraction * 100)}% · ${mb(download.received)} / ${mb(total)}`
      : download?.phase === "failed"
        ? (download.error ?? editorText("The download failed."))
        : editorText("Not downloaded ({0})",{0:mb(model.bytes)});
  return (
    <div className="wiz-check" data-testid="whisper-model" data-model={model.id} data-installed={model.installed} data-phase={download?.phase ?? ""}>
      <div className="wiz-row">
        <AudioLines />
        <div className="grow">
          <div className="wiz-name">
            {editorText("Speech model")}
          </div>
          <div className="wiz-sub" data-testid="whisper-model-status" title={transcription.dir}>
            {sub}
          </div>
          {downloading && (
            <div className="wiz-progress" data-testid="whisper-download-progress">
              <span style={{ width: `${Math.round(fraction * 100)}%` }} />
            </div>
          )}
        </div>
        <select className="wiz-select" data-testid="whisper-model-select" value={model.id} disabled={downloading || offline} onChange={(e) => void actions.selectModel(e.target.value)} title={editorText("The model transcripts use: bigger is more accurate, smaller downloads faster")}>
          {choices.map((m) => (
            <option key={m.id} value={m.id}>
              {m.name} ({mb(m.bytes)})
            </option>
          ))}
        </select>
        {model.installed ? (
          <span className="wiz-chip ok" data-testid="chip-whisper-model">
            <Check />
            {editorText("Ready")}</span>
        ) : downloading ? (
          <button className="btn ghost" data-testid="whisper-download-cancel" disabled={offline} onClick={() => void actions.cancelModelDownload(model.id)} title={editorText("Stop the download")}>
            <X />
            {editorText("Cancel")}</button>
        ) : (
          <button className="btn" data-testid="whisper-download" disabled={offline} onClick={() => void actions.downloadModel(model.id)}>
            <Download />
            {editorText("Download")}</button>
        )}
      </div>
      {transcriptionError && (
        <div className="wiz-fix" data-testid="whisper-model-error">
          {transcriptionError} {retry}
        </div>
      )}
    </div>
  );
}

/**
 * Settings › Keys (D99): the generator keys, optional. A key is typed once and goes to the server,
 * which keeps it encrypted; the screen only ever learns whether one is stored.
 */
export function KeyRows() {
  useEditorLanguage();
  const {can}=useMe();
  const canEditKeys=can('config.edit');
  const { keys, keysError, connection } = useApp();
  useEffect(() => {
    if (connection === "connected") void actions.listKeys();
  }, [connection]);
  const offline = connection !== "connected" || !canEditKeys;
  return (
    <div className="service-settings-rows wiz-keys" data-testid="keys">
      <div className="group-label">{editorText("API keys")}</div>
      {!keys && !keysError && connection === 'connected' && <PendingRow />}
      {(keys ?? []).map((k) => (
        <KeyRow key={k.provider} info={k} offline={offline} />
      ))}
      {keysError && (
        <div className="wiz-note error" data-testid="keys-error" role="alert">
          <TriangleAlert />
          {keysError}
        </div>
      )}
    </div>
  );
}

function KeyRow({ info: k, offline }: { info: KeyInfo; offline: boolean }) {
  useEditorLanguage();
  const [value, setValue] = useState("");
  const [saving, setSaving] = useState(false);
  const input = useRef<HTMLInputElement>(null);
  const save = async () => {
    if (offline || !value.trim() || saving) return;
    setSaving(true);
    const ok = await actions.setKey(k.provider, value);
    setSaving(false);
    // The field forgets it once it's stored.
    if (ok) setValue("");
  };
  const mark = KEY_MARKS[k.provider];
  return (
    <div className="wiz-check" data-testid={`key-${k.provider}`} data-set={k.set}>
      <div className="wiz-row">
        {mark ? <RowMark mark={mark} id={k.provider} /> : <KeyRound />}
        <div className="grow">
          <div className="wiz-name">{k.name}</div>
          <div className="wiz-sub">
            {editorText("For ")}{k.usedBy.map(usage=>editorText(usage)).join(", ")}
          </div>
          {k.note && (
            <div className="wiz-sub" data-testid={`key-note-${k.provider}`}>
              {k.note}
            </div>
          )}
        </div>
        {k.set ? (
          <>
            <span className="wiz-chip ok" data-testid={`key-chip-${k.provider}`}>
              <Check />
              {editorText("Saved")}</span>
            <button className="btn ghost" data-testid={`key-remove-${k.provider}`} disabled={offline} onClick={() => void actions.removeKey(k.provider)}>
              {editorText("Remove")}</button>
          </>
        ) : (
          <>
            <input
              ref={input}
              className="field input key-input"
              type="password"
              autoComplete="off"
              spellCheck={false}
              placeholder={editorText("{0} API key", { 0: k.name })}
              aria-label={editorText("{0} API key", { 0: k.name })}
              data-testid={`key-input-${k.provider}`}
              value={value}
              disabled={offline}
              onChange={(e) => setValue(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") void save();
              }}
            />
            <button className="btn" data-testid={`key-save-${k.provider}`} disabled={offline || !value.trim() || saving} onClick={() => void save()}>
              {editorText("Save")}</button>
          </>
        )}
      </div>
    </div>
  );
}

export function PendingRow() {
  useEditorLanguage();
  return (
    <div className="wiz-row pending">
      <RefreshCw className="spin" />
      <div className="grow wiz-sub">{editorText("Checking…")}</div>
    </div>
  );
}
