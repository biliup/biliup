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
    assert_eq!(normalize_pci_bus("00000000:03:00.0"), "0000:03:00.0");
    assert_eq!(normalize_pci_bus("0000:03:00.0"), "0000:03:00.0");
    assert_ne!(
        normalize_pci_bus("0000:03:00.0"),
        normalize_pci_bus("0001:03:00.0")
    );
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
