//! The same Rust client is linked separately to the C oracle and the Rust port.
use std::ffi::{c_int, c_void};
use std::hint::black_box;
use std::ptr;
use std::time::Instant;

fn measure(name: &str, iterations: usize, mut operation: impl FnMut() -> usize) {
    let started = Instant::now();
    let mut checksum = 0usize;
    for _ in 0..iterations {
        checksum = checksum.wrapping_add(black_box(operation()));
    }
    println!(
        "{name}\t{}\t{iterations}\t{checksum}",
        started.elapsed().as_nanos()
    );
}

macro_rules! width {
    ($module:ident, $bits:literal, $unit:ty) => {
        mod $module {
            use super::*;
            type Unit = $unit;
            unsafe extern "C" {
                #[link_name = concat!("pcre2_compile_", $bits)]
                fn compile(
                    pattern: *const Unit,
                    length: usize,
                    options: u32,
                    error: *mut c_int,
                    offset: *mut usize,
                    context: *mut c_void,
                ) -> *mut c_void;
                #[link_name = concat!("pcre2_code_free_", $bits)]
                fn code_free(code: *mut c_void);
                #[link_name = concat!("pcre2_match_data_create_from_pattern_", $bits)]
                fn data_create(code: *const c_void, context: *mut c_void) -> *mut c_void;
                #[link_name = concat!("pcre2_match_data_free_", $bits)]
                fn data_free(data: *mut c_void);
                #[link_name = concat!("pcre2_match_", $bits)]
                fn regex_match(
                    code: *const c_void,
                    subject: *const Unit,
                    length: usize,
                    start: usize,
                    options: u32,
                    data: *mut c_void,
                    context: *mut c_void,
                ) -> c_int;
                #[link_name = concat!("pcre2_jit_compile_", $bits)]
                fn jit_compile(code: *mut c_void, options: u32) -> c_int;
                #[link_name = concat!("pcre2_jit_match_", $bits)]
                fn jit_match(
                    code: *const c_void,
                    subject: *const Unit,
                    length: usize,
                    start: usize,
                    options: u32,
                    data: *mut c_void,
                    context: *mut c_void,
                ) -> c_int;
                #[link_name = concat!("pcre2_dfa_match_", $bits)]
                fn dfa_match(
                    code: *const c_void,
                    subject: *const Unit,
                    length: usize,
                    start: usize,
                    options: u32,
                    data: *mut c_void,
                    context: *mut c_void,
                    workspace: *mut c_int,
                    size: usize,
                ) -> c_int;
                #[link_name = concat!("pcre2_substitute_", $bits)]
                fn substitute(
                    code: *const c_void,
                    subject: *const Unit,
                    length: usize,
                    start: usize,
                    options: u32,
                    data: *mut c_void,
                    context: *mut c_void,
                    replacement: *const Unit,
                    replacement_length: usize,
                    output: *mut Unit,
                    output_length: *mut usize,
                ) -> c_int;
                #[link_name = concat!("pcre2_serialize_encode_", $bits)]
                fn encode(
                    codes: *const *const c_void,
                    count: i32,
                    bytes: *mut *mut u8,
                    size: *mut usize,
                    context: *mut c_void,
                ) -> i32;
                #[link_name = concat!("pcre2_serialize_decode_", $bits)]
                fn decode(
                    codes: *mut *mut c_void,
                    count: i32,
                    bytes: *const u8,
                    context: *mut c_void,
                ) -> i32;
                #[link_name = concat!("pcre2_serialize_free_", $bits)]
                fn serialized_free(bytes: *mut u8);
                #[link_name = concat!("pcre2_pattern_convert_", $bits)]
                fn convert(
                    pattern: *const Unit,
                    length: usize,
                    options: u32,
                    output: *mut *mut Unit,
                    output_length: *mut usize,
                    context: *mut c_void,
                ) -> c_int;
                #[link_name = concat!("pcre2_converted_pattern_free_", $bits)]
                fn converted_free(pattern: *mut Unit);
            }

            fn units(text: &str) -> Vec<Unit> {
                match $bits {
                    "8" => text.as_bytes().iter().map(|v| *v as Unit).collect(),
                    "16" => text.encode_utf16().map(|v| v as Unit).collect(),
                    _ => text.chars().map(|v| v as u32 as Unit).collect(),
                }
            }

            unsafe fn compiled(pattern: &[Unit], options: u32) -> *mut c_void {
                let mut error = 0;
                let mut offset = 0;
                let code = unsafe {
                    compile(
                        pattern.as_ptr(),
                        pattern.len(),
                        options,
                        &mut error,
                        &mut offset,
                        ptr::null_mut(),
                    )
                };
                assert!(!code.is_null(), "compile error {error}/{offset}");
                code
            }

            pub fn run() {
                unsafe {
                    for (name, pattern, options) in [
                        (
                            "compile-ascii",
                            r"^(?<user>[a-zA-Z0-9_]+)@([a-z0-9-]+\.)+[a-z]{2,}$",
                            0,
                        ),
                        (
                            "compile-unicode",
                            r"^(?<word>\p{L}+)\s+(?<number>\p{Nd}+)\b",
                            0x80000 | 0x20000,
                        ),
                        (
                            "compile-recursion",
                            r"^(?<pair>\((?:[^()]++|(?&pair))*\))$",
                            0,
                        ),
                    ] {
                        let pattern = units(pattern);
                        measure(&format!("{}/{name}", $bits), 10_000, || {
                            let code = compiled(&pattern, options);
                            code_free(code);
                            1
                        });
                    }
                    for (name, pattern, subject, options, expected) in [
                        (
                            "ascii",
                            r"^(?<user>[a-zA-Z0-9_]+)@([a-z0-9-]+\.)+[a-z]{2,}$",
                            "biliup_user123@videos.example.com",
                            0,
                            3,
                        ),
                        (
                            "unicode",
                            r"^(?<word>\p{L}+)\s+(?<number>\p{Nd}+)$",
                            "汉字视频 １２３４５",
                            0x80000 | 0x20000,
                            3,
                        ),
                        (
                            "lookaround-backref",
                            r"^(?<value>a+)(?=b)b\k<value>$",
                            "aaaaaaaaabaaaaaaaaa",
                            0,
                            2,
                        ),
                        (
                            "recursion",
                            r"^(?<pair>\((?:[^()]++|(?&pair))*\))$",
                            "((ab(cd)(ef))(ghi))",
                            0,
                            2,
                        ),
                    ] {
                        let pattern = units(pattern);
                        let subject = units(subject);
                        let code = compiled(&pattern, options);
                        let data = data_create(code, ptr::null_mut());
                        measure(&format!("{}/interpreter-{name}", $bits), 250_000, || {
                            let result = regex_match(
                                code,
                                subject.as_ptr(),
                                subject.len(),
                                0,
                                0,
                                data,
                                ptr::null_mut(),
                            );
                            assert_eq!(result, expected);
                            result as usize
                        });
                        assert_eq!(jit_compile(code, 1), 0);
                        measure(&format!("{}/jit-{name}", $bits), 1_000_000, || {
                            let result = jit_match(
                                code,
                                subject.as_ptr(),
                                subject.len(),
                                0,
                                0,
                                data,
                                ptr::null_mut(),
                            );
                            assert_eq!(result, expected);
                            result as usize
                        });
                        data_free(data);
                        code_free(code);
                    }
                    let pattern = units(r"\p{L}+|\p{Nd}+");
                    let code = compiled(&pattern, 0x80000 | 0x20000);
                    let subject = units("录播视频１２３４５abc");
                    let data = data_create(code, ptr::null_mut());
                    let mut workspace = [0; 256];
                    measure(&format!("{}/dfa-unicode", $bits), 100_000, || {
                        let result = dfa_match(
                            code,
                            subject.as_ptr(),
                            subject.len(),
                            0,
                            0,
                            data,
                            ptr::null_mut(),
                            workspace.as_mut_ptr(),
                            workspace.len(),
                        );
                        assert_eq!(result, 1);
                        result as usize
                    });
                    data_free(data);
                    code_free(code);

                    let pattern = units(r"(?<word>[a-z]+)");
                    let code = compiled(&pattern, 0);
                    let data = data_create(code, ptr::null_mut());
                    let subject = units("alpha beta gamma delta epsilon");
                    let replacement = units("[${word}]");
                    let mut output = [0 as Unit; 128];
                    measure(&format!("{}/substitute-global", $bits), 75_000, || {
                        let mut length = output.len();
                        let result = substitute(
                            code,
                            subject.as_ptr(),
                            subject.len(),
                            0,
                            0x100,
                            data,
                            ptr::null_mut(),
                            replacement.as_ptr(),
                            replacement.len(),
                            output.as_mut_ptr(),
                            &mut length,
                        );
                        assert_eq!(result, 5);
                        assert_eq!(length, subject.len() + 10);
                        length
                    });
                    let codes = [code as *const c_void];
                    measure(&format!("{}/serialize-roundtrip", $bits), 30_000, || {
                        let mut bytes = ptr::null_mut();
                        let mut size = 0;
                        assert_eq!(
                            encode(codes.as_ptr(), 1, &mut bytes, &mut size, ptr::null_mut()),
                            1
                        );
                        let mut copy = ptr::null_mut();
                        assert_eq!(decode(&mut copy, 1, bytes, ptr::null_mut()), 1);
                        serialized_free(bytes);
                        code_free(copy);
                        size
                    });
                    data_free(data);
                    code_free(code);
                    let glob = units("src/**/test?.rs");
                    measure(&format!("{}/convert-glob", $bits), 75_000, || {
                        let mut output = ptr::null_mut();
                        let mut length = 0;
                        assert_eq!(
                            convert(
                                glob.as_ptr(),
                                glob.len(),
                                0x10,
                                &mut output,
                                &mut length,
                                ptr::null_mut()
                            ),
                            0
                        );
                        converted_free(output);
                        length
                    });
                }
            }
        }
    };
}

width!(width8, "8", u8);
width!(width16, "16", u16);
width!(width32, "32", u32);

fn main() {
    width8::run();
    width16::run();
    width32::run();
}
