//! Web 用户的固定三角色与权限点（#1717）。
//!
//! 角色只是权限点的集合；路由按权限点声明所需权限（见 [`required_permission`]），
//! 没有声明的路由只允许超管访问。以后要开放自定义勾选，只需让用户自带一组权限点，
//! 不用再动路由表。

use axum::http::Method;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Admin,
    Operator,
    Viewer,
}

impl Role {
    pub const ALL: [Role; 3] = [Role::Admin, Role::Operator, Role::Viewer];

    pub fn as_str(self) -> &'static str {
        match self {
            Role::Admin => "admin",
            Role::Operator => "operator",
            Role::Viewer => "viewer",
        }
    }

    pub fn permissions(self) -> &'static [Permission] {
        use Permission::*;
        match self {
            Role::Admin => Permission::ALL,
            Role::Operator => &[
                StreamerView,
                PreviewView,
                RecordingControl,
                StreamerEdit,
                UploadSubmit,
                TemplateEdit,
                ConfigView,
                LogView,
                FileView,
            ],
            Role::Viewer => &[StreamerView, PreviewView, ConfigView, LogView, FileView],
        }
    }

    pub fn has(self, permission: Permission) -> bool {
        self.permissions().contains(&permission)
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Role {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "admin" => Ok(Role::Admin),
            "operator" => Ok(Role::Operator),
            "viewer" => Ok(Role::Viewer),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    #[serde(rename = "streamer.view")]
    StreamerView,
    #[serde(rename = "preview.view")]
    PreviewView,
    #[serde(rename = "recording.control")]
    RecordingControl,
    #[serde(rename = "streamer.edit")]
    StreamerEdit,
    /// 主播与全局配置里的 `*processor` 钩子和 `override`：钩子走 `sh -c`，等于服务器 shell
    #[serde(rename = "streamer.hooks")]
    StreamerHooks,
    #[serde(rename = "upload.submit")]
    UploadSubmit,
    #[serde(rename = "template.edit")]
    TemplateEdit,
    #[serde(rename = "account.manage")]
    AccountManage,
    /// 非超管看到的是脱敏版，见 `redact`
    #[serde(rename = "config.view")]
    ConfigView,
    #[serde(rename = "config.edit")]
    ConfigEdit,
    #[serde(rename = "log.view")]
    LogView,
    #[serde(rename = "file.view")]
    FileView,
    #[serde(rename = "user.manage")]
    UserManage,
}

impl Permission {
    pub const ALL: &'static [Permission] = &[
        Permission::StreamerView,
        Permission::PreviewView,
        Permission::RecordingControl,
        Permission::StreamerEdit,
        Permission::StreamerHooks,
        Permission::UploadSubmit,
        Permission::TemplateEdit,
        Permission::AccountManage,
        Permission::ConfigView,
        Permission::ConfigEdit,
        Permission::LogView,
        Permission::FileView,
        Permission::UserManage,
    ];
}

/// 路由（axum 的路径模板）→ 所需权限点。
///
/// 返回 `None` 表示没有声明，只允许超管访问（默认拒绝）。新增路由必须在这里登记，
/// `every_registered_route_is_classified` 测试会扫描 `router.rs` / `app.rs` 保证这一点。
pub fn required_permission(method: &Method, route: &str, raw_path: &str) -> Option<Permission> {
    use Permission::*;
    let get = method == Method::GET || method == Method::HEAD;
    let permission = match route {
        "/v1/streamers" if get => StreamerView,
        "/v1/streamers" if method == Method::POST || method == Method::PUT => StreamerEdit,
        "/v1/streamers/{id}" if method == Method::DELETE => StreamerEdit,
        "/v1/streamers/{id}/pause" if method == Method::PUT => RecordingControl,
        "/v1/streamers/{id}/cover" | "/v1/streamers/{id}/avatar" if get => StreamerView,
        "/v1/streamers/{id}/live"
        | "/v1/streamers/{id}/live-url"
        | "/v1/streamers/{id}/danmaku"
        | "/v1/danmaku"
            if get =>
        {
            PreviewView
        }
        "/v1/ws/live-rates" | "/v1/live-rates" if get => StreamerView,
        "/v1/configuration" if get => ConfigView,
        "/v1/configuration" if method == Method::PUT => ConfigEdit,
        "/v1/streamer-info" | "/v1/streamer-info/files/{id}" if get => StreamerView,
        "/v1/upload/streamers" | "/v1/upload/streamers/{id}" if get => StreamerView,
        "/v1/upload/streamers" if method == Method::POST => TemplateEdit,
        "/v1/upload/streamers/{id}" if method == Method::DELETE => TemplateEdit,
        // 编辑投稿模板时要从已登记的 B 站账号里选一个，所以列表与资料归模板编辑；
        // 新增、删除账号与扫码登录归账号管理。
        "/v1/users" | "/v1/users/{id}" if get => TemplateEdit,
        "/v1/users" if method == Method::POST => AccountManage,
        "/v1/users/{id}" if method == Method::DELETE => AccountManage,
        "/v1/users/{id}/archives" if get => AccountManage,
        "/v1/get_qrcode" if get => AccountManage,
        "/v1/login_by_qrcode" if method == Method::POST => AccountManage,
        "/bili/archive/pre" if get => UploadSubmit,
        "/v1/uploads" if method == Method::POST => UploadSubmit,
        "/v1/videos" if get => FileView,
        "/v1/status" if get => StreamerView,
        // 三个角色都有 StreamerView，等于登录即可
        "/v1/tools" if get => StreamerView,
        "/v1/ws/logs" if get => LogView,
        "/static/{path}" if get => {
            let name = raw_path.rsplit('/').next().unwrap_or_default();
            if crate::server::router::ALLOWED_LOG_FILES.contains(&name) {
                LogView
            } else {
                FileView
            }
        }
        "/v1/web-users" | "/v1/web-users/{id}" | "/v1/web-users/{id}/logout-all" => UserManage,
        _ => return None,
    };
    Some(permission)
}

