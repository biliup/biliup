//! Process-wide effects of calling the core in-process. Kept in its own test
//! binary: it counts the process's file descriptors and changes its locale.

use danmaku_factory::{ConvertOptions, ErrorKind, convert_xml_to_ass};

#[cfg(target_os = "linux")]
fn open_descriptors() -> usize {
    std::fs::read_dir("/proc/self/fd").unwrap().count()
}

#[test]
fn process_state() {
    repeated_conversions_release_files_and_memory();
    comma_decimal_locale_is_refused();
}

/// Successful and failing conversions alike release every FILE (an empty
/// input used to leak one, patch 0001).
fn repeated_conversions_release_files_and_memory() {
    let dir = tempfile::tempdir().unwrap();
    let good = dir.path().join("good.xml");
    std::fs::write(&good, "<i><d p=\"1,1,25,16777215\">a</d><d p=\"2,7,25,0\">[0,0,\"1-1\",3,\"sp\",0,0,1,1,500,0,true,\"\",1]</d></i>").unwrap();
    let empty = dir.path().join("empty.xml");
    std::fs::write(&empty, "<i></i>").unwrap();
    #[cfg(target_os = "linux")]
    let before = open_descriptors();
    for round in 0..200 {
        let output = dir.path().join(format!("{round}.ass"));
        convert_xml_to_ass(&good, &output, &ConvertOptions::default()).unwrap();
        std::fs::remove_file(&output).unwrap();
        let error = convert_xml_to_ass(&empty, &output, &ConvertOptions::default()).unwrap_err();
        assert_eq!(error.kind, ErrorKind::NoDanmaku);
    }
    #[cfg(target_os = "linux")]
    assert_eq!(open_descriptors(), before, "file descriptors leaked");
}

/// printf("%.2f") and atof() follow LC_NUMERIC. biliup never calls
/// setlocale, but an embedding host might; the core then refuses to write
/// "100,00" into the ASS file.
fn comma_decimal_locale_is_refused() {
    #[cfg(unix)]
    {
        use std::ffi::{CStr, c_char};
        /// `decimal_point` is the first member of `struct lconv` everywhere.
        #[repr(C)]
        struct Lconv {
            decimal_point: *const c_char,
        }
        unsafe extern "C" {
            fn setlocale(category: i32, locale: *const c_char) -> *mut c_char;
            fn localeconv() -> *const Lconv;
        }
        // LC_NUMERIC is 1 on glibc and musl, 4 on macOS and the BSDs.
        let lc_numeric = if cfg!(target_vendor = "apple") || cfg!(target_os = "freebsd") { 4 } else { 1 };
        let locales = [c"de_DE.UTF-8", c"de_DE.utf8", c"fr_FR.UTF-8", c"ru_RU.UTF-8"];
        // musl accepts any name but always formats numbers with '.'.
        let Some(locale) = locales.iter().find(|name| unsafe {
            !setlocale(lc_numeric, name.as_ptr()).is_null()
                && CStr::from_ptr((*localeconv()).decimal_point) == c","
        }) else {
            unsafe { setlocale(lc_numeric, c"C".as_ptr()) };
            use std::io::Write;
            let _ = writeln!(std::io::stderr(), "SKIPPED locale check: no comma-decimal locale available");
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("in.xml");
        std::fs::write(&input, "<i><d p=\"1.5,1,25,16777215\">a</d></i>").unwrap();
        let output = dir.path().join("out.ass");
        let error = convert_xml_to_ass(&input, &output, &ConvertOptions::default()).unwrap_err();
        unsafe { setlocale(lc_numeric, c"C".as_ptr()) };
        assert_eq!(error.kind, ErrorKind::Locale, "{locale:?}: {error}");
        assert!(!output.exists());
        convert_xml_to_ass(&input, &output, &ConvertOptions::default()).unwrap();
    }
}
