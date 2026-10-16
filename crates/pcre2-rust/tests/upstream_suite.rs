//! Run PCRE2's unmodified reference test programs against the Rust static library.
//!
//! This deliberately uses C only for independent test clients, never for the engine.
//! Provide PCRE2_UPSTREAM_SOURCE, PCRE2_ORACLE_BUILD and PCRE2_RUST_STATICLIB.
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn required_path(name: &str) -> PathBuf {
    let value = env::var_os(name).unwrap_or_else(|| {
        panic!("set {name} to the independently prepared PCRE2 10.49 reference artifact")
    });
    fs::canonicalize(value).unwrap_or_else(|error| panic!("{name}: {error}"))
}

fn run(command: &mut Command, log: &Path) -> Output {
    let description = format!("{command:?}");
    let output = command
        .output()
        .unwrap_or_else(|error| panic!("{description}: {error}"));
    let mut bytes = format!("{description}\nstatus={}\n", output.status).into_bytes();
    bytes.extend_from_slice(&output.stdout);
    bytes.extend_from_slice(&output.stderr);
    fs::write(log, bytes).unwrap();
    assert!(
        output.status.success(),
        "{description} failed; inspect {}",
        log.display()
    );
    output
}

fn snapshot_data(directory: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut data = BTreeMap::new();
    for item in fs::read_dir(directory).unwrap() {
        let path = item.unwrap().path();
        if path.is_file() {
            data.insert(path.clone(), fs::read(path).unwrap());
        }
    }
    data
}

fn public_functions(header: &str) -> BTreeSet<String> {
    let section = header
        .split("/* List the generic forms of all other functions")
        .nth(1)
        .expect("PCRE2 public declaration section")
        .split("/* Define macros that generate width-specific names")
        .next()
        .unwrap();
    let mut functions = BTreeSet::new();
    for suffix in section.split("pcre2_").skip(1) {
        let name_length = suffix
            .bytes()
            .take_while(|value| value.is_ascii_alphanumeric() || *value == b'_')
            .count();
        if suffix[name_length..].trim_start().starts_with('(') {
            functions.insert(format!("pcre2_{}", &suffix[..name_length]));
        }
    }
    assert_eq!(
        functions.len(),
        79,
        "audit a changed public API before updating its count"
    );
    functions
}

fn assert_complete_exports(source: &Path, library: &Path, directory: &Path) {
    let header = fs::read_to_string(source.join("src/pcre2.h.generic")).unwrap();
    let mut expected = BTreeSet::new();
    for name in public_functions(&header) {
        for width in [8, 16, 32] {
            expected.insert(format!("{name}_{width}"));
        }
    }
    for name in [
        "pcre2_regcomp",
        "pcre2_regexec",
        "pcre2_regerror",
        "pcre2_regfree",
    ] {
        expected.insert(name.to_owned());
    }
    assert_eq!(expected.len(), 241);
    let output = run(
        Command::new("nm")
            .args(["-g", "--defined-only"])
            .arg(library),
        &directory.join("exported-symbols.log"),
    );
    let actual: BTreeSet<_> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.split_whitespace().last())
        .map(str::to_owned)
        .collect();
    let missing: Vec<_> = expected.difference(&actual).collect();
    assert!(
        missing.is_empty(),
        "missing public PCRE2 symbols: {missing:?}"
    );
}

fn compile_client(
    source: &Path,
    reference: &Path,
    library: &Path,
    directory: &Path,
    name: &str,
) -> PathBuf {
    let binary = directory.join(name);
    let compiler = env::var_os("CC").unwrap_or_else(|| "cc".into());
    let mut command = Command::new(compiler);
    command.args([
        "-O3",
        "-DNDEBUG",
        "-std=gnu99",
        "-DHAVE_CONFIG_H",
        "-DPCRE2_STATIC",
        "-D_GNU_SOURCE",
    ]);
    for include in [
        reference.join("interface"),
        reference.join("src"),
        source.join("src"),
    ] {
        command.arg("-I").arg(include);
    }
    command
        .arg(source.join(format!("src/{name}.c")))
        .arg(library);
    // These are the Rust static library's platform dependencies and the
    // upstream test client's readline dependency, not native PCRE2 libraries.
    command.args(["-ldl", "-lpthread", "-lm"]);
    if name == "pcre2test" {
        command.arg("-lreadline");
    }
    command.arg("-o").arg(&binary);
    run(&mut command, &directory.join(format!("build-{name}.log")));
    binary
}

