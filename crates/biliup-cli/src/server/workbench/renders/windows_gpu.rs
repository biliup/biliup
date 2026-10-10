//! Windows WDDM telemetry, correlated with DXGI adapters by LUID.
//!
//! The parser/reducer is platform independent so recorded counter shapes can
//! be tested without Windows. Only the bounded PowerShell probe is Windows-only.

use serde::Deserialize;
use std::collections::{HashMap, HashSet};

#[cfg(windows)]
pub(super) async fn probe() -> Result<Vec<super::GpuDevice>, String> {
    use base64::Engine;

    // EncodedCommand accepts UTF-16LE. Keeping the entire script static avoids
    // interpolating device names, user input, or shell metacharacters.
    let script = include_str!("windows_gpu.ps1");
    let utf16: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let encoded = base64::engine::general_purpose::STANDARD.encode(utf16);
    let mut command = crate::tools::command("powershell.exe");
    command.args([
        "-NoLogo",
        "-NoProfile",
        "-NonInteractive",
        "-EncodedCommand",
        &encoded,
    ]);
    // Cold PowerShell startup and Add-Type compilation need more time than
    // the lightweight vendor CLI probes. The shared helper kills and reaps
    // the process on timeout and caps stdout/stderr to 64 KiB.
    let bytes = super::bounded_command_output(command, std::time::Duration::from_secs(6)).await?;
    parse_snapshot(&bytes)
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct WddmSnapshot {
    #[serde(default)]
    adapters: Vec<Adapter>,
    #[serde(default)]
    engines: Vec<EngineSample>,
    #[serde(default)]
    memory: Vec<MemorySample>,
    inventory_error: Option<String>,
    engine_error: Option<String>,
    memory_error: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Adapter {
    name: String,
    vendor_id: u32,
    luid: String,
    dedicated_video_memory: Option<u64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct EngineSample {
    name: String,
    utilization_percentage: Option<f64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct MemorySample {
    name: String,
    dedicated_usage: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct EngineKey {
    luid: String,
    physical: u32,
    engine: u32,
}

struct EngineTotal {
    utilization: f64,
    kind: String,
}

#[derive(Default)]
struct Metrics {
    overall: Option<f64>,
    encode: Option<f64>,
    dedicated_used: Option<u64>,
}

fn parse_snapshot(bytes: &[u8]) -> Result<Vec<super::GpuDevice>, String> {
    // PowerShell normally emits UTF-8 without BOM; tolerate a UTF-8 BOM for
    // hosts with a preconfigured console encoding.
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
    let snapshot: WddmSnapshot =
        serde_json::from_slice(bytes).map_err(|e| format!("Windows GPU 监测返回无效 JSON：{e}"))?;
    if let Some(error) = snapshot.inventory_error.as_deref() {
        return Err(format!("DXGI GPU 设备枚举失败：{}", diagnostic(error)));
    }
    let mut metrics = reduce_metrics(&snapshot.engines, &snapshot.memory);
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for adapter in snapshot.adapters {
        // Windows uses WDDM for all three hardware vendors. NVIDIA SMI is a
        // fallback in the caller only when this complete WDDM probe fails.
        let vendor = match adapter.vendor_id {
            0x8086 => "intel",
            0x1002 => "amd",
            0x10de => "nvidia",
            _ => continue,
        };
        let luid = normalize_luid(&adapter.luid)
            .ok_or_else(|| "DXGI GPU 设备返回无效 LUID，无法安全关联 WDDM 计数器".to_string())?;
        if !seen.insert(luid.clone()) {
            continue;
        }
        let sample = metrics.remove(&luid).unwrap_or_default();
        let memory_total = adapter.dedicated_video_memory.filter(|&n| n > 0);
        let mut errors = Vec::new();
        if let Some(error) = snapshot.engine_error.as_deref() {
            errors.push(format!("WDDM GPU 引擎计数器不可用：{}", diagnostic(error)));
        } else if sample.overall.is_none() {
            errors.push("WDDM 未报告该 GPU 的引擎利用率计数器".into());
        }
        if sample.encode.is_none() {
            errors.push("WDDM 未报告 VideoEncode 编码引擎计数器".into());
        }
        if let Some(error) = snapshot.memory_error.as_deref() {
            errors.push(format!("WDDM 独立显存计数器不可用：{}", diagnostic(error)));
        } else if sample.dedicated_used.is_none() {
            errors.push("WDDM 未报告该 GPU 的独立显存使用量".into());
        }
        if memory_total.is_none() {
            errors.push("DXGI 未报告独立显存总量（集成显卡可能使用共享内存）".into());
        }
        result.push(super::GpuDevice {
            id: format!("windows:{luid}"),
            name: if adapter.name.trim().is_empty() {
                format!(
                    "{} GPU",
                    match vendor {
                        "intel" => "Intel",
                        "amd" => "AMD",
                        _ => "NVIDIA",
                    }
                )
            } else {
                adapter.name.trim().to_string()
            },
            vendor: vendor.into(),
            render_node: None,
            utilization_percent: sample.overall,
            encoder_utilization_percent: sample.encode,
            memory_used_bytes: sample.dedicated_used,
            memory_total_bytes: memory_total,
            error: (!errors.is_empty()).then(|| errors.join("；")),
        });
    }
    result.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(result)
}

fn reduce_metrics(engines: &[EngineSample], memory: &[MemorySample]) -> HashMap<String, Metrics> {
    let mut totals: HashMap<EngineKey, EngineTotal> = HashMap::new();
    for sample in engines {
        let Some(utilization) = sample
            .utilization_percentage
            .filter(|n| n.is_finite() && *n >= 0.0)
        else {
            continue;
        };
        let Some((luid, physical, suffix)) = counter_identity(&sample.name) else {
            continue;
        };
        let Some((engine, kind)) = suffix
            .strip_prefix("_eng_")
            .and_then(|s| s.split_once("_engtype_"))
        else {
            continue;
        };
        let Some(engine) = decimal(engine) else {
            continue;
        };
        if kind.is_empty() {
            continue;
        }
        // Process counters for a given physical GPU engine describe disjoint
        // work on that engine. Add them, then clamp, instead of picking a
        // process or adding different engines into an artificial GPU total.
        let total = totals
            .entry(EngineKey {
                luid,
                physical,
                engine,
            })
            .or_insert_with(|| EngineTotal {
                utilization: 0.0,
                kind: kind.to_string(),
            });
        total.utilization = (total.utilization + utilization).min(100.0);
        // A conflicting engine type means the driver data cannot safely be
        // identified as a dedicated encode engine. Overall remains usable.
        if !total.kind.eq_ignore_ascii_case(kind) {
            total.kind.clear();
        }
    }
    let mut result: HashMap<String, Metrics> = HashMap::new();
    for (key, total) in totals {
        let metrics = result.entry(key.luid).or_default();
        metrics.overall = Some(metrics.overall.unwrap_or(0.0).max(total.utilization));
        if total.kind.eq_ignore_ascii_case("VideoEncode") {
            metrics.encode = Some(metrics.encode.unwrap_or(0.0).max(total.utilization));
        }
    }

    // GPUAdapterMemory is per physical adapter, unlike GPUProcessMemory.
    // Never include SharedUsage or sum repeated samples of one physical GPU.
    let mut dedicated: HashMap<String, HashMap<u32, u64>> = HashMap::new();
    for sample in memory {
        if sample.name.starts_with("pid_") {
            // Do not accidentally accept GPUProcessMemory if a provider or
            // future script returns a different counter class.
            continue;
        }
        let Some(usage) = sample.dedicated_usage else {
            continue;
        };
        let Some((luid, physical, suffix)) = counter_identity(&sample.name) else {
            continue;
        };
        if !suffix.is_empty() {
            continue;
        }
        dedicated
            .entry(luid)
            .or_default()
            .entry(physical)
            .and_modify(|n| *n = (*n).max(usage))
            .or_insert(usage);
    }
    for (luid, physical) in dedicated {
        result.entry(luid).or_default().dedicated_used = physical
            .values()
            .try_fold(0_u64, |total, &value| total.checked_add(value));
    }
    result
}

/// Instance names store the high DWORD first, matching DXGI's LUID display:
/// `pid_123_luid_0x00000000_0x0000abcd_phys_0_eng_1_engtype_VideoEncode`.
fn counter_identity(name: &str) -> Option<(String, u32, &str)> {
    let name = if let Some(rest) = name.strip_prefix("pid_") {
        let (pid, rest) = rest.split_once('_')?;
        decimal(pid)?;
        rest
    } else {
        name
    };
    let rest = name.strip_prefix("luid_")?;
    let (high, rest) = rest.split_once('_')?;
    let (low, rest) = rest.split_once("_phys_")?;
    let luid = format!("{:08x}:{:08x}", hex_dword(high)?, hex_dword(low)?);
    let split = rest.find('_').unwrap_or(rest.len());
    let physical = decimal(&rest[..split])?;
    Some((luid, physical, &rest[split..]))
}

fn normalize_luid(value: &str) -> Option<String> {
    let (high, low) = value.trim().split_once(':')?;
    Some(format!("{:08x}:{:08x}", hex_dword(high)?, hex_dword(low)?))
}

fn hex_dword(value: &str) -> Option<u32> {
    let digits = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .unwrap_or(value);
    if digits.is_empty() || digits.len() > 8 || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    u32::from_str_radix(digits, 16).ok()
}

fn decimal(value: &str) -> Option<u32> {
    (!value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()))
        .then(|| value.parse().ok())
        .flatten()
}

fn diagnostic(value: &str) -> String {
    value.trim().chars().take(240).collect()
}

#[cfg(test)]
#[path = "windows_gpu_tests.rs"]
mod tests;
