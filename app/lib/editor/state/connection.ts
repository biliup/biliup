/** Service results shared by editor commands; transport is biliup's authenticated HTTP API. */
export type Result<A> = { readonly ok: true; readonly value: A } | { readonly ok: false; readonly error: CallError }
export interface CallError { readonly _tag: string; readonly message: string; readonly reason?: string; readonly details?: ReadonlyArray<{ readonly code: string; readonly path: string; readonly message: string }> }
export const isTransportError = (error: CallError) => ['NetworkError', 'Timeout', 'NotConnected'].includes(error._tag)
