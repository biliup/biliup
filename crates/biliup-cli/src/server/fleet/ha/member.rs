//! 配对两台各自的同步端（ha-pair 方案 H2）：账本与待发队列（[`super::outbox`]）、认出本机的改动、
//! 落地对端的改动。
//!
//! 控制面进程与配对节点各有一个 [`Member`]，只在配对生效时存在；没有配对时这里什么都不做，
//! 也不建任何文件。本机的改动有三个来源：空间配置保存（[`super::config_changed`]）、HTTP 接口上的增删改
//! （[`super::capture`]）与每隔几秒一次的扫描（凭据文件被投稿时的刷新改写）。
//!
//! 对端的改动按版本判断（[`Book::judge`]）：新的落地，旧的与重放的忽略；落地失败时本机这份重新盖一个
//! 更新的版本发回去，两台收敛到本机这份，原因随应答带给对端记日志。日志里只有键名，从不写值：
//! 配置里有 Cookie、密码，凭据文件更是整份密钥。

use super::Link;
use super::outbox::{self, PairFile, Queued};
use super::sync::{
    ACCOUNT, Book, CONFIG, PairAck, PairEdit, PairMessage, PairSecret, Rejected, Side, Stamp,
    Verdict, account_key, config_key, digest, digest_bytes,
};
use crate::server::config::Config;
use crate::server::fleet::layers::is_per_node;
use crate::server::fleet::now_ms;
use crate::server::infrastructure::repositories::{
    delete_bilibili_cookie, register_bilibili_cookie,
};
use crate::server::infrastructure::service_register::ServiceRegister;
use crate::server::services::configuration::{ApplyConfigError, apply_config};
use biliup::uploader::credential::{LoginInfo, save_login_info};
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

/// 扫一遍凭据文件与配置的间隔（投稿时刷新的凭据靠它发现）
const SCAN_INTERVAL: Duration = Duration::from_secs(5);

/// 不同步的配置键：按机器的键（[`crate::server::fleet::layers::PER_NODE_KEYS`]）与旧的配置文件主播表
pub fn synced_config_key(name: &str) -> bool {
    !is_per_node(name) && name != "streamers"
}

fn config_object(config: &Config) -> Map<String, Value> {
    match serde_json::to_value(config) {
        Ok(Value::Object(map)) => map,
        _ => Map::new(),
    }
}

fn config_value(object: &Map<String, Value>, name: &str) -> Value {
    object.get(name).cloned().unwrap_or(Value::Null)
}

/// 本机登记的一个账号：mid、登记的配置行与凭据文件路径
#[derive(Debug, Clone)]
struct Registered {
    id: i64,
    mid: u64,
    path: String,
}

async fn registered(services: &ServiceRegister) -> Vec<Registered> {
    let rows: Vec<(i64, String)> = sqlx::query_as(
        "SELECT id, value FROM configuration WHERE key = 'bilibili-cookies' ORDER BY id",
    )
    .fetch_all(&services.pool)
    .await
    .unwrap_or_default();
    let mut found: Vec<Registered> = Vec::new();
    for (id, path) in rows {
        let Ok(text) = tokio::fs::read_to_string(&path).await else {
            continue;
        };
        let Some((mid, _)) = crate::server::fleet::accounts::parse_credential(&text) else {
            continue;
        };
        if found.iter().any(|account| account.mid == mid) {
            continue;
        }
        found.push(Registered { id, mid, path });
    }
    found
}

/// 登记着的账号行；读不了库时为空（这一轮不认删除）
async fn account_rows(services: &ServiceRegister) -> Option<BTreeSet<i64>> {
    let rows: Vec<i64> =
        sqlx::query_scalar("SELECT id FROM configuration WHERE key = 'bilibili-cookies'")
            .fetch_all(&services.pool)
            .await
            .ok()?;
    Some(rows.into_iter().collect())
}

fn modified_ms(path: &Path) -> Option<i64> {
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    let since = modified.duration_since(std::time::UNIX_EPOCH).ok()?;
    i64::try_from(since.as_millis()).ok()
}

static ANY: AtomicBool = AtomicBool::new(false);
static MEMBERS: Mutex<Vec<(usize, Weak<Member>)>> = Mutex::new(Vec::new());

fn identity(services: &ServiceRegister) -> usize {
    Arc::as_ptr(&services.config) as *const () as usize
}

/// 这套服务（本进程的主库与配置）所在机器的同步端；没有配对时只读一次原子变量
pub fn member_for(services: &ServiceRegister) -> Option<Arc<Member>> {
    if !ANY.load(Ordering::Acquire) {
        return None;
    }
    let key = identity(services);
    MEMBERS
        .lock()
        .unwrap()
        .iter()
        .find(|(id, _)| *id == key)
        .and_then(|(_, member)| member.upgrade())
}

