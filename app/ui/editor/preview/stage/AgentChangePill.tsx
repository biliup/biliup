import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { Sparkles, X } from "lucide-react";
import { useApp, actions } from "@/app/lib/editor/state/store";
import { changeLabel } from "@/app/lib/editor/state/agentChange";
import { Tooltip } from "@/app/ui/editor/controls/Tooltip";

/** D202, D235: where the agent changed things; a click jumps there. */
export function AgentChangePill({ fps }: { fps: number }) {
  useEditorLanguage();
  const { agentChange } = useApp();
  if (!agentChange) return null;
  return (
    <div className="agent-change" data-testid="agent-change" data-from={agentChange.from} data-to={agentChange.to}>
      <button className="agent-change-jump" data-testid="agent-change-jump" title={editorText("Jump there")} onClick={actions.agentJump}>
        <Sparkles />
        {changeLabel(agentChange, fps)}
      </button>
      <Tooltip label={editorText("Dismiss")}>
        <button className="icon-btn" data-testid="agent-change-dismiss" onClick={actions.dismissAgentChange}>
          <X />
        </button>
      </Tooltip>
    </div>
  );
}
