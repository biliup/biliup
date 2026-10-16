//! Exercises the public C ABI directly, without depending on translated internal layouts.
use std::ffi::{c_char, c_int, c_void};
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};

// Referencing the crate ensures that its Rust implementations participate in linking.
use pcre2_rust as _;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(memory: *mut c_void);
}

#[derive(Default)]
struct Allocations {
    allocated: AtomicUsize,
    freed: AtomicUsize,
}

struct FailingAllocations {
    counters: Allocations,
    remaining: AtomicUsize,
}

unsafe extern "C" fn fallible_malloc(size: usize, data: *mut c_void) -> *mut c_void {
    let state = unsafe { &*data.cast::<FailingAllocations>() };
    let mut remaining = state.remaining.load(Ordering::Relaxed);
    loop {
        if remaining == 0 {
            return ptr::null_mut();
        }
        match state.remaining.compare_exchange_weak(
            remaining,
            remaining - 1,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => break,
            Err(value) => remaining = value,
        }
    }
    unsafe { counted_malloc(size, ptr::from_ref(&state.counters).cast_mut().cast()) }
}

unsafe extern "C" fn fallible_free(memory: *mut c_void, data: *mut c_void) {
    let state = unsafe { &*data.cast::<FailingAllocations>() };
    unsafe { counted_free(memory, ptr::from_ref(&state.counters).cast_mut().cast()) };
}

unsafe extern "C" fn counted_malloc(size: usize, data: *mut c_void) -> *mut c_void {
    let result = unsafe { malloc(size) };
    if !result.is_null() {
        unsafe { &*data.cast::<Allocations>() }
            .allocated
            .fetch_add(1, Ordering::Relaxed);
    }
    result
}

unsafe extern "C" fn counted_free(memory: *mut c_void, data: *mut c_void) {
    if !memory.is_null() {
        unsafe { &*data.cast::<Allocations>() }
            .freed
            .fetch_add(1, Ordering::Relaxed);
        unsafe { free(memory) };
    }
}

