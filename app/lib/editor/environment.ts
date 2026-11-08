/** One mounted workbench editor owns this DOM boundary. Release it before mounting another. */
let root: HTMLElement | undefined
const listeners = new Set<{ element: HTMLElement; target: EventTarget; type: string; listener: EventListenerOrEventListenerObject; wrapped: EventListenerOrEventListenerObject; options?: AddEventListenerOptions | boolean }>()
export function editorRoot(): HTMLElement {
  if (!root) throw new Error('编辑器尚未挂载')
  return root
}
export function bindEditorRoot(element: HTMLElement) {
  if (root && root !== element) throw new Error('请先关闭当前编辑工程')
  root = element
  return () => {
    for (const entry of listeners) if (entry.element === element) { entry.target.removeEventListener(entry.type, entry.wrapped, entry.options); listeners.delete(entry) }
    if (root === element) root = undefined
  }
}
export function editorContains(target: EventTarget | null) { return target instanceof Node && !!root?.contains(target) }
/** Includes editor dialogs portalled into the browser's top layer. */
export function editorDialogOpen() {
  return !!root?.querySelector('.modal-layer') || !!document.querySelector('dialog[data-editor-dialog][open]')
    || new URLSearchParams(window.location.search).has('new')
}
/** Keep editor commands scoped here after a control gives up keyboard focus. */
export function focusEditor() {
  if (root?.isConnected && !editorDialogOpen()) root.focus({ preventScroll: true })
}
/** React controls handle bubbling keys first; only unhandled keys reach the owning editor. */
export const editorEvents = {
  addEventListener: ((type: string, listener: EventListenerOrEventListenerObject, options?: AddEventListenerOptions | boolean) => {
    const element = editorRoot(), capture = typeof options === 'boolean' ? options : !!options?.capture
    // React delegates its event handlers above this editor root. A native listener on the root
    // would run before a media tile/field can preventDefault or stopPropagation.
    const target: EventTarget = !capture && (type === 'keydown' || type === 'keyup') ? window : element
    const wrapped: EventListener = event => {
      if (root !== element || !(event.target instanceof Node) || !element.contains(event.target)) return
      if (typeof listener === 'function') listener.call(element, event)
      else listener.handleEvent(event)
    }
    listeners.add({ element, target, type, listener, wrapped, options }); target.addEventListener(type, wrapped, options)
  }) as Window['addEventListener'],
  removeEventListener: ((type: string, listener: EventListenerOrEventListenerObject, options?: EventListenerOptions | boolean) => {
    const capture = typeof options === 'boolean' ? options : !!options?.capture
    for (const entry of listeners) if (entry.type === type && entry.listener === listener && capture === (typeof entry.options === 'boolean' ? entry.options : !!entry.options?.capture)) {
      entry.target.removeEventListener(type, entry.wrapped, options); listeners.delete(entry)
    }
  }) as Window['removeEventListener'],
}
