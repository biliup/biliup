//! 按负载自动选节点（§10.2「动态负载均衡」倾向 A：控制面仲裁、不漂移）。
//!
//! 只在两种时候用：新建房间时选了「自动」，以及移除节点时选了「自动改派」。
//! 已经在录的房间不会因为负载变化被挪走（D9）。
//!
//! 先按硬约束筛：在线、版本够新、登记了模板要用的账号、带钩子的房间只给允许钩子的节点；
//! 再按软指标排：下载池空位（容量 − 占用）多的优先，其次分到的房间少的，再次录制目录剩余空间大的，
//! 都一样时取 id 小的，结果可复现。

/// 参与挑选的一台节点此刻的情况
#[derive(Debug, Clone, Default)]
pub struct Candidate {
    pub id: i64,
    pub name: String,
    pub online: bool,
    /// 协议版本太旧，收不了房间
    pub outdated: bool,
    pub allow_hooks: bool,
    pub accounts: Vec<u64>,
    /// 最近一次心跳里的下载池
    pub download_capacity: usize,
    pub download_occupied: usize,
    /// 控制面分派给它的房间数
    pub assigned_rooms: i64,
    /// 录制目录剩余空间（字节）；没有上报时按 0 算
    pub disk_available: Option<u64>,
}

/// 房间对节点的要求
#[derive(Debug, Clone, Copy, Default)]
pub struct Needs {
    pub hooks: bool,
    pub account: Option<u64>,
}

/// 一台节点为什么不能选
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rejected {
    Offline,
    Outdated,
    NoHooks,
    MissingAccount(u64),
}

impl Rejected {
    pub fn describe(&self) -> String {
        match self {
            Rejected::Offline => "离线".into(),
            Rejected::Outdated => "版本太旧".into(),
            Rejected::NoHooks => "不允许钩子".into(),
            Rejected::MissingAccount(mid) => format!("没有登记 B 站账号 {mid}"),
        }
    }
}

fn check(candidate: &Candidate, needs: Needs) -> Result<(), Rejected> {
    if !candidate.online {
        return Err(Rejected::Offline);
    }
    if candidate.outdated {
        return Err(Rejected::Outdated);
    }
    if needs.hooks && !candidate.allow_hooks {
        return Err(Rejected::NoHooks);
    }
    if let Some(mid) = needs.account
        && !candidate.accounts.contains(&mid)
    {
        return Err(Rejected::MissingAccount(mid));
    }
    Ok(())
}

fn free_slots(candidate: &Candidate) -> usize {
    candidate
        .download_capacity
        .saturating_sub(candidate.download_occupied)
}

/// 选一台节点；一台都不行时返回每台被排除的原因
pub fn choose(candidates: &[Candidate], needs: Needs) -> Result<i64, Vec<(String, Rejected)>> {
    let mut rejected = Vec::new();
    let mut eligible = Vec::new();
    for candidate in candidates {
        match check(candidate, needs) {
            Ok(()) => eligible.push(candidate),
            Err(reason) => rejected.push((candidate.name.clone(), reason)),
        }
    }
    eligible
        .into_iter()
        .min_by(|a, b| {
            free_slots(b)
                .cmp(&free_slots(a))
                .then(a.assigned_rooms.cmp(&b.assigned_rooms))
                .then(
                    b.disk_available
                        .unwrap_or(0)
                        .cmp(&a.disk_available.unwrap_or(0)),
                )
                .then(a.id.cmp(&b.id))
        })
        .map(|candidate| candidate.id)
        .ok_or(rejected)
}

/// 一台都选不出来时给界面看的说明
pub fn explain(rejected: &[(String, Rejected)]) -> String {
    if rejected.is_empty() {
        return "没有节点可选：还没有节点加入".into();
    }
    let reasons: Vec<String> = rejected
        .iter()
        .map(|(name, reason)| format!("「{name}」{}", reason.describe()))
        .collect();
    format!("没有节点满足条件：{}", reasons.join("；"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: i64, free: usize, assigned: i64, disk: u64) -> Candidate {
        Candidate {
            id,
            name: format!("n{id}"),
            online: true,
            allow_hooks: false,
            accounts: vec![1],
            download_capacity: 5,
            download_occupied: 5 - free,
            assigned_rooms: assigned,
            disk_available: Some(disk),
            ..Candidate::default()
        }
    }

    #[test]
    fn more_free_download_slots_win() {
        let nodes = [node(1, 1, 0, 900), node(2, 3, 4, 100), node(3, 2, 0, 900)];
        assert_eq!(choose(&nodes, Needs::default()), Ok(2));
    }

    #[test]
    fn a_full_node_is_still_a_last_resort() {
        let nodes = [node(1, 0, 0, 900)];
        assert_eq!(choose(&nodes, Needs::default()), Ok(1));
        let nodes = [node(1, 0, 0, 900), node(2, 1, 9, 1)];
        assert_eq!(choose(&nodes, Needs::default()), Ok(2));
    }

    #[test]
    fn ties_fall_back_to_fewer_rooms_then_more_disk_then_lower_id() {
        let nodes = [node(1, 2, 3, 900), node(2, 2, 1, 100)];
        assert_eq!(choose(&nodes, Needs::default()), Ok(2));
        let nodes = [node(1, 2, 1, 100), node(2, 2, 1, 900)];
        assert_eq!(choose(&nodes, Needs::default()), Ok(2));
        let nodes = [node(2, 2, 1, 500), node(1, 2, 1, 500)];
        assert_eq!(choose(&nodes, Needs::default()), Ok(1));
    }

    #[test]
    fn offline_and_outdated_nodes_are_skipped() {
        let mut offline = node(1, 5, 0, 900);
        offline.online = false;
        let mut outdated = node(2, 5, 0, 900);
        outdated.outdated = true;
        let nodes = [offline.clone(), outdated.clone(), node(3, 1, 3, 1)];
        assert_eq!(choose(&nodes, Needs::default()), Ok(3));
        let rejected = choose(&[offline, outdated], Needs::default()).unwrap_err();
        assert_eq!(
            rejected,
            [
                ("n1".to_string(), Rejected::Offline),
                ("n2".to_string(), Rejected::Outdated)
            ]
        );
        assert!(explain(&rejected).contains("「n1」离线"));
    }

    #[test]
    fn the_template_account_must_be_on_the_node() {
        let mut other = node(1, 5, 0, 900);
        other.accounts = vec![2];
        let needs = Needs {
            account: Some(1),
            ..Needs::default()
        };
        assert_eq!(choose(&[other.clone(), node(2, 1, 5, 1)], needs), Ok(2));
        assert_eq!(
            choose(&[other], needs).unwrap_err(),
            [("n1".to_string(), Rejected::MissingAccount(1))]
        );
    }

    #[test]
    fn hooked_rooms_only_go_to_nodes_that_allow_hooks() {
        let mut hooks = node(2, 1, 5, 1);
        hooks.allow_hooks = true;
        let needs = Needs {
            hooks: true,
            ..Needs::default()
        };
        assert_eq!(choose(&[node(1, 5, 0, 900), hooks], needs), Ok(2));
        assert_eq!(
            choose(&[node(1, 5, 0, 900)], needs).unwrap_err(),
            [("n1".to_string(), Rejected::NoHooks)]
        );
    }

    #[test]
    fn no_nodes_at_all() {
        let rejected = choose(&[], Needs::default()).unwrap_err();
        assert!(rejected.is_empty());
        assert!(explain(&rejected).contains("还没有节点"));
    }
}
