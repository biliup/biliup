//! One process-wide CPU encoding budget shared by recording and export jobs.

use std::sync::{
    Arc, LazyLock,
    atomic::{AtomicUsize, Ordering},
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio_util::sync::CancellationToken;

static ENCODERS: LazyLock<Arc<Semaphore>> = LazyLock::new(|| Arc::new(Semaphore::new(1)));
static WAITERS: AtomicUsize = AtomicUsize::new(0);
struct Waiting;
impl Drop for Waiting {
    fn drop(&mut self) {
        WAITERS.fetch_sub(1, Ordering::AcqRel);
    }
}
pub fn has_waiters() -> bool {
    WAITERS.load(Ordering::Acquire) > 0
}

/// Background caches must yield to queued foreground processing instead of joining its queue.
pub fn try_acquire() -> Option<OwnedSemaphorePermit> {
    ENCODERS.clone().try_acquire_owned().ok()
}

pub async fn acquire(cancel: &CancellationToken) -> Result<OwnedSemaphorePermit, String> {
    WAITERS.fetch_add(1, Ordering::AcqRel);
    let _waiting = Waiting;
    tokio::select! {
        biased;
        _ = cancel.cancelled() => Err("合成任务已取消".into()),
        permit = ENCODERS.clone().acquire_owned() => permit.map_err(|e| e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn cancelled_waiter_does_not_claim_encoding_slot() {
        let first = acquire(&CancellationToken::new()).await.unwrap();
        let cancelled = CancellationToken::new();
        cancelled.cancel();
        assert!(acquire(&cancelled).await.is_err());
        drop(first);
        let next = acquire(&CancellationToken::new()).await.unwrap();
        drop(next);
    }
}