/// 该角色能否访问这条路由。超管不受路由表限制。
pub fn route_allowed(role: Role, method: &Method, route: &str, raw_path: &str) -> bool {
    if role == Role::Admin {
        return true;
    }
    required_permission(method, route, raw_path).is_some_and(|permission| role.has(permission))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `.route("<path>"` 里的路径字面量，不管中间有没有换行。
    fn registered_routes(source: &str) -> Vec<String> {
        let mut routes = Vec::new();
        let mut rest = source;
        while let Some(index) = rest.find(".route(") {
            rest = &rest[index + ".route(".len()..];
            let Some(open) = rest.find('"') else { break };
            let after = &rest[open + 1..];
            let Some(close) = after.find('"') else { break };
            routes.push(after[..close].to_string());
            rest = &after[close + 1..];
        }
        routes
    }

    fn non_test_source(source: &str) -> &str {
        source.split("#[cfg(test)]").next().unwrap()
    }

    #[test]
    fn every_registered_route_is_classified() {
        let sources = [
            include_str!("../router.rs"),
            include_str!("../app.rs"),
            include_str!("../api/web_users.rs"),
        ];
        // 这些路由有意不挂权限层：登录相关对外公开，`/v1/me*` 只要求登录。
        let unguarded = [
            "/v1/users/login",
            "/v1/users/register",
            "/v1/users/biliup",
            "/v1/logout",
            "/v1/me",
            "/v1/me/password",
        ];
        let methods = [Method::GET, Method::POST, Method::PUT, Method::DELETE];
        let mut seen = 0;
        for source in sources {
            for route in registered_routes(non_test_source(source)) {
                seen += 1;
                if unguarded.contains(&route.as_str()) {
                    continue;
                }
                assert!(
                    methods
                        .iter()
                        .any(|method| required_permission(method, &route, &route).is_some()),
                    "路由 {route} 没有登记权限点，非超管会被一律拒绝；请在 required_permission 里登记"
                );
            }
        }
        assert!(seen >= 30, "只扫到 {seen} 条路由，扫描逻辑可能失效");
    }

    #[test]
    fn roles_follow_the_approved_matrix() {
        use Permission::*;
        for permission in Permission::ALL {
            assert!(Role::Admin.has(*permission));
        }
        for permission in [StreamerHooks, AccountManage, ConfigEdit, UserManage] {
            assert!(!Role::Operator.has(permission), "{permission:?}");
            assert!(!Role::Viewer.has(permission), "{permission:?}");
        }
        for permission in [RecordingControl, StreamerEdit, UploadSubmit, TemplateEdit] {
            assert!(Role::Operator.has(permission), "{permission:?}");
            assert!(!Role::Viewer.has(permission), "{permission:?}");
        }
        for permission in [StreamerView, PreviewView, ConfigView, LogView, FileView] {
            assert!(Role::Operator.has(permission), "{permission:?}");
            assert!(Role::Viewer.has(permission), "{permission:?}");
        }
    }

    #[test]
    fn unknown_routes_and_methods_are_admin_only() {
        for role in [Role::Operator, Role::Viewer] {
            assert!(!route_allowed(
                role,
                &Method::GET,
                "/v1/new-feature",
                "/v1/new-feature"
            ));
            assert!(!route_allowed(
                role,
                &Method::PATCH,
                "/v1/streamers",
                "/v1/streamers"
            ));
            assert!(!route_allowed(
                role,
                &Method::DELETE,
                "/v1/status",
                "/v1/status"
            ));
        }
        assert!(route_allowed(
            Role::Admin,
            &Method::GET,
            "/v1/new-feature",
            "/v1/new-feature"
        ));
    }

    #[test]
    fn static_route_distinguishes_logs_from_recordings() {
        assert_eq!(
            required_permission(&Method::GET, "/static/{path}", "/static/ds_update.log"),
            Some(Permission::LogView)
        );
        assert_eq!(
            required_permission(&Method::GET, "/static/{path}", "/static/a.flv"),
            Some(Permission::FileView)
        );
    }

    #[test]
    fn permission_names_match_the_frontend_contract() {
        assert_eq!(
            serde_json::to_value(Permission::StreamerHooks).unwrap(),
            "streamer.hooks"
        );
        assert_eq!(serde_json::to_value(Role::Operator).unwrap(), "operator");
    }
}