macro_rules! width_contract {
    ($module:ident, $width:literal, $unit:ty) => {
        mod $module {
            use super::*;
            type Unit = $unit;
            type Handle = *mut c_void;
            type ConstHandle = *const c_void;

            #[repr(C)]
            struct Callout {
                version: u32,
                number: u32,
                capture_top: u32,
                capture_last: u32,
                offset_vector: *mut usize,
                mark: *const Unit,
                subject: *const Unit,
                subject_length: usize,
                start_match: usize,
                current_position: usize,
                pattern_position: usize,
                next_item_length: usize,
                string_offset: usize,
                string_length: usize,
                string: *const Unit,
                flags: u32,
            }

            #[repr(C)]
            struct EnumerateCallout {
                version: u32,
                pattern_position: usize,
                next_item_length: usize,
                number: u32,
                string_offset: usize,
                string_length: usize,
                string: *const Unit,
            }

            #[repr(C)]
            struct SubstituteCallout {
                version: u32,
                input: *const Unit,
                output: *const Unit,
                output_offsets: [usize; 2],
                ovector: *mut usize,
                oveccount: u32,
                subscount: u32,
            }

            // These declarations are the public pcre2.h ABI, independently of
            // the implementation's generated Rust types.
            unsafe extern "C" {
                #[link_name = concat!("pcre2_config_", $width)]
                fn config(what: u32, where_: *mut c_void) -> c_int;
                #[link_name = concat!("pcre2_general_context_create_", $width)]
                fn general_create(
                    allocate: Option<unsafe extern "C" fn(usize, *mut c_void) -> *mut c_void>,
                    release: Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
                    data: *mut c_void,
                ) -> Handle;
                #[link_name = concat!("pcre2_general_context_copy_", $width)]
                fn general_copy(context: Handle) -> Handle;
                #[link_name = concat!("pcre2_general_context_free_", $width)]
                fn general_free(context: Handle);
                #[link_name = concat!("pcre2_compile_context_create_", $width)]
                fn compile_create(context: Handle) -> Handle;
                #[link_name = concat!("pcre2_compile_context_copy_", $width)]
                fn compile_copy(context: Handle) -> Handle;
                #[link_name = concat!("pcre2_compile_context_free_", $width)]
                fn compile_free(context: Handle);
                #[link_name = concat!("pcre2_set_bsr_", $width)]
                fn set_bsr(context: Handle, value: u32) -> c_int;
                #[link_name = concat!("pcre2_set_character_tables_", $width)]
                fn set_tables(context: Handle, value: *const u8) -> c_int;
                #[link_name = concat!("pcre2_set_compile_extra_options_", $width)]
                fn set_extra(context: Handle, value: u32) -> c_int;
                #[link_name = concat!("pcre2_set_max_pattern_length_", $width)]
                fn set_pattern_length(context: Handle, value: usize) -> c_int;
                #[link_name = concat!("pcre2_set_max_pattern_compiled_length_", $width)]
                fn set_compiled_length(context: Handle, value: usize) -> c_int;
                #[link_name = concat!("pcre2_set_max_varlookbehind_", $width)]
                fn set_lookbehind(context: Handle, value: u32) -> c_int;
                #[link_name = concat!("pcre2_set_newline_", $width)]
                fn set_newline(context: Handle, value: u32) -> c_int;
                #[link_name = concat!("pcre2_set_parens_nest_limit_", $width)]
                fn set_nest(context: Handle, value: u32) -> c_int;
                #[link_name = concat!("pcre2_set_compile_recursion_guard_", $width)]
                fn set_guard(
                    context: Handle,
                    guard: Option<unsafe extern "C" fn(u32, *mut c_void) -> c_int>,
                    data: *mut c_void,
                ) -> c_int;
                #[link_name = concat!("pcre2_set_optimize_", $width)]
                fn set_optimize(context: Handle, value: u32) -> c_int;
                #[link_name = concat!("pcre2_match_context_create_", $width)]
                fn match_create(context: Handle) -> Handle;
                #[link_name = concat!("pcre2_match_context_copy_", $width)]
                fn match_copy(context: Handle) -> Handle;
                #[link_name = concat!("pcre2_match_context_free_", $width)]
                fn match_free(context: Handle);
                #[link_name = concat!("pcre2_set_callout_", $width)]
                fn set_callout(
                    context: Handle,
                    callout: Option<unsafe extern "C" fn(*mut Callout, *mut c_void) -> c_int>,
                    data: *mut c_void,
                ) -> c_int;
                #[link_name = concat!("pcre2_set_substitute_callout_", $width)]
                fn set_substitute_callout(
                    context: Handle,
                    callout: Option<
                        unsafe extern "C" fn(*mut SubstituteCallout, *mut c_void) -> c_int,
                    >,
                    data: *mut c_void,
                ) -> c_int;
                #[link_name = concat!("pcre2_set_substitute_case_callout_", $width)]
                fn set_case_callout(
                    context: Handle,
                    callout: Option<
                        unsafe extern "C" fn(
                            *const Unit,
                            usize,
                            *mut Unit,
                            usize,
                            c_int,
                            *mut c_void,
                        ) -> usize,
                    >,
                    data: *mut c_void,
                ) -> c_int;
                #[link_name = concat!("pcre2_set_depth_limit_", $width)]
                fn set_depth(context: Handle, value: u32) -> c_int;
                #[link_name = concat!("pcre2_set_heap_limit_", $width)]
                fn set_heap(context: Handle, value: u32) -> c_int;
                #[link_name = concat!("pcre2_set_match_limit_", $width)]
                fn set_match_limit(context: Handle, value: u32) -> c_int;
                #[link_name = concat!("pcre2_set_offset_limit_", $width)]
                fn set_offset(context: Handle, value: usize) -> c_int;
                #[link_name = concat!("pcre2_set_recursion_limit_", $width)]
                fn set_recursion(context: Handle, value: u32) -> c_int;
                #[link_name = concat!("pcre2_set_recursion_memory_management_", $width)]
                fn set_recursion_memory(
                    context: Handle,
                    allocate: Option<unsafe extern "C" fn(usize, *mut c_void) -> *mut c_void>,
                    release: Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
                    data: *mut c_void,
                ) -> c_int;
                #[link_name = concat!("pcre2_convert_context_create_", $width)]
                fn convert_create(context: Handle) -> Handle;
                #[link_name = concat!("pcre2_convert_context_copy_", $width)]
                fn convert_copy(context: Handle) -> Handle;
                #[link_name = concat!("pcre2_convert_context_free_", $width)]
                fn convert_free(context: Handle);
                #[link_name = concat!("pcre2_set_glob_escape_", $width)]
                fn set_glob_escape(context: Handle, value: u32) -> c_int;
                #[link_name = concat!("pcre2_set_glob_separator_", $width)]
                fn set_glob_separator(context: Handle, value: u32) -> c_int;
                #[link_name = concat!("pcre2_compile_", $width)]
                fn compile(
                    pattern: *const Unit,
                    length: usize,
                    options: u32,
                    error: *mut c_int,
                    offset: *mut usize,
                    context: Handle,
                ) -> Handle;
                #[link_name = concat!("pcre2_code_free_", $width)]
                fn code_free(code: Handle);
                #[link_name = concat!("pcre2_code_copy_", $width)]
                fn code_copy(code: ConstHandle) -> Handle;
                #[link_name = concat!("pcre2_code_copy_with_tables_", $width)]
                fn code_copy_tables(code: ConstHandle) -> Handle;
                #[link_name = concat!("pcre2_pattern_info_", $width)]
                fn pattern_info(code: ConstHandle, what: u32, output: *mut c_void) -> c_int;
                #[link_name = concat!("pcre2_callout_enumerate_", $width)]
                fn enumerate(
                    code: ConstHandle,
                    callback: Option<
                        unsafe extern "C" fn(*mut EnumerateCallout, *mut c_void) -> c_int,
                    >,
                    data: *mut c_void,
                ) -> c_int;
                #[link_name = concat!("pcre2_match_data_create_", $width)]
                fn data_create(count: u32, context: Handle) -> Handle;
                #[link_name = concat!("pcre2_match_data_create_from_pattern_", $width)]
                fn data_from_pattern(code: ConstHandle, context: Handle) -> Handle;
                #[link_name = concat!("pcre2_match_data_free_", $width)]
                fn data_free(data: Handle);
                #[link_name = concat!("pcre2_dfa_match_", $width)]
                fn dfa_match(
                    code: ConstHandle,
                    subject: *const Unit,
                    length: usize,
                    start: usize,
                    options: u32,
                    data: Handle,
                    context: Handle,
                    workspace: *mut c_int,
                    workspace_size: usize,
                ) -> c_int;
                #[link_name = concat!("pcre2_match_", $width)]
                fn regex_match(
                    code: ConstHandle,
                    subject: *const Unit,
                    length: usize,
                    start: usize,
                    options: u32,
                    data: Handle,
                    context: Handle,
                ) -> c_int;
                #[link_name = concat!("pcre2_get_mark_", $width)]
                fn mark(data: Handle) -> *const Unit;
                #[link_name = concat!("pcre2_get_match_data_size_", $width)]
                fn data_size(data: Handle) -> usize;
                #[link_name = concat!("pcre2_get_match_data_heapframes_size_", $width)]
                fn frames_size(data: Handle) -> usize;
                #[link_name = concat!("pcre2_get_ovector_count_", $width)]
                fn vector_count(data: Handle) -> u32;
                #[link_name = concat!("pcre2_get_ovector_pointer_", $width)]
                fn vector(data: Handle) -> *mut usize;
                #[link_name = concat!("pcre2_get_startchar_", $width)]
                fn startchar(data: Handle) -> usize;
                #[link_name = concat!("pcre2_next_match_", $width)]
                fn next_match(data: Handle, start: *mut usize, options: *mut u32) -> c_int;
                #[link_name = concat!("pcre2_substring_copy_byname_", $width)]
                fn substring_copy_name(
                    data: Handle,
                    name: *const Unit,
                    output: *mut Unit,
                    length: *mut usize,
                ) -> c_int;
                #[link_name = concat!("pcre2_substring_copy_bynumber_", $width)]
                fn substring_copy_number(
                    data: Handle,
                    number: u32,
                    output: *mut Unit,
                    length: *mut usize,
                ) -> c_int;
                #[link_name = concat!("pcre2_substring_get_byname_", $width)]
                fn substring_get_name(
                    data: Handle,
                    name: *const Unit,
                    output: *mut *mut Unit,
                    length: *mut usize,
                ) -> c_int;
                #[link_name = concat!("pcre2_substring_get_bynumber_", $width)]
                fn substring_get_number(
                    data: Handle,
                    number: u32,
                    output: *mut *mut Unit,
                    length: *mut usize,
                ) -> c_int;
                #[link_name = concat!("pcre2_substring_free_", $width)]
                fn substring_free(string: *mut Unit);
                #[link_name = concat!("pcre2_substring_length_byname_", $width)]
                fn substring_length_name(
                    data: Handle,
                    name: *const Unit,
                    length: *mut usize,
                ) -> c_int;
                #[link_name = concat!("pcre2_substring_length_bynumber_", $width)]
                fn substring_length_number(data: Handle, number: u32, length: *mut usize) -> c_int;
                #[link_name = concat!("pcre2_substring_nametable_scan_", $width)]
                fn name_scan(
                    code: ConstHandle,
                    name: *const Unit,
                    first: *mut *const Unit,
                    last: *mut *const Unit,
                ) -> c_int;
                #[link_name = concat!("pcre2_substring_number_from_name_", $width)]
                fn number_from_name(code: ConstHandle, name: *const Unit) -> c_int;
                #[link_name = concat!("pcre2_substring_list_get_", $width)]
                fn list_get(
                    data: Handle,
                    strings: *mut *mut *mut Unit,
                    lengths: *mut *mut usize,
                ) -> c_int;
                #[link_name = concat!("pcre2_substring_list_free_", $width)]
                fn list_free(strings: *mut *mut Unit);
                #[link_name = concat!("pcre2_serialize_encode_", $width)]
                fn serialize_encode(
                    codes: *const ConstHandle,
                    count: i32,
                    bytes: *mut *mut u8,
                    size: *mut usize,
                    context: Handle,
                ) -> i32;
                #[link_name = concat!("pcre2_serialize_decode_", $width)]
                fn serialize_decode(
                    codes: *mut Handle,
                    count: i32,
                    bytes: *const u8,
                    context: Handle,
                ) -> i32;
                #[link_name = concat!("pcre2_serialize_get_number_of_codes_", $width)]
                fn serialize_count(bytes: *const u8) -> i32;
                #[link_name = concat!("pcre2_serialize_free_", $width)]
                fn serialize_free(bytes: *mut u8);
                #[link_name = concat!("pcre2_substitute_", $width)]
                fn substitute(
                    code: ConstHandle,
                    subject: *const Unit,
                    length: usize,
                    start: usize,
                    options: u32,
                    data: Handle,
                    context: Handle,
                    replacement: *const Unit,
                    replacement_length: usize,
                    output: *mut Unit,
                    output_length: *mut usize,
                ) -> c_int;
                #[link_name = concat!("pcre2_pattern_convert_", $width)]
                fn pattern_convert(
                    pattern: *const Unit,
                    length: usize,
                    options: u32,
                    output: *mut *mut Unit,
                    output_length: *mut usize,
                    context: Handle,
                ) -> c_int;
                #[link_name = concat!("pcre2_converted_pattern_free_", $width)]
                fn converted_free(pattern: *mut Unit);
                #[link_name = concat!("pcre2_jit_compile_", $width)]
                fn jit_compile(code: Handle, options: u32) -> c_int;
                #[link_name = concat!("pcre2_jit_match_", $width)]
                fn jit_match(
                    code: ConstHandle,
                    subject: *const Unit,
                    length: usize,
                    start: usize,
                    options: u32,
                    data: Handle,
                    context: Handle,
                ) -> c_int;
                #[link_name = concat!("pcre2_jit_free_unused_memory_", $width)]
                fn jit_free_unused(context: Handle);
                #[link_name = concat!("pcre2_jit_stack_create_", $width)]
                fn stack_create(start: usize, maximum: usize, context: Handle) -> Handle;
                #[link_name = concat!("pcre2_jit_stack_assign_", $width)]
                fn stack_assign(
                    context: Handle,
                    callback: Option<unsafe extern "C" fn(*mut c_void) -> Handle>,
                    data: *mut c_void,
                );
                #[link_name = concat!("pcre2_jit_stack_free_", $width)]
                fn stack_free(stack: Handle);
                #[link_name = concat!("pcre2_get_error_message_", $width)]
                fn error_message(code: c_int, output: *mut Unit, length: usize) -> c_int;
                #[link_name = concat!("pcre2_maketables_", $width)]
                fn make_tables(context: Handle) -> *const u8;
                #[link_name = concat!("pcre2_maketables_free_", $width)]
                fn tables_free(context: Handle, tables: *const u8);
            }

            fn text(value: &str) -> Vec<Unit> {
                match $width {
                    "8" => value.as_bytes().iter().map(|v| *v as Unit).collect(),
                    "16" => value.encode_utf16().map(|v| v as Unit).collect(),
                    _ => value.chars().map(|v| v as u32 as Unit).collect(),
                }
            }

            fn name(value: &str) -> Vec<Unit> {
                let mut result = text(value);
                result.push(0);
                result
            }

            unsafe fn compiled(pattern: &str, options: u32, context: Handle) -> Handle {
                let pattern = text(pattern);
                let mut error = 0;
                let mut offset = 0;
                let code = unsafe {
                    compile(
                        pattern.as_ptr(),
                        pattern.len(),
                        options,
                        &mut error,
                        &mut offset,
                        context,
                    )
                };
                assert!(!code.is_null(), "compile error {error} at {offset}");
                code
            }

            unsafe fn capture(data: Handle, number: usize) -> [usize; 2] {
                let offsets = unsafe { vector(data).add(2 * number) };
                [unsafe { *offsets }, unsafe { *offsets.add(1) }]
            }

            unsafe extern "C" fn guard(_: u32, data: *mut c_void) -> c_int {
                unsafe { &*data.cast::<AtomicUsize>() }.fetch_add(1, Ordering::Relaxed);
                0
            }

            unsafe extern "C" fn callout(block: *mut Callout, data: *mut c_void) -> c_int {
                let block = unsafe { &*block };
                // Callback ABI errors are returned rather than unwinding through C.
                if block.number != 7
                    || block.subject_length != 3
                    || block.current_position != 1
                    || block.subject.is_null()
                {
                    return -37;
                }
                unsafe { &*data.cast::<AtomicUsize>() }.fetch_add(1, Ordering::Relaxed);
                0
            }

            unsafe extern "C" fn enum_callout(
                block: *mut EnumerateCallout,
                data: *mut c_void,
            ) -> c_int {
                if unsafe { (*block).number } != 7 {
                    return -37;
                }
                unsafe { &*data.cast::<AtomicUsize>() }.fetch_add(1, Ordering::Relaxed);
                0
            }

            unsafe extern "C" fn subst_callout(
                block: *mut SubstituteCallout,
                data: *mut c_void,
            ) -> c_int {
                let block = unsafe { &*block };
                if block.input.is_null()
                    || block.output.is_null()
                    || block.output_offsets[1] <= block.output_offsets[0]
                    || block.subscount == 0
                {
                    return -37;
                }
                unsafe { &*data.cast::<AtomicUsize>() }.fetch_add(1, Ordering::Relaxed);
                0
            }

            unsafe extern "C" fn case_callout(
                input: *const Unit,
                length: usize,
                output: *mut Unit,
                capacity: usize,
                _: c_int,
                data: *mut c_void,
            ) -> usize {
                unsafe { &*data.cast::<AtomicUsize>() }.fetch_add(1, Ordering::Relaxed);
                if capacity < length {
                    return length;
                }
                for i in 0..length {
                    let unit = unsafe { *input.add(i) };
                    let upper = if unit >= b'a' as Unit && unit <= b'z' as Unit {
                        unit - 32
                    } else {
                        unit
                    };
                    unsafe { *output.add(i) = upper };
                }
                length
            }

            #[test]
            fn contexts_use_callbacks_and_balance_custom_allocations() {
                let allocations = Allocations::default();
                let data = ptr::from_ref(&allocations).cast_mut().cast();
                let guards = AtomicUsize::new(0);
                unsafe {
                    let general = general_create(Some(counted_malloc), Some(counted_free), data);
                    assert!(!general.is_null());
                    let general2 = general_copy(general);
                    assert!(!general2.is_null());
                    let context = compile_create(general);
                    let match_context = match_create(general);
                    let convert_context = convert_create(general);
                    assert!(
                        !context.is_null()
                            && !match_context.is_null()
                            && !convert_context.is_null()
                    );
                    let tables = make_tables(general);
                    assert!(!tables.is_null());
                    assert_eq!(set_tables(context, tables), 0);
                    assert_eq!(set_bsr(context, 2), 0);
                    assert_eq!(set_bsr(context, 99), -29);
                    assert_eq!(set_newline(context, 3), 0);
                    assert_eq!(set_newline(context, 99), -29);
                    assert_eq!(set_extra(context, 0), 0);
                    assert_eq!(set_pattern_length(context, 4096), 0);
                    assert_eq!(set_compiled_length(context, 1_000_000), 0);
                    assert_eq!(set_lookbehind(context, 200), 0);
                    assert_eq!(set_nest(context, 250), 0);
                    assert_eq!(
                        set_guard(
                            context,
                            Some(guard),
                            ptr::from_ref(&guards).cast_mut().cast()
                        ),
                        0
                    );
                    assert_eq!(set_optimize(context, 0), 0);
                    assert_eq!(set_depth(match_context, 10_000), 0);
                    assert_eq!(set_heap(match_context, 10_000), 0);
                    assert_eq!(set_match_limit(match_context, 10_000), 0);
                    assert_eq!(set_offset(match_context, 1_000), 0);
                    assert_eq!(set_recursion(match_context, 10_000), 0);
                    assert_eq!(
                        set_recursion_memory(
                            match_context,
                            Some(counted_malloc),
                            Some(counted_free),
                            data
                        ),
                        0
                    );
                    assert_eq!(set_glob_escape(convert_context, b'\\' as u32), 0);
                    assert_eq!(set_glob_separator(convert_context, b'/' as u32), 0);
                    let context2 = compile_copy(context);
                    let match_context2 = match_copy(match_context);
                    let convert_context2 = convert_copy(convert_context);
                    assert!(
                        !context2.is_null()
                            && !match_context2.is_null()
                            && !convert_context2.is_null()
                    );
                    let code = compiled("((ab)+)", 0x800000, context2);
                    assert!(guards.load(Ordering::Relaxed) > 0);
                    let copied = code_copy(code);
                    let copied_tables = code_copy_tables(code);
                    assert!(!copied.is_null() && !copied_tables.is_null());
                    let subject = text("abab");
                    let match_data = data_from_pattern(copied_tables, general);
                    assert_eq!(
                        regex_match(
                            copied_tables,
                            subject.as_ptr(),
                            subject.len(),
                            0,
                            0,
                            match_data,
                            match_context2
                        ),
                        3
                    );
                    assert_eq!(capture(match_data, 0), [0, 4]);
                    assert!(data_size(match_data) > 6 * std::mem::size_of::<usize>());
                    assert!(frames_size(match_data) > 0);
                    data_free(match_data);
                    code_free(copied_tables);
                    code_free(copied);
                    code_free(code);
                    compile_free(context2);
                    compile_free(context);
                    match_free(match_context2);
                    match_free(match_context);
                    convert_free(convert_context2);
                    convert_free(convert_context);
                    tables_free(general, tables);
                    jit_free_unused(general);
                    general_free(general2);
                    general_free(general);
                }
                assert!(allocations.allocated.load(Ordering::Relaxed) >= 12);
                assert_eq!(
                    allocations.allocated.load(Ordering::Relaxed),
                    allocations.freed.load(Ordering::Relaxed)
                );
            }

            #[test]
            fn named_captures_marks_substrings_and_global_iteration_follow_abi() {
                unsafe {
                    let code = compiled("(?<word>[a-z]+)([0-9]+)?(*MARK:done)", 0, ptr::null_mut());
                    let subject = text("abc def12");
                    let data = data_from_pattern(code, ptr::null_mut());
                    assert_eq!(
                        regex_match(
                            code,
                            subject.as_ptr(),
                            subject.len(),
                            0,
                            0,
                            data,
                            ptr::null_mut()
                        ),
                        2
                    );
                    assert_eq!(vector_count(data), 3);
                    assert_eq!(capture(data, 0), [0, 3]);
                    assert_eq!(capture(data, 1), [0, 3]);
                    assert_eq!(capture(data, 2), [usize::MAX, usize::MAX]);
                    assert_eq!(startchar(data), 0);
                    assert_eq!(std::slice::from_raw_parts(mark(data), 5), &name("done"));
                    let word = name("word");
                    assert_eq!(number_from_name(code, word.as_ptr()), 1);
                    assert_eq!(number_from_name(code, name("missing").as_ptr()), -49);
                    let mut first = ptr::null();
                    let mut last = ptr::null();
                    assert!(name_scan(code, word.as_ptr(), &mut first, &mut last) > 0);
                    assert_eq!(first, last);
                    assert!(!first.is_null());
                    let mut length = 0;
                    assert_eq!(substring_length_name(data, word.as_ptr(), &mut length), 0);
                    assert_eq!(length, 3);
                    assert_eq!(substring_length_number(data, 1, &mut length), 0);
                    assert_eq!(length, 3);
                    assert_eq!(substring_length_number(data, 2, &mut length), -55);
                    let mut output = [0 as Unit; 8];
                    length = output.len();
                    assert_eq!(
                        substring_copy_name(data, word.as_ptr(), output.as_mut_ptr(), &mut length),
                        0
                    );
                    assert_eq!(length, 3);
                    assert_eq!(&output[..4], &name("abc"));
                    length = output.len();
                    assert_eq!(
                        substring_copy_number(data, 1, output.as_mut_ptr(), &mut length),
                        0
                    );
                    assert_eq!(&output[..4], &name("abc"));
                    length = 2;
                    assert_eq!(
                        substring_copy_number(data, 1, output.as_mut_ptr(), &mut length),
                        -48
                    );
                    let mut owned = ptr::null_mut();
                    assert_eq!(
                        substring_get_name(data, word.as_ptr(), &mut owned, &mut length),
                        0
                    );
                    assert_eq!(std::slice::from_raw_parts(owned, length + 1), &name("abc"));
                    substring_free(owned);
                    assert_eq!(substring_get_number(data, 0, &mut owned, &mut length), 0);
                    assert_eq!(std::slice::from_raw_parts(owned, length + 1), &name("abc"));
                    substring_free(owned);
                    let mut strings = ptr::null_mut();
                    let mut lengths = ptr::null_mut();
                    assert_eq!(list_get(data, &mut strings, &mut lengths), 0);
                    assert_eq!(std::slice::from_raw_parts(lengths, 2), &[3, 3]);
                    assert!((*strings.add(2)).is_null());
                    assert_eq!(std::slice::from_raw_parts(*strings, 4), &name("abc"));
                    list_free(strings);
                    let mut start = 0;
                    let mut options = 0;
                    assert_eq!(next_match(data, &mut start, &mut options), 1);
                    assert_eq!(start, 3);
                    assert_eq!(
                        regex_match(
                            code,
                            subject.as_ptr(),
                            subject.len(),
                            start,
                            options,
                            data,
                            ptr::null_mut()
                        ),
                        3
                    );
                    assert_eq!(capture(data, 0), [4, 9]);
                    let mut captures = 0u32;
                    assert_eq!(
                        pattern_info(code, 4, ptr::from_mut(&mut captures).cast()),
                        0
                    );
                    assert_eq!(captures, 2);
                    data_free(data);
                    code_free(code);
                }
            }

            #[test]
            fn callouts_substitutions_dfa_partial_and_jit_are_real_operations() {
                unsafe {
                    let calls = AtomicUsize::new(0);
                    let data_ptr = ptr::from_ref(&calls).cast_mut().cast();
                    let context = match_create(ptr::null_mut());
                    assert_eq!(set_callout(context, Some(callout), data_ptr), 0);
                    let code = compiled("a(?C7)(?<value>bc)", 0, ptr::null_mut());
                    assert_eq!(enumerate(code, Some(enum_callout), data_ptr), 0);
                    assert_eq!(calls.swap(0, Ordering::Relaxed), 1);
                    let data = data_from_pattern(code, ptr::null_mut());
                    let subject = text("abc");
                    assert_eq!(
                        regex_match(code, subject.as_ptr(), subject.len(), 0, 0, data, context),
                        2
                    );
                    assert_eq!(calls.swap(0, Ordering::Relaxed), 1);
                    let mut available = 0u32;
                    assert_eq!(config(1, ptr::from_mut(&mut available).cast()), 0);
                    assert_eq!(available, 1, "complete port includes the JIT engine");
                    assert_eq!(jit_compile(code, 1 | 2 | 4), 0);
                    let stack = stack_create(32_768, 524_288, ptr::null_mut());
                    assert!(!stack.is_null());
                    stack_assign(context, None, stack);
                    assert_eq!(
                        jit_match(code, subject.as_ptr(), subject.len(), 0, 0, data, context),
                        2
                    );
                    assert_eq!(capture(data, 1), [1, 3]);
                    assert_eq!(calls.swap(0, Ordering::Relaxed), 1);
                    assert_eq!(set_callout(context, None, ptr::null_mut()), 0);
                    let incomplete = text("ab");
                    assert_eq!(
                        jit_match(
                            code,
                            incomplete.as_ptr(),
                            incomplete.len(),
                            0,
                            0x20,
                            data,
                            context
                        ),
                        -2
                    );
                    stack_assign(context, None, ptr::null_mut());
                    stack_free(stack);
                    data_free(data);
                    code_free(code);

                    let code = compiled("abc|ab|a", 0, ptr::null_mut());
                    let data = data_create(10, ptr::null_mut());
                    let mut workspace = [0; 128];
                    assert_eq!(
                        dfa_match(
                            code,
                            subject.as_ptr(),
                            subject.len(),
                            0,
                            0,
                            data,
                            context,
                            workspace.as_mut_ptr(),
                            workspace.len()
                        ),
                        3
                    );
                    assert_eq!(capture(data, 0), [0, 3]);
                    assert_eq!(capture(data, 1), [0, 2]);
                    assert_eq!(capture(data, 2), [0, 1]);
                    code_free(code);

                    let code = compiled("(?<word>[a-z]+)", 0, ptr::null_mut());
                    let input = text("abc def");
                    let replacement = text("${word}!");
                    let mut output = [0 as Unit; 64];
                    let mut length = output.len();
                    assert_eq!(
                        set_substitute_callout(context, Some(subst_callout), data_ptr),
                        0
                    );
                    assert_eq!(
                        substitute(
                            code,
                            input.as_ptr(),
                            input.len(),
                            0,
                            0x100,
                            data,
                            context,
                            replacement.as_ptr(),
                            replacement.len(),
                            output.as_mut_ptr(),
                            &mut length
                        ),
                        2
                    );
                    assert_eq!(&output[..length], &text("abc! def!"));
                    assert_eq!(calls.swap(0, Ordering::Relaxed), 2);
                    assert_eq!(set_substitute_callout(context, None, ptr::null_mut()), 0);
                    assert_eq!(set_case_callout(context, Some(case_callout), data_ptr), 0);
                    let replacement = text("\\U$0\\E");
                    length = output.len();
                    assert_eq!(
                        substitute(
                            code,
                            input.as_ptr(),
                            input.len(),
                            0,
                            0x100 | 0x200,
                            data,
                            context,
                            replacement.as_ptr(),
                            replacement.len(),
                            output.as_mut_ptr(),
                            &mut length
                        ),
                        2
                    );
                    assert_eq!(&output[..length], &text("ABC DEF"));
                    assert!(calls.load(Ordering::Relaxed) >= 2);
                    assert_eq!(set_case_callout(context, None, ptr::null_mut()), 0);
                    data_free(data);
                    code_free(code);
                    match_free(context);
                    jit_free_unused(ptr::null_mut());
                }
            }

            #[test]
            fn serialization_conversion_and_utf_ucp_preserve_functionality() {
                unsafe {
                    let code = compiled(
                        "(?<word>\\p{L}+)\\s+(\\p{Nd}+)",
                        0x80000 | 0x20000,
                        ptr::null_mut(),
                    );
                    let subject = text("汉字 １２");
                    let mut bytes = ptr::null_mut();
                    let mut size = 0;
                    let codes = [code as ConstHandle];
                    assert_eq!(
                        serialize_encode(codes.as_ptr(), 1, &mut bytes, &mut size, ptr::null_mut()),
                        1
                    );
                    assert!(size > 64 && !bytes.is_null());
                    assert_eq!(serialize_count(bytes), 1);
                    let mut decoded = ptr::null_mut();
                    assert_eq!(serialize_decode(&mut decoded, 1, bytes, ptr::null_mut()), 1);
                    serialize_free(bytes);
                    code_free(code);
                    let data = data_from_pattern(decoded, ptr::null_mut());
                    assert_eq!(
                        regex_match(
                            decoded,
                            subject.as_ptr(),
                            subject.len(),
                            0,
                            0,
                            data,
                            ptr::null_mut()
                        ),
                        3
                    );
                    assert_eq!(capture(data, 1), [0, text("汉字").len()]);
                    let mut unicode = 0u32;
                    assert_eq!(config(9, ptr::from_mut(&mut unicode).cast()), 0);
                    assert_eq!(unicode, 1);
                    let mut version = [0 as Unit; 64];
                    assert!(config(11, version.as_mut_ptr().cast()) > 0);
                    assert_eq!(&version[..5], &text("10.49"));
                    assert!(error_message(-1, version.as_mut_ptr(), version.len()) > 0);
                    assert_eq!(
                        error_message(-9999, version.as_mut_ptr(), version.len()),
                        -29
                    );
                    data_free(data);
                    code_free(decoded);

                    let context = convert_create(ptr::null_mut());
                    assert_eq!(set_glob_separator(context, b'/' as u32), 0);
                    assert_eq!(set_glob_escape(context, b'\\' as u32), 0);
                    let glob = text("src/**/test?.rs");
                    let mut pattern = ptr::null_mut();
                    let mut length = 0;
                    assert_eq!(
                        pattern_convert(
                            glob.as_ptr(),
                            glob.len(),
                            0x10,
                            &mut pattern,
                            &mut length,
                            context
                        ),
                        0
                    );
                    assert!(!pattern.is_null() && length > glob.len());
                    let mut error = 0;
                    let mut offset = 0;
                    let code =
                        compile(pattern, length, 0, &mut error, &mut offset, ptr::null_mut());
                    assert!(!code.is_null(), "converted pattern failed {error}/{offset}");
                    converted_free(pattern);
                    let data = data_from_pattern(code, ptr::null_mut());
                    let matching = text("src/deep/nested/test1.rs");
                    assert_eq!(
                        regex_match(
                            code,
                            matching.as_ptr(),
                            matching.len(),
                            0,
                            0,
                            data,
                            ptr::null_mut()
                        ),
                        1
                    );
                    let nonmatching = text("src/deep/nested/test12.rs");
                    assert_eq!(
                        regex_match(
                            code,
                            nonmatching.as_ptr(),
                            nonmatching.len(),
                            0,
                            0,
                            data,
                            ptr::null_mut()
                        ),
                        -1
                    );
                    data_free(data);
                    code_free(code);
                    convert_free(context);
                }
            }

            #[test]
            fn compile_and_match_limits_produce_original_errors() {
                unsafe {
                    let context = compile_create(ptr::null_mut());
                    let pattern = text("abcdef");
                    let mut error = 0;
                    let mut offset = 0;
                    assert_eq!(set_pattern_length(context, 3), 0);
                    assert!(compile(
                        pattern.as_ptr(),
                        pattern.len(),
                        0,
                        &mut error,
                        &mut offset,
                        context
                    )
                    .is_null());
                    assert_eq!(error, 188);
                    assert_eq!(set_pattern_length(context, usize::MAX), 0);
                    assert_eq!(set_compiled_length(context, 1), 0);
                    assert!(compile(
                        pattern.as_ptr(),
                        pattern.len(),
                        0,
                        &mut error,
                        &mut offset,
                        context
                    )
                    .is_null());
                    assert_eq!(error, 201);
                    compile_free(context);
                    let code = compiled(
                        "(*NO_AUTO_POSSESS)(*NO_START_OPT)(a+)+$",
                        0,
                        ptr::null_mut(),
                    );
                    let subject = text("aaaaaaaaaaaaaaaaaaaaaaaaaaaab");
                    let data = data_from_pattern(code, ptr::null_mut());
                    let context = match_create(ptr::null_mut());
                    assert_eq!(set_match_limit(context, 10), 0);
                    assert_eq!(
                        regex_match(code, subject.as_ptr(), subject.len(), 0, 0, data, context),
                        -47
                    );
                    data_free(data);
                    code_free(code);
                    match_free(context);
                }
            }

            #[test]
            fn jit_allocator_failure_is_reported_and_resources_can_be_reused() {
                let allocations = FailingAllocations {
                    counters: Allocations::default(),
                    remaining: AtomicUsize::new(0),
                };
                let state = ptr::from_ref(&allocations).cast_mut().cast();
                unsafe {
                    assert!(
                        general_create(Some(fallible_malloc), Some(fallible_free), state).is_null()
                    );
                    allocations.remaining.store(10_000, Ordering::Relaxed);
                    let general = general_create(Some(fallible_malloc), Some(fallible_free), state);
                    assert!(!general.is_null());
                    let context = compile_create(general);
                    let code = compiled("(?<word>[a-z]+)[0-9]+", 0, context);
                    allocations.remaining.store(0, Ordering::Relaxed);
                    assert!(stack_create(32_768, 524_288, general).is_null());
                    assert_eq!(jit_compile(code, 1), -48);
                    allocations.remaining.store(10_000, Ordering::Relaxed);
                    assert_eq!(jit_compile(code, 1), 0);
                    let data = data_from_pattern(code, general);
                    let subject = text("abc12");
                    assert_eq!(
                        jit_match(
                            code,
                            subject.as_ptr(),
                            subject.len(),
                            0,
                            0,
                            data,
                            ptr::null_mut()
                        ),
                        2
                    );
                    assert_eq!(capture(data, 1), [0, 3]);
                    data_free(data);
                    code_free(code);
                    compile_free(context);
                    jit_free_unused(general);
                    general_free(general);
                }
                assert_eq!(
                    allocations.counters.allocated.load(Ordering::Relaxed),
                    allocations.counters.freed.load(Ordering::Relaxed)
                );
            }

            #[test]
            fn binary_code_units_are_supported_and_invalid_utf_has_precise_errors() {
                let (invalid, error): (Vec<Unit>, c_int) = match $width {
                    "8" => (vec![0xffu32 as Unit], -23),
                    "16" => (vec![0xd800u32 as Unit], -24),
                    _ => (vec![0x110000u32 as Unit], -28),
                };
                unsafe {
                    let code = compiled(".", 0, ptr::null_mut());
                    let data = data_from_pattern(code, ptr::null_mut());
                    assert_eq!(
                        regex_match(
                            code,
                            invalid.as_ptr(),
                            invalid.len(),
                            0,
                            0,
                            data,
                            ptr::null_mut()
                        ),
                        1
                    );
                    code_free(code);
                    let code = compiled(".", 0x80000, ptr::null_mut());
                    assert_eq!(
                        regex_match(
                            code,
                            invalid.as_ptr(),
                            invalid.len(),
                            0,
                            0,
                            data,
                            ptr::null_mut()
                        ),
                        error
                    );
                    data_free(data);
                    code_free(code);
                }
            }

            #[test]
            fn global_empty_matches_terminate_and_respect_unicode_boundaries() {
                unsafe {
                    let code = compiled("(?:|a)", 0, ptr::null_mut());
                    let data = data_from_pattern(code, ptr::null_mut());
                    let subject = text("aa");
                    let mut start = 0;
                    let mut options = 0;
                    for (index, expected) in
                        [[0, 0], [0, 1], [1, 1], [1, 2], [2, 2]].iter().enumerate()
                    {
                        assert_eq!(
                            regex_match(
                                code,
                                subject.as_ptr(),
                                subject.len(),
                                start,
                                options,
                                data,
                                ptr::null_mut()
                            ),
                            1
                        );
                        assert_eq!(capture(data, 0), *expected);
                        assert_eq!(
                            next_match(data, &mut start, &mut options),
                            if index == 4 { 0 } else { 1 }
                        );
                    }
                    data_free(data);
                    code_free(code);

                    let code = compiled("", 0x80000, ptr::null_mut());
                    let data = data_from_pattern(code, ptr::null_mut());
                    let subject = text("汉🙂");
                    start = 0;
                    options = 0;
                    for (index, position) in [0, text("汉").len(), subject.len()].iter().enumerate()
                    {
                        assert_eq!(
                            regex_match(
                                code,
                                subject.as_ptr(),
                                subject.len(),
                                start,
                                options,
                                data,
                                ptr::null_mut()
                            ),
                            1
                        );
                        assert_eq!(capture(data, 0), [*position, *position]);
                        assert_eq!(
                            next_match(data, &mut start, &mut options),
                            if index == 2 { 0 } else { 1 }
                        );
                    }
                    data_free(data);
                    code_free(code);
                }
            }

            #[test]
            fn concurrent_jit_compile_match_and_free_use_independent_resources() {
                let ready = std::sync::Arc::new(std::sync::Barrier::new(8));
                let threads: Vec<_> = (0..8)
                    .map(|_| {
                        let ready = std::sync::Arc::clone(&ready);
                        std::thread::spawn(move || {
                            for iteration in 0..64 {
                                unsafe {
                                    let code = compiled(
                                        "^(?<value>a+)(?=b)b\\k<value>$",
                                        0,
                                        ptr::null_mut(),
                                    );
                                    if iteration == 0 {
                                        // Running this test alone also exercises concurrent
                                        // first use of the backend's shared CPU tables.
                                        ready.wait();
                                    }
                                    assert_eq!(jit_compile(code, 1), 0);
                                    let data = data_from_pattern(code, ptr::null_mut());
                                    let subject = text("aaabaaa");
                                    assert_eq!(
                                        jit_match(
                                            code,
                                            subject.as_ptr(),
                                            subject.len(),
                                            0,
                                            0,
                                            data,
                                            ptr::null_mut()
                                        ),
                                        2
                                    );
                                    assert_eq!(capture(data, 1), [0, 3]);
                                    data_free(data);
                                    code_free(code);
                                    jit_free_unused(ptr::null_mut());
                                }
                            }
                        })
                    })
                    .collect();
                for thread in threads {
                    thread.join().unwrap();
                }
            }
        }
    };
}

