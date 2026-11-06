import {DanmakuEditor} from '../inspector/Danmaku';
import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// The side panel: Inspector (the selected item, a selected cut's transition, or the project's
// settings when nothing is selected), Agent and History. Split out of Editor.tsx.
import type { ProjectSnapshot } from "@/app/lib/editor/contracts";
import { componentInfo, isDrawnComponent } from "@/app/lib/editor/core/visual/components";
import { SOLID } from "@/app/lib/editor/core/visual/render";
import { AtSign, Film, Music, Type } from "lucide-react";
import { useEffect, type ReactNode } from "react";

import { hasSound } from "@/app/ui/editor/audio/sound";
import { AgentPanel } from "@/app/ui/editor/panels/Agent";
import { ComponentEditor } from "@/app/ui/editor/inspector/component";
import { HistoryPanel } from "@/app/ui/editor/panels/History";
import { AudioEditor } from "@/app/ui/editor/inspector/audio";
import { hasSpeed, NamesEditor, SpeedEditor } from "@/app/ui/editor/inspector/speed";
import { BackgroundEditor } from "@/app/ui/editor/inspector/background";
import { CaptionsEditor } from "@/app/ui/editor/inspector/captions";
import { ClipBoxEditor } from "@/app/ui/editor/inspector/clip";
import { EffectsEditor } from "@/app/ui/editor/inspector/effects";
import { MultiInspector } from "@/app/ui/editor/inspector/multi";
import { Section } from "@/app/ui/editor/inspector/controls";
import { fxSkips } from "@/app/lib/editor/core/visual/fx";
import { TextEditor } from "@/app/ui/editor/inspector/text";
import { TransitionInspector } from "@/app/ui/editor/inspector/transition";
import { playback, usePlayback } from "@/app/ui/editor/preview/playback";
import { ProjectSettings } from "@/app/ui/editor/settings/ProjectSettings";
import { frameTimecode } from "@/app/lib/editor/state/format";
import { itemLabel } from "@/app/lib/editor/state/project";
import { actions, useApp, type Project } from "@/app/lib/editor/state/store";
import { isBoxed } from "@/app/ui/editor/timeline/clip";
import { findItem } from "@/app/ui/editor/timeline/edit";
import { useSideTab, rememberSideTab, type SideTab } from "@/app/ui/editor/workspace/workspaceLayout";
import { useDocking } from "@/app/ui/editor/workspace/agentStage";

/** `staged` (#75, D255): the agent is centre stage; the Agent tab says so (one AgentPanel at a time). */
export function Side({ snapshot, project, online, staged = false, onDock }: { snapshot: ProjectSnapshot; project: Project; online: boolean; staged?: boolean; onDock?: () => void }) {
  useEditorLanguage();
  const tab = useSideTab(snapshot.dir);
  const setTab = (next: SideTab) => rememberSideTab(snapshot.dir, next);
  const selectedId = usePlayback((s) => s.selected);
  const also = usePlayback((s) => s.also);
  const selected = findItem(project, selectedId);
  // F2 (D147): several items selected: what they share, edited on all of them at once.
  // Texts only keep the text Inspector, which already restyles every selected text at once (X3).
  const picked = selected ? [selected.item.id, ...also.filter((id) => findItem(project, id))] : [];
  const many = picked.every((id) => findItem(project, id)?.item.type === "text") ? [] : picked;
  // A selected cut (D73): only while the clip after it is still on the main track, after another clip.
  const cutId = usePlayback((s) => s.cut);
  const cut = findItem(project, cutId);
  // D224: not when a gap is before it (the clips don't meet).
  const cutPlaced = cut && cut.track.role === "main" && cut.index > 0 && !((cut.item.gap ?? 0) > 0) ? cut : undefined;
  // A cut that no longer exists (its clip deleted, moved off main or first now) isn't selected any
  // more; one whose transition was undone stays selected as a hard cut.
  useEffect(() => {
    if (cutId !== undefined && !cutPlaced) playback.selectCut(undefined);
  }, [cutId, cutPlaced]);
  const { biliupCanEdit } = useApp();
  const docking = useDocking(snapshot.dir);
  return (
    <aside className="side">
      <div className="tabs">
        <button className={tab === "inspector" ? "active" : ""} data-testid="tab-inspector" onClick={() => setTab("inspector")}>
          {editorText("Inspector")}</button>
        <button className={tab === "agent" ? "active" : ""} data-testid="tab-agent" onClick={() => setTab("agent")}>
          {editorText("Agent")}</button>
        <button className={tab === "history" ? "active" : ""} data-testid="tab-history" onClick={() => setTab("history")}>
          {editorText("History")}</button>
      </div>
      <div className={`side-body ${docking ? "docking" : ""}`}>
        {tab === "inspector" && cutPlaced ? (
          <TransitionInspector key={cutPlaced.item.id} project={project} placed={cutPlaced} online={online && !!biliupCanEdit} />
        ) : tab === "inspector" && many.length > 1 ? (
          <MultiInspector project={project} ids={many} disabled={!online || !biliupCanEdit} />
        ) : tab === "inspector" && selected ? (
          <ItemInspector key={selected.item.id} project={project} placed={selected} disabled={!online||!biliupCanEdit} />
        ) : tab === "inspector" ? (
          // Keyed on the folder: switching projects drops any half-typed value with the old Inspector.
          <ProjectInspector key={snapshot.dir} snapshot={snapshot} project={project} online={online} />
        ) : tab === "agent" && staged ? (
          <div className="side-staged" data-testid="side-staged">
            <button className="btn" data-testid="side-dock" onClick={onDock}>
              {editorText("Move it here")}</button>
          </div>
        ) : tab === "agent" ? (
          <AgentPanel project={project} online={online} />
        ) : (
          <HistoryPanel key={snapshot.dir} snapshot={snapshot} online={online} />
        )}
      </div>
    </aside>
  );
}

