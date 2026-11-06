import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
// Failed/skipped files and adjustments made to imported content, plus a
// Dismiss. Media shows it; so does the agent centre stage (D266), where a drop on the card would
// otherwise fail out of sight under its backdrop.
import { actions, useApp } from "@/app/lib/editor/state/store";

export function ImportReport({ testId = "import-report", className = "" }: { testId?: string; className?: string }) {
  useEditorLanguage();
  const { importReport } = useApp();
  if (!importReport) return null;
  return (
    <div className={`import-report ${className}`.trim()} data-testid={testId} role="status">
      {importReport.failed.map((f, index) => (
        <div key={`f-${index}-${f.path}`} className="import-line error">
          <span className="ellipsis" title={f.path}>
            {f.path.split("/").pop()}
          </span>
          <span>{f.message}</span>
        </div>
      ))}
      {importReport.skipped.map((s, index) => (
        <div key={`s-${index}-${s.path}`} className="import-line">
          <span className="ellipsis" title={s.path}>
            {s.path.split("/").pop()}
          </span>
          <span>{editorText(s.reason)}</span>
        </div>
      ))}
      {importReport.warnings?.map((warning,index) => (
        <div key={`w-${index}-${warning.path}`} className="import-line">
          <span className="ellipsis" title={warning.path}>{warning.path.split("/").pop()}</span>
          <span>{warning.message}</span>
        </div>
      ))}
      <button className="link-btn" onClick={actions.dismissImportReport}>
        {editorText("Dismiss")}</button>
    </div>
  );
}
