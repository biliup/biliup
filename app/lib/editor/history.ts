import { applyEdit, createSession, EditConflict, EditError } from '@/app/lib/editor/core/timeline/apply-ops'
import { validateOps } from '@/app/lib/editor/core/timeline/validate'
import type { Edit, Project, Session } from '@/app/lib/editor/core/timeline/types'
import { editorProblems } from './project'
import {withLinkedAudio} from './core/timeline/arrange/retime'

export interface EditorHistory {
  current: Session
  undo: Project[]
  redo: Project[]
}

export function createHistory(project: Project): EditorHistory {
  const problems = editorProblems(project)
  if (problems.length) throw new EditError('工程数据无效', problems)
  return { current: createSession(project), undo: [], redo: [] }
}

/** Manual and model edits use the same atomic validator and revision/conflict checks. */
export function editHistory(history: EditorHistory, edit: Edit): EditorHistory {
  if (edit.baseRevision !== undefined && (!Number.isSafeInteger(edit.baseRevision) || edit.baseRevision < 0 || edit.baseRevision > history.current.revision)) {
    throw new EditError('编辑版本号无效')
  }
  if (edit.actor === 'agent' && edit.baseRevision !== history.current.revision) {
    throw new EditConflict(['project'])
  }
  const problems = validateOps(edit.ops)
  if (problems.length) throw new EditError('编辑操作无效', problems)
  const current = applyEdit(history.current, {...edit,ops:withLinkedAudio(history.current.project,edit.ops)}, editorProblems)
  return { ...history, current, undo: [...history.undo, history.current.project], redo: [] }
}

function restore(history: EditorHistory, direction: 'undo' | 'redo'): EditorHistory {
  const stack = history[direction]
  const target = stack.at(-1)
  if (!target) return history
  // Keep revisions monotonic: an AI request started before undo must remain stale afterwards.
  const current = applyEdit(history.current, {
    actor: 'user', summary: direction === 'undo' ? '撤销' : '重做',
    ops: [{ op: 'update_project', set: Object.fromEntries([
      ...Object.keys(history.current.project).filter(key => !(key in target)).map(key => [key, null]),
      ...Object.entries(target),
    ]) }],
  }, editorProblems)
  return { ...history, current,
    undo: direction === 'undo' ? stack.slice(0, -1) : [...history.undo, history.current.project],
    redo: direction === 'redo' ? stack.slice(0, -1) : [...history.redo, history.current.project],
  }
}

export const undoHistory = (history: EditorHistory) => restore(history, 'undo')
export const redoHistory = (history: EditorHistory) => restore(history, 'redo')
