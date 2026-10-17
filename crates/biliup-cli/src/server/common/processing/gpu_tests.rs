use super::*;

#[test]
fn parses_nvidia_rows_even_when_encoder_is_separately_unavailable() {
    let rows = parse_nvidia_csv(
        "00000000:03:00.0, 0, NVIDIA RTX, 42, 1024, 8192\n00000000:04:00.0, 1, NVIDIA Old, [Not Supported], 0, 4096\n",
    );
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].bus_id, "00000000:03:00.0");
    assert_eq!(rows[0].gpu, Some(42.0));
    assert_eq!(rows[0].encoder, None);
    assert_eq!(rows[0].mem_used, Some(1024 * 1024 * 1024));
    assert_eq!(rows[1].gpu, None);
}

#[test]
fn parses_nvidia_dmon_encoder_column() {
    let sample = "# gpu   curr   sm   mem   enc   dec\n0      1200   35   20    17    0\n1      900    10   12    4     0\n";
    let rows = parse_nvidia_dmon(sample).unwrap();
    assert_eq!(rows.get("0"), Some(&17.0));
    assert_eq!(rows.get("1"), Some(&4.0));
}

#[test]
fn parses_nvidia_query_xml_encoder_memory_and_pci_identity() {
    let xml = br#"<?xml version="1.0"?>
<!DOCTYPE nvidia_smi_log SYSTEM "nvsmi_device_v13.dtd">
<nvidia_smi_log><gpu id="00000000:00:04.0">
<product_name>Tesla T4</product_name><minor_number>0</minor_number>
<pci><pci_bus_id>00000000:00:04.0</pci_bus_id></pci>
<fb_memory_usage><total>15360 MiB</total><used>218 MiB</used></fb_memory_usage>
<utilization><gpu_util>2 %</gpu_util><encoder_util>27 %</encoder_util></utilization>
</gpu></nvidia_smi_log>"#;
    let rows = parse_nvidia_xml(xml);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].bus_id, "00000000:00:04.0");
    assert_eq!(rows[0].index, "0");
    assert_eq!(rows[0].name, "Tesla T4");
    assert_eq!(rows[0].gpu, Some(2.0));
    assert_eq!(rows[0].encoder, Some(27.0));
    assert_eq!(rows[0].mem_used, Some(218 * 1024 * 1024));
    assert_eq!(rows[0].mem_total, Some(15360 * 1024 * 1024));
}

#[test]
fn nvidia_xml_keeps_unsupported_metrics_null() {
    let xml = br#"<nvidia_smi_log><gpu id="00000000:00:04.0">
<product_name>Tesla T4</product_name><minor_number>0</minor_number>
<utilization><gpu_util>N/A</gpu_util><encoder_util>N/A</encoder_util></utilization>
</gpu></nvidia_smi_log>"#;
    let rows = parse_nvidia_xml(xml);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].gpu, None);
    assert_eq!(rows[0].encoder, None);
    assert_eq!(rows[0].mem_used, None);
}

#[test]
fn parses_intel_array_choosing_last_sample_and_exact_video_engine() {
    let json = serde_json::json!([
        {"engines": {"Video": {"busy": 12.0}, "VideoEnhance": {"busy": 55.0}}},
        {"engines": {"Render/3D": {"busy": 88.0}, "Video": {"busy": 21.0}}}
    ]);
    let metrics = parse_intel_metrics(&json);
    assert_eq!(metrics.overall, None);
    assert_eq!(metrics.video, Some(21.0));
    assert_eq!(metrics.engines.len(), 2);
    assert_eq!(metrics.engines[0].name, "Render/3D");
    assert_eq!(metrics.engines[0].utilization_percent, Some(88.0));
    assert_eq!(metrics.engines[1].name, "Video");
    assert_eq!(metrics.engines[1].utilization_percent, Some(21.0));
}

