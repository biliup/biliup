//! 不依赖 HTTP 的业务操作：处理函数与之后的其他入口共用同一份实现。

/// 保存并应用全局配置
pub mod configuration;
/// 主播的增删改与暂停
pub mod streamers;

/// 斗鱼 Web Cookie（acf_auth）自动续期服务
pub mod douyu_keeper;
