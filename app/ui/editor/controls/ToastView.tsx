import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// An error toast (#67, D239) that closes itself after toastMs(text), stops counting while the
// pointer is over it or focus is in it, and keeps its close button. A new `token` (the store's
// notice object, fresh for every refusal) starts the count again, even for the same text. A `sticky`
// one (D262: "can't reach the server" while the connection is still down) doesn't count at all: it
// stays until the problem is over (the store drops it on reconnect) or it is closed.
import { X } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { toastMs } from "@/app/ui/editor/controls/toast";
import { Tooltip } from "@/app/ui/editor/controls/Tooltip";

export function ErrorToast({ text, token, onClose, testId, sticky = false }: { text: string; token: unknown; onClose: () => void; testId?: string; sticky?: boolean }) {
  useEditorLanguage();
  const message = editorText(text);
  const ms = toastMs(message);
  const [hovered, setHovered] = useState(false);
  const [focused, setFocused] = useState(false);
  const paused = hovered || focused;
  const left = useRef(ms);
  const counted = useRef<unknown>(token);
  // The timer line restarts with each new token too.
  const gen = useRef({ token, n: 0 });
  if (gen.current.token !== token) gen.current = { token, n: gen.current.n + 1 };
  const close = useRef(onClose);
  close.current = onClose;

  useEffect(() => {
    if (counted.current !== token) {
      counted.current = token;
      left.current = ms;
    }
    if (paused || sticky) return;
    const from = performance.now();
    const t = setTimeout(() => close.current(), left.current);
    return () => {
      clearTimeout(t);
      left.current = Math.max(0, left.current - (performance.now() - from));
    };
  }, [paused, token, ms, sticky]);

  return (
    <div
      className="banner error toast"
      data-testid={testId}
      data-auto-close={sticky ? "off" : ms}
      data-paused={paused}
      onPointerEnter={() => setHovered(true)}
      onPointerLeave={() => setHovered(false)}
      onFocus={() => setFocused(true)}
      onBlur={(e) => !e.currentTarget.contains(e.relatedTarget as Node | null) && setFocused(false)}
    >
      <span className="banner-text">{message}</span>
      <Tooltip label={editorText("Dismiss")}>
        <button className="icon-btn" onClick={onClose}>
          <X />
        </button>
      </Tooltip>
      {!sticky && <span key={gen.current.n} className="toast-timer" style={{ animationDuration: `${ms}ms` }} />}
    </div>
  );
}
