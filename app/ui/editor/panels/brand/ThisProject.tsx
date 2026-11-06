import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import type { BrandState } from "@/app/lib/editor/contracts";
import { actions } from "@/app/lib/editor/state/store";
import {useEditorAction} from '../../controls/useEditorAction';

export function ThisProject({ brand, hasProject, online }: { brand: BrandState; hasProject: boolean; online: boolean }) {
  useEditorLanguage();
  const {busy,error,editable,run}=useEditorAction();
  const disabled = !hasProject || !online || !editable || busy;
  const yours = brand.kits.filter((k) => k.scope === "global");
  const ours = brand.kits.filter((k) => k.scope === "project");
  return (
    <div className="lib-section" data-testid="brand-this-project">
      <span className="lib-label">{editorText("This project")}</span>
      {!hasProject && <div className="muted small">{editorText("No project open")}</div>}
      <select
        className="field input"
        data-testid="brand-kit-select"
        disabled={disabled}
        value={brand.active.kit ?? ""}
        onChange={(e) => {const kit=e.target.value||null;void run(()=>actions.brandSetActive({kit,skills:brand.active.skills}))}}
      >
        <option value="">{editorText("No kit")}</option>
        {yours.length > 0 && (
          <optgroup label={editorText("Library")}>
            {yours.map((k) => (
              <option key={k.id} value={k.id}>
                {k.name}
              </option>
            ))}
          </optgroup>
        )}
        {ours.length > 0 && (
          <optgroup label={editorText("Pinned to this project")}>
            {ours.map((k) => (
              <option key={k.id} value={k.id}>
                {k.name}
              </option>
            ))}
          </optgroup>
        )}
      </select>
      {error&&<p className="inline-error" role="alert">{error}</p>}
    </div>
  );
}