fn register(member: &Arc<Member>) {
    let key = identity(&member.services);
    let mut members = MEMBERS.lock().unwrap();
    members.retain(|(id, member)| *id != key && member.strong_count() > 0);
    members.push((key, Arc::downgrade(member)));
    ANY.store(true, Ordering::Release);
}

fn unregister(services: &ServiceRegister) {
    let key = identity(services);
    let mut members = MEMBERS.lock().unwrap();
    members.retain(|(id, member)| *id != key && member.strong_count() > 0);
    ANY.store(!members.is_empty(), Ordering::Release);
}

struct State {
    file: PairFile,
    /// 这条连接上已经发出的最大序号
    sent: u64,
}

/// 配对里一台机器的同步端
pub struct Member {
    side: Side,
    dir: PathBuf,
    path: PathBuf,
    services: ServiceRegister,
    state: tokio::sync::Mutex<State>,
    link: Mutex<Option<Link>>,
    /// 当前的主机（同一毫秒的两条修改谁赢）
    primary: Mutex<Side>,
    tasks: Mutex<Vec<JoinHandle<()>>>,
}

impl Member {
    /// 配对生效时（控制面：载入或指定配对；节点：第一次收到带 `pair` 的期望状态，或重启时有上次的文件）。
    /// `dir` 是 `data/`，`peer` 见 [`PairFile::peer`]
    pub async fn start(
        side: Side,
        dir: &Path,
        peer: &str,
        services: ServiceRegister,
        primary: Side,
    ) -> Arc<Self> {
        let path = outbox::path_in(dir);
        let previous = PairFile::load(&path, peer);
        let resumed = previous.is_some();
        let member = Arc::new(Member {
            side,
            dir: dir.to_path_buf(),
            path,
            services,
            state: tokio::sync::Mutex::new(State {
                file: previous.unwrap_or_else(|| PairFile::new(peer)),
                sent: 0,
            }),
            link: Mutex::default(),
            primary: Mutex::new(primary),
            tasks: Mutex::default(),
        });
        {
            let mut state = member.state.lock().await;
            if !resumed {
                member.seed(&mut state);
            }
            member.scan_locked(&mut state).await;
            member.persist(&state);
            info!(
                side = side.as_str(),
                resumed,
                queued = state.file.queue.len(),
                "配对同步：开始"
            );
        }
        register(&member);
        let task = tokio::spawn(watch(Arc::downgrade(&member)));
        member.tasks.lock().unwrap().push(task);
        member
    }

    pub fn side(&self) -> Side {
        self.side
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn set_primary(&self, primary: Side) {
        *self.primary.lock().unwrap() = primary;
    }

    pub fn primary(&self) -> Side {
        *self.primary.lock().unwrap()
    }

    /// 停下（进程退出、节点代理停止），文件留着，下次接着来
    pub fn stop(&self) {
        for task in self.tasks.lock().unwrap().drain(..) {
            task.abort();
        }
        unregister(&self.services);
    }

    /// 解除配对：停下并删掉 `pair-outbox.json`
    pub fn dissolve(&self) {
        self.stop();
        PairFile::forget(&self.path);
        info!("配对同步：配对已解除，同步账本已删除");
    }

    /// 第一次配对：配置按「主机赢」起步。控制面把每个键盖上此刻的版本排进队列；节点只记下摘要、
    /// 版本是 0，控制面的一到就收下。凭据文件两边都按文件的修改时间起步（[`Self::scan_locked`]），
    /// 两边都有同一个账号时新的那份赢
    fn seed(&self, state: &mut State) {
        let object = config_object(&self.services.config.read().unwrap());
        let now = now_ms();
        for (name, value) in &object {
            if !synced_config_key(name) {
                continue;
            }
            let key = config_key(name);
            let hash = Some(digest(value));
            match self.side {
                Side::Controller => {
                    state.file.book.write(&key, self.side, now, hash);
                    state.file.enqueue(&key);
                }
                Side::Node => state.file.book.put(
                    &key,
                    Stamp {
                        at: 0,
                        side: Side::Node,
                    },
                    hash,
                ),
            }
        }
    }

    fn persist(&self, state: &State) {
        if let Err(e) = state.file.save(&self.path) {
            warn!(error = ?e, "could not write {}", self.path.display());
        }
    }

    /// 连上对端：整个队列从头发一遍。同一条连接再调一次什么都不做
    pub async fn link_up(&self, link: Link) {
        if self
            .link
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|current| current.same(&link))
        {
            return;
        }
        let mut state = self.state.lock().await;
        *self.link.lock().unwrap() = Some(link);
        state.sent = 0;
        let pending = state.file.queue.len();
        if pending > 0 {
            info!(pending, "配对同步：连上对端，先发离线期间排下的修改");
        }
        self.flush_locked(&mut state).await;
    }

