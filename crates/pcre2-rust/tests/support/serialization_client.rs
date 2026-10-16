use std::ffi::{c_int, c_void};
use std::path::Path;
use std::{env, fs, ptr};

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
                #[link_name = concat!("pcre2_get_ovector_pointer_", $bits)]
                fn vector(data: *mut c_void) -> *mut usize;
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
                #[link_name = concat!("pcre2_serialize_get_number_of_codes_", $bits)]
                fn count(bytes: *const u8) -> i32;
                #[link_name = concat!("pcre2_serialize_free_", $bits)]
                fn serialized_free(bytes: *mut u8);
            }

            fn units(text: &str) -> Vec<Unit> {
                match $bits {
                    "8" => text.as_bytes().iter().map(|value| *value as Unit).collect(),
                    "16" => text.encode_utf16().map(|value| value as Unit).collect(),
                    _ => text.chars().map(|value| value as u32 as Unit).collect(),
                }
            }

            pub fn run(mode: &str, directory: &Path) {
                let cases = [
                    (
                        r"(?<word>\p{L}+)\s+(\p{Nd}+)",
                        "汉字 １２",
                        0x80000 | 0x20000,
                        3,
                    ),
                    (
                        r"^(?<par>\((?:[^()]++|(?&par))*\))$",
                        "((ab(cd))(ef))",
                        0,
                        2,
                    ),
                    (r"(?<=ab{1,3})c", "abbc", 0, 1),
                    (r"(?<x>a+)\k<x>", "aaaa", 0, 2),
                ];
                let mut codes = [ptr::null_mut(); 4];
                let path = directory.join(concat!("patterns-", $bits, ".bin"));
                unsafe {
                    if mode == "write" {
                        for (index, (pattern, _, options, _)) in cases.iter().enumerate() {
                            let pattern = units(pattern);
                            let mut error = 0;
                            let mut offset = 0;
                            codes[index] = compile(
                                pattern.as_ptr(),
                                pattern.len(),
                                *options,
                                &mut error,
                                &mut offset,
                                ptr::null_mut(),
                            );
                            assert!(!codes[index].is_null(), "{error}/{offset}");
                        }
                        let pointers: Vec<_> =
                            codes.iter().map(|code| *code as *const c_void).collect();
                        let mut bytes = ptr::null_mut();
                        let mut length = 0;
                        assert_eq!(
                            encode(
                                pointers.as_ptr(),
                                4,
                                &mut bytes,
                                &mut length,
                                ptr::null_mut()
                            ),
                            4
                        );
                        fs::write(path, std::slice::from_raw_parts(bytes, length)).unwrap();
                        serialized_free(bytes);
                    } else {
                        assert_eq!(mode, "read");
                        let bytes = fs::read(path).unwrap();
                        assert!(bytes.len() > 64);
                        assert_eq!(count(bytes.as_ptr()), 4);
                        assert_eq!(
                            decode(codes.as_mut_ptr(), 4, bytes.as_ptr(), ptr::null_mut()),
                            4
                        );
                    }
                    for (index, (_, subject, _, expected)) in cases.iter().enumerate() {
                        let subject = units(subject);
                        let data = data_create(codes[index], ptr::null_mut());
                        let result = regex_match(
                            codes[index],
                            subject.as_ptr(),
                            subject.len(),
                            0,
                            0,
                            data,
                            ptr::null_mut(),
                        );
                        assert_eq!(result, *expected);
                        let offsets = std::slice::from_raw_parts(vector(data), 2 * result as usize);
                        println!("{}/{index}: {result} {offsets:?}", $bits);
                        data_free(data);
                        code_free(codes[index]);
                    }
                }
            }
        }
    };
}

width!(width8, "8", u8);
width!(width16, "16", u16);
width!(width32, "32", u32);

fn main() {
    let arguments: Vec<_> = env::args().collect();
    assert_eq!(arguments.len(), 3);
    let directory = Path::new(&arguments[2]);
    fs::create_dir_all(directory).unwrap();
    width8::run(&arguments[1], directory);
    width16::run(&arguments[1], directory);
    width32::run(&arguments[1], directory);
}
