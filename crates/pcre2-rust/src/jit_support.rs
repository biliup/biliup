//! Platform services shared by the Rust SLJIT backends.
//!
//! The regular expression and instruction generation algorithms live in the
//! width-specific modules. Only operating system services are called here.

use core::ffi::c_void;
use core::ptr;
use std::sync::{Mutex, MutexGuard, OnceLock};

static EXECUTABLE_ALLOCATOR_LOCK: Mutex<()> = Mutex::new(());

pub(crate) fn allocator_lock() -> MutexGuard<'static, ()> {
    EXECUTABLE_ALLOCATOR_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub(crate) fn page_alignment() -> usize {
    static PAGE_ALIGNMENT: OnceLock<usize> = OnceLock::new();
    *PAGE_ALIGNMENT.get_or_init(|| {
        #[cfg(unix)]
        let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as isize;
        #[cfg(windows)]
        let page_size = unsafe {
            let mut info = core::mem::zeroed::<windows::SystemInfo>();
            windows::GetSystemInfo(&mut info);
            info.page_size as isize
        };
        if page_size > 0 {
            page_size as usize - 1
        } else {
            4095
        }
    })
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub(crate) unsafe fn cpu_id(info: *mut u32) {
    #[cfg(target_arch = "x86")]
    use core::arch::x86::__cpuid_count;
    #[cfg(target_arch = "x86_64")]
    use core::arch::x86_64::__cpuid_count;

    let result = __cpuid_count(*info, *info.add(2));
    *info = result.eax;
    *info.add(1) = result.ebx;
    *info.add(2) = result.ecx;
    *info.add(3) = result.edx;
}

/// The backend checks CPUID's OSXSAVE bit before calling this function.
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "xsave")]
pub(crate) unsafe fn xcr0_low() -> u32 {
    #[cfg(target_arch = "x86")]
    use core::arch::x86::_xgetbv;
    #[cfg(target_arch = "x86_64")]
    use core::arch::x86_64::_xgetbv;

    _xgetbv(0) as u32
}

#[cfg(all(target_vendor = "apple", target_arch = "x86_64"))]
unsafe fn apple_map_jit_flag() -> libc::c_int {
    static MAP_JIT_FLAG: OnceLock<libc::c_int> = OnceLock::new();
    *MAP_JIT_FLAG.get_or_init(|| {
        // Intel macOS only needs MAP_JIT under the hardened runtime. Preserve
        // support for older kernels which do not accept that mmap flag.
        let mut name = core::mem::zeroed::<libc::utsname>();
        if libc::uname(&mut name) != 0 {
            return 0;
        }
        let mut major = 0u32;
        for digit in name.release {
            if !(b'0'..=b'9').contains(&(digit as u8)) {
                break;
            }
            major = major.saturating_mul(10) + (digit as u8 - b'0') as u32;
        }
        if major < 18 {
            return 0;
        }
        let size = page_alignment() + 1;
        let probe = libc::mmap(
            ptr::null_mut(),
            size,
            libc::PROT_WRITE | libc::PROT_EXEC,
            libc::MAP_PRIVATE | libc::MAP_ANON,
            -1,
            0,
        );
        if probe == libc::MAP_FAILED {
            libc::MAP_JIT
        } else {
            libc::munmap(probe, size);
            0
        }
    })
}

/// Change the current thread's access to MAP_JIT mappings on Apple Silicon.
pub(crate) unsafe fn write_protect(enable_exec: bool) {
    #[cfg(all(target_vendor = "apple", target_arch = "aarch64"))]
    libc::pthread_jit_write_protect_np(i32::from(enable_exec));
    #[cfg(not(all(target_vendor = "apple", target_arch = "aarch64")))]
    let _ = enable_exec;
}

pub(crate) unsafe fn allocate_executable(size: usize) -> *mut c_void {
    #[cfg(unix)]
    {
        let flags = libc::MAP_PRIVATE | libc::MAP_ANON;
        #[cfg(all(target_vendor = "apple", target_arch = "x86_64"))]
        let flags = flags | apple_map_jit_flag();
        #[cfg(all(target_vendor = "apple", target_arch = "aarch64"))]
        let flags = flags | libc::MAP_JIT;

        let result = libc::mmap(
            ptr::null_mut(),
            size,
            libc::PROT_READ | libc::PROT_WRITE | libc::PROT_EXEC,
            flags,
            -1,
            0,
        );
        if result == libc::MAP_FAILED {
            ptr::null_mut()
        } else {
            write_protect(false);
            result
        }
    }
    #[cfg(windows)]
    {
        windows::VirtualAlloc(
            ptr::null_mut(),
            size,
            windows::MEM_RESERVE | windows::MEM_COMMIT,
            windows::PAGE_EXECUTE_READWRITE,
        )
    }
}

pub(crate) unsafe fn free_executable(address: *mut c_void, size: usize) {
    #[cfg(unix)]
    libc::munmap(address, size);
    #[cfg(windows)]
    {
        let _ = size;
        windows::VirtualFree(address, 0, windows::MEM_RELEASE);
    }
}

/// Reserve the virtual address range for a backtracking stack. Windows commits
/// its pages as the stack grows; POSIX anonymous pages are committed on demand.
pub(crate) unsafe fn reserve_stack(size: usize) -> *mut u8 {
    #[cfg(unix)]
    {
        let result = libc::mmap(
            ptr::null_mut(),
            size,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANON,
            -1,
            0,
        );
        if result == libc::MAP_FAILED {
            ptr::null_mut()
        } else {
            result.cast()
        }
    }
    #[cfg(windows)]
    {
        windows::VirtualAlloc(
            ptr::null_mut(),
            size,
            windows::MEM_RESERVE,
            windows::PAGE_READWRITE,
        )
        .cast()
    }
}

