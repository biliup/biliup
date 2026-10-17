use super::*;
use serde_json::{Value, json};

fn parse(value: Value) -> Vec<super::super::GpuDevice> {
    parse_snapshot(&serde_json::to_vec(&value).unwrap()).unwrap()
}

fn adapter(name: &str, vendor_id: u32, luid: &str, memory: u64) -> Value {
    json!({
        "Name": name,
        "VendorId": vendor_id,
        "Luid": luid,
        "DedicatedVideoMemory": memory,
    })
}

fn engine(name: &str, utilization: Value) -> Value {
    json!({"Name": name, "UtilizationPercentage": utilization})
}

#[test]
fn real_counter_shape_uses_high_then_low_luid_and_physical_engine() {
    let (luid, physical, suffix) =
        counter_identity("pid_12088_luid_0x00000001_0x00b6e0db_phys_0_eng_2_engtype_VideoEncode")
            .unwrap();
    assert_eq!(luid, "00000001:00b6e0db");
    assert_eq!(physical, 0);
    assert_eq!(suffix, "_eng_2_engtype_VideoEncode");
    assert_eq!(
        counter_identity("luid_0x00000001_0x00b6e0db_phys_0"),
        Some(("00000001:00b6e0db".into(), 0, ""))
    );
    assert_eq!(
        normalize_luid("0X1:0xB6E0DB"),
        Some("00000001:00b6e0db".into())
    );
}

#[test]
fn joins_reordered_multi_vendor_adapters_by_exact_luid() {
    let devices = parse(json!({
        "Adapters": [
            adapter("AMD Radeon", 0x1002, "00000001:00abc001", 16_000_000_000),
            adapter("Intel Arc", 0x8086, "00000002:00abc001", 8_000_000_000),
            adapter("NVIDIA RTX", 0x10de, "00000000:00abc002", 24_000_000_000),
            adapter("Unknown", 0x1234, "00000000:00abc003", 1_000_000_000),
        ],
        "Engines": [
            engine("pid_50_luid_0x00000000_0x00abc002_phys_0_eng_0_engtype_3D", json!(31)),
            engine("pid_20_luid_0x00000002_0x00abc001_phys_0_eng_1_engtype_VideoEncode", json!(12)),
            engine("pid_40_luid_0x00000001_0x00abc001_phys_0_eng_2_engtype_VideoEncode", json!(41)),
        ],
        "Memory": [
            {"Name": "luid_0x00000002_0x00abc001_phys_0", "DedicatedUsage": 222},
            {"Name": "luid_0x00000000_0x00abc002_phys_0", "DedicatedUsage": 333},
            {"Name": "luid_0x00000001_0x00abc001_phys_0", "DedicatedUsage": 111},
        ],
    }));
    assert_eq!(devices.len(), 3);
    let intel = devices.iter().find(|d| d.vendor == "intel").unwrap();
    assert_eq!(intel.id, "windows:00000002:00abc001");
    assert_eq!(intel.utilization_percent, Some(12.0));
    assert_eq!(intel.encoder_utilization_percent, Some(12.0));
    assert_eq!(intel.memory_used_bytes, Some(222));
    assert_eq!(intel.memory_total_bytes, Some(8_000_000_000));
    assert!(intel.error.is_none());
    let amd = devices.iter().find(|d| d.vendor == "amd").unwrap();
    assert_eq!(amd.encoder_utilization_percent, Some(41.0));
    assert_eq!(amd.memory_used_bytes, Some(111));
    let nvidia = devices.iter().find(|d| d.vendor == "nvidia").unwrap();
    assert_eq!(nvidia.utilization_percent, Some(31.0));
    assert_eq!(nvidia.encoder_utilization_percent, None);
    assert_eq!(nvidia.memory_used_bytes, Some(333));
}

#[test]
fn memory_kind_uses_explicit_dxgi_capacity_instead_of_vendor_guessing() {
    let devices = parse(json!({"Adapters": [
        adapter("Intel integrated", 0x8086, "00000000:00100001", 0),
        {"Name": "Intel unknown", "VendorId": 0x8086, "Luid": "00000000:00100002", "DedicatedVideoMemory": null},
        adapter("AMD integrated", 0x1002, "00000000:00100003", 0),
        adapter("Intel Arc", 0x8086, "00000000:00100004", 8_000_000_000),
    ]}));
    assert_eq!(devices[0].memory_kind, "shared");
    assert_eq!(devices[1].memory_kind, "unknown");
    assert_eq!(devices[2].memory_kind, "shared");
    assert_eq!(devices[3].memory_kind, "dedicated");
}