#[test]
fn intel_engine_fixture_keeps_video_instances_separate_and_ignores_similar_names() {
    let metrics = parse_intel_metrics(&serde_json::json!({"engines": {
        "Video/0": {"busy": 12.5},
        "Video/1": {"busy": 35.0},
        "VideoEnhance/0": {"busy": 4.0},
        "Compute/0": {"busy": 75.0},
        "VideoFoo": {"busy": 98.0}
    }}));
    assert_eq!(
        metrics.video, None,
        "multiple video engines have no single aggregate counter"
    );
    assert_eq!(metrics.engines.len(), 4);
    assert_eq!(
        metrics
            .engines
            .iter()
            .filter(|engine| intel_engine_class(&engine.name) == Some(2))
            .count(),
        2
    );
    assert!(
        !metrics
            .engines
            .iter()
            .any(|engine| engine.name == "VideoFoo")
    );
}

#[test]
fn intel_engine_idle_zero_is_distinct_from_missing_or_invalid_counter() {
    let metrics = parse_intel_metrics(&serde_json::json!({"engines": {
        "Render/3D": {"busy": 0.0},
        "Blitter": {},
        "Video": {"busy": 0.0},
        "VideoEnhance": {"busy": -1.0},
        "Compute": {"busy": 101.0},
    }}));
    assert_eq!(metrics.video, Some(0.0));
    assert_eq!(
        metrics
            .engines
            .iter()
            .map(|engine| engine.utilization_percent)
            .collect::<Vec<_>>(),
        vec![Some(0.0), None, Some(0.0), None, None]
    );
}

fn i915_region(class: u16, probed: u64, unallocated: u64) -> [u8; 88] {
    let mut region = [0u8; 88];
    region[..2].copy_from_slice(&class.to_ne_bytes());
    region[8..16].copy_from_slice(&probed.to_ne_bytes());
    region[16..24].copy_from_slice(&unallocated.to_ne_bytes());
    region
}

fn i915_memory_fixture(regions: &[[u8; 88]]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(16 + regions.len() * 88);
    bytes.extend_from_slice(&(regions.len() as u32).to_ne_bytes());
    bytes.resize(16, 0);
    for region in regions {
        bytes.extend_from_slice(region);
    }
    bytes
}

#[test]
fn parses_i915_memory_regions_without_mislabeling_shared_memory_as_vram() {
    let fixture = i915_memory_fixture(&[
        i915_region(0, 16 * 1024 * 1024 * 1024, 8 * 1024 * 1024 * 1024),
        i915_region(1, 8 * 1024 * 1024 * 1024, 3 * 1024 * 1024 * 1024),
        i915_region(1, 4 * 1024 * 1024 * 1024, 2 * 1024 * 1024 * 1024),
    ]);
    assert_eq!(
        parse_i915_memory(&fixture, true),
        Some(IntelMemory {
            dedicated: true,
            total: Some(12 * 1024 * 1024 * 1024),
            used: Some(7 * 1024 * 1024 * 1024),
        })
    );
    assert_eq!(
        parse_i915_memory(&fixture, false),
        Some(IntelMemory {
            dedicated: true,
            total: Some(12 * 1024 * 1024 * 1024),
            used: None,
        })
    );

    let integrated = i915_memory_fixture(&[i915_region(
        0,
        32 * 1024 * 1024 * 1024,
        20 * 1024 * 1024 * 1024,
    )]);
    assert_eq!(
        parse_i915_memory(&integrated, true),
        Some(IntelMemory {
            dedicated: false,
            total: None,
            used: None,
        })
    );
}

#[test]
fn rejects_incomplete_or_unavailable_i915_region_fixtures() {
    assert_eq!(parse_i915_memory(&[], true), None);
    assert_eq!(parse_i915_memory(&[0; 16], true), None);
    let unavailable = i915_memory_fixture(&[i915_region(1, u64::MAX, u64::MAX)]);
    assert_eq!(parse_i915_memory(&unavailable, true), None);
    assert!(!memory_accounting_capability("CapEff:\t0000000000000000\n"));
    assert!(memory_accounting_capability("CapEff:\t0000004000200000\n"));
}

#[test]
fn i915_permission_fallback_never_becomes_zero_used_even_with_ambiguous_privileged_data() {
    let fixture = i915_memory_fixture(&[i915_region(1, 8_522_825_728, 8_522_825_728)]);
    for privileged in [false, true] {
        assert_eq!(
            parse_i915_memory(&fixture, privileged),
            Some(IntelMemory {
                dedicated: true,
                total: Some(8_522_825_728),
                used: None,
            })
        );
    }
    let invalid_remaining = i915_memory_fixture(&[i915_region(1, 100, 101)]);
    assert_eq!(
        parse_i915_memory(&invalid_remaining, true).unwrap().used,
        None
    );
    let mut truncated = fixture.clone();
    truncated.pop();
    assert_eq!(parse_i915_memory(&truncated, true), None);
}

