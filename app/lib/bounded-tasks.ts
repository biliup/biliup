/** Bound network fan-out while preserving result order and individual failures. */
export async function boundedTasks<T, R>(items: readonly T[], task: (item: T) => Promise<R>, concurrency = 4): Promise<PromiseSettledResult<R>[]> {
  const results: PromiseSettledResult<R>[] = new Array(items.length)
  let next = 0
  await Promise.all(Array.from({ length: Math.min(Math.max(1, concurrency), items.length) }, async () => {
    while (next < items.length) {
      const index = next++
      try { results[index] = { status: 'fulfilled', value: await task(items[index]) } }
      catch (reason) { results[index] = { status: 'rejected', reason } }
    }
  }))
  return results
}
