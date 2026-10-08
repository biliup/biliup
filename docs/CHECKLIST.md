# 画面遮挡功能验证清单

复核日期：2026-10-09。本清单取代初次实施时的完成度估计与未完成项判断。实现证据见 [IMPLEMENTATION_COMPLETE.md](IMPLEMENTATION_COMPLETE.md)，功能范围见 [MOSAIC_FEATURE.md](MOSAIC_FEATURE.md)。

## 本次已核实

- [x] 前端开关与区域采用受控配置，Semi Form 保存对象，路径为 `override.mosaic_config`。
- [x] JSON 与编辑器同步，反向拖拽和越界处理正确；区域数量、坐标、颜色与强度有校验。
- [x] 后端使用有效全局/主播配置；启用但为空、格式错误或处理失败时不上传原片。
- [x] 未遮挡标记从录制文件命名保留到处理完成；自动及手动上传入口拒绝带标记原片。
- [x] Noop / 无投稿模板的完成分段仍经过遮挡流程。
- [x] 实际 FFmpeg 混合效果与奇数坐标覆盖测试通过，损坏输入保持隔离。
- [x] 转码后更新工作台分段、文件列表及索引，等待录制写入结束。
- [x] 工作台快速/精确切片拒绝带未遮挡标记的待处理来源。
- [x] sync-downloader 与遮挡组合被拒绝；FFmpeg 调用有超时和并发限制。
- [x] 后端相关范围测试、编译检查，以及前端回归测试、TypeScript 和生产构建已执行。

## 仍需结合部署环境验收

- [ ] 使用实际直播间录制完整分段，等待处理完成并目视检查正式输出。
- [ ] 使用该机器配置的 FFmpeg 测试目标容器、实际音频编码与磁盘容量。
- [ ] 验证真实投稿、上传失败后重试以及 Noop 后处理符合自己的工作流。
- [ ] 在实际 Fleet HA 部署中验证接管、重启、分段路径和文件恢复。
- [ ] 进行多房间、长时间运行与积压测试，确定 CPU、磁盘和队列容量。
- [ ] 在需要支持的 Windows、macOS 和 Linux 环境分别验收。

## 不属于当前功能的保证

直播预览、直连和录制中的原始文件不会被实时遮挡。马赛克/模糊不保证无法辨认。GPU 加速、进度展示、真实效果预览及历史录像追溯处理尚未实现。旧文档中的性能表和时间估计不能作为容量或隐私保证。

测试入口：

```bash
cargo test -p biliup-cli --lib mosaic
node --test app/lib/mosaic-config.test.cjs
node_modules/.bin/tsc --noEmit --incremental false
npm run build
```

不要同时运行 TypeScript 检查与 Next 生产构建：构建会重新生成 `.next/types`，可能造成短暂的生成文件缺失。