fn serialization_interoperability(reference: &Path, library: &Path, directory: &Path) {
    let rustc = env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let source =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/support/serialization_client.rs");
    let original = directory.join("serialize-reference");
    let port = directory.join("serialize-rust");
    run(
        Command::new(&rustc)
            .args(["--edition", "2021", "-C", "opt-level=3"])
            .arg(&source)
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
        &directory.join("build-serialize-reference.log"),
    );
    run(
        Command::new(&rustc)
            .args(["--edition", "2021", "-C", "opt-level=3"])
            .arg(&source)
            .arg("-C")
            .arg(format!("link-arg={}", library.display()))
            .arg("-o")
            .arg(&port),
        &directory.join("build-serialize-rust.log"),
    );
    for (name, encoder, decoder) in [
        ("C-to-Rust", &original, &port),
        ("Rust-to-C", &port, &original),
    ] {
        let patterns = directory.join(name);
        let encoded = run(
            Command::new(encoder).arg("write").arg(&patterns),
            &directory.join(format!("serialize-{name}-encode.log")),
        );
        let decoded = run(
            Command::new(decoder).arg("read").arg(&patterns),
            &directory.join(format!("serialize-{name}-decode.log")),
        );
        assert_eq!(
            decoded.stdout, encoded.stdout,
            "cross-engine serialized pattern behavior differs: {name}"
        );
    }
}

#[test]
#[ignore = "requires the upstream 10.49 source/oracle build and a release Rust staticlib"]
fn all_unmodified_upstream_syntax_api_jit_and_posix_suites() {
    let source = required_path("PCRE2_UPSTREAM_SOURCE");
    let reference = required_path("PCRE2_ORACLE_BUILD");
    let library = required_path("PCRE2_RUST_STATICLIB");
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = env::temp_dir().join(format!(
        "biliup-pcre2-rust-upstream-{}-{timestamp}",
        std::process::id()
    ));
    fs::create_dir_all(&directory).unwrap();
    eprintln!("PCRE2 regression artifacts: {}", directory.display());
    let fixtures = snapshot_data(&source.join("testdata"));
    assert!(
        fixtures.len() > 80,
        "incomplete official reference testdata"
    );
    for number in 1..=29 {
        assert!(source.join(format!("testdata/testinput{number}")).is_file());
    }
    assert_complete_exports(&source, &library, &directory);
    let pcre2test = compile_client(&source, &reference, &library, &directory, "pcre2test");
    let jit = compile_client(&source, &reference, &library, &directory, "pcre2_jit_test");
    let posix = compile_client(&source, &reference, &library, &directory, "pcre2posix_test");
    let config = run(
        Command::new(&pcre2test).arg("-C"),
        &directory.join("configuration.log"),
    );
    let config = String::from_utf8_lossy(&config.stdout);
    for capability in [
        "10.49",
        "8-bit support",
        "16-bit support",
        "32-bit support",
        "UTF and UCP support",
        "Unicode version 17.0.0",
        "Just-in-time compiler support",
        "Can allocate executable memory: Yes",
    ] {
        assert!(
            config.contains(capability),
            "missing capability {capability}: {config}"
        );
    }
    // RunTest decides which non-UTF, UTF, UCP, DFA, JIT, POSIX,
    // serialization, conversion and Unicode-property datasets are applicable.
    // Its exact expected-output comparisons are retained without rewriting.
    let regression = run(
        Command::new("sh")
            .args(["-c", ". \"$srcdir/RunTest\""])
            .env("srcdir", &source)
            .env("pcre2test", &pcre2test)
            .current_dir(&directory),
        &directory.join("RunTest.log"),
    );
    let regression = String::from_utf8_lossy(&regression.stdout);
    assert!(
        !regression.contains("Cannot test locale-specific features"),
        "the full upstream suite requires a working French locale; inspect {}",
        directory.join("RunTest.log").display()
    );
    assert_eq!(
        regression
            .matches("Test 3: Locale-specific features (using '")
            .count(),
        3,
        "locale-specific tests must run at every character width"
    );
    run(
        Command::new(jit).current_dir(&directory),
        &directory.join("jit-regressions.log"),
    );
    run(
        Command::new(posix).current_dir(&directory),
        &directory.join("posix-regressions.log"),
    );
    serialization_interoperability(&reference, &library, &directory);
    assert_eq!(
        snapshot_data(&source.join("testdata")),
        fixtures,
        "official reference fixtures changed during validation"
    );
}
