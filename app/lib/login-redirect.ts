/** Return an absolute same-origin URL after validating an optional login return target. */
export function resolveLoginDestination(nextPath: string, origin: string): string | null {
  try {
    const destination = new URL(nextPath, origin)
    return destination.origin === origin ? destination.href : null
  } catch {
    return null
  }
}