#[test]
fn sums_processes_per_physical_engine_then_uses_busiest_engine() {
    let devices = parse(json!({
        "Adapters": [adapter("Intel Arc", 0x8086, "00000000:00123456", 8_000_000_000)],
        "Engines": [
            engine("pid_10_luid_0x00000000_0x00123456_phys_0_eng_0_engtype_3D", json!(40)),
            engine("pid_11_luid_0x00000000_0x00123456_phys_0_eng_0_engtype_3D", json!(25)),
            engine("pid_10_luid_0x00000000_0x00123456_phys_0_eng_1_engtype_VideoEncode", json!(20)),
            engine("pid_11_luid_0x00000000_0x00123456_phys_0_eng_1_engtype_VideoEncode", json!(15)),
            engine("pid_10_luid_0x00000000_0x00123456_phys_1_eng_1_engtype_VideoEncode", json!(38)),
            engine("pid_11_luid_0x00000000_0x00123456_phys_0_eng_2_engtype_Copy", json!(10)),
        ],
    }));
    assert_eq!(devices[0].utilization_percent, Some(65.0));
    assert_eq!(devices[0].encoder_utilization_percent, Some(38.0));
}

#[test]
fn clamps_same_engine_process_sum_and_accepts_aggregated_names() {
    let devices = parse(json!({
        "Adapters": [adapter("AMD", 0x1002, "00000000:00123456", 4096)],
        "Engines": [
            engine("pid_10_luid_0x00000000_0x00123456_phys_0_eng_1_engtype_VideoEncode", json!(72)),
            engine("pid_11_luid_0x00000000_0x00123456_phys_0_eng_1_engtype_VideoEncode", json!(48)),
            engine("luid_0x00000000_0x00123456_phys_0_eng_0_engtype_3D", json!(125)),
        ],
    }));
    assert_eq!(devices[0].utilization_percent, Some(100.0));
    assert_eq!(devices[0].encoder_utilization_percent, Some(100.0));
}

#[test]
fn decode_and_video_processing_are_never_reported_as_encode() {
    let devices = parse(json!({
        "Adapters": [adapter("Intel", 0x8086, "00000000:00123456", 4096)],
        "Engines": [
            engine("pid_10_luid_0x00000000_0x00123456_phys_0_eng_0_engtype_VideoDecode", json!(88)),
            engine("pid_10_luid_0x00000000_0x00123456_phys_0_eng_1_engtype_VideoProcessing", json!(20)),
            engine("pid_10_luid_0x00000000_0x00123456_phys_0_eng_2_engtype_VideoEncodeExtra", json!(31)),
        ],
    }));
    assert_eq!(devices[0].utilization_percent, Some(88.0));
    assert_eq!(devices[0].encoder_utilization_percent, None);
    assert!(devices[0].error.as_deref().unwrap().contains("VideoEncode"));
}

#[test]
fn dedicated_memory_excludes_shared_and_preserves_64_bit_total() {
    let devices = parse(json!({
        "Adapters": [adapter("AMD", 0x1002, "00000000:00123456", 24 * 1024 * 1024 * 1024)],
        "Memory": [
            {"Name": "luid_0x00000000_0x00123456_phys_0", "DedicatedUsage": 7_000_000_000_u64, "SharedUsage": 9_000_000_000_u64},
            {"Name": "luid_0x00000000_0x00123456_phys_0", "DedicatedUsage": 7_000_000_000_u64},
            {"Name": "luid_0x00000000_0x00123456_phys_1", "DedicatedUsage": 100},
            {"Name": "pid_15_luid_0x00000000_0x00123456_phys_2", "DedicatedUsage": 999, "SharedUsage": 500},
        ],
    }));
    assert_eq!(devices[0].memory_used_bytes, Some(7_000_000_100));
    assert_eq!(devices[0].memory_total_bytes, Some(25_769_803_776));
}

