//! Process readings the Startup Lab reports about itself.
//!
//! Only measurements the host can take about its own process are exposed. This
//! is diagnostics, not a capability: nothing here reads another process, the
//! filesystem, or any user data.

use std::mem;

use super::Dword;

#[repr(C)]
#[derive(Default)]
struct ProcessMemoryCounters {
    cb: Dword,
    page_fault_count: Dword,
    peak_working_set_size: usize,
    working_set_size: usize,
    quota_peak_paged_pool_usage: usize,
    quota_paged_pool_usage: usize,
    quota_peak_non_paged_pool_usage: usize,
    quota_non_paged_pool_usage: usize,
    pagefile_usage: usize,
    peak_pagefile_usage: usize,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetCurrentProcess() -> isize;
    fn K32GetProcessMemoryInfo(
        process: isize,
        counters: *mut ProcessMemoryCounters,
        size: Dword,
    ) -> i32;
}

/// Returns this process's working set in bytes, or `0` when unavailable.
///
/// A failed query reports zero rather than propagating: a diagnostic reading
/// must never be able to stop the surface from opening.
pub(super) fn working_set_bytes() -> u64 {
    let mut counters = ProcessMemoryCounters {
        cb: mem::size_of::<ProcessMemoryCounters>() as Dword,
        ..ProcessMemoryCounters::default()
    };
    // SAFETY: GetCurrentProcess returns a pseudo-handle for this process, and
    // `counters` is writable storage whose declared size matches the struct.
    let queried = unsafe {
        K32GetProcessMemoryInfo(
            GetCurrentProcess(),
            &mut counters,
            mem::size_of::<ProcessMemoryCounters>() as Dword,
        )
    };
    if queried == 0 {
        0
    } else {
        counters.working_set_size as u64
    }
}

#[cfg(test)]
mod tests {
    use super::working_set_bytes;

    #[test]
    fn the_working_set_is_a_plausible_size() {
        let bytes = working_set_bytes();
        assert!(bytes > 256 * 1024, "implausibly small working set: {bytes}");
        assert!(bytes < 8 * 1024 * 1024 * 1024, "implausibly large: {bytes}");
    }
}