#[cfg(target_os = "linux")]
#[test]
fn i915_query_layout_matches_the_kernel_uapi() {
    assert_eq!(std::mem::size_of::<I915Query>(), 16);
    assert_eq!(std::mem::size_of::<I915QueryItem>(), 24);
}

#[test]
fn missing_or_invalid_intel_counters_stay_unavailable() {
    for value in [
        serde_json::json!({"engines": {"Render/3D": {"busy": 50.0}}}),
        serde_json::json!({"engines": {"Video": {"busy": 101.0}}}),
        serde_json::json!({}),
    ] {
        let metrics = parse_intel_metrics(&value);
        assert_eq!(metrics.overall, None);
        assert_eq!(metrics.video, None);
    }
}

#[test]
fn pci_vendor_mapping_percent_bounds_and_bus_normalization() {
    assert_eq!(
        device_name("intel", Some(0x4682)),
        "Intel UHD Graphics 730 (device 0x4682)"
    );
    assert_eq!(
        normalize_pci_bus("00000000:03:00.0").as_deref(),
        Some("0000:03:00.0")
    );
    assert_eq!(
        normalize_pci_bus("0000:03:00.0").as_deref(),
        Some("0000:03:00.0")
    );
    assert_ne!(
        normalize_pci_bus("0000:03:00.0"),
        normalize_pci_bus("0001:03:00.0")
    );
    for invalid in [
        "0000:03:invalid:data",
        "0000:100:00.0",
        "0000:03:20.0",
        "0000:03:00.8",
        "0000:03:00.g",
    ] {
        assert_eq!(normalize_pci_bus(invalid), None, "accepted {invalid}");
    }
    assert_eq!(parse_pct("100"), Some(100.0));
    assert_eq!(parse_pct("101"), None);
    assert_eq!(parse_pct("N/A"), None);
}

#[test]
fn detects_linux_render_devices_from_sysfs_fixture() {
    let root = tempfile::tempdir().unwrap();
    let render = root.path().join("renderD128").join("device");
    std::fs::create_dir_all(&render).unwrap();
    std::fs::write(render.join("vendor"), "0x8086\n").unwrap();
    std::fs::write(render.join("device"), "0x4682\n").unwrap();
    std::fs::create_dir_all(root.path().join("card0")).unwrap();

    let devices = linux_drm_devices_at(root.path());
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].vendor, "intel");
    assert_eq!(devices[0].name, "Intel UHD Graphics 730 (device 0x4682)");
    assert_eq!(
        devices[0].render_node.as_deref(),
        Some("/dev/dri/renderD128")
    );
    assert_eq!(devices[0].utilization_percent, None);
}

