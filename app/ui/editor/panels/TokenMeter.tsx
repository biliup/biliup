// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Usage comes from the configured model. Draft and per-section totals are estimates;
// an unknown context-window size stays unknown.
import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
import { useApp } from "@/app/lib/editor/state/store";
import { Tooltip } from "@/app/ui/editor/controls/Tooltip";

/** 950 → "950", 1234 → "1.2k", 42_300 → "42k", 1_000_000 → "1M". */
export function tokens(n: number): string {
  const v = Math.max(0, Math.round(n));
  if (v < 1000) return String(v);
  if (v < 10_000) return `${(v / 1000).toFixed(1).replace(/\.0$/, "")}k`;
  if (v < 1_000_000) return `${Math.round(v / 1000)}k`;
  return `${(v / 1_000_000).toFixed(1).replace(/\.0$/, "")}M`;
}

const R = 5.5;
const RING = 2 * Math.PI * R;

export function TokenMeter() {
  const language = useEditorLanguage();
  const { agent, agentDraft } = useApp();
  const c = agent?.context;
  if (!c || !(c.used > 0)) return null;
  const window = c.window && c.window > 0 ? c.window : undefined;
  const share = window ? Math.min(1, c.used / window) : 0;
  const level = share >= 0.95 ? "full" : share >= 0.8 ? "high" : "ok";
  const draft = agentDraft.trim() ? Math.round(agentDraft.trim().length / 4) + c.message : 0;
  const lines = [
    window
      ? editorText('Context: {0} / {1} tokens ({2}%)', {0:c.used.toLocaleString(language),1:window.toLocaleString(language),2:Math.round(share * 100)})
      : editorText('Context: {0} tokens; window size unavailable', {0:c.used.toLocaleString(language)}),
    editorText('System prompt + tools: ~{0}', {0:tokens(c.systemTools)}),
    editorText('Conversation: ~{0}', {0:tokens(c.conversation)}),
    editorText('Message context: ~{0}', {0:tokens(c.message)}),
    ...(draft ? [editorText('Draft estimate: +~{0}', {0:tokens(draft)})] : []),
    ...(level === "full"
      ? [editorText('Context almost full. Start a new conversation to continue.')]
      : level === "high"
        ? [editorText('Context usage exceeds 80%.')]
        : []),
  ];
  return (
    <Tooltip label={lines.join("\n")}>
      <span className={`token-meter ${level}`} data-testid="agent-token-meter" data-level={level} tabIndex={0}>
        <svg viewBox="0 0 14 14" aria-hidden="true">
          <circle className="track" cx="7" cy="7" r={R} />
          <circle className="fill" cx="7" cy="7" r={R} strokeDasharray={`${share * RING} ${RING}`} />
        </svg>
        <span className="token-meter-text" data-testid="agent-token-used">
          {window ? `${tokens(c.used)} / ${tokens(window)}` : tokens(c.used)}
        </span>
        {draft > 0 && (
          <span className="token-meter-draft" data-testid="agent-token-draft">
            +~{tokens(draft)}
          </span>
        )}
      </span>
    </Tooltip>
  );
}
