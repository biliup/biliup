//! One process-wide CPU encoding budget shared by recording and export jobs.

use std::sync::{Arc, LazyLock};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio_util::sync::CancellationToken;

static ENCODERS: LazyLock<Arc<Semaphore>> = LazyLock::new(|| Arc::new(Semaphore::new(1)));

pub async fn acquire(cancel: &CancellationToken) -> Result<OwnedSemaphorePermit, String> {
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
