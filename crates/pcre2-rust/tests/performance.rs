//! Compare optimized implementations using an identical independently linked Rust client.
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug)]
struct Sample {
    nanoseconds: u128,
    iterations: u128,
    checksum: u128,
}

fn required_path(name: &str) -> PathBuf {
    fs::canonicalize(env::var_os(name).unwrap_or_else(|| panic!("set {name}")))
        .unwrap_or_else(|error| panic!("{name}: {error}"))
}

fn checked(command: &mut Command, log: &Path) -> Vec<u8> {
    let output = command
        .output()
        .unwrap_or_else(|error| panic!("{command:?}: {error}"));
    let mut contents = output.stdout.clone();
    contents.extend_from_slice(&output.stderr);
    fs::write(log, contents).unwrap();
    assert!(
        output.status.success(),
        "{command:?} failed; see {}",
        log.display()
    );
    output.stdout
}

fn samples(program: &Path, log: &Path) -> BTreeMap<String, Sample> {
    let output = checked(&mut Command::new(program), log);
    let mut samples = BTreeMap::new();
    for line in std::str::from_utf8(&output).unwrap().lines() {
        let columns: Vec<_> = line.split('\t').collect();
        assert_eq!(columns.len(), 4, "invalid measurement: {line}");
        let sample = Sample {
            nanoseconds: columns[1].parse().unwrap(),
            iterations: columns[2].parse().unwrap(),
            checksum: columns[3].parse().unwrap(),
        };
        assert!(sample.nanoseconds > 0 && sample.iterations > 0);
        assert!(samples.insert(columns[0].to_owned(), sample).is_none());
    }
    assert_eq!(
        samples.len(),
        45,
        "every workload must run at all three widths"
    );
    samples
}

fn median(mut values: Vec<u128>) -> u128 {
    values.sort_unstable();
    values[values.len() / 2]
}

#[test]
#[ignore = "requires an optimized Rust staticlib and the independent optimized C oracle build"]
fn optimized_compile_interpreter_jit_dfa_and_utility_performance() {
    let reference = required_path("PCRE2_ORACLE_BUILD");
    let library = required_path("PCRE2_RUST_STATICLIB");
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = env::temp_dir().join(format!(
        "biliup-pcre2-performance-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir_all(&directory).unwrap();
    eprintln!("PCRE2 performance artifacts: {}", directory.display());
    let workload =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/support/performance_workload.rs");
    let rustc = env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let original = directory.join("reference-client");
    let port = directory.join("rust-client");
    checked(
        Command::new(&rustc)
            .args(["--edition", "2021", "-C", "opt-level=3"])
            .arg(&workload)
            .arg("-L")
            .arg(format!("native={}", reference.display()))
            .args([
                "-l",
                "static=pcre2-8",
                "-l",
                "static=pcre2-16",
                "-l",
                "static=pcre2-32",
            ])
            .arg("-o")
            .arg(&original),
        &directory.join("build-reference.log"),
    );
    checked(
        Command::new(&rustc)
            .args(["--edition", "2021", "-C", "opt-level=3"])
            .arg(&workload)
            .arg("-C")
            .arg(format!("link-arg={}", library.display()))
            .arg("-o")
            .arg(&port),
        &directory.join("build-rust.log"),
    );

    // Warm both engines and alternate sample ordering to limit systematic CPU
    // temperature/cache bias. Every timed operation also verifies its result.
    samples(&original, &directory.join("reference-warmup.tsv"));
    samples(&port, &directory.join("rust-warmup.tsv"));
    let mut reference_times: BTreeMap<String, Vec<u128>> = BTreeMap::new();
    let mut rust_times: BTreeMap<String, Vec<u128>> = BTreeMap::new();
    for round in 0..5 {
        let reference_log = directory.join(format!("reference-{round}.tsv"));
        let rust_log = directory.join(format!("rust-{round}.tsv"));
        let (original_sample, port_sample) = if round % 2 == 0 {
            (
                samples(&original, &reference_log),
                samples(&port, &rust_log),
            )
        } else {
            let port_sample = samples(&port, &rust_log);
            (samples(&original, &reference_log), port_sample)
        };
        assert_eq!(
            original_sample.keys().collect::<Vec<_>>(),
            port_sample.keys().collect::<Vec<_>>()
        );
        for (name, original) in original_sample {
            let port = port_sample.get(&name).unwrap();
            assert_eq!(port.iterations, original.iterations, "workload {name}");
            assert_eq!(
                port.checksum, original.checksum,
                "functional mismatch in {name}"
            );
            reference_times
                .entry(name.clone())
                .or_default()
                .push(original.nanoseconds);
            rust_times.entry(name).or_default().push(port.nanoseconds);
        }
    }
    let mut report = "workload\tC_median_ns\tRust_median_ns\tRust_over_C\n".to_owned();
    let mut ratios = Vec::new();
    let mut regressions = Vec::new();
    for (name, original) in reference_times {
        let original = median(original);
        let port = median(rust_times.remove(&name).unwrap());
        let ratio = port as f64 / original as f64;
        ratios.push(ratio);
        report.push_str(&format!("{name}\t{original}\t{port}\t{ratio:.4}\n"));
        if ratio > 1.50 {
            regressions.push(format!("{name}: {ratio:.3}×"));
        }
    }
    let geometric_mean =
        (ratios.iter().map(|ratio| ratio.ln()).sum::<f64>() / ratios.len() as f64).exp();
    report.push_str(&format!("geometric_mean\t\t\t{geometric_mean:.4}\n"));
    fs::write(directory.join("comparison.tsv"), &report).unwrap();
    eprintln!("{report}");
    assert!(
        regressions.is_empty(),
        "workloads exceed the fixed 1.50× regression ceiling: {regressions:?}; {}",
        directory.display()
    );
    assert!(
        geometric_mean <= 1.25,
        "aggregate regression {geometric_mean:.3}× exceeds 1.25×; {}",
        directory.display()
    );
}
