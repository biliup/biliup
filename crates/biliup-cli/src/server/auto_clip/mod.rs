//! 自动切片（实验）：用 OpenAI 兼容的 chat 与转写接口，从录像里挑出候选片段。
//!
//! 按场次的后台任务：抽音频、静音跳过、转写，再算弹幕密度、采关键帧缩图，分窗让 chat
//! 模型提出候选区间，校验后落库成待处理的候选；没配置 `auto_clip` 时什么都不跑。

pub mod analyze;
pub mod audio;
pub mod candidates;
pub mod cleanup;
pub mod danmaku;
#[cfg(test)]
pub(crate) mod fake;
pub mod files;
pub mod jobs;
pub mod model;
pub mod probe;
pub mod prompt;
pub mod runner;
pub mod settings;
pub mod suggestions;
pub mod thumbs;