    pub fn link_down(&self) {
        if self.link.lock().unwrap().take().is_some() {
            debug!("配对同步：与对端断开，之后的修改先排进队列");
        }
    }

    pub fn linked(&self) -> bool {
        self.link.lock().unwrap().is_some()
    }

    fn current_link(&self) -> Option<Link> {
        self.link.lock().unwrap().clone()
    }

    /// 排着没发到对端（或对端没应答）的修改条数
    pub async fn pending(&self) -> usize {
        self.state.lock().await.file.queue.len()
    }

    /// 本机上可能有改动：扫一遍，有就排队、发出
    pub async fn scan(&self) {
        let mut state = self.state.lock().await;
        if self.scan_locked(&mut state).await {
            self.persist(&state);
            self.flush_locked(&mut state).await;
        }
    }

    async fn scan_locked(&self, state: &mut State) -> bool {
        let config = self.scan_config(state);
        let accounts = self.scan_accounts(state).await;
        config || accounts
    }

    fn scan_config(&self, state: &mut State) -> bool {
        let object = config_object(&self.services.config.read().unwrap());
        let mut names: BTreeSet<String> = object
            .keys()
            .filter(|name| synced_config_key(name))
            .cloned()
            .collect();
        names.extend(
            state
                .file
                .book
                .with_prefix(CONFIG)
                .map(|(key, _)| key[CONFIG.len()..].to_string()),
        );
        let now = now_ms();
        let mut changed = Vec::new();
        for name in names {
            let key = config_key(&name);
            let hash = digest(&config_value(&object, &name));
            if state.file.book.get(&key).and_then(|r| r.digest.as_deref()) == Some(hash.as_str()) {
                continue;
            }
            state.file.book.write(&key, self.side, now, Some(hash));
            state.file.enqueue(&key);
            changed.push(name);
        }
        if !changed.is_empty() {
            info!(keys = ?changed, "配对同步：本机改了配置");
        }
        !changed.is_empty()
    }

    /// 登录与刷新按凭据文件的内容认出；删除按登记行认出：记下的那一行没了、也没有别的行登记着同一个 mid。
    /// 文件一时读不到（行还在）不算删除
    async fn scan_accounts(&self, state: &mut State) -> bool {
        let accounts = registered(&self.services).await;
        let mut changed = Vec::new();
        for account in &accounts {
            let Ok(bytes) = tokio::fs::read(&account.path).await else {
                continue;
            };
            let key = account_key(account.mid);
            let hash = digest_bytes(&bytes);
            if state.file.book.get(&key).and_then(|r| r.digest.as_deref()) != Some(hash.as_str()) {
                let at = modified_ms(Path::new(&account.path)).unwrap_or_else(now_ms);
                state.file.book.write(&key, self.side, at, Some(hash));
                state.file.enqueue(&key);
                changed.push(account.mid);
            }
            if let Some(record) = state.file.book.get_mut(&key) {
                record.local = Some(account.id);
            }
        }
        if !changed.is_empty() {
            info!(mids = ?changed, "配对同步：本机的 B 站账号凭据有新的（登录或刷新）");
        }
        let Some(rows) = account_rows(&self.services).await else {
            return !changed.is_empty();
        };
        let gone: Vec<(String, u64)> = state
            .file
            .book
            .with_prefix(ACCOUNT)
            .filter(|(_, record)| {
                !record.deleted() && record.local.is_some_and(|id| !rows.contains(&id))
            })
            .filter_map(|(key, _)| {
                let mid = key[ACCOUNT.len()..].parse::<u64>().ok()?;
                (!accounts.iter().any(|account| account.mid == mid)).then(|| (key.to_string(), mid))
            })
            .collect();
        for (key, mid) in &gone {
            state.file.book.write(key, self.side, now_ms(), None);
            state.file.enqueue(key);
            info!(mid, "配对同步：本机删掉了一个 B 站账号");
        }
        !changed.is_empty() || !gone.is_empty()
    }

    async fn flush_locked(&self, state: &mut State) {
        let Some(link) = self.current_link() else {
            return;
        };
        let pending: Vec<Queued> = state.file.pending(state.sent).cloned().collect();
        let mut dropped = Vec::new();
        for queued in pending {
            match self.materialize(&state.file, &queued).await {
                Some(message) => {
                    if !link.pair(message) {
                        self.link_down();
                        return;
                    }
                }
                None => dropped.push(queued.seq),
            }
            state.sent = queued.seq;
        }
        if !dropped.is_empty() {
            state
                .file
                .queue
                .retain(|queued| !dropped.contains(&queued.seq));
            self.persist(state);
        }
    }

