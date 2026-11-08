import { editorText, useEditorLanguage } from '@/app/lib/editor/i18n';
// Adapted from Uncut, MIT; see licenses/uncut/LICENSE.
import type { ProjectSnapshot } from '@/app/lib/editor/contracts';
import { useState } from 'react';
import { MAX_NAME_LENGTH, projectOf, useApp, type Project } from '@/app/lib/editor/state/store';
import { NumInput, SettledColorField } from '@/app/ui/editor/inspector/controls';
import { useTextDraft } from '@/app/ui/editor/inspector/text/draft';
import { planRename, readName } from '@/app/ui/editor/timeline/edit/names';
import {useProjectEdit} from '../workspace/projectEdit';
import {editorGestureOrigin} from '../workspace/pointerGesture';
import { usePlayback } from '@/app/ui/editor/preview/playback';
import { ASPECTS, aspectOf, FPS_CHOICES, planCanvas, type Canvas } from './canvas';
import {canMatchFirstClip} from '@/app/lib/editor/core/timeline/canvasMatch';

export function ProjectSettings({ snapshot, project, online }: { snapshot: ProjectSnapshot; project: Project; online: boolean }) {
  useEditorLanguage();
  const { biliupCanEdit } = useApp();
  const editable = online && biliupCanEdit === true;
  const draft = usePlayback(state=>state.draft);
  const base = projectOf(snapshot);
  const shown = draft?.kind !== 'history' && draft?.base === base ? draft.project : project;
  const { width, height, fps } = shown.canvas;
  const hasItems = project.tracks.some(track => track.items.length);
  const aspect = aspectOf(shown.canvas);
  const [keep, setKeep] = useState(false);
  const edit = useCanvasEdit(snapshot, keep);
  return (
    <div data-testid="project-settings">
      <ProjectName key={snapshot.dir} project={project} online={editable} />
      {canMatchFirstClip(project) && <div className="group"><label className="check-row">
        <input type="checkbox" checked={shown.canvas.matchFirstClip === true} disabled={!editable} onChange={e => edit.commit({matchFirstClip: e.target.checked})}/>
        {editorText('Match first video')}
      </label></div>}
      <div className="group">
        <div className="group-label">{editorText('Aspect')}</div>
        <div className="seg aspect-seg" data-testid="canvas-aspect">
          {ASPECTS.map(a => (
            <button key={a.id} className={aspect === a.id ? 'active' : ''} data-testid="canvas-aspect-option" data-aspect={a.id} aria-pressed={aspect === a.id} disabled={!editable} onClick={() => edit.commit({ width: a.width, height: a.height })}>
              {editorText(a.label)}
            </button>
          ))}
        </div>
      </div>
      <div className="group">
        <div className="group-label">{editorText('Resolution')}</div>
        <div className="field-row">
          <NumInput key={'width:' + snapshot.dir} testId="canvas-width" label="W" ariaLabel={editorText('Width')} value={width} min={16} max={8192} step={2} disabled={!editable} onPreview={v => edit.preview({ width: v })} onCancel={edit.cancel} onCommit={(v,origin) => v !== undefined && edit.commit({ width: v },origin)} />
          <NumInput key={'height:' + snapshot.dir} testId="canvas-height" label="H" ariaLabel={editorText('Height')} value={height} min={16} max={8192} step={2} disabled={!editable} onPreview={v => edit.preview({ height: v })} onCancel={edit.cancel} onCommit={(v,origin) => v !== undefined && edit.commit({ height: v },origin)} />
        </div>
        {hasItems && (
          <label className="check-row" title={editorText('Off: clip boxes, text and elements are rescaled with the canvas')}>
            <input type="checkbox" data-testid="canvas-keep-positions" checked={keep} disabled={!editable} onChange={e => setKeep(e.target.checked)} />
            {editorText('Keep positions')}
          </label>
        )}
      </div>
      <div className="group">
        <div className="group-label">{editorText('Frame rate')}</div>
        <span className="fps-wrap" data-testid="canvas-fps-why" title={hasItems ? editorText('Every time in the project is counted in frames, so the frame rate is set while the timeline is empty') : undefined}>
          <select className="field input" data-testid="canvas-fps" value={fps} disabled={!editable || hasItems} onChange={e => edit.commit({ fps: Number(e.target.value) as Canvas['fps'] })}>
            {FPS_CHOICES.map(f => <option key={f} value={f}>{f} {editorText(' fps')}</option>)}
          </select>
        </span>
      </div>
      <div className="group">
        <div className="group-label">{editorText('Background')}</div>
        <SettledColorField onCancel={edit.cancel} key={'background:' + snapshot.dir} project={shown} testId="canvas-background" value={shown.canvas.background} fallback="#000000" noAlpha disabled={!editable} onPreview={background => edit.preview({ background })} onCommit={(background,origin) => edit.commit({ background },origin)} />
      </div>
    </div>
  );
}

/** Canvas controls use the same preview and save queue as item properties. */
function useCanvasEdit(snapshot:ProjectSnapshot,keepPositions:boolean){
 const edit=useProjectEdit()
 return {cancel:edit.cancel,
  preview:(patch:Partial<Canvas>)=>{const base=projectOf(snapshot);edit.preview(base,planCanvas(base,patch,{keepPositions}))},
  commit:(patch:Partial<Canvas>,origin=editorGestureOrigin())=>edit.commit(project=>planCanvas(project,patch,{keepPositions}),origin),
 }
}
function ProjectName({ project, online }: { project: Project; online: boolean }) {
  useEditorLanguage();
  const field = useTextDraft({project, value: project.name, disabled: !online, multiline: false, saveOnPause: false, normalize: name => name.trim(), read: p => readName(p, {kind: 'project'}), plan: (p, name) => planRename(p, {kind: 'project'}, name)});

  return (
    <div className="group">
      <div className="group-label">{editorText('Name')}</div>
      <input
        className="field input"
        data-testid="inspector-name"
        disabled={!online}
        maxLength={MAX_NAME_LENGTH}
        spellCheck={false}
        {...field.input}
      />
    </div>
  );
}
