/// 数据库连接池管理
pub mod connection_pool;
/// 应用程序上下文和工作器
pub mod context;
/// 数据传输对象
pub mod dto;
/// 数据模型定义
pub mod models;
/// Web 用户的角色与权限点
pub mod permissions;
/// 授权决策点：按主体属性、动作与字段判断「能不能」
pub mod policy;
/// 数据仓库层
pub mod repositories;
/// 服务注册器
pub mod service_register;
/// 用户认证相关
pub mod users;
