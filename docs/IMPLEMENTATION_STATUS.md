# 画面遮挡功能当前状态

更新日期：2026-10-09。以本次源码审查与修复为准，原来的完成度百分比、旧日期与实施时间估计已撤下。详细证据见 [IMPLEMENTATION_COMPLETE.md](IMPLEMENTATION_COMPLETE.md)，操作说明见 [MOSAIC_FEATURE.md](MOSAIC_FEATURE.md)。

| 模块 | 当前行为 |
| --- | --- |
| 配置 | 读取全局及主播 `mosaic_config`，前端对象保存到 `override.mosaic_config` |
| 编辑器 | 支持归一化矩形、正反方向拖拽、触屏、删除、效果/强度/纯色颜色与保存校验 |
| 滤镜 | 马赛克、模糊、纯色，多区域按顺序处理并覆盖像素边界 |
| 文件保护 | 未遮挡标记贯穿落盘与处理失败路径，上传入口拒绝带标记原片 |
| 上传流程 | 关闭分段先遮挡，再执行原有 segment_processor 和上传；Noop 也处理 |
| 工作台 | 分段处理完成后更新路径和索引，避免复用转码前字节偏移 |
| 资源控制 | FFmpeg 转码单任务执行，探测与转码有超时 |
| 边录边传 | 与 sync-downloader 不兼容，启用遮挡时拒绝该路径 |

实时预览、处理中与录制中的原片仍可能包含未遮挡画面。功能不会自动处理历史录像；不提供 GPU 编码、进度条或真实效果预览。

本地配置、滤镜、真实 FFmpeg、小范围上传/工作台/Fleet 回归与前端构建已检查。跨平台、真实房间长时间运行、真实投稿和并发容量验证仍应按 [CHECKLIST.md](CHECKLIST.md) 执行，不能沿用原报告的「已达到生产可用」结论。

斗鱼 Cookie 自动续期的独立说明见 [DOUYU_AUTO_REFRESH.md](DOUYU_AUTO_REFRESH.md)。