    /// 按本机此刻的内容组一条消息；本机已经没有这份内容（凭据文件没了）时不发
    async fn materialize(&self, file: &PairFile, queued: &Queued) -> Option<PairMessage> {
        let record = file.book.get(&queued.key)?;
        let stamp = record.stamp;
        if let Some(name) = queued.key.strip_prefix(CONFIG) {
            let object = config_object(&self.services.config.read().unwrap());
            return Some(PairMessage::Edit(PairEdit {
                seq: queued.seq,
                key: queued.key.clone(),
                stamp,
                value: Some(config_value(&object, name)),
            }));
        }
        if let Some(mid) = queued.key.strip_prefix(ACCOUNT) {
            let mid: u64 = mid.parse().ok()?;
            let content = if record.deleted() {
                None
            } else {
                let account = registered(&self.services)
                    .await
                    .into_iter()
                    .find(|account| account.mid == mid)?;
                Some(tokio::fs::read_to_string(&account.path).await.ok()?)
            };
            return Some(PairMessage::Secret(PairSecret {
                seq: queued.seq,
                mid,
                stamp,
                content,
            }));
        }
        debug!(key = queued.key, "配对同步：不认识的键，不发");
        None
    }

    /// 对端发来的同步消息
    pub async fn receive(&self, message: PairMessage) {
        let mut state = self.state.lock().await;
        match message {
            PairMessage::Edit(edit) => {
                let seq = edit.seq;
                let rejected = self.apply_edit(&mut state, edit).await;
                self.reply(seq, rejected);
            }
            PairMessage::Secret(secret) => {
                let seq = secret.seq;
                let rejected = self.apply_secret(&mut state, secret).await;
                self.reply(seq, rejected);
            }
            PairMessage::Ack(ack) => {
                state.file.acked(ack.upto);
                for rejected in &ack.rejected {
                    warn!(
                        key = rejected.key,
                        reason = rejected.reason,
                        "配对同步：对端没有采用本机的修改，按对端的那份为准"
                    );
                }
            }
            other => debug!(op = other.op(), "配对同步：这条消息由配对角色处理"),
        }
        self.persist(&state);
        self.flush_locked(&mut state).await;
    }

    fn reply(&self, upto: u64, rejected: Option<Rejected>) {
        if let Some(link) = self.current_link() {
            link.pair(PairMessage::Ack(PairAck {
                upto,
                rejected: rejected.into_iter().collect(),
            }));
        }
    }

    /// 本机这份盖一个更新的版本、排进队列：对端的那份没能落地时，两台收敛到本机这份
    fn restamp(&self, state: &mut State, key: &str, hash: Option<String>) {
        state.file.book.write(key, self.side, now_ms(), hash);
        state.file.enqueue(key);
    }

    async fn apply_edit(&self, state: &mut State, edit: PairEdit) -> Option<Rejected> {
        let Some(name) = edit.key.strip_prefix(CONFIG).map(str::to_string) else {
            debug!(key = edit.key, "配对同步：这一版还不同步这种记录");
            return None;
        };
        if !synced_config_key(&name) {
            return Some(Rejected {
                key: edit.key,
                reason: "按机器的配置键不在两台之间同步".into(),
            });
        }
        match state
            .file
            .book
            .judge(&edit.key, &edit.stamp, self.primary())
        {
            Verdict::Same | Verdict::Keep => return None,
            Verdict::Take => {}
        }
        let value = edit.value.unwrap_or(Value::Null);
        let current = config_value(&config_object(&self.services.config.read().unwrap()), &name);
        if digest(&current) == digest(&value) {
            state
                .file
                .book
                .put(&edit.key, edit.stamp, Some(digest(&current)));
            return None;
        }
        match self.apply_config_value(&name, value).await {
            Ok(applied) => {
                state
                    .file
                    .book
                    .put(&edit.key, edit.stamp, Some(digest(&applied)));
                info!(key = name, "配对同步：按对端的修改更新了配置");
                None
            }
            Err(reason) => {
                warn!(
                    key = name,
                    reason, "配对同步：对端改的配置在本机不能生效，保留本机的"
                );
                self.restamp(state, &edit.key, Some(digest(&current)));
                Some(Rejected {
                    key: edit.key,
                    reason,
                })
            }
        }
    }

    /// 换掉配置里的一个键并让它生效，返回生效后这个键的值
    async fn apply_config_value(&self, name: &str, value: Value) -> Result<Value, String> {
        let mut object = config_object(&self.services.config.read().unwrap());
        object.insert(name.to_string(), value);
        let config: Config = serde_json::from_value(Value::Object(object))
            .map_err(|e| format!("配置的形状不对：{e}"))?;
        let applied = apply_config(
            &self.services.config,
            &self.services.pool,
            &self.services.managers,
            &self.services.log_handle,
            config,
        )
        .await
        .map_err(|e| match e {
            ApplyConfigError::Invalid(reason) => reason,
            ApplyConfigError::Internal(report) => format!("{report:?}"),
        })?;
        Ok(config_value(&config_object(&applied), name))
    }