#[cfg(unix)]
#[tokio::test]
async fn timed_out_probe_is_killed_and_reaped() {
    let mut command = Command::new("sh");
    command
        .args(["-c", "sleep 5"])
        .stdout(std::process::Stdio::piped());
    let started = Instant::now();
    let result = bounded_command_output(command, Duration::from_millis(40)).await;
    assert!(result.unwrap_err().contains("超时"));
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[cfg(unix)]
#[tokio::test]
async fn oversized_stderr_is_rejected_before_waiting_for_stdout_eof() {
    let mut command = Command::new("sh");
    command
        .args(["-c", "yes x | head -c 65537 >&2; exec sleep 5"])
        .stderr(std::process::Stdio::piped());
    let started = Instant::now();
    let result = bounded_command_output(command, Duration::from_secs(2)).await;
    let error = result.expect_err("oversized stderr must reject the probe");
    assert!(error.contains("超过 64 KiB"), "actual probe error: {error}");
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "rejection took {:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn hardware_gpu_monitor_nvidia_sustained_encode() {
    if std::env::var("BILIUP_TEST_ENCODER").ok().as_deref() != Some("h264_nvenc") {
        return;
    }

    let before = snapshot().await;
    let before_ids: std::collections::HashSet<_> = before
        .devices
        .iter()
        .filter(|device| device.vendor == "nvidia")
        .map(|device| device.id.clone())
        .collect();
    assert!(
        !before_ids.is_empty(),
        "NVIDIA hardware host must expose a GPU"
    );
    tokio::time::sleep(CACHE_TTL + Duration::from_millis(100)).await;

    let mut encode = crate::tools::ffmpeg_command();
    encode.args([
        "-hide_banner",
        "-loglevel",
        "error",
        "-re",
        "-f",
        "lavfi",
        "-i",
        "testsrc2=size=1280x720:rate=30",
        "-t",
        "15",
        "-c:v",
        "h264_nvenc",
    ]);
    encode
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);
    if let Ok(device) = std::env::var("BILIUP_TEST_ENCODER_DEVICE") {
        if !device.is_empty() && device.bytes().all(|byte| byte.is_ascii_digit()) {
            encode.args(["-gpu", &device]);
        }
    }
    encode.args(["-f", "null", "-"]);
    let mut child = encode
        .spawn()
        .expect("ffmpeg must be installed when NVIDIA hardware test is enabled");

    let mut samples = Vec::new();
    for _ in 0..3 {
        tokio::time::sleep(CACHE_TTL + Duration::from_millis(100)).await;
        samples.push(snapshot().await);
    }
    let workload_running = child.try_wait().unwrap().is_none();
    let _ = child.kill().await;
    let _ = child.wait().await;
    assert!(
        workload_running,
        "the NVENC workload must keep running while sampling"
    );

    let last = samples.last().expect("collect at least one monitor sample");
    let nvidia: Vec<_> = last
        .devices
        .iter()
        .filter(|device| device.vendor == "nvidia")
        .collect();
    let ids: std::collections::HashSet<_> = nvidia.iter().map(|device| device.id.clone()).collect();
    assert_eq!(
        nvidia.len(),
        ids.len(),
        "GPU inventory must not duplicate PCI devices"
    );
    assert_eq!(ids, before_ids, "GPU inventory must stay stable under load");
    assert!(
        samples
            .windows(2)
            .all(|pair| pair[1].sampled_at > pair[0].sampled_at),
        "fresh samples must advance"
    );
    assert!(
        last.sampled_at > before.sampled_at,
        "monitor sample timestamp must advance"
    );
    assert!(
        samples
            .iter()
            .any(|sample| sample.devices.iter().any(|device| {
                device.vendor == "nvidia"
                    && device
                        .encoder_utilization_percent
                        .is_some_and(|value| value > 0.0)
            })),
        "sustained NVENC workload must produce a positive encoder utilization sample"
    );
    assert!(
        nvidia[0].utilization_percent.is_some(),
        "overall GPU utilization should be reported"
    );
    assert!(
        nvidia[0].memory_used_bytes.is_some() && nvidia[0].memory_total_bytes.is_some(),
        "NVIDIA memory counters should be reported"
    );
}

#[test]
fn malformed_nvidia_xml_never_returns_partial_gpu_rows() {
    assert!(parse_nvidia_xml(br#"<nvidia_smi_log><gpu id="00000000:00:04.0"><minor_number>0</minor_number></gpu><broken>"#).is_empty());
}

#[test]
fn xml_encoder_fallback_joins_by_pci_when_minor_number_differs_or_is_unavailable() {
    let rows = parse_nvidia_csv(
        "00000000:03:00.0, 0, NVIDIA RTX, 2, 218, 15360\n00000000:04:00.0, 1, NVIDIA RTX, 3, 200, 8192\n",
    );
    let metrics = HashMap::from([
        ("pci:0000:03:00.0".to_owned(), 27.),
        ("pci:0000:04:00.0".to_owned(), 15.),
    ]);
    assert_eq!(lookup_nvidia_encoder(&rows[0], &metrics), Some(27.));
    assert_eq!(lookup_nvidia_encoder(&rows[1], &metrics), Some(15.));
    let dmon = HashMap::from([("index:1".to_owned(), 35.)]);
    assert_eq!(lookup_nvidia_encoder(&rows[1], &dmon), Some(35.));
    assert_eq!(lookup_nvidia_encoder(&rows[0], &dmon), None);
}
