/** A synchronous lock whose owner can outlive a dialog/footer instance. */
export class SubmissionLock {
  private locked = false

  get isLocked(): boolean {
    return this.locked
  }

  acquire(): boolean {
    if (this.locked) return false
    this.locked = true
    return true
  }

  release(): void {
    this.locked = false
  }
}
