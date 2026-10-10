//! Builds the vendored sources into static libraries linked into the crate
//! that depends on this one. Only committed files are used: nothing is
//! downloaded and no system DanmakuFactory or PCRE2 is looked up.

use std::env;
use std::path::{Path, PathBuf};

/// The conversion core: everything main() of the CLI calls for XML -> ASS.
const CORE_SOURCES: &[&str] = &[
    "XmlFile.c",
    "AssFile/AssFile.c",
    "AssFile/AssStringProcessing.c",
    "List/DanmakuFactoryList.c",
    "String/DanmakuFactoryString.c",
    "FileUtil/FileUtil.c",
];

/// PCRE2's 8-bit library without JIT (its CMakeLists.txt source list).
const PCRE2_SOURCES: &[&str] = &[
    "pcre2_auto_possess.c",
    "pcre2_chartables.c",
    "pcre2_chkdint.c",
    "pcre2_compile.c",
    "pcre2_compile_cgroup.c",
    "pcre2_compile_class.c",
    "pcre2_config.c",
    "pcre2_context.c",
    "pcre2_convert.c",
    "pcre2_dfa_match.c",
    "pcre2_error.c",
    "pcre2_extuni.c",
    "pcre2_find_bracket.c",
    "pcre2_jit_compile.c",
    "pcre2_maketables.c",
    "pcre2_match.c",
    "pcre2_match_data.c",
    "pcre2_match_next.c",
    "pcre2_newline.c",
    "pcre2_ord2utf.c",
    "pcre2_pattern_info.c",
    "pcre2_script_run.c",
    "pcre2_serialize.c",
    "pcre2_string_utils.c",
    "pcre2_study.c",
    "pcre2_substitute.c",
    "pcre2_substring.c",
    "pcre2_tables.c",
    "pcre2_ucd.c",
    "pcre2_valid_utf.c",
    "pcre2_xclass.c",
];

fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let core = root.join("vendor/DanmakuFactory/src");
    let pcre2 = root.join("vendor/pcre2/src");
    let csrc = root.join("csrc");
    for path in ["build.rs", "csrc", "vendor"] {
        println!("cargo:rerun-if-changed={path}");
    }
    let target_os = env::var("CARGO_CFG_TARGET_OS").expect("CARGO_CFG_TARGET_OS");

    // Compiled in dependency order (wrapper -> core -> PCRE2) so that linkers
    // that read archives once still resolve every symbol.
    let mut wrapper = build(&target_os);
    namespaced(&mut wrapper, &csrc);
    wrapper
        .include(&core)
        .include(&csrc)
        .file(csrc.join("df_ffi.c"))
        .warnings(true)
        .extra_warnings(true);
    wrapper.compile("biliup_df_ffi");

    // Upstream code is not ours to clean up: its warnings would only bury
    // biliup's own.
    let mut factory = build(&target_os);
    namespaced(&mut factory, &csrc);
    factory
        .include(&core)
        .include(&pcre2)
        .define("PCRE2_CODE_UNIT_WIDTH", "8")
        .define("PCRE2_STATIC", None)
        .files(CORE_SOURCES.iter().map(|file| core.join(file)))
        .warnings(false)
        .cargo_warnings(false);
    factory.compile("biliup_df_core");

    let mut regex = build(&target_os);
    regex
        .include(&pcre2)
        .define("HAVE_CONFIG_H", None)
        .define("PCRE2_CODE_UNIT_WIDTH", "8")
        .define("PCRE2_STATIC", None)
        .define("SUPPORT_PCRE2_8", None)
        .define("SUPPORT_UNICODE", None)
        .define("HAVE_STDLIB_H", None)
        .define("HAVE_STRING_H", None)
        .define("HAVE_LIMITS_H", None)
        .define("HAVE_STDINT_H", None)
        .define("HAVE_INTTYPES_H", None)
        .files(PCRE2_SOURCES.iter().map(|file| pcre2.join(file)))
        .warnings(false)
        .cargo_warnings(false);
    if target_os == "windows" {
        regex.define("HAVE_WINDOWS_H", None);
    }
    regex.compile("biliup_df_pcre2");

    // FileUtil.c also contains GetCommandLineUTF8() (CommandLineToArgvW).
    // Nothing needs libm: fabs/isfinite are builtins (and on musl an explicit
    // -lm would pick up the host's glibc libm.a).
    if target_os == "windows" {
        println!("cargo:rustc-link-lib=shell32");
    }
    println!("cargo:include={}", csrc.display());
}

/// Settings shared by every C file. The flags make output independent of the
/// platform: the CLI baseline was built for x86 (signed `char`, no fused
/// multiply-add) and MSVC must read the UTF-8 sources and string literals as
/// UTF-8 whatever the system code page.
fn build(target_os: &str) -> cc::Build {
    let mut build = cc::Build::new();
    // Same code at every profile, and fast enough for hours of danmaku in
    // debug builds and tests.
    build.opt_level(2);
    if build.get_compiler().is_like_msvc() {
        build
            .flag("/utf-8")
            .flag("/std:c11")
            .define("_CRT_SECURE_NO_WARNINGS", None);
    } else {
        build
            .flag("-std=gnu11")
            .flag("-fsigned-char")
            .flag("-ffp-contract=off");
        if target_os != "windows" {
            // Not exported from the final executable or Python module, so the
            // copies here never interpose a system PCRE2 loaded by others.
            build.flag("-fvisibility=hidden");
        }
    }
    build
}

/// Prefixes the core's generic global names (match, trim, ...).
fn namespaced(build: &mut cc::Build, csrc: &Path) {
    let header = csrc.join("df_namespace.h");
    if build.get_compiler().is_like_msvc() {
        build.flag(format!("/FI{}", header.display()));
    } else {
        build.flag("-include").flag(header.as_os_str());
    }
}