function ProjectInspector({ snapshot, project, online }: { snapshot: ProjectSnapshot; project: Project; online: boolean }) {
  useEditorLanguage();
  return (
    <>
      <div className="insp-title">
        <Film />
        {editorText("Project")}</div>
      <ProjectSettings key={snapshot.dir} snapshot={snapshot} project={project} online={online} />
    </>
  );
}

const TYPE_NAMES: Readonly<Record<string, string>> = { clip: "Clip", component: "Component", graphic: "Graphic", text: "Text", captions: "Captions", danmaku:"Danmaku", audio: "Audio" };

/** The selected item: text and backgrounds get their editors (chunk 4), every item its basics (chunk 3). */
function ItemInspector({ project, placed, disabled }: { project: Project; placed: NonNullable<ReturnType<typeof findItem>>; disabled:boolean }) {
  useEditorLanguage();
  const { item, track, start } = placed;
  const contentLocked = track.locked === true || item.locked === true;
  const locked = disabled || contentLocked;
  const fps = project.canvas.fps;
  const tc = (f: number) => `${frameTimecode(f, fps)}`;
  const media = item.media ? project.media.find((m) => m.id === item.media) : undefined;
  const Row = ({ k, id, v, title }: { k: string; id: string; v: ReactNode; title?: string }) => (
    <div className="kv-row">
      <span className="muted">{k}</span>
      <span className="mono small" data-testid={id} title={title}>
        {v}
      </span>
    </div>
  );
  return (
    <div data-testid="inspector-item" data-item={item.id}>
      <div className="insp-title">
        {item.type === "text" || item.type === "captions" ? <Type /> : item.type === "audio" ? <Music /> : <Film />}
        <span className="ellipsis grow">{itemLabel(project, item)}</span>
        {/* D141: this item as a chip in the Agent tab's composer. */}
        <button className="small-btn insp-chat" data-testid="insp-add-to-chat" title={editorText("Add this item to the agent chat")} onClick={() => actions.addItemToChat(item.id)}>
          <AtSign />
          {editorText("Add to chat")}</button>
      </div>
      <div className="insp-sub">
        {item.type === "component" && item.component === SOLID ? editorText("Background") : editorText(componentInfo(item.component)?.name ?? TYPE_NAMES[item.type] ?? item.type)} {editorText(" on ")}{track.name}
        {contentLocked ? editorText(", locked") : ""}
      </div>
      {item.type === "text" && <TextEditor project={project} item={item} locked={locked} />}
      {item.type === "danmaku" && <DanmakuEditor project={project} item={item} locked={locked} />}
      {item.type === "captions" && <CaptionsEditor project={project} item={item} locked={locked} />}
      {item.type === "component" && item.component === SOLID && <BackgroundEditor project={project} item={item} locked={locked} />}
      {isDrawnComponent(item) && <ComponentEditor project={project} item={item} locked={locked} />}
      {/* F17 (D160): the clip's controls in sections that fold, each one's state remembered on this Mac. */}
      {isBoxed(project, item) && (
        <Section title={editorText("Transform")} testId="insp-sec-transform" remember="transform">
          <ClipBoxEditor project={project} item={item} locked={locked} />
        </Section>
      )}
      {item.type === "clip" && media && media.kind !== "audio" && !fxSkips(media) && <EffectsEditor project={project} item={item} locked={locked} />}
      {hasSpeed(project, item) && (
        <Section title={editorText("Speed")} testId="insp-sec-speed" remember="speed">
          <SpeedEditor project={project} item={item} locked={locked} />
        </Section>
      )}
      {hasSound(project, item) && (
        <Section title={editorText("Audio")} testId="insp-sec-audio" remember="audio">
          <AudioEditor project={project} item={item} locked={locked} />
        </Section>
      )}
      {item.mute && (
        <div className="group" data-testid="insp-muted">
          <div className="group-label row">
            {editorText("Sound off")}<button className="small-btn" data-testid="insp-unmute" disabled={locked} onClick={() => void actions.toggleItem(item.id, "mute")}>
              {editorText("Unmute")}</button>
          </div>
        </div>
      )}
      <Section title={editorText("Advanced")} testId="insp-sec-advanced" remember="advanced">
      <NamesEditor project={project} item={item} locked={locked} />
      <div className="group">
        <Row k="Type" id="insp-type" v={TYPE_NAMES[item.type] ?? item.type} />
        <Row k="Track" id="insp-track" v={track.name} />
        <Row k="Start" id="insp-start" v={tc(start)} title={editorText("frame {0}", { 0: start })} />
        <Row k="Duration" id="insp-duration" v={`${(item.duration / fps).toFixed(2)}s`} title={editorText("{0} frames", { 0: item.duration })} />
        {media && (
          <>
            <Row k="Media" id="insp-media" v={media.name} />
            <Row k="Source in" id="insp-in" v={tc(item.in ?? 0)} title={editorText("frame {0}", { 0: item.in ?? 0 })} />
            <Row k="Source out" id="insp-out" v={tc((item.in ?? 0) + item.duration)} title={editorText("frame {0}", { 0: (item.in ?? 0) + item.duration })} />
          </>
        )}
      </div>
      </Section>
    </div>
  );
}