pub(crate) unsafe fn release_stack(address: *mut u8, size: usize) {
    #[cfg(unix)]
    libc::munmap(address.cast(), size);
    #[cfg(windows)]
    {
        let _ = size;
        windows::VirtualFree(address.cast(), 0, windows::MEM_RELEASE);
    }
}

pub(crate) unsafe fn resize_stack(old_start: *mut u8, new_start: *mut u8) -> bool {
    let alignment = page_alignment();
    let old_page = old_start as usize & !alignment;
    let new_page = new_start as usize & !alignment;

    #[cfg(windows)]
    {
        if new_page < old_page {
            return !windows::VirtualAlloc(
                new_page as *mut c_void,
                old_page - new_page,
                windows::MEM_COMMIT,
                windows::PAGE_READWRITE,
            )
            .is_null();
        }
        if new_page > old_page {
            return windows::VirtualFree(
                old_page as *mut c_void,
                new_page - old_page,
                windows::MEM_DECOMMIT,
            ) != 0;
        }
    }
    #[cfg(unix)]
    if new_page > old_page {
        // Discard only complete pages no longer belonging to the stack. The
        // mapping remains valid if either advisory call is unsupported.
        #[cfg(not(target_os = "android"))]
        libc::posix_madvise(
            old_page as *mut c_void,
            new_page - old_page,
            libc::POSIX_MADV_DONTNEED,
        );
        #[cfg(any(target_os = "linux", target_os = "android", target_vendor = "apple"))]
        libc::madvise(
            old_page as *mut c_void,
            new_page - old_page,
            libc::MADV_FREE,
        );
    }
    true
}

pub(crate) unsafe fn flush_instruction_cache(start: *mut u8, end: *mut u8) {
    #[cfg(windows)]
    {
        windows::FlushInstructionCache(
            windows::GetCurrentProcess(),
            start.cast(),
            end.offset_from(start) as usize,
        );
    }
    #[cfg(all(target_vendor = "apple", not(windows)))]
    {
        unsafe extern "C" {
            fn sys_icache_invalidate(address: *mut c_void, size: usize);
        }
        sys_icache_invalidate(start.cast(), end.offset_from(start) as usize);
    }
    #[cfg(all(target_arch = "aarch64", not(target_vendor = "apple"), not(windows)))]
    {
        use core::arch::asm;
        let cache_type: usize;
        asm!("mrs {ctr}, ctr_el0", ctr = out(reg) cache_type, options(nomem, nostack));
        if cache_type & (1 << 28) == 0 {
            let line = 4 << ((cache_type >> 16) & 15);
            let mut address = start as usize & !(line - 1);
            while address < end as usize {
                asm!("dc cvau, {address}", address = in(reg) address, options(nostack));
                address += line;
            }
        }
        asm!("dsb ish", options(nostack));
        if cache_type & (1 << 29) == 0 {
            let line = 4 << (cache_type & 15);
            let mut address = start as usize & !(line - 1);
            while address < end as usize {
                asm!("ic ivau, {address}", address = in(reg) address, options(nostack));
                address += line;
            }
            asm!("dsb ish", options(nostack));
        }
        asm!("isb", options(nostack));
    }
    #[cfg(all(target_arch = "arm", any(target_os = "linux", target_os = "android")))]
    {
        const ARM_NR_CACHEFLUSH: libc::c_long = 0x0f0002;
        libc::syscall(ARM_NR_CACHEFLUSH, start, end, 0);
    }
    #[cfg(all(
        any(target_arch = "x86", target_arch = "x86_64", target_arch = "s390x"),
        not(target_vendor = "apple"),
        not(windows)
    ))]
    {
        let _ = (start, end);
        std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
    }
    #[cfg(not(any(
        windows,
        target_vendor = "apple",
        target_arch = "aarch64",
        target_arch = "x86",
        target_arch = "x86_64",
        target_arch = "s390x",
        all(target_arch = "arm", any(target_os = "linux", target_os = "android"))
    )))]
    {
        unsafe extern "C" {
            fn __clear_cache(start: *mut u8, end: *mut u8);
        }
        __clear_cache(start, end);
    }
}

#[cfg(windows)]
mod windows {
    use super::c_void;

    pub(super) const MEM_COMMIT: u32 = 0x1000;
    pub(super) const MEM_RESERVE: u32 = 0x2000;
    pub(super) const MEM_DECOMMIT: u32 = 0x4000;
    pub(super) const MEM_RELEASE: u32 = 0x8000;
    pub(super) const PAGE_READWRITE: u32 = 0x04;
    pub(super) const PAGE_EXECUTE_READWRITE: u32 = 0x40;

    #[repr(C)]
    pub(super) struct SystemInfo {
        architecture_and_reserved: u32,
        pub(super) page_size: u32,
        minimum_address: *mut c_void,
        maximum_address: *mut c_void,
        active_processor_mask: usize,
        number_of_processors: u32,
        processor_type: u32,
        allocation_granularity: u32,
        processor_level: u16,
        processor_revision: u16,
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        pub(super) fn GetSystemInfo(info: *mut SystemInfo);
        pub(super) fn VirtualAlloc(
            address: *mut c_void,
            size: usize,
            allocation_type: u32,
            protection: u32,
        ) -> *mut c_void;
        pub(super) fn VirtualFree(address: *mut c_void, size: usize, free_type: u32) -> i32;
        pub(super) fn FlushInstructionCache(
            process: *mut c_void,
            address: *const c_void,
            size: usize,
        ) -> i32;
        pub(super) fn GetCurrentProcess() -> *mut c_void;
    }
}
