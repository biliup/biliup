// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Issue #63 (D234): the labs' marks next to each model in the Generate tab (and, D277, on the
// setup wizard's agent and key rows). The OpenAI mark is from t3code (MIT);
// Google and ElevenLabs are Simple Icons' paths (simple-icons 16.35.0, CC0 1.0). D277 dropped the
// brand-named coming-soon models, and with them the xAI and ByteDance marks. See THIRD_PARTY_NOTICES.
// The marks are their owners' trademarks, shown only to say who makes a model; Biliup isn't made
// by or affiliated with any of them.
import type { ReactElement, SVGProps } from "react";

import { OpenAIMark } from "@/app/ui/editor/panels/composer/providerMarks";
import type { Brand } from "@/app/ui/editor/panels/generate/catalog";

type Mark = (props: SVGProps<SVGSVGElement>) => ReactElement;

/** Google's G (Simple Icons "google"). */
export const GoogleMark: Mark = (props) => (
  <svg {...props} viewBox="0 0 24 24" fill="currentColor">
    <path d="M12.48 10.92v3.28h7.84c-.24 1.84-.853 3.187-1.787 4.133-1.147 1.147-2.933 2.4-6.053 2.4-4.827 0-8.6-3.893-8.6-8.72s3.773-8.72 8.6-8.72c2.6 0 4.507 1.027 5.907 2.347l2.307-2.307C18.747 1.44 16.133 0 12.48 0 5.867 0 .307 5.387.307 12s5.56 12 12.173 12c3.573 0 6.267-1.173 8.373-3.36 2.16-2.16 2.84-5.213 2.84-7.667 0-.76-.053-1.467-.173-2.053H12.48z" />
  </svg>
);

/** ElevenLabs' two bars (Simple Icons "elevenlabs"). */
export const ElevenLabsMark: Mark = (props) => (
  <svg {...props} viewBox="0 0 24 24" fill="currentColor">
    <path d="M4.6035 0v24h4.9317V0zm9.8613 0v24h4.9317V0z" />
  </svg>
);

const MARKS: Partial<Record<Brand, Mark>> = { openai: OpenAIMark, google: GoogleMark, elevenlabs: ElevenLabsMark };

/** A model's lab mark in a small tile; a monogram where there's no mark to use. */
export function LabMark({ brand, lab }: { brand: Brand; lab: string }) {
  const Mark = MARKS[brand];
  return (
    <span className={`gen-mark ${brand}`} data-testid="generator-mark" data-brand={brand} aria-hidden>
      {Mark ? <Mark className="gen-mark-logo" focusable="false" /> : <span className="gen-mark-letter">{lab.slice(0, 1).toUpperCase()}</span>}
    </span>
  );
}
