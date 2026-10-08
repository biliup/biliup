# 画面遮挡功能快速参考

更新日期：2026-10-09。以当前审查后的实现为准。完整说明见 [MOSAIC_FEATURE.md](MOSAIC_FEATURE.md)，修复与验证记录见 [IMPLEMENTATION_COMPLETE.md](IMPLEMENTATION_COMPLETE.md)。

## 操作

直播间 → 配置覆写 → 画面遮挡（实验性） → 开启 → 拖拽区域 → 选择效果、强度/颜色 → 保存。

| 项目 | 当前值/范围 |
| --- | --- |
| 存储位置 | 主播 `override.mosaic_config`；无覆写时跟随全局配置 |
| 区域字段 | `id`、`x`、`y`、`width`、`height`、`effectType`、`strength`、可选 `color` |
| 坐标 | 归一化 0–1，宽高至少 0.01，区域不得超出画面 |
| 区域数量 | 最多 32 |
| 马赛克强度 | 4–64 整数 |
| 模糊强度 | 1–100 整数 |
| 纯色 | `#RRGGBB`，默认黑色 |
| 依赖 | 含 libx264 的 FFmpeg |
| 时机 | 磁盘分段关闭后处理，之后上传；Noop 也处理 |
| 失败 | 原片保持未遮挡标记并跳过上传 |
| 原片标记 | `name.unmasked.flv(.part)`；兼容 `name.flv.unmasked` |
| 并发 | 本进程一次转码一个分段 |
| 不兼容 | sync-downloader 边录边传 |

**实时预览、录制中的文件、处理中和处理失败的原片仍可能含未遮挡内容。** 编辑画布只是 16:9 示意图，马赛克/模糊也不保证无法辨认。检查完成后的正式输出才可确认实际效果。

## 关键代码

- 配置和验证：`crates/biliup-cli/src/server/plugins/mosaic/config.rs`。
- 滤镜和 FFmpeg 处理：`plugins/mosaic/ffmpeg_filter.rs`、`processor.rs`。
- 分段与上传保护：`server/common/download.rs`、`server/common/upload.rs`。
- 前端：`app/ui/MosaicEditor.tsx`、`MosaicPanel.tsx`、`OverrideModal.tsx`。
- 前端校验与回归测试：`app/lib/mosaic-config.ts`、`mosaic-config.test.cjs`。

验证命令和部署验收项见 [CHECKLIST.md](CHECKLIST.md)。斗鱼自动续期见 [DOUYU_AUTO_REFRESH.md](DOUYU_AUTO_REFRESH.md)。
