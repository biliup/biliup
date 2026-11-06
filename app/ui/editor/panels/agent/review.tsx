import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import { REVIEW_SCORES, type Review, type IntendedFinding } from "@/app/lib/editor/contracts";
import { OPTIONAL_BAR, REQUIRED_BAR, type FindingRef, intendedOf, isVideoType, VIDEO_TYPE_NAMES, isIntended } from "@/app/lib/editor/core/skills/skills";
import { Check, X } from "lucide-react";
import { actions, useAppSelect } from "@/app/lib/editor/state/store";
import { seconds } from "@/app/ui/editor/panels/agent/storyboard";

const SCORE_NAMES: Readonly<Record<(typeof REVIEW_SCORES)[number], string>> = {
  hook: "Hook in 2 s",
  readability: "Phone readability",
  motion: "Motion",
  variety: "Variety",
  sound: "Sound sync",
  brand: "Brand",
};

/**
 * A review's scores as bars, with the critic's evidence under each (D217: only the categories the
 * video's rubric scores; a required one passes at 8, an optional one at 6; not applicable ones greyed).
 */
export function ScoreBars({ review }: { review: Review }) {
  useEditorLanguage();
  return (
    <div className="scores" data-testid="agent-review-scores">
      {REVIEW_SCORES.map((k) => {
        const level = review.rubric?.[k] ?? "required";
        if (level === "na") {
          return (
            <div key={k} className="score na" data-testid="agent-review-score" data-score={k} data-applies="na">
              <span className="score-name">{SCORE_NAMES[k]}</span>
              <span className="score-bar" />
              <span className="score-value">{editorText("N/A")}</span>
              <span className="score-evidence">{editorText("Not judged for this video")}</span>
            </div>
          );
        }
        const v = review.scores[k] ?? 0;
        const bar = level === "optional" ? OPTIONAL_BAR : REQUIRED_BAR;
        const ev = review.evidence?.[k];
        return (
          <div key={k} className={`score ${v >= bar ? "ok" : "low"} ${level}`} data-testid="agent-review-score" data-score={k} data-value={v} data-applies={level}>
            <span className="score-name">
              {SCORE_NAMES[k]}
              {level === "optional" && <span className="muted"> {editorText(" (optional)")}</span>}
            </span>
            <span className="score-bar">
              <span style={{ width: `${v * 10}%` }} />
            </span>
            <span className="score-value">{v}</span>
            {ev && (
              <span className="score-evidence" title={ev}>
                {ev}
              </span>
            )}
          </div>
        );
      })}
    </div>
  );
}

const BLOCKER_NAMES: Readonly<Record<string, string>> = {
  placeholder: "Empty screen",
  "missing-asset": "Missing asset",
  font: "Font",
  overflow: "Overflow",
  illegible: "Hard to read",
  "brand-colour": "Off-brand colour",
  invented: "Invented",
  blank: "Blank frame",
  "off-centre": "Off-centre",
  "unverified-fix": "Fix not seen",
  sound: "Sound",
  other: "Blocker",
};

/** D217: mark a finding Intended (the critic stops raising it), or take the mark back. */
function IntendedButton({ finding, what, marked, online }: { finding: FindingRef; what: string; marked: boolean; online: boolean }) {
  useEditorLanguage();
  const saving = useAppSelect(state => state.agentPlanSaving);
  return (
    <button
      className={`link-btn intended-btn ${marked ? "active" : ""}`}
      data-testid="agent-review-intended"
      aria-pressed={marked}
      disabled={!online || saving}
      title={marked ? editorText("The critic won't raise this again. Click to take it back.") : editorText("This is on purpose: the critic stops raising it")}
      onClick={() => void actions.agentMarkIntended({ ...intendedOf(finding), what }, !marked)}
    >
      {marked ? <Check /> : null}
      {editorText("Intended")}</button>
  );
}

