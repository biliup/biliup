import { editorText, useEditorLanguage, setEditorLanguage, type EditorLanguage } from '@/app/lib/editor/i18n';
import { editorEvents, focusEditor } from "@/app/lib/editor/environment";
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Project settings, browser preferences and the current biliup service's editor configuration.
import { Clapperboard, Keyboard, SlidersHorizontal, Sparkles, X } from "lucide-react";
import { useEffect } from "react";

import { layout, rail } from "@/app/ui/editor/workspace/workspaceLayout";
import { playback, usePlayback } from "@/app/ui/editor/preview/playback";
import { actions, isTextEntry, projectOf, useApp, type SettingsSection } from "@/app/lib/editor/state/store";
import { ProjectSettings } from "@/app/ui/editor/settings/ProjectSettings";
import { ServiceSettings } from '@/app/ui/editor/settings/ServiceSettings';
import { ShortcutSettings } from "@/app/ui/editor/settings/Shortcuts";
import { isRecordingKey } from "@/app/ui/editor/workspace/keymap";
import { DefaultLengths } from "@/app/ui/editor/settings/DefaultLengths";
import { setProxiesOn, useProxiesOn } from "@/app/ui/editor/settings/proxies";
import { setScrubCacheOn, useScrubCacheOn } from "@/app/ui/editor/settings/scrub";
import { setGpuCompositorOn, useGpuCompositorOn } from "@/app/ui/editor/settings/gpu";
import { DefaultCaptionStyleRow } from "@/app/ui/editor/inspector/captionStyles";
import { Tooltip } from "@/app/ui/editor/controls/Tooltip";
import { isTextComposition } from '@/app/lib/editor/keyboard';

const SECTIONS: ReadonlyArray<{ readonly id: SettingsSection; readonly label: string; readonly icon: typeof X }> = [
  { id: "project", get label() { return editorText("Project"); }, icon: Clapperboard },
  { id: "app", get label() { return editorText("App"); }, icon: SlidersHorizontal },
  { id: "services", get label() { return editorText("Models & transcription"); }, icon: Sparkles },
  { id: "shortcuts", get label() { return editorText("Shortcuts"); }, icon: Keyboard },
];

export function Settings() {
  useEditorLanguage();
  const { snapshot, settingsSection, connection } = useApp();
  const online = connection === "connected";
  const section = settingsSection === "project" && !snapshot ? "app" : settingsSection;
  useEffect(() => {
    let active = true;
    const onKey = (e: KeyboardEvent) => {
      // #78: Esc while Settings › Shortcuts listens for a key cancels that (its own handler).
      if (e.key !== "Escape" || e.defaultPrevented || isRecordingKey() || isTextComposition(e)) return;
      // Esc in a field puts its value back first (the field's own handler); the next one closes.
      // A switch, checkbox, radio, slider or colour has no typed value to put back: Esc closes.
      const el = e.target;
      if (el instanceof HTMLElement && isTextEntry(el)) {
        const modal = el.closest<HTMLElement>('[data-testid=settings]');
        requestAnimationFrame(() => { if (active && !e.defaultPrevented && document.activeElement === el) modal?.focus({ preventScroll: true }); });
        return;
      }
      e.preventDefault();
      e.stopImmediatePropagation();
      actions.closeSettings();
    };
    editorEvents.addEventListener("keydown", onKey);
    return () => { active = false; editorEvents.removeEventListener("keydown", onKey); focusEditor(); };
  }, []);
  return (
    <div className="modal-layer" data-testid="settings" tabIndex={-1} onPointerDown={(e) => e.target === e.currentTarget && actions.closeSettings()}>
      <div className="modal settings-modal">
        <nav className="settings-nav">
          <div className="settings-title">{editorText("Settings")}</div>
          {SECTIONS.map((s) => {
            const Icon = s.icon;
            return (
              <button key={s.id} className={section === s.id ? "active" : ""} data-testid={`settings-nav-${s.id}`} disabled={s.id === "project" && !snapshot} onClick={() => actions.openSettings(s.id)}>
                <Icon />
                {editorText(s.label)}
              </button>
            );
          })}
        </nav>
        <div className="settings-body" data-testid={`settings-${section}`}>
          <div className="sheet-head">
            <span className="grow">{editorText(SECTIONS.find((s) => s.id === section)!.label)}</span>
            <Tooltip label={editorText("Close")} keys={["Esc"]}>
              <button className="icon-btn" data-testid="settings-close" onClick={actions.closeSettings}>
                <X />
              </button>
            </Tooltip>
          </div>
          {section === "project" && snapshot && <ProjectSettings key={snapshot.dir} snapshot={snapshot} project={projectOf(snapshot)} online={online} />}
          {section === "app" && <AppSettings />}
          {section === "services" && <ServiceSettings />}
          {section === "shortcuts" && <ShortcutSettings />}
        </div>
      </div>
    </div>
  );
}

function AppSettings() {
  const language = useEditorLanguage();
  const snapping = usePlayback((s) => s.snapping);
  const proxies = useProxiesOn();
  const scrubCache = useScrubCacheOn();
  const gpu = useGpuCompositorOn();
  return (
    <div data-testid="app-settings">
      <div className="group">
        <div className="group-label">{editorText('Language')}</div>
        <label className="field">
          <span className="k">{editorText('Editor language')}</span>
          <select className="v" data-testid="editor-language" aria-label={editorText('Editor language')} value={language}
            onChange={event => setEditorLanguage(event.target.value as EditorLanguage)}>
            <option value="zh-CN" lang="zh-CN">简体中文</option>
            <option value="en" lang="en">English</option>
          </select>
        </label>
      </div>
      <div className="group">
        <div className="group-label">{editorText("Timeline")}</div>
        <label className="check-row">
          <input type="checkbox" data-testid="settings-snapping" checked={snapping} onChange={playback.toggleSnapping} />
          {editorText("Snap to the playhead, item edges and beats")}</label>
      </div>
      <DefaultLengths />
      <div className="group">
        <div className="group-label">{editorText("Preview")}</div>
        <label className="check-row">
          <input type="checkbox" data-testid="settings-proxies" checked={proxies} onChange={(e) => setProxiesOn(e.target.checked)} />
          {editorText("Use proxies for preview")}</label>
        <label className="check-row">
          <input type="checkbox" data-testid="settings-scrub-cache" checked={scrubCache} onChange={(e) => setScrubCacheOn(e.target.checked)} />
          {editorText("Decode frames ahead for scrubbing")}</label>
        <label className="check-row">
          <input type="checkbox" data-testid="settings-gpu-compositor" checked={gpu} onChange={(e) => setGpuCompositorOn(e.target.checked)} />
          {editorText("Draw videos and images on the GPU")}</label>
      </div>
      <div className="group">
        <div className="group-label">{editorText("Captions")}</div>
        <DefaultCaptionStyleRow />
      </div>
      <div className="group">
        <div className="group-label">{editorText("Layout")}</div>
        <div className="settings-buttons">
          <button className="btn" data-testid="settings-reset-layout" onClick={() => layout.reset()}>
            {editorText("Reset panel sizes")}</button>
          <button
            className="btn"
            data-testid="settings-reset-rail"
            onClick={() => rail.reset()}
          >
            {editorText("Reset tab order")}</button>
        </div>
      </div>
      <div className="group">
        <div className="group-label">{editorText("Saving")}</div>
        <div className="kv-row">
          <span className="muted">{editorText("Autosave")}</span>
          <span data-testid="settings-autosave">{editorText("每次编辑自动保存到 biliup")}</span>
        </div>
      </div>
    </div>
  );
}

