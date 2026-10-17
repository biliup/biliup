//! Best-effort GPU telemetry for the render workbench.
//!
//! Metrics are deliberately optional: vendor tooling and kernel counters are
//! frequently unavailable to an unprivileged process.

use serde::Serialize;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::{io::AsyncReadExt, process::Command, sync::Mutex as AsyncMutex, time::timeout};

#[cfg(any(windows, test))]
#[path = "windows_gpu.rs"]
mod windows_gpu;

const CACHE_TTL: Duration = Duration::from_secs(2);
const COMMAND_TIMEOUT: Duration = Duration::from_millis(2200);
const MAX_COMMAND_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Serialize)]
pub struct GpuSnapshot {
    pub devices: Vec<GpuDevice>,
    pub sampled_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct GpuDevice {
    pub id: String,
    pub name: String,
    pub vendor: String,
    pub render_node: Option<String>,
    pub utilization_percent: Option<f64>,
    pub encoder_utilization_percent: Option<f64>,
    pub engine_utilization: Vec<GpuEngine>,
    pub memory_used_bytes: Option<u64>,
    pub memory_total_bytes: Option<u64>,
    pub memory_kind: String,
    pub memory_source: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GpuEngine {
    pub name: String,
    pub utilization_percent: Option<f64>,
}

#[derive(Clone)]
struct Cached {
    at: Instant,
    value: GpuSnapshot,
}

static CACHE: OnceLock<Mutex<Option<Cached>>> = OnceLock::new();
static SAMPLE_LOCK: OnceLock<AsyncMutex<()>> = OnceLock::new();

/// Return a cached, coalesced snapshot. External probes are bounded and only
/// run once per cache window, even when several API requests arrive together.
pub async fn snapshot() -> GpuSnapshot {
    let cache = CACHE.get_or_init(|| Mutex::new(None));
    if let Some(cached) = cache.lock().ok().and_then(|c| c.clone()) {
        if cached.at.elapsed() < CACHE_TTL {
            return cached.value;
        }
    }

    let _guard = SAMPLE_LOCK.get_or_init(|| AsyncMutex::new(())).lock().await;
    if let Some(cached) = cache.lock().ok().and_then(|c| c.clone()) {
        if cached.at.elapsed() < CACHE_TTL {
            return cached.value;
        }
    }
    let value = collect().await;
    if let Ok(mut slot) = cache.lock() {
        *slot = Some(Cached {
            at: Instant::now(),
            value: value.clone(),
        });
    }
    value
}

async fn collect() -> GpuSnapshot {
    let mut devices: Vec<GpuDevice> = Vec::new();
    #[cfg(target_os = "linux")]
    {
        devices.extend(linux_drm_devices());
    }
    #[cfg(target_os = "windows")]
    let mut unavailable_wddm_nvidia = Vec::new();
    #[cfg(target_os = "windows")]
    {
        match windows_gpu::probe().await {
            Ok(wddm) if !wddm.is_empty() => {
                // WDDM is the default for all vendors on Windows. LUIDs join
                // engines/memory to DXGI adapters without guessing by name.
                if wddm.iter().any(|gpu| {
                    gpu.utilization_percent.is_some()
                        || gpu.encoder_utilization_percent.is_some()
                        || gpu.memory_used_bytes.is_some()
                }) {
                    return GpuSnapshot {
                        devices: wddm,
                        sampled_at: unix_ms(),
                    };
                }
                // If the system counters are unavailable as a whole, retain
                // Intel/AMD inventory and allow the NVIDIA driver fallback.
                for gpu in wddm {
                    if gpu.vendor == "nvidia" {
                        unavailable_wddm_nvidia.push(gpu);
                    } else {
                        devices.push(gpu);
                    }
                }
            }
            _ => devices.extend(windows_gpu_inventory().await),
        }
    }

    // nvidia-smi also covers NVIDIA devices on Windows, where DRM sysfs is not
    // present. Do not synthesize an NVIDIA entry if its driver tool is absent.
    match nvidia_rows().await {
        Some((rows, error)) => {
            let (encoder_metrics, encoder_error) = match nvidia_encoder_rows().await {
                Ok(metrics) => (metrics, None),
                Err(error) => (std::collections::HashMap::new(), Some(error)),
            };
            let mut seen = std::collections::HashSet::new();
            for row in rows {
                // On Linux, retain the stable PCI identity from DRM sysfs and
                // enrich it by normalized PCI address. On other systems
                // the driver query supplies the device list and identity.
                let Some(normalized_bus) = normalize_pci_bus(&row.bus_id) else {
                    continue;
                };
                let id = format!("nvidia:{normalized_bus}");
                seen.insert(id.clone());
                let mut row = row;
                if !row.source_xml {
                    row.encoder = lookup_nvidia_encoder(&row, &encoder_metrics);
                }
                if let Some(device) = devices.iter_mut().find(|d| d.id == id) {
                    device.name = row.name;
                    device.utilization_percent = row.gpu;
                    device.encoder_utilization_percent = row.encoder;
                    device.memory_used_bytes = row.mem_used;
                    device.memory_total_bytes = row.mem_total;
                    device.memory_kind = "dedicated".into();
                    device.memory_source = Some("nvidia-smi".into());
                    device.error = error.clone();
                    if row.encoder.is_none() {
                        device.error = append_error(
                            device.error.take(),
                            encoder_error
                                .as_deref()
                                .unwrap_or("nvidia-smi dmon 未报告编码引擎利用率"),
                        );
                    }
                } else {
                    devices.push(GpuDevice {
                        id,
                        name: row.name,
                        vendor: "nvidia".into(),
                        render_node: None,
                        utilization_percent: row.gpu,
                        encoder_utilization_percent: row.encoder,
                        engine_utilization: Vec::new(),
                        memory_used_bytes: row.mem_used,
                        memory_total_bytes: row.mem_total,
                        memory_kind: "dedicated".into(),
                        memory_source: Some("nvidia-smi".into()),
                        error: error.clone().or_else(|| {
                            row.encoder.is_none().then(|| {
                                encoder_error.clone().unwrap_or_else(|| {
                                    "nvidia-smi dmon 未报告编码引擎利用率".into()
                                })
                            })
                        }),
                    });
                }
            }
            for device in devices
                .iter_mut()
                .filter(|d| d.vendor == "nvidia" && !seen.contains(&d.id))
            {
                device.error = error
                    .clone()
                    .or_else(|| Some("nvidia-smi 未返回该检测到的 NVIDIA 设备数据".into()));
            }
        }
        None => {
            for device in devices.iter_mut().filter(|d| d.vendor == "nvidia") {
                device.error = Some("找不到 nvidia-smi，无法读取 NVIDIA 监测指标".into());
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        // Apply optional vendor telemetry to detected DRM devices.
        for device in devices.iter_mut().filter(|d| d.vendor == "amd") {
            if let Some(path) = device.render_node.as_deref().and_then(render_device_path) {
                apply_amd_sysfs(device, &path);
            }
        }
        for device in devices.iter_mut().filter(|d| d.vendor == "intel") {
            apply_intel_memory(device);
            match intel_gpu_top(device.render_node.as_deref()).await {
                Ok(sample) => {
                    device.utilization_percent = sample.overall;
                    device.encoder_utilization_percent = sample.video;
                    device.engine_utilization = sample.engines;
                    if !device.engine_utilization.iter().any(|engine| {
                        intel_engine_class(&engine.name) == Some(2)
                            && engine.utilization_percent.is_some()
                    }) {
                        device.error = append_error(
                            device.error.take(),
                            "intel_gpu_top 未报告 Video 引擎计数器",
                        );
                    }
                }
                Err(err) => device.error = append_error(device.error.take(), &err),
            }
        }
    }

    #[cfg(target_os = "windows")]
    if !devices.iter().any(|gpu| gpu.vendor == "nvidia") {
        devices.extend(unavailable_wddm_nvidia);
    }
    GpuSnapshot {
        devices,
        sampled_at: unix_ms(),
    }
}

fn linux_drm_devices() -> Vec<GpuDevice> {
    linux_drm_devices_at(Path::new("/sys/class/drm"))
}

fn linux_drm_devices_at(root: &Path) -> Vec<GpuDevice> {
    let mut result = Vec::new();
    let Ok(entries) = std::fs::read_dir(root) else {
        return result;
    };
    for entry in entries.flatten() {
        let node = entry.file_name().to_string_lossy().to_string();
        if !node.starts_with("renderD") {
            continue;
        }
        let device = entry.path().join("device");
        let Ok(vendor_id) = read_hex(&device.join("vendor")) else {
            continue;
        };
        let vendor = match vendor_id {
            0x10de => "nvidia",
            0x8086 => "intel",
            0x1002 => "amd",
            _ => "unknown",
        };
        let pci_id = read_hex(&device.join("device")).ok();
        let bus = std::fs::canonicalize(&device)
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
            .unwrap_or_else(|| {
                // Test fixture and systems exposing a non-canonical sysfs
                // device path still receive a stable identity.
                device
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or(node.clone())
            });
        result.push(GpuDevice {
            id: format!(
                "{vendor}:{}",
                normalize_pci_bus(&bus).unwrap_or_else(|| bus.to_ascii_lowercase())
            ),
            name: device_name(vendor, pci_id),
            vendor: vendor.into(),
            render_node: Some(format!("/dev/dri/{node}")),
            utilization_percent: None,
            encoder_utilization_percent: None,
            engine_utilization: Vec::new(),
            memory_used_bytes: None,
            memory_total_bytes: None,
            memory_kind: "unknown".into(),
            memory_source: None,
            error: if vendor == "unknown" {
                Some(format!("Unsupported PCI vendor 0x{vendor_id:04x}"))
            } else {
                None
            },
        });
    }
    result.sort_by(|a, b| a.id.cmp(&b.id));
    result
}

fn device_name(vendor: &str, pci_id: Option<u32>) -> String {
    match (vendor, pci_id) {
        ("intel", Some(0x4682)) => "Intel UHD Graphics 730 (device 0x4682)".into(),
        ("intel", Some(0x56a1)) => "Intel Arc A750 (device 0x56a1)".into(),
        ("intel", Some(id)) => format!("Intel GPU (device 0x{id:04x})"),
        ("amd", Some(id)) => format!("AMD GPU (device 0x{id:04x})"),
        ("nvidia", Some(id)) => format!("NVIDIA GPU (device 0x{id:04x})"),
        ("intel", _) => "Intel GPU".into(),
        ("amd", _) => "AMD GPU".into(),
        ("nvidia", _) => "NVIDIA GPU".into(),
        (_, Some(id)) => format!("GPU (vendor {vendor}, device 0x{id:04x})"),
        _ => format!("GPU ({vendor})"),
    }
}

#[cfg(target_os = "windows")]
async fn windows_gpu_inventory() -> Vec<GpuDevice> {
    let mut command = crate::tools::command("powershell.exe");
    command.args([
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        "Get-CimInstance Win32_VideoController | Select-Object Name,PNPDeviceID | ConvertTo-Json -Compress",
    ]);
    let Ok(bytes) = bounded_command_output(command, COMMAND_TIMEOUT).await else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return Vec::new();
    };
    let items: Vec<&serde_json::Value> = match &value {
        serde_json::Value::Array(items) => items.iter().collect(),
        serde_json::Value::Object(_) => vec![&value],
        _ => Vec::new(),
    };
    items
        .into_iter()
        .filter_map(|item| {
            let name = item.get("Name")?.as_str()?.trim();
            let pnp = item.get("PNPDeviceID")?.as_str()?.to_ascii_uppercase();
            let vendor = if pnp.contains("VEN_8086") {
                "intel"
            } else if pnp.contains("VEN_1002") {
                "amd"
            } else {
                return None;
            };
            Some(GpuDevice {
                id: format!("windows:{}", pnp.to_ascii_lowercase()),
                name: name.to_string(),
                vendor: vendor.into(),
                render_node: None,
                utilization_percent: None,
                encoder_utilization_percent: None,
                engine_utilization: Vec::new(),
                memory_used_bytes: None,
                memory_total_bytes: None,
                memory_kind: "unknown".into(),
                memory_source: None,
                error: Some("Windows 未提供可读取的 GPU 利用率、编码引擎或显存计数器".into()),
            })
        })
        .collect()
}

fn read_hex(path: &Path) -> Result<u32, ()> {
    std::fs::read_to_string(path)
        .map_err(|_| ())?
        .trim()
        .strip_prefix("0x")
        .ok_or(())
        .and_then(|s| u32::from_str_radix(s, 16).map_err(|_| ()))
}

fn normalize_pci_bus(bus: &str) -> Option<String> {
    let bus = bus.trim().to_ascii_lowercase();
    let parts: Vec<_> = bus.split(':').collect();
    let (domain, bus_number, device_function) = match parts.as_slice() {
        [bus_number, device_function] => (0, *bus_number, *device_function),
        [domain, bus_number, device_function] => {
            (parse_pci_hex(domain, 8)?, *bus_number, *device_function)
        }
        _ => return None,
    };
    let (device, function) = device_function.split_once('.')?;
    if function.contains('.') {
        return None;
    }
    let bus_number = parse_pci_hex(bus_number, 2)?;
    let device = parse_pci_hex(device, 2)?;
    let function = parse_pci_hex(function, 1)?;
    if domain > u16::MAX as u32 || bus_number > u8::MAX as u32 || device > 0x1f || function > 7 {
        return None;
    }
    Some(format!(
        "{domain:04x}:{bus_number:02x}:{device:02x}.{function:x}"
    ))
}

fn parse_pci_hex(value: &str, max_digits: usize) -> Option<u32> {
    if value.is_empty()
        || value.len() > max_digits
        || !value.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return None;
    }
    u32::from_str_radix(value, 16).ok()
}

fn render_device_path(node: &str) -> Option<PathBuf> {
    let name = Path::new(node).file_name()?;
    let drm_path = Path::new("/sys/class/drm").join(name).join("device");
    std::fs::canonicalize(drm_path).ok()
}

fn apply_amd_sysfs(device: &mut GpuDevice, path: &Path) {
    device.utilization_percent =
        read_number(&path.join("gpu_busy_percent")).filter(|n| (0.0..=100.0).contains(n));
    device.memory_used_bytes = read_bytes(&path.join("mem_info_vram_used"));
    device.memory_total_bytes = read_bytes(&path.join("mem_info_vram_total"));
    if device.memory_total_bytes.is_some_and(|total| total > 0) {
        device.memory_kind = "dedicated".into();
        device.memory_source = Some("amd-sysfs".into());
    }
    if device.utilization_percent.is_none() && device.memory_used_bytes.is_none() {
        device.error = Some("AMD sysfs 未提供 GPU 利用率或显存计数器".into());
    } else if device.encoder_utilization_percent.is_none() {
        device.error = Some("AMD sysfs 未提供独立的编码引擎利用率计数器".into());
    }
}

fn read_number(path: &Path) -> Option<f64> {
    let value: f64 = std::fs::read_to_string(path).ok()?.trim().parse().ok()?;
    (value.is_finite() && value >= 0.0).then_some(value)
}

fn read_bytes(path: &Path) -> Option<u64> {
    let value = read_number(path)?;
    (value.fract() == 0.0 && value <= u64::MAX as f64).then_some(value as u64)
}

#[derive(Debug, PartialEq)]
struct IntelMemory {
    dedicated: bool,
    total: Option<u64>,
    used: Option<u64>,
}

#[cfg(target_os = "linux")]
fn apply_intel_memory(device: &mut GpuDevice) {
    let Some(node) = device.render_node.as_deref() else {
        return;
    };
    match i915_memory(node) {
        Ok(memory) => {
            device.memory_kind = if memory.dedicated {
                "dedicated"
            } else {
                "shared"
            }
            .into();
            device.memory_total_bytes = memory.total;
            device.memory_used_bytes = memory.used;
            device.memory_source = Some(
                if memory.used.is_some() {
                    "i915-memory-regions"
                } else {
                    "i915-memory-regions-total"
                }
                .into(),
            );
        }
        Err(error) => {
            device.error = append_error(
                device.error.take(),
                &format!("Intel 内存指标不可用：{error}"),
            )
        }
    }
}

// Stable i915 uAPI layouts from drm/i915_drm.h. The query is read-only and
// returns SYSTEM/DEVICE regions rather than inferring capacity from PCI names.
#[cfg(target_os = "linux")]
#[repr(C)]
struct I915QueryItem {
    query_id: u64,
    length: i32,
    flags: u32,
    data_ptr: u64,
}

#[cfg(target_os = "linux")]
#[repr(C)]
struct I915Query {
    num_items: u32,
    flags: u32,
    items_ptr: u64,
}

#[cfg(target_os = "linux")]
fn i915_memory(node: &str) -> Result<IntelMemory, String> {
    use std::os::fd::AsRawFd;
    let file = std::fs::File::open(node).map_err(|error| error.to_string())?;
    let mut item = I915QueryItem {
        query_id: 4,
        length: 0,
        flags: 0,
        data_ptr: 0,
    };
    let mut query = I915Query {
        num_items: 1,
        flags: 0,
        items_ptr: (&mut item as *mut I915QueryItem) as u64,
    };
    const REQUEST: libc::Ioctl = libc::_IOWR::<I915Query>(b'd' as u32, 0x40 + 0x39);
    // SAFETY: query and its single item remain alive throughout both calls;
    // the data buffer stays allocated and its exact size is provided to the OS.
    if unsafe { libc::ioctl(file.as_raw_fd(), REQUEST, &mut query) } < 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    if !(16..=MAX_COMMAND_BYTES as i32).contains(&item.length) {
        return Err(format!("i915 memory query length {}", item.length));
    }
    let mut bytes = vec![0u8; item.length as usize];
    item.data_ptr = bytes.as_mut_ptr() as u64;
    if unsafe { libc::ioctl(file.as_raw_fd(), REQUEST, &mut query) } < 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    if item.length < 16 || item.length as usize > bytes.len() {
        return Err("i915 memory query returned an invalid length".into());
    }
    let privileged = std::fs::read_to_string("/proc/self/status")
        .ok()
        .is_some_and(|status| memory_accounting_capability(&status));
    parse_i915_memory(&bytes[..item.length as usize], privileged)
        .ok_or_else(|| "i915 memory query returned invalid regions".into())
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn memory_accounting_capability(status: &str) -> bool {
    status
        .lines()
        .find_map(|line| line.strip_prefix("CapEff:"))
        .and_then(|bits| u64::from_str_radix(bits.trim(), 16).ok())
        .is_some_and(|bits| bits & ((1u64 << 21) | (1u64 << 38)) != 0)
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn parse_i915_memory(bytes: &[u8], privileged: bool) -> Option<IntelMemory> {
    let count = u32::from_ne_bytes(bytes.get(..4)?.try_into().ok()?) as usize;
    let needed = count.checked_mul(88)?.checked_add(16)?;
    if count == 0 || needed > bytes.len() {
        return None;
    }
    let mut total = 0u64;
    let mut used = Some(0u64);
    let mut system = false;
    let mut dedicated = false;
    for region in bytes[16..needed].chunks_exact(88) {
        let class = u16::from_ne_bytes(region[..2].try_into().ok()?);
        let probed = u64::from_ne_bytes(region[8..16].try_into().ok()?);
        let unallocated = u64::from_ne_bytes(region[16..24].try_into().ok()?);
        if probed == 0 || probed == u64::MAX {
            return None;
        }
        if class == 0 {
            system = true;
        }
        if class != 1 {
            continue;
        }
        dedicated = true;
        total = total.checked_add(probed)?;
        // Without CAP_PERFMON/CAP_SYS_ADMIN, kernels deliberately return
        // unallocated == probed. Older kernels can also return that fallback:
        // keep this ambiguous value unavailable even with effective capability.
        used = if privileged && unallocated < probed {
            used.and_then(|used| used.checked_add(probed - unallocated))
        } else {
            None
        };
    }
    if !dedicated && !system {
        return None;
    }
    Some(IntelMemory {
        dedicated,
        total: dedicated.then_some(total),
        used: dedicated.then_some(used).flatten(),
    })
}

#[derive(Default)]
struct IntelMetrics {
    overall: Option<f64>,
    video: Option<f64>,
    engines: Vec<GpuEngine>,
}

async fn intel_gpu_top(render_node: Option<&str>) -> Result<IntelMetrics, String> {
    let mut cmd = crate::tools::command("intel_gpu_top");
    cmd.args(["-J", "-s", "250", "-n", "2", "-o", "-"]);
    if let Some(node) = render_node {
        cmd.args(["-d", &format!("drm:{node}")]);
    }
    let bytes = bounded_command_output(cmd, COMMAND_TIMEOUT).await?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|e| format!("intel_gpu_top returned invalid JSON: {e}"))?;
    Ok(parse_intel_metrics(&value))
}

async fn bounded_command_output(mut cmd: Command, limit: Duration) -> Result<Vec<u8>, String> {
    cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("GPU 监测工具不可用：{e}"))?;
    let Some(mut stdout) = child.stdout.take() else {
        return Err("GPU 监测工具没有输出".into());
    };
    let Some(mut stderr) = child.stderr.take() else {
        return Err("GPU 监测工具错误输出不可用".into());
    };
    let result = timeout(limit, async {
        let (out, err) = tokio::try_join!(read_capped(&mut stdout), read_capped(&mut stderr))?;
        let status = child
            .wait()
            .await
            .map_err(|e| format!("等待 GPU 监测工具结束失败：{e}"))?;
        if !status.success() {
            let detail = String::from_utf8_lossy(&err)
                .trim()
                .chars()
                .take(500)
                .collect::<String>();
            return Err(if detail.is_empty() {
                format!("GPU 监测工具退出状态：{status}")
            } else {
                format!("GPU 监测工具失败：{detail}")
            });
        }
        Ok(out)
    })
    .await;
    match result {
        Ok(Ok(bytes)) => Ok(bytes),
        Ok(Err(error)) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            Err(error)
        }
        Err(_) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            Err("GPU 监测工具超时".into())
        }
    }
}