export function ReviewBody({ review, intended, online }: { review: Review; intended: ReadonlyArray<IntendedFinding>; online: boolean }) {
  useEditorLanguage();
  const blockers = review.blockers ?? [];
  const type = isVideoType(review.videoType) ? VIDEO_TYPE_NAMES[review.videoType] : undefined;
  return (
    <>
      <div className="review-by small muted" data-testid="agent-review-by" data-by={review.by ?? "agent"} title={review.by === "critic" ? editorText("A separate reviewer that sees only the frames, the brief, the kit and the storyboard") : undefined}>
        {review.by === "critic" ? editorText("Independent critic") : editorText("Self-scored")}
        {type && <span data-testid="agent-review-type">{editorText(", judged as ")}{type.toLowerCase()}</span>}
        {review.intended ? <span data-testid="agent-review-left-out">, {review.intended === 1 ? editorText("1 finding") : editorText("{0} findings", { 0: review.intended })} {editorText(" you marked Intended left out")}</span> : null}
      </div>
      {review.error && <div className="small err">{review.error}</div>}
      {blockers.length > 0 && (
        <div className="blockers" data-testid="agent-review-blockers">
          {blockers.map((b, i) => {
            const marked = isIntended(b, intended);
            return (
              <div key={i} className={`blocker ${marked ? "marked" : ""}`} data-testid="agent-review-blocker" data-kind={b.kind} data-source={b.source} data-intended={marked}>
                <span className="blocker-kind">{BLOCKER_NAMES[b.kind] ?? b.kind}</span>
                {b.at !== undefined && <span className="defect-at">{seconds(b.at)}</span>}
                <span className="grow">
                  {b.what}
                  {b.source === "check" && <span className="muted"> {editorText(" (Biliup's ")}{b.kind === "sound" ? editorText("sound") : editorText("layout")} {editorText(" check)")}</span>}
                </span>
                {b.kind !== "unverified-fix" && <IntendedButton finding={b} what={b.what} marked={marked} online={online} />}
              </div>
            );
          })}
        </div>
      )}
      {review.fixes && review.fixes.length > 0 && (
        <div className="fixes" data-testid="agent-review-fixes">
          {review.fixes.map((f, i) => (
            <div key={i} className={`fix ${f.verified ? "ok" : "bad"}`} data-testid="agent-review-fix" data-verified={f.verified}>
              {f.verified ? <Check /> : <X />}
              <span className="grow">
                {f.what}
                {f.note && <span className="muted">, {f.note}</span>}
              </span>
            </div>
          ))}
        </div>
      )}
      <ScoreBars review={review} />
      {review.defects.length > 0 && (
        <div className="defects" data-testid="agent-review-defects">
          {review.defects.map((d, i) => {
            const marked = isIntended(d, intended);
            return (
              <div key={i} className={`defect ${marked ? "marked" : ""}`} data-testid="agent-review-defect" data-kind={d.kind} data-source={d.source} data-rule={d.rule} data-intended={marked}>
                <span className="defect-at">{seconds(d.at)}</span>
                <span className="grow">
                  {d.what}
                  {d.fix && <span className="muted">{editorText(", fix: ")}{d.fix}</span>}
                  {d.source === "check" && <span className="muted"> {editorText(" (Biliup's ")}{d.rule?.startsWith("sound") ? editorText("sound") : editorText("layout")} {editorText(" check)")}</span>}
                </span>
                {review.by === "critic" && <IntendedButton finding={d} what={d.what} marked={marked} online={online} />}
              </div>
            );
          })}
        </div>
      )}
      {review.notes && review.by === "critic" && <div className="small">{review.notes}</div>}
      {review.determinism && review.determinism.checked.length > 0 && (
        <div className="small muted" data-testid="agent-review-determinism">
          {review.determinism.differed.length ? editorText("Drew differently twice: {0}", { 0: review.determinism.differed.join(", ") }) : editorText("{0} drew the same twice", { 0: review.determinism.checked.length === 1 ? "1 graphic" : `${review.determinism.checked.length} graphics` })}
        </div>
      )}
    </>
  );
}
