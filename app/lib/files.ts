import { apiFetch, handleResponse } from './api-streamer'

export type FileLocationRequest =
  | { kind: 'recording'; file_name: string }
  | { kind: 'render'; id: number }

export interface FileLocation {
  path: string
  can_open: boolean
}

export async function getFileLocation(request: FileLocationRequest): Promise<FileLocation> {
  const query = new URLSearchParams({ kind: request.kind })
  if (request.kind === 'recording') query.set('file_name', request.file_name)
  else query.set('id', String(request.id))
  const response = await apiFetch(`/v1/files/location?${query.toString()}`)
  await handleResponse(response)
  return response.json()
}

export async function revealFile(request: FileLocationRequest): Promise<void> {
  const response = await apiFetch('/v1/files/reveal', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(request),
  })
  await handleResponse(response)
}
