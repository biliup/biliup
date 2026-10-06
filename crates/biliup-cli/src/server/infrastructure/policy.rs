//! 授权决策点：所有「能不能」都在这里回答。
//!
//! 决策只看属性：主体属性（[`Subject`]：角色，以及环境属性 `--auth` 是否开启）、
//! 动作（[`Permission`]）、路由（[`RouteRequirement`]）和字段（[`Field`]）。
//! 访问控制中间件、处理函数里的字段保护与脱敏、长连接复查、`/v1/me` 下发的权限点都只调这里，
//! 不再各自拿 `Role` 判断。以后加新的主体属性或资源属性（例如按主播分配），只改这一处。

use crate::server::infrastructure::permissions::{Permission, Role, required_permission};
use axum::http::Method;

/// 发起请求的主体。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Subject {
    /// `--auth` 关闭时没有登录用户，为 `None`
    pub user_id: Option<i64>,
    pub role: Role,
    /// 环境属性：`--auth` 是否开启。关闭时零鉴权、视为超管，但没有登录用户可管。
    pub auth_enabled: bool,
}

impl Subject {
    /// `--auth` 开启时的登录用户。
    pub fn user(id: i64, role: Role) -> Self {
        Subject {
            user_id: Some(id),
            role,
            auth_enabled: true,
        }
    }

    /// `--auth` 关闭时：零鉴权，视为超管。
    pub fn unrestricted() -> Self {
        Subject {
            user_id: None,
            role: Role::Admin,
            auth_enabled: false,
        }
    }

    pub fn can(&self, action: Permission) -> bool {
        match action {
            // 零鉴权模式下没有登录用户，也就没有用户可管
            Permission::UserManage if !self.auth_enabled => false,
            _ => self.role.has(action),
        }
    }

    /// 实际生效的全部权限点，`/v1/me` 原样下发给前端。
    pub fn permissions(&self) -> Vec<Permission> {
        Permission::ALL
            .iter()
            .copied()
            .filter(|permission| self.can(*permission))
            .collect()
    }

    pub fn satisfies(&self, requirement: RouteRequirement) -> bool {
        match requirement {
            RouteRequirement::Permission(permission) => self.can(permission),
            RouteRequirement::AdminOnly => self.role == Role::Admin,
        }
    }

    /// 字段的读写是同一条规则：看不到的字段，保存时也以库里原值为准（见各 [`Field`] 的说明）。
    pub fn can_access(&self, field: Field) -> bool {
        self.can(field.guard())
    }
}

/// 一条路由对主体的要求。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteRequirement {
    Permission(Permission),
    /// 路由表里没有登记：只允许超管（默认拒绝）
    AdminOnly,
}

impl RouteRequirement {
    pub fn of(method: &Method, route: &str, raw_path: &str) -> Self {
        required_permission(method, route, raw_path)
            .map_or(RouteRequirement::AdminOnly, RouteRequirement::Permission)
    }
}

/// 路由之内按字段区分的数据：有权限的人原样读写，没有的人读到的是脱敏版、写不进去。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    /// 主播与全局配置里的 `*processor` 钩子和 `override`：钩子走 `sh -c`，等于服务器 shell
    StreamerHooks,
    /// 全局配置里白名单之外的字段（凭据、路径形式的凭据、钩子），见 `redact::config`
    ConfigSecrets,
    /// 投稿模板绑定的 B 站账号（`user_cookie`）：没有权限的人只能从已登记的账号里选，状态接口里也看不到
    TemplateAccount,
    /// 投稿模板的封面路径：会被原样读出来上传，界面上不提供编辑，没有权限的人保存时保留原值
    TemplateCoverPath,
}

impl Field {
    fn guard(self) -> Permission {
        match self {
            Field::StreamerHooks => Permission::StreamerHooks,
            Field::ConfigSecrets => Permission::ConfigEdit,
            Field::TemplateAccount | Field::TemplateCoverPath => Permission::AccountManage,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIELDS: [Field; 4] = [
        Field::StreamerHooks,
        Field::ConfigSecrets,
        Field::TemplateAccount,
        Field::TemplateCoverPath,
    ];

    #[test]
    fn unknown_routes_and_methods_are_admin_only() {
        let requirement = |method: Method, path: &str| RouteRequirement::of(&method, path, path);
        for role in [Role::Operator, Role::Viewer] {
            let subject = Subject::user(1, role);
            assert!(!subject.satisfies(requirement(Method::GET, "/v1/new-feature")));
            assert!(!subject.satisfies(requirement(Method::PATCH, "/v1/streamers")));
            assert!(!subject.satisfies(requirement(Method::DELETE, "/v1/status")));
        }
        assert_eq!(
            requirement(Method::GET, "/v1/new-feature"),
            RouteRequirement::AdminOnly
        );
        assert!(Subject::user(1, Role::Admin).satisfies(RouteRequirement::AdminOnly));
    }

    #[test]
    fn user_management_needs_auth() {
        let unrestricted = Subject::unrestricted();
        assert!(!unrestricted.can(Permission::UserManage));
        assert!(!unrestricted.permissions().contains(&Permission::UserManage));
        // 其它权限点照旧全部拥有
        assert_eq!(unrestricted.permissions().len(), Permission::ALL.len() - 1);
        assert!(Subject::user(1, Role::Admin).can(Permission::UserManage));
    }

    #[test]
    fn permissions_list_matches_can() {
        for role in Role::ALL {
            for subject in [
                Subject::user(1, role),
                Subject {
                    auth_enabled: false,
                    ..Subject::user(1, role)
                },
            ] {
                let listed = subject.permissions();
                for permission in Permission::ALL {
                    assert_eq!(
                        listed.contains(permission),
                        subject.can(*permission),
                        "{subject:?} {permission:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn clip_editing_is_for_operators_and_admins() {
        assert!(Subject::user(1, Role::Admin).can(Permission::ClipEdit));
        assert!(Subject::unrestricted().can(Permission::ClipEdit));
        assert!(Subject::user(1, Role::Operator).can(Permission::ClipEdit));
        assert!(!Subject::user(1, Role::Viewer).can(Permission::ClipEdit));
    }

    #[test]
    fn protected_fields_follow_the_approved_matrix() {
        for field in FIELDS {
            assert!(Subject::user(1, Role::Admin).can_access(field), "{field:?}");
            assert!(Subject::unrestricted().can_access(field), "{field:?}");
            assert!(
                !Subject::user(1, Role::Operator).can_access(field),
                "{field:?}"
            );
            assert!(
                !Subject::user(1, Role::Viewer).can_access(field),
                "{field:?}"
            );
        }
    }
}