width_contract!(width8, "8", u8);
width_contract!(width16, "16", u16);
width_contract!(width32, "32", u32);

#[repr(C)]
struct PosixRegex {
    code: *mut c_void,
    match_data: *mut c_void,
    end: *const c_char,
    captures: usize,
    error_offset: usize,
    flags: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
struct PosixMatch {
    start: c_int,
    end: c_int,
}

unsafe extern "C" {
    fn pcre2_regcomp(regex: *mut PosixRegex, pattern: *const c_char, flags: c_int) -> c_int;
    fn pcre2_regexec(
        regex: *const PosixRegex,
        subject: *const c_char,
        count: usize,
        matches: *mut PosixMatch,
        flags: c_int,
    ) -> c_int;
    fn pcre2_regerror(
        error: c_int,
        regex: *const PosixRegex,
        output: *mut c_char,
        size: usize,
    ) -> usize;
    fn pcre2_regfree(regex: *mut PosixRegex);
}

#[test]
fn posix_abi_reports_captures_ranges_and_pattern_errors() {
    unsafe {
        let mut regex: PosixRegex = std::mem::zeroed();
        assert_eq!(
            pcre2_regcomp(&mut regex, c"([a-z]+)([0-9]+)".as_ptr(), 1),
            0
        );
        assert_eq!(regex.captures, 2);
        let mut matches = [PosixMatch { start: -1, end: -1 }; 3];
        assert_eq!(
            pcre2_regexec(&regex, c"--AbC12--".as_ptr(), 3, matches.as_mut_ptr(), 0),
            0
        );
        assert_eq!(
            matches,
            [
                PosixMatch { start: 2, end: 7 },
                PosixMatch { start: 2, end: 5 },
                PosixMatch { start: 5, end: 7 }
            ]
        );
        matches[0] = PosixMatch { start: 0, end: 4 };
        assert_eq!(
            pcre2_regexec(&regex, c"--AbC12--".as_ptr(), 3, matches.as_mut_ptr(), 0x80),
            17
        );
        pcre2_regfree(&mut regex);
        let error = pcre2_regcomp(&mut regex, c"(".as_ptr(), 0);
        assert_eq!(error, 11);
        let mut message = [0 as c_char; 128];
        assert!(pcre2_regerror(error, &regex, message.as_mut_ptr(), message.len()) > 1);
        assert_eq!(
            std::ffi::CStr::from_ptr(message.as_ptr()).to_str().unwrap(),
            "unbalanced () at offset 1"
        );
    }
}