    async fn apply_secret(&self, state: &mut State, secret: PairSecret) -> Option<Rejected> {
        let key = account_key(secret.mid);
        match state.file.book.judge(&key, &secret.stamp, self.primary()) {
            Verdict::Same | Verdict::Keep => return None,
            Verdict::Take => {}
        }
        let result = match secret.content {
            Some(content) => self
                .write_secret(secret.mid, &content)
                .await
                .map(|(hash, id)| (Some(hash), Some(id))),
            None => self.remove_account(secret.mid).await.map(|()| (None, None)),
        };
        match result {
            Ok((hash, id)) => {
                state.file.book.put(&key, secret.stamp, hash);
                if let (Some(id), Some(record)) = (id, state.file.book.get_mut(&key)) {
                    record.local = Some(id);
                }
                None
            }
            Err(reason) => {
                warn!(
                    mid = secret.mid,
                    reason, "配对同步：对端的账号凭据没能在本机落地"
                );
                let current = self.current_secret_digest(secret.mid).await;
                self.restamp(state, &key, current);
                Some(Rejected { key, reason })
            }
        }
    }

    async fn current_secret_digest(&self, mid: u64) -> Option<String> {
        let account = registered(&self.services)
            .await
            .into_iter()
            .find(|account| account.mid == mid)?;
        let bytes = tokio::fs::read(&account.path).await.ok()?;
        Some(digest_bytes(&bytes))
    }

    /// 写凭据文件（与扫码登录同一个写法：`save_login_info`，仅本人可读、原子替换），然后登记。
    /// 本机已经登记过这个账号时写回原来的文件，否则写 `data/<mid>.json`
    async fn write_secret(&self, mid: u64, content: &str) -> Result<(String, i64), String> {
        let info: LoginInfo =
            serde_json::from_str(content).map_err(|_| "不是有效的 B 站凭据文件".to_string())?;
        if info.token_info.mid != mid {
            return Err("凭据文件里的 mid 与消息不符".into());
        }
        let path = match registered(&self.services)
            .await
            .into_iter()
            .find(|account| account.mid == mid)
        {
            Some(account) => PathBuf::from(account.path),
            None => self.dir.join(format!("{mid}.json")),
        };
        save_login_info(&path, &info)
            .await
            .map_err(|e| format!("写不了凭据文件：{e}"))?;
        let row = register_bilibili_cookie(&self.services.pool, &path)
            .await
            .map_err(|e| format!("登记不了凭据文件：{e:?}"))?
            .ok_or_else(|| "登记不了凭据文件：写完读不到".to_string())?;
        let bytes = tokio::fs::read(&path)
            .await
            .map_err(|e| format!("读不回凭据文件：{e}"))?;
        info!(mid, file = %path.display(), "配对同步：收到对端的 B 站账号凭据，已写入本机并登记");
        Ok((digest_bytes(&bytes), row.id))
    }

    async fn remove_account(&self, mid: u64) -> Result<(), String> {
        let Some(account) = registered(&self.services)
            .await
            .into_iter()
            .find(|account| account.mid == mid)
        else {
            return Ok(());
        };
        let deleted = delete_bilibili_cookie(&self.services.pool, account.id)
            .await
            .map_err(|e| format!("删不掉这个账号：{e:?}"))?;
        info!(
            mid,
            file_deleted = deleted.file_deleted,
            "配对同步：对端删掉了一个 B 站账号，本机跟着删"
        );
        Ok(())
    }

    /// 测试与报告用：本机账本
    pub async fn book(&self) -> Book {
        self.state.lock().await.file.book.clone()
    }
}

async fn watch(member: Weak<Member>) {
    let mut changes = super::config_changes();
    let mut ticker = tokio::time::interval(SCAN_INTERVAL);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            changed = changes.changed() => {
                if changed.is_err() {
                    return;
                }
            }
            _ = ticker.tick() => {}
        }
        let Some(member) = member.upgrade() else {
            return;
        };
        member.scan().await;
    }
}