#[test]
fn absent_counters_and_zero_dedicated_total_remain_unavailable() {
    let devices = parse(json!({
        "Adapters": [adapter("Intel UHD", 0x8086, "00000000:00123456", 0)],
        "Engines": [],
        "Memory": [],
    }));
    let device = &devices[0];
    assert_eq!(device.utilization_percent, None);
    assert_eq!(device.encoder_utilization_percent, None);
    assert_eq!(device.memory_used_bytes, None);
    assert_eq!(device.memory_total_bytes, None);
    assert_eq!(device.render_node, None);
    let error = device.error.as_deref().unwrap();
    assert!(error.contains("引擎利用率"));
    assert!(error.contains("VideoEncode"));
    assert!(error.contains("共享内存"));
}

#[test]
fn idle_counters_are_zero_and_driver_failures_keep_valid_metrics() {
    let devices = parse(json!({
        "Adapters": [adapter("NVIDIA", 0x10de, "00000000:00123456", 4096)],
        "Engines": [engine("luid_0x00000000_0x00123456_phys_0_eng_1_engtype_VideoEncode", json!(0))],
        "Memory": [{"Name": "luid_0x00000000_0x00123456_phys_0", "DedicatedUsage": 0}],
        "MemoryError": "provider query failed",
    }));
    assert_eq!(devices[0].utilization_percent, Some(0.0));
    assert_eq!(devices[0].encoder_utilization_percent, Some(0.0));
    assert_eq!(devices[0].memory_used_bytes, Some(0));
    assert!(
        devices[0]
            .error
            .as_deref()
            .unwrap()
            .contains("provider query failed")
    );
}

#[test]
fn malformed_samples_do_not_join_or_synthesize_zero_metrics() {
    let devices = parse(json!({
        "Adapters": [adapter("AMD", 0x1002, "00000000:00123456", 4096)],
        "Engines": [
            engine("_Total", json!(90)),
            engine("pid_x_luid_0x00000000_0x00123456_phys_0_eng_1_engtype_VideoEncode", json!(10)),
            engine("pid_10_luid_0x000000000_0x00123456_phys_0_eng_1_engtype_VideoEncode", json!(10)),
            engine("pid_10_luid_0x00000000_0x00123456_phys_x_eng_1_engtype_VideoEncode", json!(10)),
            engine("pid_10_luid_0x00000000_0x00123456_phys_0_eng_x_engtype_VideoEncode", json!(10)),
            engine("pid_10_luid_0x00000000_0x00123456_phys_0_eng_1_engtype_VideoEncode", json!(-1)),
            engine("pid_10_luid_0x00000000_0x00123456_phys_0_eng_1_engtype_VideoEncode", Value::Null),
            engine("pid_10_luid_0x00000001_0x00123456_phys_0_eng_1_engtype_VideoEncode", json!(42)),
        ],
        "Memory": [{"Name": "luid_0x00000000_0x00123456_phys_0", "DedicatedUsage": null, "SharedUsage": 999}],
    }));
    assert_eq!(devices[0].utilization_percent, None);
    assert_eq!(devices[0].encoder_utilization_percent, None);
    assert_eq!(devices[0].memory_used_bytes, None);
}

#[test]
fn duplicate_adapters_keep_a_single_luid_identity() {
    let devices = parse(json!({
        "Adapters": [
            adapter("Intel", 0x8086, "0:123456", 4096),
            adapter("Intel duplicate", 0x8086, "00000000:00123456", 4096),
        ],
    }));
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].id, "windows:00000000:00123456");
}

#[test]
fn invalid_inventory_luid_and_dxgi_errors_require_safe_fallback() {
    let invalid = json!({"Adapters": [adapter("Intel", 0x8086, "invalid", 4096)]});
    assert!(
        parse_snapshot(&serde_json::to_vec(&invalid).unwrap())
            .unwrap_err()
            .contains("LUID")
    );
    let failed = json!({"InventoryError": "DXGI unavailable"});
    assert!(
        parse_snapshot(&serde_json::to_vec(&failed).unwrap())
            .unwrap_err()
            .contains("DXGI unavailable")
    );
    assert!(parse_snapshot(b"not JSON").is_err());
}

#[test]
fn utf8_bom_and_missing_optional_values_are_tolerated() {
    let value = json!({"Adapters": [{"Name": "", "VendorId": 0x10de, "Luid": "0:1"}]});
    let mut bytes = vec![0xef, 0xbb, 0xbf];
    bytes.extend(serde_json::to_vec(&value).unwrap());
    let devices = parse_snapshot(&bytes).unwrap();
    assert_eq!(devices[0].name, "NVIDIA GPU");
    assert_eq!(devices[0].memory_total_bytes, None);
}
