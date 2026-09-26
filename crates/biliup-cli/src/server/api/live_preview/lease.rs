//! 中转预览的租约：一条预览响应只在它所属页面还「活着」时继续推流。
//!
//! 服务端从 TCP 层看不出客户端是否还在看——浏览器经过会替它读完上游的代理 / 隧道时，关掉
//! 播放器 TCP 也不断，幽灵连接会一直按码率往代理发数据（#1750）。所以让页面自己证明还在：
//!
//! - **会话**（`session`）：页面级随机串，由浏览器生成。页面用它连码率 WebSocket
//!   （`/v1/ws/live-rates?session=`）即持有租约；服务端每 [`PING_INTERVAL`] 发一次 Ping
//!   （浏览器自动回 Pong，后台标签页也照回），[`LEASE_TIMEOUT`] 内没收到 Pong 就断开。
//!   心跳连接断开时，该会话下的全部预览立即结束（[`TicketEnd::LeaseLost`]）。WebSocket 连不上时
//!   前端退回每秒轮询 `/v1/live-rates?session=`，每次轮询把租约续到 [`LEASE_TIMEOUT`] 之后。
//! - **连接**（`conn`）：每条预览响应一个，也由浏览器生成（`/live?session=&conn=`）。
//!   `DELETE /v1/streamers/{id}/live?conn=`（页面关闭时 `sendBeacon` 发同一路径的 POST）
//!   立即结束它（[`TicketEnd::Released`]）。
//! - **宽限**：没有会话（`curl`、外部播放器）或会话暂时没有心跳时，预览最多再推
//!   [`LEASE_TIMEOUT`]；到点仍没有心跳 / 轮询续约就结束。
//!
//! 满员挤掉最早的一条与单条连接的最长寿命仍在（见 `live_preview`），作为租约之外的兜底。
//! 这里的一切都发生在 HTTP / WebSocket 处理路径上，录制写入端不碰。

use biliup::downloader::preview::{PreviewTicket, TicketEnd};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};
use std::time::Duration;
use tokio::sync::watch;
use tokio::time::{Instant, Sleep};

/// 多久收不到心跳就认为页面已经不在了。
pub const LEASE_TIMEOUT: Duration = Duration::from_secs(45);
/// 服务端在心跳连接上发 Ping 的间隔；[`LEASE_TIMEOUT`] 内能容忍丢两次。
pub const PING_INTERVAL: Duration = Duration::from_secs(15);
const MAX_ID_LEN: usize = 64;

/// 会话号 / 连接号的合法形状：1–64 个 `[A-Za-z0-9._-]`。不合法的当作没带。
pub fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_ID_LEN
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

struct Session {
    /// 持有这份租约的心跳连接数（页面重连时新旧两条可能短暂并存）
    heartbeats: usize,
    /// 没有心跳连接时租约到期的时刻；有心跳连接时为 `None`
    deadline: watch::Sender<Option<Instant>>,
    conns: usize,
}

struct Conn {
    session: String,
    ticket: PreviewTicket,
}

#[derive(Default)]
struct Inner {
    sessions: HashMap<String, Session>,
    conns: HashMap<String, Conn>,
}

/// 租约登记表。`Clone` 得到同一张表；进程里用 [`leases`] 那一张。
#[derive(Clone)]
pub struct Leases {
    inner: Arc<Mutex<Inner>>,
    timeout: Duration,
}

impl Default for Leases {
    fn default() -> Self {
        Self::with_timeout(LEASE_TIMEOUT)
    }
}

/// 进程内唯一的租约登记表。
pub fn leases() -> &'static Leases {
    static LEASES: OnceLock<Leases> = OnceLock::new();
    LEASES.get_or_init(Leases::default)
}

