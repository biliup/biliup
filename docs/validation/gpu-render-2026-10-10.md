# GPU 合成与任务列表验收（2026-10-10）

产品改动包括 H.264 GPU 编码、真实可用性检测、按设备配置、GPU 监测，以及跨场次压制任务页。默认自动优先 GPU，手动指定编码器失败时不回退其他编码器。HEVC / AV1 的下面结果是硬件能力补测；本次产品导出仍是 H.264 MP4。

## 实机环境与结果

| 环境 | 验收结果 |
| --- | --- |
| Intel UHD Graphics 730，`/dev/dri/renderD128` | H.264 VAAPI 和 QSV 的真实两分段合成通过 |
| Intel Arc A750，`/dev/dri/renderD129` | H.264 VAAPI / QSV 两分段合成通过；VAAPI 硬件解码通过 |
| Colab Tesla T4，驱动 580.82.07，FFmpeg 6.1.1 | H.264 NVENC 的 Rust 引擎和任务 API 合成通过；监测 API 持续负载验收通过 |
| AMD / Windows | 编码参数、配置校验和监测解析有自动测试；Windows 默认使用 WDDM 系统计数器，无对应实机，不能视为实机验收通过 |

H.264 实机合成使用不同尺寸的两个源分段，验证遮挡像素、统一画布/帧率、音频、流复制 MP4 拼接、完整解码、实际编码器/设备、有效进度/速度，以及原始文件哈希未变化。T4 的服务端测试还覆盖能力检测、数据库任务状态和监测 API。

A750 最初 VAAPI 可用而 QSV 报 `MFX session -9`。机器已有 `libvpl` 和 `intel-media-driver`，但缺少 `vpl-gpu-rt`。从 Arch 官方仓库安装版本 26.3.5-1 后，两张 Intel GPU 的 QSV 真实编码与完整合成均通过。该操作是本机环境修复，不属于仓库安装脚本。

T4 暴露了 NVENC 的最小画面限制：128×128 能力探测失败但正常尺寸编码成功。通用能力探测改为 640×360；任务仍按实际尺寸和帧率探测。NVIDIA 设备通过 `nvidia-smi` 序号发现并逐卡探测，能力结果包含具体适配器。

## HEVC / AV1 补测

每项使用 1920×1080、30 fps、3 秒/90 帧的视频。编码结果由 ffprobe 检查规格，再进行真实硬件解码（硬件帧经 hwdownload 输出）与完整软件解码。

| 显卡 / 路径 | HEVC 8-bit | HEVC 10-bit | AV1 8-bit | AV1 10-bit |
| --- | --- | --- | --- | --- |
| A750 VAAPI 编码 → VAAPI 解码 | 通过 | 通过 | 通过 | 通过 |
| A750 QSV 编码 → VAAPI 解码 | 通过 | 通过 | 通过 | 通过 |
| T4 NVENC 编码 → CUDA 解码 | 通过 | 通过 | 硬件不支持 | 硬件不支持 |

QSV 编码的 AV1 8-bit 样本第一次 VAAPI 解码进程异常退出，独立重试和完整矩阵重跑均通过；保留此观察，不能由短样本验收推断所有长片源都没有驱动问题。T4 的 `av1_nvenc` 实际报 `Codec not supported / No capable devices found`，`av1_cuvid` 报不支持；不会将软件解码回退记为硬件通过。

## NVIDIA 监测

项目 API 在 T4 上连续读取空闲与 1080p60 实时 NVENC 负载快照：

- 设备按 PCI 地址关联，只有一个 T4 条目，没有 sysfs / SMI 重复。
- 负载时整体 GPU 利用率约 2%，独立编码引擎约 24–30%。
- 显存约 218 MiB / 15 GiB；空闲时为 0，负载时增加。
- 每次新采样时间都推进，三个指标分别读取，监测未报错误。

Intel 本机 `intel_gpu_top` 因缺少 `CAP_PERFMON` 不能读取占用。API 保留两张设备，无法读取的计数器返回 null，界面显示不可用；这不影响 VAAPI / QSV 编码。Intel Video 计数器若可用时包含编码与解码活动，界面有对应口径说明。

Windows 默认采用 WDDM `GPU Engine` / `GPU Adapter Memory` 系统性能计数器，三家显卡使用同一条路径；通过 DXGI LUID 关联设备，整体占用取最忙引擎，独立编码占用只取 VideoEncode。独立显存已用量来自计数器，总量来自 DXGI 的 64 位字段。不存在的计数器保持 null，系统计数器整体不可用时才尝试 NVIDIA SMI 后备。12 个解析与归并样例通过；静态 PowerShell 脚本的语法、C# 编译、结构布局及模拟 CIM 数据序列化也通过。没有 Windows 实机，尚未验证实际驱动计数器表现。

NVIDIA 后备实现同时支持 CSV / dmon 和 XML。XML 后备按完整 PCI 地址关联 GPU，避免把 minor number 当作 NVML index；两种格式的显存都在解析时转为字节。硬件 workflow 额外运行持续 NVENC 负载监测测试，要求采样推进、设备标识唯一、编码利用率为正，以及整体占用和显存指标可读。

## 回归与复现

- 前端 `npm run build`、`npx tsc --noEmit` 和修改范围 ESLint。
- `node --test app/lib/*.test.cjs` 和 `node --test npm/tests/*.test.cjs`。
- 强制字体和真实媒体测试：设置 `BILIUP_RENDER_TOOLS_DIR`（包含 fonts）和 `BILIUP_REQUIRE_RENDER_TESTS=1`，执行 `cargo test --locked --workspace`。
- 硬件合成：设置 `BILIUP_TEST_ENCODER=h264_qsv` 或其他硬件编码器，以及 `BILIUP_TEST_ENCODER_DEVICE`，执行 `cargo test --locked -p biliup-cli --lib hardware_render -- --nocapture`。
- GitHub 的 `GPU render acceptance` workflow 为硬件 runner 提供同一验收入口；通常的 `Test` workflow 验证 CPU 输出、配置、回退与监测解析。
- DanmakuFactory C 核心的 sanitizer 和符号检查通过。

任务 API 的本机验收覆盖自动 GPU 选择、创建/取消/重试、跨场次筛选分页、完成后持久化编码器/速度/耗时，以及成品 Range 下载。新增数据库迁移使用第 19 号文件，旧迁移未被改写；升级与迁移校验基线同步更新。

最终全工作区回归（NVIDIA XML 后备扩展前）：1,393 项通过、0 失败、9 项既有忽略测试；一个小数点 locale 检查因系统未提供相应 locale 跳过。扩展后的最终合成与监测范围回归为 72 项通过、0 失败，其中 GPU / WDDM 解析与归并范围为 24 项通过。

T4 的 Rust 合成、能力检测及持续负载监测 API 验收均已通过。新增的 `hardware_gpu_monitor` opt-in 测试编译并纳入范围回归，但向 T4 上传最终测试二进制时 `nvenc-test` 已无活动会话，因此没有在启用实机环境变量的条件下运行该新测试，也未完成强制 XML 后备的远程 API 验收；XML 的单位、PCI 关联、N/A 和不完整输出由自动测例验证。没有自动重建或启动新的付费 Colab 会话。