async fn read_capped(reader: &mut (impl tokio::io::AsyncRead + Unpin)) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    (&mut *reader)
        .take((MAX_COMMAND_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .await
        .map_err(|e| format!("读取 GPU 监测输出失败：{e}"))?;
    if bytes.len() > MAX_COMMAND_BYTES {
        return Err("GPU 监测工具输出超过 64 KiB 限制".into());
    }
    Ok(bytes)
}

fn parse_intel_metrics(value: &serde_json::Value) -> IntelMetrics {
    // intel_gpu_top reports per-engine busy percentages. Do not sum the
    // independent engines into a pretend overall percentage. The Video class
    // combines encode/decode work, so expose it only in the engine field.
    let sample = match value {
        serde_json::Value::Array(samples) => samples
            .iter()
            .rev()
            .find(|sample| sample.get("engines").is_some()),
        _ => Some(value),
    };
    let engines = sample
        .and_then(|s| s.get("engines"))
        .and_then(|v| v.as_object());
    let mut engines: Vec<GpuEngine> = engines
        .map(|engines| {
            engines
                .iter()
                .filter_map(|(name, entry)| {
                    intel_engine_class(name).map(|_| GpuEngine {
                        name: name.clone(),
                        utilization_percent: entry.get("busy").and_then(as_percent),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    engines.sort_by(|a, b| {
        intel_engine_class(&a.name)
            .cmp(&intel_engine_class(&b.name))
            .then(a.name.cmp(&b.name))
    });
    let videos: Vec<_> = engines
        .iter()
        .filter(|engine| intel_engine_class(&engine.name) == Some(2))
        .collect();
    // Keep the legacy single-engine field only when there is exactly one
    // Video engine. Multiple hardware engines remain separately observable.
    let video = if videos.len() == 1 {
        videos[0].utilization_percent
    } else {
        None
    };
    IntelMetrics {
        overall: None,
        video,
        engines,
    }
}

fn intel_engine_class(name: &str) -> Option<u8> {
    let name = name.to_ascii_lowercase();
    ["render/3d", "blitter", "video", "videoenhance", "compute"]
        .iter()
        .position(|class| {
            name == *class
                || name.strip_prefix(class).is_some_and(|tail| {
                    tail.starts_with('/') || tail.starts_with(' ') || tail.starts_with('-')
                })
        })
        .map(|class| class as u8)
}

fn as_percent(value: &serde_json::Value) -> Option<f64> {
    let number = value.as_f64()?;
    (number.is_finite() && (0.0..=100.0).contains(&number)).then_some(number)
}

#[derive(Default)]
struct NvidiaRow {
    index: String,
    bus_id: String,
    name: String,
    gpu: Option<f64>,
    encoder: Option<f64>,
    mem_used: Option<u64>,
    mem_total: Option<u64>,
    source_xml: bool,
}

async fn nvidia_rows() -> Option<(Vec<NvidiaRow>, Option<String>)> {
    let mut cmd = crate::tools::command("nvidia-smi");
    cmd.args([
        "--query-gpu=pci.bus_id,index,name,utilization.gpu,memory.used,memory.total",
        "--format=csv,noheader,nounits",
    ]);
    let bytes = match bounded_command_output(cmd, COMMAND_TIMEOUT).await {
        Ok(bytes) => bytes,
        Err(error) if error.starts_with("GPU 监测工具不可用") => return None,
        Err(error) => {
            return nvidia_xml_rows()
                .await
                .or_else(|| Some((Vec::new(), Some(error))));
        }
    };
    let text = String::from_utf8_lossy(&bytes);
    let rows = parse_nvidia_csv(&text);
    if rows.is_empty() {
        if let Some((xml_rows, error)) = nvidia_xml_rows().await {
            if !xml_rows.is_empty() {
                return Some((xml_rows, error));
            }
        }
    }
    if rows.is_empty() {
        Some((rows, Some("nvidia-smi 未返回可解析的 GPU 数据".into())))
    } else {
        Some((rows, None))
    }
}

async fn nvidia_encoder_rows() -> Result<std::collections::HashMap<String, f64>, String> {
    let mut cmd = crate::tools::command("nvidia-smi");
    cmd.args(["dmon", "-s", "u", "-c", "1"]);
    match bounded_command_output(cmd, COMMAND_TIMEOUT).await {
        Ok(bytes) => {
            if let Some(values) = parse_nvidia_dmon(&String::from_utf8_lossy(&bytes)) {
                return Ok(values
                    .into_iter()
                    .map(|(index, value)| (format!("index:{index}"), value))
                    .collect());
            }
        }
        Err(_) => {}
    }
    let Some((rows, _)) = nvidia_xml_rows().await else {
        return Err("nvidia-smi dmon 不可用，XML 查询也未返回编码引擎利用率".into());
    };
    let values: HashMap<_, _> = rows
        .into_iter()
        // XML minor_number is a device-file identifier, not an NVML index,
        // and can be N/A on Windows. PCI is shared with the CSV inventory.
        .filter_map(|row| {
            row.encoder
                .filter(|_| !row.bus_id.is_empty())
                .and_then(|value| {
                    normalize_pci_bus(&row.bus_id).map(|bus| (format!("pci:{bus}"), value))
                })
        })
        .collect();
    if values.is_empty() {
        Err("nvidia-smi dmon 不可用，XML 查询也未返回编码引擎利用率".into())
    } else {
        Ok(values)
    }
}

fn lookup_nvidia_encoder(row: &NvidiaRow, metrics: &HashMap<String, f64>) -> Option<f64> {
    metrics
        .get(&format!("pci:{}", normalize_pci_bus(&row.bus_id)?))
        .or_else(|| metrics.get(&format!("index:{}", row.index)))
        .copied()
}

async fn nvidia_xml_rows() -> Option<(Vec<NvidiaRow>, Option<String>)> {
    let mut cmd = crate::tools::command("nvidia-smi");
    cmd.args(["-q", "-x"]);
    let bytes = bounded_command_output(cmd, COMMAND_TIMEOUT).await.ok()?;
    let rows = parse_nvidia_xml(&bytes);
    (!rows.is_empty()).then_some((rows, None))
}

fn parse_nvidia_xml(bytes: &[u8]) -> Vec<NvidiaRow> {
    use quick_xml::{Reader, events::Event};

    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().trim_text(true);
    let mut buffer = Vec::new();
    let mut stack = Vec::<String>::new();
    let mut rows = Vec::new();
    let mut row: Option<NvidiaRow> = None;

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(event)) => {
                let tag = String::from_utf8_lossy(event.name().as_ref()).into_owned();
                if tag == "gpu" {
                    row = Some(NvidiaRow {
                        source_xml: true,
                        ..Default::default()
                    });
                    if let Some(gpu) = row.as_mut() {
                        gpu.bus_id = event
                            .attributes()
                            .flatten()
                            .find(|attr| attr.key.as_ref() == b"id")
                            .and_then(|attr| attr.unescape_value().ok().map(|v| v.into_owned()))
                            .unwrap_or_default();
                    }
                }
                stack.push(tag);
            }
            Ok(Event::Empty(event)) => {
                let tag = String::from_utf8_lossy(event.name().as_ref()).into_owned();
                if tag == "gpu" {
                    let mut gpu = NvidiaRow {
                        source_xml: true,
                        ..Default::default()
                    };
                    gpu.bus_id = event
                        .attributes()
                        .flatten()
                        .find(|attr| attr.key.as_ref() == b"id")
                        .and_then(|attr| attr.unescape_value().ok().map(|v| v.into_owned()))
                        .unwrap_or_default();
                    if !gpu.bus_id.is_empty() {
                        rows.push(gpu);
                    }
                }
            }
            Ok(Event::Text(event)) => {
                if let Some(gpu) = row.as_mut() {
                    let text = event.unescape().unwrap_or_default().trim().to_string();
                    if text.is_empty() {
                        buffer.clear();
                        continue;
                    }
                    let path = stack.join("/");
                    match path.as_str() {
                        "nvidia_smi_log/gpu/product_name" => gpu.name = text,
                        "nvidia_smi_log/gpu/minor_number" => gpu.index = text,
                        "nvidia_smi_log/gpu/pci/pci_bus_id" => gpu.bus_id = text,
                        "nvidia_smi_log/gpu/utilization/gpu_util" => gpu.gpu = parse_pct(&text),
                        "nvidia_smi_log/gpu/utilization/encoder_util" => {
                            gpu.encoder = parse_pct(&text)
                        }
                        "nvidia_smi_log/gpu/fb_memory_usage/used" => {
                            gpu.mem_used = parse_xml_memory_bytes(&text)
                        }
                        "nvidia_smi_log/gpu/fb_memory_usage/total" => {
                            gpu.mem_total = parse_xml_memory_bytes(&text)
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::End(event)) => {
                let tag = String::from_utf8_lossy(event.name().as_ref()).into_owned();
                if tag == "gpu" {
                    if let Some(gpu) = row.take() {
                        if !gpu.bus_id.is_empty() || !gpu.index.is_empty() {
                            rows.push(gpu);
                        }
                    }
                }
                if stack.last().is_some_and(|last| last == &tag) {
                    stack.pop();
                }
            }
            Ok(Event::Eof) => {
                if !stack.is_empty() {
                    return Vec::new();
                }
                break;
            }
            Err(_) => return Vec::new(),
            _ => {}
        }
        buffer.clear();
    }
    rows
}

fn parse_xml_memory_bytes(value: &str) -> Option<u64> {
    let mut parts = value.split_whitespace();
    let amount: f64 = parts.next()?.parse().ok()?;
    if !amount.is_finite() || amount < 0.0 {
        return None;
    }
    let multiplier = match parts.next()?.to_ascii_lowercase().as_str() {
        "kib" => 1024.0,
        "mib" => 1024.0 * 1024.0,
        "gib" => 1024.0 * 1024.0 * 1024.0,
        "tib" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => return None,
    };
    let bytes = amount * multiplier;
    (bytes.is_finite() && bytes <= u64::MAX as f64).then_some(bytes as u64)
}

fn parse_nvidia_dmon(text: &str) -> Option<std::collections::HashMap<String, f64>> {
    let mut encoder_col = None;
    let mut values = std::collections::HashMap::new();
    for line in text.lines() {
        let fields: Vec<_> = line.split_whitespace().collect();
        if fields.first() == Some(&"#") && fields.iter().any(|f| *f == "gpu") {
            encoder_col = fields.iter().position(|f| *f == "enc");
            continue;
        }
        let Some(col) = encoder_col.map(|col| col.saturating_sub(1)) else {
            continue;
        };
        if fields.len() <= col || fields[0].starts_with('#') {
            continue;
        }
        if let Some(value) = parse_pct(fields[col]) {
            values.insert(fields[0].to_string(), value);
        }
    }
    (!values.is_empty()).then_some(values)
}

fn parse_nvidia_csv(text: &str) -> Vec<NvidiaRow> {
    text.lines()
        .filter_map(|line| {
            // Names can contain commas; NVIDIA's query fields are otherwise
            // scalar. Split from both ends, leaving the middle as the device name.
            let cols: Vec<_> = line.split(',').map(str::trim).collect();
            if cols.len() < 6 {
                return None;
            }
            let bus_id = cols[0].to_string();
            let index = cols[1].to_string();
            let name = if cols.len() == 6 {
                cols[2].to_string()
            } else {
                cols[2..cols.len() - 3].join(", ")
            };
            let tail = &cols[cols.len() - 3..];
            Some(NvidiaRow {
                bus_id,
                index,
                name,
                gpu: parse_pct(tail[0]),
                encoder: None,
                mem_used: parse_u64(tail[1]).map(|v| v.saturating_mul(1024 * 1024)),
                mem_total: parse_u64(tail[2]).map(|v| v.saturating_mul(1024 * 1024)),
                source_xml: false,
            })
        })
        .collect()
}

fn parse_pct(text: &str) -> Option<f64> {
    let text = text.trim().trim_end_matches('%').trim();
    if text.eq_ignore_ascii_case("[not supported]") || text.eq_ignore_ascii_case("n/a") {
        return None;
    }
    let n: f64 = text.parse().ok()?;
    (n.is_finite() && (0.0..=100.0).contains(&n)).then_some(n)
}

fn parse_u64(text: &str) -> Option<u64> {
    if text.eq_ignore_ascii_case("[not supported]") || text.eq_ignore_ascii_case("n/a") {
        return None;
    }
    text.parse().ok()
}

fn append_error(current: Option<String>, next: &str) -> Option<String> {
    Some(match current {
        Some(current) => format!("{current}; {next}"),
        None => next.to_string(),
    })
}

fn unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64
}

#[cfg(test)]
#[path = "gpu_tests.rs"]
mod tests;