impl Leases {
    pub fn with_timeout(timeout: Duration) -> Self {
        Self {
            inner: Arc::default(),
            timeout,
        }
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// 把一条预览登记到会话下。`session` / `conn` 为 `None` 时用这条连接独有的内部键
    /// （外部拿不到，也就续不了约，只有宽限）。会话此刻没有心跳连接时，租约至少续到
    /// 现在起 [`LEASE_TIMEOUT`]；同一个 `conn` 再次登记时旧的那条被释放。
    pub fn bind(
        &self,
        session: Option<&str>,
        conn: Option<&str>,
        ticket: &PreviewTicket,
    ) -> LeaseBinding {
        let session = session.map_or_else(|| format!("~{}", ticket.id()), str::to_string);
        let conn = conn.map_or_else(|| format!("~{}", ticket.id()), str::to_string);
        let deadline = Instant::now() + self.timeout;
        let mut inner = self.lock();
        let entry = inner
            .sessions
            .entry(session.clone())
            .or_insert_with(|| Session {
                heartbeats: 0,
                deadline: watch::Sender::new(Some(deadline)),
                conns: 0,
            });
        entry.conns += 1;
        if entry.heartbeats == 0 {
            entry.deadline.send_if_modified(|current| match current {
                Some(at) if *at >= deadline => false,
                _ => {
                    *current = Some(deadline);
                    true
                }
            });
        }
        let rx = entry.deadline.subscribe();
        if let Some(previous) = inner.conns.insert(
            conn.clone(),
            Conn {
                session: session.clone(),
                ticket: ticket.clone(),
            },
        ) {
            previous.ticket.end(TicketEnd::Released);
        }
        drop(inner);
        LeaseBinding {
            leases: self.clone(),
            session,
            conn,
            ticket: ticket.id(),
            deadline: rx,
            sleep: Box::pin(tokio::time::sleep(Duration::from_secs(365 * 24 * 3600))),
            armed: None,
        }
    }

    /// 心跳连接连上：租约不再有期限，直到这个守卫 drop。drop 时若会话已没有别的心跳连接，
    /// 结束该会话下的全部预览。
    pub fn heartbeat(&self, session: &str) -> Heartbeat {
        let mut inner = self.lock();
        let entry = inner
            .sessions
            .entry(session.to_string())
            .or_insert_with(|| Session {
                heartbeats: 0,
                deadline: watch::Sender::new(None),
                conns: 0,
            });
        entry.heartbeats += 1;
        entry.deadline.send_replace(None);
        Heartbeat {
            leases: self.clone(),
            session: session.to_string(),
        }
    }

    /// 轮询续约（心跳连接用不了时）：已有预览的会话若此刻没有心跳连接，把租约续到
    /// 现在起 [`LEASE_TIMEOUT`]。没有预览的会话不登记，免得每个轮询的页面都留一条。
    pub fn renew(&self, session: &str) {
        let deadline = Instant::now() + self.timeout;
        let inner = self.lock();
        if let Some(entry) = inner.sessions.get(session)
            && entry.heartbeats == 0
        {
            entry.deadline.send_replace(Some(deadline));
        }
    }

    /// 客户端声明不再需要这条预览。返回是否找到了它。
    pub fn release(&self, conn: &str) -> bool {
        let inner = self.lock();
        match inner.conns.get(conn) {
            Some(entry) => {
                entry.ticket.end(TicketEnd::Released);
                true
            }
            None => false,
        }
    }

    /// 当前登记的会话数与预览数（测试 / 排障用）。
    pub fn counts(&self) -> (usize, usize) {
        let inner = self.lock();
        (inner.sessions.len(), inner.conns.len())
    }

    fn unbind(&self, session: &str, conn: &str, ticket: u64) {
        let mut inner = self.lock();
        if inner
            .conns
            .get(conn)
            .is_some_and(|entry| entry.ticket.id() == ticket)
        {
            inner.conns.remove(conn);
        }
        if let Some(entry) = inner.sessions.get_mut(session) {
            entry.conns = entry.conns.saturating_sub(1);
            if entry.conns == 0 && entry.heartbeats == 0 {
                inner.sessions.remove(session);
            }
        }
    }

    fn heartbeat_gone(&self, session: &str) {
        let mut inner = self.lock();
        let Some(entry) = inner.sessions.get_mut(session) else {
            return;
        };
        entry.heartbeats = entry.heartbeats.saturating_sub(1);
        if entry.heartbeats > 0 {
            return;
        }
        entry.deadline.send_replace(Some(Instant::now()));
        if entry.conns == 0 {
            inner.sessions.remove(session);
            return;
        }
        for conn in inner.conns.values().filter(|c| c.session == session) {
            conn.ticket.end(TicketEnd::LeaseLost);
        }
    }
}

/// 心跳连接持有的租约，drop 即交还（见 [`Leases::heartbeat`]）。
pub struct Heartbeat {
    leases: Leases,
    session: String,
}

impl Drop for Heartbeat {
    fn drop(&mut self) {
        self.leases.heartbeat_gone(&self.session);
    }
}

/// 一条预览在租约表里的登记，随响应体一起活，drop 即注销。
pub struct LeaseBinding {
    leases: Leases,
    session: String,
    conn: String,
    ticket: u64,
    deadline: watch::Receiver<Option<Instant>>,
    /// 常驻的计时器，只在期限变化时重设（每个分块都会轮询一次，不能每次新建）
    sleep: Pin<Box<Sleep>>,
    armed: Option<Instant>,
}

impl LeaseBinding {
    /// 浏览器生成的连接号（没带时是内部键）
    pub fn conn(&self) -> &str {
        &self.conn
    }

    /// 是否属于浏览器页面的会话（不是 `curl` 这类没带会话的客户端）
    pub fn has_session(&self) -> bool {
        !self.session.starts_with('~')
    }

