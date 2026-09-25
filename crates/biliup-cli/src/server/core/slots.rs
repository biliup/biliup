//! 容量可在运行时调整的并发槽位。
//!
//! 下载池（`pool1_size`）与上传池（`pool2_size`）的容量来自配置，Web 界面保存配置后
//! 通过 [`Slots::resize`] 立即生效，不需要重启。`tokio::sync::Semaphore` 发出去的许可
//! 收不回来，调小容量要另外记账，所以这里直接记录「容量 / 已占用」。
//!
//! 调小容量不会打断已占用槽位的任务（正在录制、正在上传的照常跑完），
//! 只是占用数降到新容量以下之前不再发放新槽位。

use std::sync::{Arc, Mutex};
use tokio::sync::Notify;

/// 一组容量可调的并发槽位
#[derive(Debug)]
pub struct Slots {
    state: Mutex<SlotsState>,
    /// 有槽位归还或容量变化时唤醒 [`Slots::acquire`] 的等待者
    changed: Notify,
}

#[derive(Debug)]
struct SlotsState {
    capacity: usize,
    occupied: usize,
}

impl Slots {
    pub fn new(capacity: usize) -> Self {
        Self {
            state: Mutex::new(SlotsState {
                capacity,
                occupied: 0,
            }),
            changed: Notify::new(),
        }
    }

    /// 调整容量，对之后的占用立即生效
    pub fn resize(&self, capacity: usize) {
        self.state.lock().unwrap().capacity = capacity;
        self.changed.notify_waiters();
    }

    /// 当前容量
    pub fn capacity(&self) -> usize {
        self.state.lock().unwrap().capacity
    }

    /// 当前已占用的槽位数。调小容量后可能暂时大于容量
    pub fn occupied(&self) -> usize {
        self.state.lock().unwrap().occupied
    }

    /// 有空闲槽位就占用一个，没有则立即返回 `None`
    pub fn try_acquire(self: &Arc<Self>) -> Option<Slot> {
        let mut state = self.state.lock().unwrap();
        if state.occupied >= state.capacity {
            return None;
        }
        state.occupied += 1;
        Some(Slot {
            slots: Arc::clone(self),
        })
    }

    /// 等到有空闲槽位再占用一个
    pub async fn acquire(self: &Arc<Self>) -> Slot {
        loop {
            let changed = self.changed.notified();
            tokio::pin!(changed);
            // 先登记再检查：检查之后、开始等待之前发生的归还或扩容也能唤醒这里
            changed.as_mut().enable();
            if let Some(slot) = self.try_acquire() {
                return slot;
            }
            changed.await;
        }
    }
}

/// 已占用的槽位，drop 时归还
#[derive(Debug)]
pub struct Slot {
    slots: Arc<Slots>,
}

impl Drop for Slot {
    fn drop(&mut self) {
        self.slots.state.lock().unwrap().occupied -= 1;
        self.slots.changed.notify_waiters();
    }
}

#[cfg(test)]
mod tests {
    use super::Slots;
    use std::sync::Arc;
    use std::task::Poll;

    #[test]
    fn slots_are_bounded_by_capacity_and_freed_on_drop() {
        let slots = Arc::new(Slots::new(2));
        let first = slots.try_acquire().unwrap();
        let _second = slots.try_acquire().unwrap();
        assert!(slots.try_acquire().is_none());
        assert_eq!(slots.occupied(), 2);

        drop(first);
        assert_eq!(slots.occupied(), 1);
        assert!(slots.try_acquire().is_some());
    }

    #[test]
    fn growing_the_capacity_takes_effect_immediately() {
        let slots = Arc::new(Slots::new(1));
        let _first = slots.try_acquire().unwrap();
        assert!(slots.try_acquire().is_none());

        slots.resize(3);
        assert_eq!(slots.capacity(), 3);
        let _second = slots.try_acquire().unwrap();
        let _third = slots.try_acquire().unwrap();
        assert!(slots.try_acquire().is_none());
    }

    /// 调小容量不收回已占用的槽位，占用数降到新容量以下之前不再发放
    #[test]
    fn shrinking_keeps_occupied_slots_until_they_are_released() {
        let slots = Arc::new(Slots::new(3));
        let held: Vec<_> = (0..3).map(|_| slots.try_acquire().unwrap()).collect();

        slots.resize(1);
        assert_eq!(slots.occupied(), 3);
        assert!(slots.try_acquire().is_none());

        let mut held = held.into_iter();
        drop(held.next());
        assert!(slots.try_acquire().is_none(), "还占着 2 个，超过新容量 1");
        drop(held.next());
        assert!(slots.try_acquire().is_none(), "还占着 1 个，等于新容量 1");
        drop(held.next());
        assert!(slots.try_acquire().is_some());
    }

    #[test]
    fn zero_capacity_hands_out_nothing() {
        let slots = Arc::new(Slots::new(0));
        assert!(slots.try_acquire().is_none());
    }

    #[tokio::test]
    async fn acquire_waits_for_a_release() {
        let slots = Arc::new(Slots::new(1));
        let held = slots.try_acquire().unwrap();

        let mut waiting = Box::pin(slots.acquire());
        assert!(futures::poll!(waiting.as_mut()).is_pending());

        drop(held);
        let Poll::Ready(_slot) = futures::poll!(waiting.as_mut()) else {
            panic!("归还后等待者应拿到槽位");
        };
        assert_eq!(slots.occupied(), 1);
    }

    #[tokio::test]
    async fn acquire_wakes_up_when_the_capacity_grows() {
        let slots = Arc::new(Slots::new(0));

        let mut waiting = Box::pin(slots.acquire());
        assert!(futures::poll!(waiting.as_mut()).is_pending());

        slots.resize(1);
        assert!(futures::poll!(waiting.as_mut()).is_ready());
    }

    /// 缩容后等待者要等到占用数降到新容量以下才拿到槽位
    #[tokio::test]
    async fn acquire_respects_a_shrunk_capacity() {
        let slots = Arc::new(Slots::new(2));
        let first = slots.try_acquire().unwrap();
        let second = slots.try_acquire().unwrap();
        slots.resize(1);

        let mut waiting = Box::pin(slots.acquire());
        assert!(futures::poll!(waiting.as_mut()).is_pending());

        drop(first);
        assert!(futures::poll!(waiting.as_mut()).is_pending());

        drop(second);
        assert!(futures::poll!(waiting.as_mut()).is_ready());
    }
}
