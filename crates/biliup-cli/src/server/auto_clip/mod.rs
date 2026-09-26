//! 自动切片（实验）：用 OpenAI 兼容的 chat 与转写接口，从录像里挑出候选片段。
//!
//! 目前只有模型配置、客户端与连通性测试；没配置 `auto_clip` 时什么都不跑。

pub mod audio;
#[cfg(test)]
pub(crate) mod fake;
pub mod files;
pub mod jobs;
pub mod model;
pub mod probe;
pub mod settings;
