//! 一主一备（HA Pair，ha-pair 方案 H1）。
//!
//! 主机 = 控制面 + 「本机」节点（F5），备机 = 一台被指定为备机的普通节点。主机「本机」持有的房间与模板
//! 自动镜像给备机；两台对每一场直播交换场次消息，决定「这一场谁投」，保证不出重复稿件、主机离线时不漏投。
//!
//! 录制与投稿流程里只有几行调用（`core/monitor.rs`、`common/download.rs`、`common/upload.rs`）。
//! 主机的一侧在 [`primary`]，备机的决策在 [`standby`]、接线在 [`agent`]。
//! 没有配对时 [`ROLE`] 是空的，每处调用只读一次原子变量就返回：不分配、不记日志、不改任何状态。

pub mod agent;
pub mod key;
pub mod pairing;
pub mod params;
pub mod primary;
pub mod standby;
pub mod store;
pub mod upload;
pub mod wire;

use crate::server::infrastructure::context::Context;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

/// 本进程在配对里的角色；没配对时为空
#[derive(Clone)]
pub(crate) enum Role {
    Primary(Arc<primary::Primary>),
    Standby(Arc<agent::Standby>),
}

static ACTIVE: AtomicBool = AtomicBool::new(false);
static ROLE: RwLock<Option<Role>> = RwLock::new(None);

pub(crate) fn set_role(role: Option<Role>) {
    let mut current = ROLE.write().unwrap();
    ACTIVE.store(role.is_some(), Ordering::Release);
    *current = role;
}

fn role() -> Option<Role> {
    if !ACTIVE.load(Ordering::Acquire) {
        return None;
    }
    ROLE.read().unwrap().clone()
}

/// 角色是整个进程共用的，设置角色的测试先拿这把锁
#[cfg(test)]
pub(crate) async fn test_guard() -> tokio::sync::MutexGuard<'static, ()> {
    static LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    LOCK.lock().await
}

/// 一次拉流任务（`DownloadTask::execute`）录下的一段：房间地址 + 检测到开播的时刻。
/// 投稿流程拿到的 [`Context`] 是同一个的克隆，按它就能认回是哪一段
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Unit {
    pub url: String,
    /// 检测到开播的时刻（`LiveStream::date`，Unix 毫秒）
    pub started_at: i64,
    /// 切片工作台的场次 id（`stream_sessions.id`）
    pub session: i64,
}

impl Unit {
    fn of(ctx: &Context) -> Self {
        Unit {
            url: ctx.live_streamer().url.clone(),
            started_at: ctx.live_stream().date.timestamp_millis(),
            session: ctx.id(),
        }
    }
}

/// 这一段交给投稿流程的情况（`SegmentEventProcessor` 数的）
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UnitOutput {
    /// 录完的分段数
    pub seen: usize,
    /// 没被过滤、交给了投稿流程的分段数
    pub sent: usize,
}

/// 暂不开录的原因
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hold {
    pub reason: String,
    /// 很快就会放开（等备机上报最多 10 秒）：监控循环按快速轮换再看，不睡一整个检测周期
    pub quick: bool,
}

/// 只有要投稿的房间才需要决定「谁投」
fn uploads(ctx: &Context) -> bool {
    ctx.upload_config()
        .as_ref()
        .is_some_and(|config| !config.is_noop_uploader())
}

/// 监控循环检测到开播、开录之前（`core/monitor.rs`）
pub fn hold_recording(url: &str) -> Option<Hold> {
    match role()? {
        Role::Primary(primary) => primary.hold(url),
        Role::Standby(standby) => standby.hold(url),
    }
}

/// 一次拉流任务开始录（`common/download.rs`）
pub fn unit_started(ctx: &Context) {
    let Some(role) = role() else {
        return;
    };
    if !uploads(ctx) {
        return;
    }
    match role {
        Role::Primary(primary) => primary.unit_started(&Unit::of(ctx)),
        Role::Standby(standby) => standby.unit_started(&Unit::of(ctx), ctx),
    }
}

/// 一次拉流任务录完（`common/download.rs`，场次收尾之后）
pub fn unit_ended(ctx: &Context, output: UnitOutput) {
    let Some(role) = role() else {
        return;
    };
    match role {
        Role::Primary(primary) => primary.unit_ended(&Unit::of(ctx), output),
        Role::Standby(standby) => standby.unit_ended(&Unit::of(ctx), output),
    }
}

/// 场次收尾后要不要跑自动切片（`common/download.rs`）：自动切片只在主机跑（§5.2），备机镜像过来的房间不跑
pub fn auto_clip_allowed(ctx: &Context) -> bool {
    match role() {
        Some(Role::Standby(standby)) => !standby.mirrors(&ctx.live_streamer().url),
        Some(Role::Primary(_)) | None => true,
    }
}

/// 投稿流程开始时（`common/upload.rs`）：配对里的房间走 [`upload::Plan`]，其余返回 `None` 照常投
pub async fn upload_plan(ctx: &Context) -> Option<upload::Plan> {
    match role()? {
        Role::Primary(primary) => primary.plan(&Unit::of(ctx)),
        Role::Standby(standby) => standby.plan(&Unit::of(ctx)),
    }
}