impl std::fmt::Debug for Member {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Member")
            .field("side", &self.side)
            .field("path", &self.path)
            .finish()
    }
}
#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::server::core::download_manager::DownloadManager;
    use crate::server::fleet::protocol::{ControllerMessage, NodeMessage};
    use crate::server::infrastructure::connection_pool::ConnectionManager;
    use std::sync::RwLock;
    use tokio::sync::mpsc;
    use tracing_subscriber::{EnvFilter, reload};

    pub(crate) async fn services(dir: &Path) -> ServiceRegister {
        std::fs::create_dir_all(dir).unwrap();
        let pool = ConnectionManager::new_pool(dir.join("data.sqlite3").to_str().unwrap())
            .await
            .unwrap();
        let config = Config::default();
        let managers = DownloadManager::new(config.pool1_size, config.pool2_size, pool.clone());
        let (_layer, log_handle) = reload::Layer::new(EnvFilter::new("info"));
        ServiceRegister::new(pool, Arc::new(RwLock::new(config)), managers, log_handle).await
    }

    /// 伪造的凭据文件：形状与扫码登录写出的一样，值全是占位串（`tag` 区分版本）
    pub(crate) fn credential(mid: u64, tag: &str) -> String {
        serde_json::json!({
            "cookie_info": { "cookies": [{ "name": "SESSDATA", "value": format!("PLACEHOLDER-SESSDATA-{tag}") }] },
            "sso": [],
            "token_info": {
                "access_token": format!("PLACEHOLDER-ACCESS-{tag}"),
                "expires_in": 1,
                "mid": mid,
                "refresh_token": format!("PLACEHOLDER-REFRESH-{tag}"),
            },
            "platform": "BiliTV",
        })
        .to_string()
    }

    pub(crate) async fn login(services: &ServiceRegister, path: &Path, mid: u64, tag: &str) {
        std::fs::write(path, credential(mid, tag)).unwrap();
        register_bilibili_cookie(&services.pool, path)
            .await
            .unwrap()
            .unwrap();
    }

    async fn set_config(services: &ServiceRegister, edit: impl FnOnce(&mut Config)) {
        let mut config = services.config.read().unwrap().clone();
        edit(&mut config);
        apply_config(
            &services.config,
            &services.pool,
            &services.managers,
            &services.log_handle,
            config,
        )
        .await
        .unwrap();
    }

    fn segment_time(services: &ServiceRegister) -> Option<String> {
        services.config.read().unwrap().segment_time.clone()
    }

    pub(crate) async fn eventually<F, Fut>(what: &str, mut check: F)
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = bool>,
    {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
        while !check().await {
            assert!(tokio::time::Instant::now() < deadline, "timed out: {what}");
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }

    struct Side2 {
        dir: tempfile::TempDir,
        services: ServiceRegister,
    }

    async fn machine() -> Side2 {
        let dir = tempfile::tempdir().unwrap();
        let services = services(dir.path()).await;
        Side2 { dir, services }
    }

    /// 两台之间的一条连接：控制面发的帧交给节点的同步端，反过来也一样。`drop_acks` 为真时丢掉
    /// 控制面发出的应答（模拟应答在断线时丢了）
    struct Wire {
        tasks: Vec<JoinHandle<()>>,
    }

    impl Wire {
        async fn connect(c: &Arc<Member>, n: &Arc<Member>, drop_acks: bool) -> Wire {
            let (to_node, mut at_node) = mpsc::unbounded_channel::<ControllerMessage>();
            let (to_controller, mut at_controller) = mpsc::unbounded_channel::<NodeMessage>();
            let node = n.clone();
            let controller = c.clone();
            let tasks = vec![
                tokio::spawn(async move {
                    while let Some(frame) = at_node.recv().await {
                        match frame {
                            ControllerMessage::Pair(PairMessage::Ack(_)) if drop_acks => {}
                            ControllerMessage::Pair(message) => node.receive(message).await,
                            _ => {}
                        }
                    }
                }),
                tokio::spawn(async move {
                    while let Some(frame) = at_controller.recv().await {
                        if let NodeMessage::Pair(message) = frame {
                            controller.receive(message).await;
                        }
                    }
                }),
            ];
            c.link_up(Link::Controller(to_node)).await;
            n.link_up(Link::Node(to_controller)).await;
            Wire { tasks }
        }

        fn cut(self, c: &Member, n: &Member) {
            c.link_down();
            n.link_down();
            for task in self.tasks {
                task.abort();
            }
        }
    }

    async fn start(side: Side, machine: &Side2) -> Arc<Member> {
        Member::start(
            side,
            machine.dir.path(),
            "peer",
            machine.services.clone(),
            Side::Controller,
        )
        .await
    }

    /// 第一次配对时配置按主机（控制面）的起步，按机器的键不动；之后哪边改都同步到另一边；
    /// 两边离线各改同一个键，重连后后改的赢；落地失败的发回本机那份
    #[tokio::test]
    async fn config_starts_from_the_primary_then_syncs_both_ways_last_writer_wins() {
        let c = machine().await;
        let n = machine().await;
        set_config(&c.services, |config| {
            config.segment_time = Some("01:00:00".into());
            config.pool1_size = 3;
        })
        .await;
        set_config(&n.services, |config| {
            config.segment_time = Some("02:00:00".into());
            config.pool1_size = 7;
            config.filename_prefix = Some("node-only".into());
        })
        .await;
        let cm = start(Side::Controller, &c).await;
        let nm = start(Side::Node, &n).await;
        assert!(cm.pending().await > 10, "控制面把所有同步键排进队列");
        assert_eq!(nm.pending().await, 0, "节点起步不发");
        let wire = Wire::connect(&cm, &nm, false).await;
        eventually("节点收到控制面的配置", || async {
            segment_time(&n.services).as_deref() == Some("01:00:00")
        })
        .await;
        eventually("控制面的队列清空", || async {
            cm.pending().await == 0
        })
        .await;
        assert_eq!(
            n.services.config.read().unwrap().pool1_size,
            7,
            "按机器的键不同步"
        );
        assert_eq!(n.services.config.read().unwrap().filename_prefix, None);

        // 节点上改，控制面跟着变
        set_config(&n.services, |config| {
            config.filename_prefix = Some("N".into())
        })
        .await;
        nm.scan().await;
        eventually("控制面收到节点的修改", || async {
            c.services.config.read().unwrap().filename_prefix.as_deref() == Some("N")
        })
        .await;

        // 断开后两边各改同一个键：控制面先改，节点后改；重连后都是节点的
        wire.cut(&cm, &nm);
        set_config(&c.services, |config| {
            config.segment_time = Some("03:00:00".into())
        })
        .await;
        cm.scan().await;
        tokio::time::sleep(Duration::from_millis(5)).await;
        set_config(&n.services, |config| {
            config.segment_time = Some("04:00:00".into())
        })
        .await;
        nm.scan().await;
        assert_eq!(cm.pending().await, 1);
        assert_eq!(nm.pending().await, 1);
        let wire = Wire::connect(&cm, &nm, false).await;
        eventually("两边收敛到后改的那份", || async {
            segment_time(&c.services).as_deref() == Some("04:00:00")
                && segment_time(&n.services).as_deref() == Some("04:00:00")
        })
        .await;
        eventually("两边的队列清空", || async {
            cm.pending().await == 0 && nm.pending().await == 0
        })
        .await;
        let key = config_key("segment_time");
        assert_eq!(
            cm.book().await.get(&key).unwrap().stamp,
            nm.book().await.get(&key).unwrap().stamp
        );

        // 同一毫秒、不同来源：主机（控制面）赢。节点账本里是节点在 X 写的，控制面在同一个 X 写的到了就收下
        let recorded = nm.book().await.get(&key).unwrap().stamp;
        assert_eq!(recorded.side, Side::Node);
        nm.receive(PairMessage::Edit(PairEdit {
            seq: 99,
            key: key.clone(),
            stamp: Stamp {
                side: Side::Controller,
                ..recorded
            },
            value: Some(serde_json::json!("05:00:00")),
        }))
        .await;
        assert_eq!(segment_time(&n.services).as_deref(), Some("05:00:00"));

        // 对端的值在本机不合法：不生效，本机这份盖新版本发回去
        let before = n.services.config.read().unwrap().clone();
        nm.receive(PairMessage::Edit(PairEdit {
            seq: 101,
            key: config_key("filtering_threshold"),
            stamp: Stamp {
                at: now_ms() + 60_000,
                side: Side::Controller,
            },
            value: Some(serde_json::json!("not a number")),
        }))
        .await;
        assert_eq!(
            n.services.config.read().unwrap().filtering_threshold,
            before.filtering_threshold
        );
        eventually("本机那份发回控制面", || async {
            nm.pending().await == 0
        })
        .await;
        wire.cut(&cm, &nm);
        cm.stop();
        nm.stop();
    }

    /// 队列落在 `pair-outbox.json`：进程重启后还在；应答丢了、重连后整队重发，对端认出重放不重复生效
    #[tokio::test]
    async fn the_outbox_survives_restarts_and_replays_idempotently() {
        let c = machine().await;
        let n = machine().await;
        let cm = start(Side::Controller, &c).await;
        let nm = start(Side::Node, &n).await;
        let wire = Wire::connect(&cm, &nm, false).await;
        eventually("起步同步完", || async { cm.pending().await == 0 }).await;
        wire.cut(&cm, &nm);

        set_config(&n.services, |config| {
            config.segment_time = Some("00:10:00".into())
        })
        .await;
        nm.scan().await;
        assert_eq!(nm.pending().await, 1);
        let path = outbox::path_in(n.dir.path());
        let saved = std::fs::read_to_string(&path).unwrap();
        assert!(saved.contains("config/segment_time"));
        assert!(
            !saved.contains("00:10:00"),
            "队列里只有键，内容发的时候现取"
        );
        // 节点进程重启：队列还在
        nm.stop();
        drop(nm);
        let nm = start(Side::Node, &n).await;
        assert_eq!(nm.pending().await, 1);

        // 第一次连上：控制面收下了，但应答丢了
        let wire = Wire::connect(&cm, &nm, true).await;
        eventually("控制面收到", || async {
            segment_time(&c.services).as_deref() == Some("00:10:00")
        })
        .await;
        let applied = cm.book().await.get(&config_key("segment_time")).cloned();
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(nm.pending().await, 1, "没收到应答就留在队列里");
        wire.cut(&cm, &nm);
        // 重连：整队重发，控制面认出是同一个版本，只回应答
        let wire = Wire::connect(&cm, &nm, false).await;
        eventually("重放之后队列清空", || async {
            nm.pending().await == 0
        })
        .await;
        assert_eq!(
            cm.book().await.get(&config_key("segment_time")).cloned(),
            applied
        );
        wire.cut(&cm, &nm);
        cm.dissolve();
        nm.dissolve();
        assert!(!path.exists() && !outbox::path_in(c.dir.path()).exists());
        assert!(member_for(&c.services).is_none());
    }

    #[derive(Clone, Default)]
    struct Captured(Arc<Mutex<Vec<u8>>>);

    impl std::io::Write for Captured {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for Captured {
        type Writer = Captured;
        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    /// 一边登录（伪造的凭据文件），另一边写进本机、权限只有自己、登记成账号；刷新冲突按文件时间后者赢；
    /// 删除跟着删。凭据内容不出现在日志（trace 级别全开）、`pair-outbox.json` 与 `Debug` 输出里
    #[tokio::test]
    async fn credentials_sync_to_the_peer_and_never_leak() {
        let captured = Captured::default();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(captured.clone())
            .with_max_level(tracing::Level::TRACE)
            .with_ansi(false)
            .finish();
        let _logs = tracing::subscriber::set_default(subscriber);

        let c = machine().await;
        let n = machine().await;
        let cm = start(Side::Controller, &c).await;
        let nm = start(Side::Node, &n).await;
        let wire = Wire::connect(&cm, &nm, false).await;
        eventually("起步同步完", || async { cm.pending().await == 0 }).await;

        let login_file = c.dir.path().join("4242.json");
        login(&c.services, &login_file, 4242, "c1").await;
        cm.scan().await;
        let received = n.dir.path().join("4242.json");
        eventually("节点收到凭据", || async {
            accounts_of(&n.services).await == [4242]
        })
        .await;
        let text = std::fs::read_to_string(&received).unwrap();
        assert!(text.contains("PLACEHOLDER-ACCESS-c1"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&received).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600, "与扫码登录一样只有自己能读");
        }
        eventually("节点账本记下", || async {
            nm.book().await.get(&account_key(4242)).is_some()
        })
        .await;

        // 刷新冲突：断线期间两边先后刷新同一个账号，重连后按文件时间后刷新的赢
        wire.cut(&cm, &nm);
        std::fs::write(&login_file, credential(4242, "c2")).unwrap();
        cm.scan().await;
        tokio::time::sleep(Duration::from_millis(20)).await;
        std::fs::write(&received, credential(4242, "n2")).unwrap();
        nm.scan().await;
        let wire = Wire::connect(&cm, &nm, false).await;
        eventually("两边都是后刷新的那份", || async {
            let a = std::fs::read_to_string(&login_file).unwrap();
            let b = std::fs::read_to_string(&received).unwrap();
            a.contains("PLACEHOLDER-ACCESS-n2") && b.contains("PLACEHOLDER-ACCESS-n2")
        })
        .await;

        // 凭据文件一时读不到不算删除
        let aside = received.with_extension("aside");
        std::fs::rename(&received, &aside).unwrap();
        nm.scan().await;
        assert!(!nm.book().await.get(&account_key(4242)).unwrap().deleted());
        std::fs::rename(&aside, &received).unwrap();

        // 删除跟着删
        let id = registered(&n.services).await[0].id;
        delete_bilibili_cookie(&n.services.pool, id).await.unwrap();
        nm.scan().await;
        assert!(nm.book().await.get(&account_key(4242)).unwrap().deleted());
        eventually("控制面跟着删", || async {
            accounts_of(&c.services).await.is_empty()
        })
        .await;
        eventually("队列清空", || async {
            cm.pending().await == 0 && nm.pending().await == 0
        })
        .await;

        let secret = PairMessage::Secret(PairSecret {
            seq: 1,
            mid: 1,
            stamp: Stamp {
                at: 1,
                side: Side::Node,
            },
            content: Some(credential(1, "debug")),
        });
        assert!(!format!("{secret:?}").contains("PLACEHOLDER"));
        for dir in [c.dir.path(), n.dir.path()] {
            let outbox = std::fs::read_to_string(outbox::path_in(dir)).unwrap();
            assert!(!outbox.contains("PLACEHOLDER"), "账本里只有摘要");
        }
        let logs = String::from_utf8(captured.0.lock().unwrap().clone()).unwrap();
        assert!(logs.contains("配对同步"), "日志确实被截获了");
        assert!(!logs.contains("PLACEHOLDER"), "日志里不能有凭据内容");
        wire.cut(&cm, &nm);
        cm.stop();
        nm.stop();
    }

    async fn accounts_of(services: &ServiceRegister) -> Vec<u64> {
        registered(services)
            .await
            .into_iter()
            .map(|account| account.mid)
            .collect()
    }
}
