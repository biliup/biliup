import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import type { Brief, AgentPlan } from "@/app/lib/editor/contracts";
import { soundProfileText } from "@/app/lib/editor/core/checks/checks";
import { isVideoType, VIDEO_TYPE_NAMES, soundProfileFor, VIDEO_TYPES } from "@/app/lib/editor/core/skills/skills";
import { useContext, useEffect, useRef, useState } from "react";
import { useApp, actions } from "@/app/lib/editor/state/store";
import { currentEditorStore, EditorStoreContext } from "@/app/lib/editor/state/store/core";

/**
 * D232 (issue #60): nobody picks skills. The latest Brief card lists the skills the project uses
 * (read by the agent or called with /name) and the ones the brief suggests, as plain names.
 */
function BriefSkills({ brief }: { brief: Brief }) {
  useEditorLanguage();
  const brand = useApp().brand;
  if (!brand) return null;
  const used = brand.active.skills;
  const suggested = (brief.skills ?? []).filter((id) => !used.includes(id) && brand.skills.some((x) => x.id === id));
  const name = (id: string) => {
    const skill = brand.skills.find(entry => entry.id === id);
    return skill ? (skill.builtin ? editorText(skill.name) : skill.name) : id;
  };
  return (
    <div className="brief-row" data-testid="brief-skills">
      <span className="brief-key">{editorText("Skills")}</span>
      <span>{[...used, ...suggested].map(name).join(", ") || editorText("None")}</span>
    </div>
  );
}

export function BriefBody({ brief, live }: { brief: Brief; live?: CardLive | undefined }) {
  useEditorLanguage();
  const { brand, agent } = useApp();
  const active = brand?.active.skills ?? [];
  const type = isVideoType(brief.type) ? editorText(VIDEO_TYPE_NAMES[brief.type]) : "";
  // The user's words set the sound profile as the critic reads them (the main request: the longest message).
  const request = (agent?.turns ?? []).reduce((a, t) => ((t.prompt ?? "").length > a.length ? (t.prompt ?? "") : a), "");
  const sound = soundProfileText(soundProfileFor(brief, active, request || undefined));
  const rows: Array<[string, string]> = [
    ["Film concept", brief.film],
    ["Type", type],
    ["Audience", brief.audience],
    ["Length", brief.duration],
    ["Formats", brief.formats.join(", ")],
    ["Assets", brief.assets.join(", ")],
    ["Look", brief.visual],
    ["Never", brief.hardLines.join("; ")],
    ["Deliver", brief.deliverables.join(", ")],
    ["Notes", brief.notes ?? ""],
  ];
  // The latest card follows the project's skills (the agent or the Brand tab may have changed them).
  useEffect(() => {
    if (live) void actions.brandGet();
  }, [live?.plan.brief?.updatedAt]);
  return (
    <div className="brief-rows" data-testid="agent-brief-rows">
      {rows
        .filter(([, v]) => v.trim())
        .map(([k, v]) => (
          <div key={k} className="brief-row">
            <span className="brief-key">{editorText(k)}</span>
            <span>{v}</span>
          </div>
        ))}
      {live && (
        <div className="brief-row" data-testid="brief-sound">
          <span className="brief-key">{editorText("Sound")}</span>
          <span>{sound.charAt(0).toUpperCase() + sound.slice(1)}</span>
        </div>
      )}
      {live && <BriefSkills brief={brief} />}
    </div>
  );
}

/** What the latest card of its type can do (D197): edit the brief, approve the storyboard. */
export interface CardLive {
  readonly plan: AgentPlan;
  readonly online: boolean;
  readonly busy: boolean;
  /** A storyboard approval card is open. */
  readonly waiting: boolean;
}

const lines = (v: string) => v.split("\n").map((x) => x.trim()).filter(Boolean);

/** The brief as a form (the user's edit, D188). */
export function BriefForm({ brief, revision, onDone }: { brief: Brief; revision: number; onDone: () => void }) {
  useEditorLanguage();
  const owner = useContext(EditorStoreContext);
  const app = useApp();
  const [b, setB] = useState(brief);
  const [lists, setLists] = useState(() => ({
    formats: brief.formats.join("\n"), assets: brief.assets.join("\n"),
    hardLines: brief.hardLines.join("\n"), deliverables: brief.deliverables.join("\n"),
  }));
  const [baseRevision] = useState(revision);
  const mounted = useRef(true);
  useEffect(() => {
    mounted.current = true;
    return () => { mounted.current = false; };
  }, []);
  const saving = app.agentPlanSaving === true;
  const disabled = saving || app.connection !== "connected" || !app.biliupCanEdit;
  const field = (key: "film" | "audience" | "duration" | "visual" | "notes", label: string, rows = 1) => (
    <label className="brief-field">
      <span className="brief-key">{editorText(label)}</span>
      {rows > 1 ? <textarea className="field input" rows={rows} disabled={disabled} data-testid={`plan-brief-${key}`} value={b[key] ?? ""} onChange={(e) => setB({ ...b, [key]: e.target.value })} /> : <input className="field input" disabled={disabled} data-testid={`plan-brief-${key}`} value={b[key] ?? ""} onChange={(e) => setB({ ...b, [key]: e.target.value })} />}
    </label>
  );
  const list = (key: "formats" | "assets" | "hardLines" | "deliverables", label: string) => (
    <label className="brief-field">
      <span className="brief-key">{editorText(label)}</span>
      <textarea className="field input" rows={2} disabled={disabled} data-testid={`plan-brief-${key}`} value={lists[key]} onChange={(e) => setLists({ ...lists, [key]: e.target.value })} />
    </label>
  );
  return (
    <div className="brief-form" data-testid="plan-brief-form">
      {field("film", "Film concept", 2)}
      <label className="brief-field">
        <span className="brief-key">{editorText("Type")}</span>
        <select className="field input" data-testid="plan-brief-type" disabled={disabled} value={b.type ?? ""} onChange={(e) => setB(e.target.value ? { ...b, type: e.target.value } : (({ type: _t, ...rest }) => rest)(b))}>
          <option value="">{editorText("Not set")}</option>
          {VIDEO_TYPES.map((t) => (
            <option key={t} value={t}>
              {editorText(VIDEO_TYPE_NAMES[t])}
            </option>
          ))}
        </select>
      </label>
      {field("audience", "Audience")}
      {field("duration", "Length")}
      {list("formats", "Formats")}
      {list("assets", "Assets")}
      {field("visual", "Look", 2)}
      {list("hardLines", "Never")}
      {list("deliverables", "Deliver")}
      {field("notes", "Notes", 2)}
      <div className="approval-btns">
        <button className="btn ghost" data-testid="plan-brief-cancel" disabled={saving} onClick={onDone}>
          {editorText("Cancel")}</button>
        <button
          className="btn primary"
          data-testid="plan-brief-save"
          disabled={disabled}
          onClick={async () => {
            if (disabled || !mounted.current || currentEditorStore() !== owner) return;
            const saved = await actions.agentPlanSave({ brief: {
              ...b, formats: lines(lists.formats), assets: lines(lists.assets),
              hardLines: lines(lists.hardLines), deliverables: lines(lists.deliverables),
            } }, baseRevision);
            if (saved && mounted.current && currentEditorStore() === owner) onDone();
          }}
        >
          {editorText("Save")}</button>
      </div>
    </div>
  );
}
