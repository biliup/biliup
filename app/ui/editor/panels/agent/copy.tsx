import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Issue #49: a small Copy button for the conversation (a reply, a code block, a tool's output). The
// text itself is selectable too (⌘C, the right-click menu); this copies the whole piece in one click.
import { Check, Copy } from "lucide-react";
import { useEffect, useState } from "react";

import { copyText } from "@/app/lib/editor/clipboard";

/** True while the user has text selected: a click that ended a drag-select shouldn't fold or open a row. */
export const selecting = () => (document.getSelection()?.toString() ?? "") !== "";

export function CopyButton({ text, what = "text", className = "" }: { text: string; what?: string; className?: string }) {
  useEditorLanguage();
  const [copied, setCopied] = useState(false);
  useEffect(() => {
    if (!copied) return;
    const t = setTimeout(() => setCopied(false), 1500);
    return () => clearTimeout(t);
  }, [copied]);
  return (
    <button
      type="button"
      className={`icon-btn copy-btn ${className}`}
      data-testid="agent-copy"
      data-copied={copied ? "true" : undefined}
      title={copied ? editorText("Copied") : editorText("Copy {0}", { 0: what })}
      aria-label={editorText("Copy {0}", { 0: what })}
      onClick={(e) => {
        e.stopPropagation();
        e.currentTarget.blur();
        void copyText(text).then(() => setCopied(true), () => {});
      }}
    >
      {copied ? <Check /> : <Copy />}
    </button>
  );
}