    /// 租约到期时完成：会话没有心跳连接、也没有被轮询续约，过了期限。可放进 `select!`。
    pub async fn lapsed(&mut self) {
        loop {
            let deadline = *self.deadline.borrow_and_update();
            match deadline {
                None => {
                    if self.deadline.changed().await.is_err() {
                        std::future::pending::<()>().await;
                    }
                }
                Some(at) => {
                    if self.armed != Some(at) {
                        self.sleep.as_mut().reset(at);
                        self.armed = Some(at);
                    }
                    tokio::select! {
                        biased;
                        _ = self.sleep.as_mut() => return,
                        changed = self.deadline.changed() => {
                            if changed.is_err() {
                                self.sleep.as_mut().await;
                                return;
                            }
                        }
                    }
                }
            }
        }
    }
}

impl Drop for LeaseBinding {
    fn drop(&mut self) {
        self.leases.unbind(&self.session, &self.conn, self.ticket);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_short_and_url_safe() {
        assert!(valid_id("a1b2.3"));
        assert!(valid_id("Ab_-9"));
        assert!(!valid_id(""));
        assert!(!valid_id("~1"));
        assert!(!valid_id("a b"));
        assert!(!valid_id(&"x".repeat(65)));
    }

    /// 没有会话的预览只有宽限：到点租约就到期；有心跳连接时不到期。
    #[tokio::test]
    async fn a_binding_without_heartbeat_lapses_after_the_timeout() {
        let leases = Leases::with_timeout(Duration::from_millis(200));
        let mut anonymous = leases.bind(None, None, &PreviewTicket::new());
        assert!(!anonymous.has_session());
        let started = Instant::now();
        anonymous.lapsed().await;
        assert!(started.elapsed() >= Duration::from_millis(200));
        assert!(started.elapsed() < Duration::from_secs(2));

        let _heartbeat = leases.heartbeat("page");
        let mut held = leases.bind(Some("page"), Some("c1"), &PreviewTicket::new());
        assert!(held.has_session());
        assert!(
            tokio::time::timeout(Duration::from_millis(600), held.lapsed())
                .await
                .is_err(),
            "a lease with a heartbeat never lapses"
        );
    }

    /// 预览先到、心跳连接后到：心跳一连上就不再有期限；轮询续约把期限往后推；
    /// 心跳断开则立刻到期。
    #[tokio::test]
    async fn a_late_heartbeat_or_polling_keeps_the_lease() {
        let leases = Leases::with_timeout(Duration::from_millis(300));
        let mut binding = leases.bind(Some("page"), Some("c1"), &PreviewTicket::new());
        tokio::time::sleep(Duration::from_millis(200)).await;
        leases.renew("page");
        assert!(
            tokio::time::timeout(Duration::from_millis(250), binding.lapsed())
                .await
                .is_err()
        );
        let heartbeat = leases.heartbeat("page");
        assert!(
            tokio::time::timeout(Duration::from_millis(500), binding.lapsed())
                .await
                .is_err()
        );
        drop(heartbeat);
        tokio::time::timeout(Duration::from_millis(50), binding.lapsed())
            .await
            .expect("losing the heartbeat ends the lease at once");
    }

    /// 心跳连接断开：该会话下的预览全部结束（别的会话不受影响）；登记随绑定 drop 清空。
    #[tokio::test]
    async fn losing_the_heartbeat_ends_every_preview_of_the_session() {
        let leases = Leases::default();
        let heartbeat = leases.heartbeat("page");
        let (a, b, other) = (
            PreviewTicket::new(),
            PreviewTicket::new(),
            PreviewTicket::new(),
        );
        let bind_a = leases.bind(Some("page"), Some("a"), &a);
        let bind_b = leases.bind(Some("page"), Some("b"), &b);
        let bind_other = leases.bind(Some("other"), Some("o"), &other);
        assert_eq!(leases.counts(), (2, 3));
        drop(heartbeat);
        assert_eq!(a.ended_by(), Some(TicketEnd::LeaseLost));
        assert_eq!(b.ended_by(), Some(TicketEnd::LeaseLost));
        assert_eq!(other.ended_by(), None);
        drop((bind_a, bind_b, bind_other));
        assert_eq!(leases.counts(), (0, 0));
    }

    /// 客户端释放：只结束那一条；同一个连接号重新登记时旧的那条被释放，新的不受旧绑定注销影响。
    #[tokio::test]
    async fn release_ends_exactly_that_preview() {
        let leases = Leases::default();
        let (a, b) = (PreviewTicket::new(), PreviewTicket::new());
        let _bind_a = leases.bind(Some("page"), Some("a"), &a);
        let _bind_b = leases.bind(Some("page"), Some("b"), &b);
        assert!(leases.release("a"));
        assert!(!leases.release("missing"));
        assert_eq!(a.ended_by(), Some(TicketEnd::Released));
        assert_eq!(b.ended_by(), None);

        let (old, new) = (PreviewTicket::new(), PreviewTicket::new());
        let old_bind = leases.bind(Some("page"), Some("same"), &old);
        let _new_bind = leases.bind(Some("page"), Some("same"), &new);
        assert_eq!(old.ended_by(), Some(TicketEnd::Released));
        drop(old_bind);
        assert!(
            leases.release("same"),
            "the newer binding is still registered"
        );
        assert_eq!(new.ended_by(), Some(TicketEnd::Released));
    }
}
