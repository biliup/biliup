// SLJIT emits machine code and must use the calling convention of its target.
#[cfg(all(target_arch = "x86_64", not(windows)))]
#[path = "pcre2_jit_compile_x86_64.rs"]
mod backend;
#[cfg(all(target_arch = "x86_64", windows))]
#[path = "pcre2_jit_compile_win64.rs"]
mod backend;
#[cfg(target_arch = "aarch64")]
#[path = "pcre2_jit_compile_aarch64.rs"]
mod backend;
#[cfg(target_arch = "arm")]
#[path = "pcre2_jit_compile_armv6.rs"]
mod backend;

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64", target_arch = "arm")))]
compile_error!("The Rust PCRE2 JIT backend has not been ported for this architecture");

pub use backend::*;
